use crate::backends;
use crate::config::{Config, RagConfig, RagMode};
use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook_auto, Reader};
use rusqlite::{params, Connection};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct RetrievedDocument {
    pub file_path: String,
    pub chunk_text: String,
    pub score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagRuntimeOverride {
    Default,
    ForceOn,
    ForceOff,
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub indexed_files: usize,
    pub indexed_chunks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    Keyword,
    Vector,
    Hybrid,
}

#[derive(Debug, Clone)]
pub struct RagSystem {
    config: RagConfig,
}

impl RagSystem {
    pub fn new(config: RagConfig) -> Self {
        Self { config }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn parse_prompt_override(prompt: &str) -> (String, RagRuntimeOverride) {
        let trimmed = prompt.trim_start();
        if let Some(rest) = trimmed.strip_prefix("!rag off") {
            return (rest.trim_start().to_string(), RagRuntimeOverride::ForceOff);
        }
        if let Some(rest) = trimmed.strip_prefix("!rag on") {
            return (rest.trim_start().to_string(), RagRuntimeOverride::ForceOn);
        }
        if let Some(rest) = trimmed.strip_prefix("!rag") {
            return (rest.trim_start().to_string(), RagRuntimeOverride::ForceOn);
        }

        (prompt.to_string(), RagRuntimeOverride::Default)
    }

    pub async fn index_documents(&self, backend: &str, config: &Config) -> Result<IndexStats> {
        self.index_documents_with_mode(
            backend,
            config,
            retrieval_mode_from_config(self.config.mode),
        )
        .await
    }

    pub async fn index_documents_with_mode(
        &self,
        backend: &str,
        config: &Config,
        mode: RetrievalMode,
    ) -> Result<IndexStats> {
        let docs_folder = self
            .config
            .documents_folder
            .clone()
            .ok_or_else(|| anyhow!("RAG documents_folder is not configured"))?;
        let docs_folder = normalize_path(&docs_folder);
        if !docs_folder.exists() {
            return Err(anyhow!(
                "RAG documents folder does not exist: {}",
                docs_folder.display()
            ));
        }

        let db_path = normalize_path(&self.config.vector_db_path);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for vector DB at {}",
                    db_path.display()
                )
            })?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open vector DB at {}", db_path.display()))?;
        ensure_schema(&conn)?;
        let retrieval_scope = retrieval_scope_key(backend, config);
        clear_backend_entries(&conn, &retrieval_scope)?;

        let mut indexed_files = 0usize;
        let mut indexed_chunks = 0usize;
        let store_embeddings = matches!(mode, RetrievalMode::Vector | RetrievalMode::Hybrid);
        let store_keyword_index = matches!(mode, RetrievalMode::Keyword | RetrievalMode::Hybrid);

        for entry in WalkDir::new(&docs_folder)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.path();
            if !is_supported(path) {
                continue;
            }
            let text = match extract_text(path) {
                Ok(text) => text,
                Err(err) => {
                    log::warn!("Skipping {}: {err:#}", path.display());
                    continue;
                }
            };
            let chunks = chunk_text(&text, self.config.chunk_size);
            if chunks.is_empty() {
                continue;
            }

            indexed_files += 1;
            for (chunk_index, chunk) in chunks.iter().enumerate() {
                let embedding = if store_embeddings {
                    Some(
                        backends::embed_text(backend, chunk, config)
                            .await
                            .map_err(|err| anyhow!("Embedding generation failed: {err}"))?,
                    )
                } else {
                    None
                };
                persist_chunk(
                    &conn,
                    &retrieval_scope,
                    path,
                    chunk_index,
                    chunk,
                    embedding.as_deref(),
                    self.config.chunk_size,
                    store_keyword_index,
                )?;
                indexed_chunks += 1;
            }
        }

        Ok(IndexStats {
            indexed_files,
            indexed_chunks,
        })
    }

    pub async fn retrieve(
        &self,
        backend: &str,
        query: &str,
        config: &Config,
    ) -> Result<Vec<RetrievedDocument>> {
        self.retrieve_with_mode(
            backend,
            query,
            config,
            retrieval_mode_from_config(self.config.mode),
        )
        .await
    }

    pub async fn retrieve_with_mode(
        &self,
        backend: &str,
        query: &str,
        config: &Config,
        mode: RetrievalMode,
    ) -> Result<Vec<RetrievedDocument>> {
        let retrieval_scope = retrieval_scope_key(backend, config);
        match mode {
            RetrievalMode::Keyword => self.retrieve_keyword(&retrieval_scope, query),
            RetrievalMode::Vector => {
                let embedding = backends::embed_text(backend, query, config)
                    .await
                    .map_err(|err| anyhow!("Failed to embed query for retrieval: {err}"))?;
                Ok(self.retrieve_from_embedding(
                    &retrieval_scope,
                    &embedding,
                    self.config.max_retrieved_docs,
                )?)
            }
            RetrievalMode::Hybrid => {
                let embedding = backends::embed_text(backend, query, config)
                    .await
                    .map_err(|err| anyhow!("Failed to embed query for retrieval: {err}"))?;
                self.retrieve_hybrid_from_embedding(
                    &retrieval_scope,
                    query,
                    &embedding,
                    self.config.max_retrieved_docs,
                )
            }
        }
    }

    pub fn retrieve_keyword(&self, backend: &str, query: &str) -> Result<Vec<RetrievedDocument>> {
        let candidates =
            self.retrieve_keyword_candidates(backend, query, self.config.max_retrieved_docs)?;
        let scores = normalize_keyword_scores(&candidates);
        Ok(candidates
            .into_iter()
            .map(|candidate| RetrievedDocument {
                score: *scores
                    .get(&(candidate.file_path.clone(), candidate.chunk_index))
                    .unwrap_or(&0.0),
                file_path: candidate.file_path,
                chunk_text: candidate.chunk_text,
            })
            .collect())
    }

    pub fn retrieve_hybrid_from_embedding(
        &self,
        backend: &str,
        query: &str,
        query_embedding: &[f32],
        top_n: usize,
    ) -> Result<Vec<RetrievedDocument>> {
        if query_embedding.is_empty() || top_n == 0 {
            return Ok(Vec::new());
        }

        let candidate_limit = top_n.saturating_mul(4).max(top_n);
        let vector_candidates =
            self.retrieve_vector_candidates(backend, query_embedding, candidate_limit)?;
        let keyword_candidates =
            self.retrieve_keyword_candidates(backend, query, candidate_limit)?;

        let mut merged: HashMap<(String, i64), HybridAggregate> = HashMap::new();

        for candidate in vector_candidates {
            let key = (candidate.file_path.clone(), candidate.chunk_index);
            let normalized = normalize_vector_score(candidate.score);
            merged
                .entry(key)
                .and_modify(|entry| {
                    entry.vector_score = Some(normalized);
                })
                .or_insert_with(|| HybridAggregate {
                    file_path: candidate.file_path,
                    chunk_text: candidate.chunk_text,
                    vector_score: Some(normalized),
                    keyword_score: None,
                });
        }

        let keyword_scores = normalize_keyword_scores(&keyword_candidates);
        for candidate in keyword_candidates {
            let key = (candidate.file_path.clone(), candidate.chunk_index);
            let normalized = keyword_scores.get(&key).copied().unwrap_or(1.0);
            merged
                .entry(key)
                .and_modify(|entry| {
                    entry.keyword_score = Some(normalized);
                })
                .or_insert_with(|| HybridAggregate {
                    file_path: candidate.file_path,
                    chunk_text: candidate.chunk_text,
                    vector_score: None,
                    keyword_score: Some(normalized),
                });
        }

        let mut scored = merged
            .into_values()
            .map(|entry| {
                let mut total = 0.0f32;
                let mut count = 0.0f32;
                if let Some(score) = entry.vector_score {
                    total += score;
                    count += 1.0;
                }
                if let Some(score) = entry.keyword_score {
                    total += score;
                    count += 1.0;
                }
                RetrievedDocument {
                    file_path: entry.file_path,
                    chunk_text: entry.chunk_text,
                    score: if count > 0.0 { total / count } else { 0.0 },
                }
            })
            .collect::<Vec<_>>();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        scored.truncate(top_n);
        Ok(scored)
    }

    pub fn retrieve_from_embedding(
        &self,
        backend: &str,
        query_embedding: &[f32],
        top_n: usize,
    ) -> Result<Vec<RetrievedDocument>> {
        let candidates = self.retrieve_vector_candidates(backend, query_embedding, top_n)?;
        Ok(candidates
            .into_iter()
            .map(|candidate| RetrievedDocument {
                file_path: candidate.file_path,
                chunk_text: candidate.chunk_text,
                score: candidate.score,
            })
            .collect())
    }

    fn retrieve_vector_candidates(
        &self,
        backend: &str,
        query_embedding: &[f32],
        top_n: usize,
    ) -> Result<Vec<ScoredChunk>> {
        if query_embedding.is_empty() || top_n == 0 {
            return Ok(Vec::new());
        }

        let db_path = normalize_path(&self.config.vector_db_path);
        if !db_path.exists() {
            return Ok(Vec::new());
        }
        let vector_chunks = load_vector_chunks_cached(&db_path, backend)?;
        let query_norm = vector_norm(query_embedding);
        if query_norm <= f32::EPSILON {
            return Ok(Vec::new());
        }

        let mut scored = Vec::new();
        for chunk in vector_chunks.iter() {
            if chunk.embedding.is_empty() || chunk.embedding.len() != query_embedding.len() {
                continue;
            }
            let score = cosine_similarity_precomputed(
                query_embedding,
                query_norm,
                &chunk.embedding,
                chunk.embedding_norm,
            );
            scored.push(ScoredChunk {
                file_path: chunk.file_path.clone(),
                chunk_index: chunk.chunk_index,
                chunk_text: chunk.chunk_text.clone(),
                score,
            });
        }

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        scored.truncate(top_n);
        Ok(scored)
    }

    fn retrieve_keyword_candidates(
        &self,
        backend: &str,
        query: &str,
        top_n: usize,
    ) -> Result<Vec<ScoredChunk>> {
        if top_n == 0 {
            return Ok(Vec::new());
        }

        let Some(match_query) = build_keyword_match_query(query) else {
            return Ok(Vec::new());
        };

        let db_path = normalize_path(&self.config.vector_db_path);
        if !db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open vector DB at {}", db_path.display()))?;
        ensure_schema(&conn)?;

        let mut stmt = conn.prepare(
            "SELECT file_path, chunk_index, chunk_text, bm25(rag_vectors_fts) AS raw_score
             FROM rag_vectors_fts
             WHERE rag_vectors_fts MATCH ?1 AND backend = ?2
             ORDER BY raw_score ASC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![match_query, backend, top_n as i64], |row| {
            let file_path: String = row.get(0)?;
            let chunk_index: i64 = row.get(1)?;
            let chunk_text: String = row.get(2)?;
            let score: f32 = row.get::<_, f64>(3)? as f32;
            Ok(ScoredChunk {
                file_path,
                chunk_index,
                chunk_text,
                score,
            })
        })?;

        let mut scored = Vec::new();
        for row in rows {
            scored.push(row?);
        }

        Ok(scored)
    }
}

#[derive(Debug, Clone)]
struct ScoredChunk {
    file_path: String,
    chunk_index: i64,
    chunk_text: String,
    score: f32,
}

#[derive(Debug, Clone)]
struct HybridAggregate {
    file_path: String,
    chunk_text: String,
    vector_score: Option<f32>,
    keyword_score: Option<f32>,
}

#[derive(Debug, Clone)]
struct CachedVectorChunk {
    file_path: String,
    chunk_index: i64,
    chunk_text: String,
    embedding: Vec<f32>,
    embedding_norm: f32,
}

#[derive(Debug, Clone)]
struct VectorCacheEntry {
    db_signature: String,
    chunks: Arc<Vec<CachedVectorChunk>>,
}

static VECTOR_CACHE: OnceLock<Mutex<HashMap<String, VectorCacheEntry>>> = OnceLock::new();

fn normalize_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn clear_backend_entries(conn: &Connection, backend: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM rag_vectors_fts WHERE backend = ?1",
        params![backend],
    )?;
    conn.execute(
        "DELETE FROM rag_vectors WHERE backend = ?1",
        params![backend],
    )?;
    Ok(())
}

fn retrieval_mode_from_config(mode: RagMode) -> RetrievalMode {
    match mode {
        RagMode::Keyword => RetrievalMode::Keyword,
        RagMode::Vector => RetrievalMode::Vector,
        RagMode::Hybrid => RetrievalMode::Hybrid,
    }
}

fn retrieval_scope_key(backend: &str, config: &Config) -> String {
    let embedding_backend = config
        .rag
        .embedding_backend
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let embedding_model = config
        .rag
        .embedding_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if embedding_backend.is_none() && embedding_model.is_none() {
        return backend.to_string();
    }

    let scope_backend = embedding_backend.unwrap_or(backend);
    let scope_model = embedding_model
        .or_else(|| configured_model_for_backend(scope_backend, config))
        .unwrap_or("default");

    format!("{scope_backend}::{scope_model}")
}

pub fn retrieval_scope_preview(backend: &str, config: &Config) -> String {
    retrieval_scope_key(backend, config)
}

fn load_vector_chunks_cached(db_path: &Path, backend: &str) -> Result<Arc<Vec<CachedVectorChunk>>> {
    let cache_key = format!("{}::{backend}", db_path.display());
    let signature = db_signature(db_path)?;
    let cache = VECTOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let lock = cache
            .lock()
            .map_err(|_| anyhow!("Vector cache lock poisoned"))?;
        if let Some(entry) = lock.get(&cache_key) {
            if entry.db_signature == signature {
                return Ok(entry.chunks.clone());
            }
        }
    }

    let chunks = Arc::new(load_vector_chunks_from_db(db_path, backend)?);
    {
        let mut lock = cache
            .lock()
            .map_err(|_| anyhow!("Vector cache lock poisoned"))?;
        lock.insert(
            cache_key,
            VectorCacheEntry {
                db_signature: signature,
                chunks: chunks.clone(),
            },
        );
    }
    Ok(chunks)
}

fn load_vector_chunks_from_db(db_path: &Path, backend: &str) -> Result<Vec<CachedVectorChunk>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open vector DB at {}", db_path.display()))?;
    ensure_schema(&conn)?;

    let mut stmt = conn.prepare(
        "SELECT file_path, chunk_index, chunk_text, embedding_json FROM rag_vectors WHERE backend = ?1",
    )?;
    let rows = stmt.query_map([backend], |row| {
        let file_path: String = row.get(0)?;
        let chunk_index: i64 = row.get(1)?;
        let chunk_text: String = row.get(2)?;
        let embedding_json: String = row.get(3)?;
        Ok((file_path, chunk_index, chunk_text, embedding_json))
    })?;

    let mut chunks = Vec::new();
    for row in rows {
        let (file_path, chunk_index, chunk_text, embedding_json) = row?;
        let embedding: Vec<f32> = serde_json::from_str(&embedding_json).unwrap_or_default();
        if embedding.is_empty() {
            continue;
        }
        chunks.push(CachedVectorChunk {
            file_path,
            chunk_index,
            chunk_text,
            embedding_norm: vector_norm(&embedding),
            embedding,
        });
    }
    Ok(chunks)
}

fn db_signature(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Could not read metadata for {}", path.display()))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    Ok(format!("{}:{modified}", metadata.len()))
}

fn configured_model_for_backend<'a>(backend: &str, config: &'a Config) -> Option<&'a str> {
    match backend {
        "chatgpt" => config.chatgpt.as_ref().map(|value| value.model.as_str()),
        "claude" => config.claude.as_ref().map(|value| value.model.as_str()),
        "gemini" => config.gemini.as_ref().map(|value| value.model.as_str()),
        "ollama" => config.ollama.as_ref().map(|value| value.model.as_str()),
        _ => None,
    }
    .map(str::trim)
    .filter(|value| !value.is_empty())
}

fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rag_vectors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            backend TEXT NOT NULL,
            file_path TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            chunk_text TEXT NOT NULL,
            embedding_json TEXT NOT NULL,
            chunk_size INTEGER NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS rag_vectors_fts USING fts5(
            backend UNINDEXED,
            file_path UNINDEXED,
            chunk_index UNINDEXED,
            chunk_text
        );
        CREATE INDEX IF NOT EXISTS idx_rag_vectors_backend ON rag_vectors(backend);
        CREATE INDEX IF NOT EXISTS idx_rag_vectors_file ON rag_vectors(file_path);",
    )?;
    Ok(())
}

fn persist_chunk(
    conn: &Connection,
    backend: &str,
    file_path: &Path,
    chunk_index: usize,
    chunk_text: &str,
    embedding: Option<&[f32]>,
    chunk_size: usize,
    index_keyword: bool,
) -> Result<()> {
    let embedding_json = match embedding {
        Some(embedding) => serde_json::to_string(embedding)?,
        None => "[]".to_string(),
    };
    conn.execute(
        "INSERT INTO rag_vectors (backend, file_path, chunk_index, chunk_text, embedding_json, chunk_size)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            backend,
            file_path.to_string_lossy().to_string(),
            chunk_index as i64,
            chunk_text,
            embedding_json,
            chunk_size as i64
        ],
    )?;
    if index_keyword {
        let rowid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO rag_vectors_fts (rowid, backend, file_path, chunk_index, chunk_text)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                rowid,
                backend,
                file_path.to_string_lossy().to_string(),
                chunk_index as i64,
                chunk_text,
            ],
        )?;
    }
    Ok(())
}

fn normalize_vector_score(score: f32) -> f32 {
    ((score + 1.0) / 2.0).clamp(0.0, 1.0)
}

fn normalize_keyword_score(raw_score: f32, min_raw: f32, max_raw: f32) -> f32 {
    if (max_raw - min_raw).abs() < f32::EPSILON {
        1.0
    } else {
        ((max_raw - raw_score) / (max_raw - min_raw)).clamp(0.0, 1.0)
    }
}

fn normalize_keyword_scores(candidates: &[ScoredChunk]) -> HashMap<(String, i64), f32> {
    if candidates.is_empty() {
        return HashMap::new();
    }

    let min_raw = candidates
        .iter()
        .map(|candidate| candidate.score)
        .fold(f32::INFINITY, f32::min);
    let max_raw = candidates
        .iter()
        .map(|candidate| candidate.score)
        .fold(f32::NEG_INFINITY, f32::max);

    candidates
        .iter()
        .map(|candidate| {
            (
                (candidate.file_path.clone(), candidate.chunk_index),
                normalize_keyword_score(candidate.score, min_raw, max_raw),
            )
        })
        .collect()
}

fn build_keyword_match_query(query: &str) -> Option<String> {
    let mut terms = Vec::new();
    for term in query.split(|ch: char| !ch.is_alphanumeric()) {
        let term = term.trim().to_ascii_lowercase();
        if term.is_empty() {
            continue;
        }
        if !terms.contains(&term) {
            terms.push(term);
        }
        if terms.len() >= 12 {
            break;
        }
    }

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" AND "))
    }
}

fn is_supported(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "md" | "py" | "java" | "xml" | "txt" | "pdf" | "doc" | "docx" | "xls" | "xlsx"
    )
}

fn extract_text(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match ext.as_str() {
        "pdf" => Ok(pdf_extract::extract_text(path)?),
        "docx" => extract_docx_text(path),
        "xls" | "xlsx" => extract_spreadsheet_text(path),
        "doc" => extract_doc_legacy_text(path),
        _ => Ok(fs::read_to_string(path)?),
    }
}

fn extract_docx_text(path: &Path) -> Result<String> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut xml_file = archive
        .by_name("word/document.xml")
        .context("DOCX missing word/document.xml")?;
    let mut xml = String::new();
    xml_file.read_to_string(&mut xml)?;
    Ok(strip_xml_tags(&xml))
}

fn extract_spreadsheet_text(path: &Path) -> Result<String> {
    let mut workbook = open_workbook_auto(path)?;
    let mut out = String::new();
    for name in workbook.sheet_names().to_owned() {
        if let Ok(range) = workbook.worksheet_range(&name) {
            out.push_str(&format!("Sheet: {name}\n"));
            for row in range.rows() {
                let line = row
                    .iter()
                    .map(|cell| cell.to_string())
                    .collect::<Vec<_>>()
                    .join(" | ");
                if !line.trim().is_empty() {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
            out.push('\n');
        }
    }
    Ok(out)
}

fn extract_doc_legacy_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let ascii = bytes
        .iter()
        .map(|byte| {
            if (32..=126).contains(byte) || *byte == b'\n' || *byte == b'\t' || *byte == b' ' {
                *byte as char
            } else {
                ' '
            }
        })
        .collect::<String>();
    let normalized = ascii
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    Ok(normalized)
}

fn strip_xml_tags(xml: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in xml.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}

fn chunk_text(text: &str, chunk_size: usize) -> Vec<String> {
    let chunk_size = chunk_size.max(128);
    let normalized = text.replace("\r\n", "\n");
    let mut chunks = Vec::new();
    let mut current = String::new();

    for paragraph in normalized.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        if current.len() + trimmed.len() + 2 <= chunk_size {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(trimmed);
            continue;
        }

        if !current.is_empty() {
            chunks.push(current.clone());
            current.clear();
        }

        if trimmed.len() <= chunk_size {
            current.push_str(trimmed);
            continue;
        }

        let chars = trimmed.chars().collect::<Vec<_>>();
        let mut start = 0usize;
        while start < chars.len() {
            let end = (start + chunk_size).min(chars.len());
            let part = chars[start..end]
                .iter()
                .collect::<String>()
                .trim()
                .to_string();
            if !part.is_empty() {
                chunks.push(part);
            }
            start = end;
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current);
    }

    chunks
}

fn vector_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn cosine_similarity_precomputed(a: &[f32], a_norm: f32, b: &[f32], b_norm: f32) -> f32 {
    if a_norm <= f32::EPSILON || b_norm <= f32::EPSILON {
        return 0.0;
    }

    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    dot / (a_norm * b_norm)
}

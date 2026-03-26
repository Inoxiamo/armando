mod scoring;
mod text;

use crate::backends;
use crate::config::{Config, RagConfig, RagMode};
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, Transaction};
use scoring::{
    finalize_hybrid_results, merge_keyword_candidates, merge_vector_candidates,
    normalize_keyword_scores,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use text::{
    build_keyword_match_query, chunk_text, cosine_similarity_precomputed, extract_text,
    is_supported, vector_norm,
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct RetrievedDocument {
    pub file_path: String,
    pub chunk_text: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub indexed_files: usize,
    pub indexed_chunks: usize,
    pub total_lines: usize,
}

#[derive(Debug, Clone, Default)]
pub struct RagCorpusStats {
    pub file_count: usize,
    pub total_lines: usize,
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
        let docs_folder = self.resolve_documents_folder()?;
        let db_path = self.prepare_db_path()?;
        let prepared = self
            .collect_pending_index_chunks(backend, config, mode, &docs_folder)
            .await?;
        self.persist_pending_index_chunks(backend, config, &db_path, &prepared)?;

        Ok(IndexStats {
            indexed_files: prepared.indexed_files,
            indexed_chunks: prepared.indexed_chunks,
            total_lines: prepared.total_lines,
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
                self.retrieve_vector_for_query(&retrieval_scope, backend, query, config)
                    .await
            }
            RetrievalMode::Hybrid => {
                self.retrieve_hybrid_for_query(&retrieval_scope, backend, query, config)
                    .await
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
        merge_vector_candidates(&mut merged, vector_candidates);
        merge_keyword_candidates(&mut merged, keyword_candidates);
        Ok(finalize_hybrid_results(merged, top_n))
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

    fn resolve_documents_folder(&self) -> Result<PathBuf> {
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
        Ok(docs_folder)
    }

    fn prepare_db_path(&self) -> Result<PathBuf> {
        let db_path = normalize_path(&self.config.vector_db_path);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory for vector DB at {}",
                    db_path.display()
                )
            })?;
        }
        Ok(db_path)
    }

    async fn collect_pending_index_chunks(
        &self,
        backend: &str,
        config: &Config,
        mode: RetrievalMode,
        docs_folder: &Path,
    ) -> Result<PendingIndexData> {
        let store_embeddings = matches!(mode, RetrievalMode::Vector | RetrievalMode::Hybrid);
        let store_keyword_index = matches!(mode, RetrievalMode::Keyword | RetrievalMode::Hybrid);
        let mut indexed_files = 0usize;
        let mut indexed_chunks = 0usize;
        let mut total_lines = 0usize;
        let mut chunks = Vec::new();

        for entry in WalkDir::new(docs_folder)
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
            let file_chunks = chunk_text(&text, self.config.chunk_size);
            if file_chunks.is_empty() {
                continue;
            }

            indexed_files += 1;
            total_lines = total_lines
                .saturating_add(text.lines().filter(|line| !line.trim().is_empty()).count());
            for (chunk_index, chunk_text) in file_chunks.into_iter().enumerate() {
                let embedding = if store_embeddings {
                    Some(
                        backends::embed_text(backend, &chunk_text, config)
                            .await
                            .map_err(|err| anyhow!("Embedding generation failed: {err}"))?,
                    )
                } else {
                    None
                };
                chunks.push(PendingChunk {
                    file_path: path.to_path_buf(),
                    chunk_index,
                    chunk_text,
                    embedding,
                });
                indexed_chunks += 1;
            }
        }

        Ok(PendingIndexData {
            indexed_files,
            indexed_chunks,
            total_lines,
            store_keyword_index,
            chunks,
        })
    }

    fn persist_pending_index_chunks(
        &self,
        backend: &str,
        config: &Config,
        db_path: &Path,
        prepared: &PendingIndexData,
    ) -> Result<()> {
        let mut conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open vector DB at {}", db_path.display()))?;
        ensure_schema(&conn)?;
        let retrieval_scope = retrieval_scope_key(backend, config);
        // Clear + insert are committed together to avoid partial reindex states.
        let tx = conn.transaction()?;
        clear_backend_entries(&tx, &retrieval_scope)?;
        for chunk in &prepared.chunks {
            let persist_input = PersistChunkInput {
                file_path: &chunk.file_path,
                chunk_index: chunk.chunk_index,
                chunk_text: &chunk.chunk_text,
                embedding: chunk.embedding.as_deref(),
                chunk_size: self.config.chunk_size,
                index_keyword: prepared.store_keyword_index,
            };
            persist_chunk(&tx, &retrieval_scope, &persist_input)?;
        }
        tx.commit()?;
        Ok(())
    }

    async fn retrieve_vector_for_query(
        &self,
        retrieval_scope: &str,
        backend: &str,
        query: &str,
        config: &Config,
    ) -> Result<Vec<RetrievedDocument>> {
        let embedding = self
            .embed_query_for_retrieval(backend, query, config)
            .await?;
        self.retrieve_from_embedding_with_limit(retrieval_scope, &embedding)
    }

    async fn retrieve_hybrid_for_query(
        &self,
        retrieval_scope: &str,
        backend: &str,
        query: &str,
        config: &Config,
    ) -> Result<Vec<RetrievedDocument>> {
        let embedding = self
            .embed_query_for_retrieval(backend, query, config)
            .await?;
        self.retrieve_hybrid_from_embedding(
            retrieval_scope,
            query,
            &embedding,
            self.config.max_retrieved_docs,
        )
    }

    async fn embed_query_for_retrieval(
        &self,
        backend: &str,
        query: &str,
        config: &Config,
    ) -> Result<Vec<f32>> {
        backends::embed_text(backend, query, config)
            .await
            .map_err(|err| anyhow!("Failed to embed query for retrieval: {err}"))
    }

    fn retrieve_from_embedding_with_limit(
        &self,
        retrieval_scope: &str,
        embedding: &[f32],
    ) -> Result<Vec<RetrievedDocument>> {
        self.retrieve_from_embedding(retrieval_scope, embedding, self.config.max_retrieved_docs)
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
struct PendingChunk {
    file_path: PathBuf,
    chunk_index: usize,
    chunk_text: String,
    embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone)]
struct PendingIndexData {
    indexed_files: usize,
    indexed_chunks: usize,
    total_lines: usize,
    store_keyword_index: bool,
    chunks: Vec<PendingChunk>,
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

fn clear_backend_entries(conn: &Transaction<'_>, backend: &str) -> Result<()> {
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

struct PersistChunkInput<'a> {
    file_path: &'a Path,
    chunk_index: usize,
    chunk_text: &'a str,
    embedding: Option<&'a [f32]>,
    chunk_size: usize,
    index_keyword: bool,
}

fn persist_chunk(
    conn: &Transaction<'_>,
    backend: &str,
    chunk: &PersistChunkInput<'_>,
) -> Result<()> {
    let embedding_json = match chunk.embedding {
        Some(embedding) => serde_json::to_string(embedding)?,
        None => "[]".to_string(),
    };
    conn.execute(
        "INSERT INTO rag_vectors (backend, file_path, chunk_index, chunk_text, embedding_json, chunk_size)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            backend,
            chunk.file_path.to_string_lossy().to_string(),
            chunk.chunk_index as i64,
            chunk.chunk_text,
            embedding_json,
            chunk.chunk_size as i64
        ],
    )?;
    if chunk.index_keyword {
        let rowid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO rag_vectors_fts (rowid, backend, file_path, chunk_index, chunk_text)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                rowid,
                backend,
                chunk.file_path.to_string_lossy().to_string(),
                chunk.chunk_index as i64,
                chunk.chunk_text,
            ],
        )?;
    }
    Ok(())
}

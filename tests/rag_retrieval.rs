use armando::config::{Config, RagConfig, RagMode, RagRuntimeOverride};
use armando::rag::RagSystem;
use rusqlite::{params, Connection};
use std::fs;

mod support;

fn create_rag_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE rag_vectors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            backend TEXT NOT NULL,
            file_path TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            chunk_text TEXT NOT NULL,
            embedding_json TEXT NOT NULL,
            chunk_size INTEGER NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        );
        CREATE VIRTUAL TABLE rag_vectors_fts USING fts5(
            backend UNINDEXED,
            file_path UNINDEXED,
            chunk_index UNINDEXED,
            chunk_text
        );",
    )
    .unwrap();
}

fn insert_chunk(
    conn: &Connection,
    backend: &str,
    file_path: &str,
    chunk_index: i64,
    chunk_text: &str,
    embedding_json: &str,
    index_keyword: bool,
) {
    conn.execute(
        "INSERT INTO rag_vectors (backend, file_path, chunk_index, chunk_text, embedding_json, chunk_size)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![backend, file_path, chunk_index, chunk_text, embedding_json, 500_i64],
    )
    .unwrap();
    if index_keyword {
        let rowid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO rag_vectors_fts (rowid, backend, file_path, chunk_index, chunk_text)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rowid, backend, file_path, chunk_index, chunk_text],
        )
        .unwrap();
    }
}

#[test]
fn rag_vector_retrieval_returns_most_similar_chunks_first() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("rag-vector-retrieval");
    let db_path = temp_dir.join("rag.sqlite3");

    let conn = Connection::open(&db_path).unwrap();
    create_rag_schema(&conn);
    insert_chunk(
        &conn,
        "ollama",
        "/docs/a.md",
        0,
        "alpha",
        "[1.0,0.0]",
        false,
    );
    insert_chunk(
        &conn,
        "ollama",
        "/docs/b.md",
        0,
        "beta",
        "[0.2,0.98]",
        false,
    );

    let rag = RagSystem::new(RagConfig {
        enabled: true,
        mode: RagMode::Vector,
        runtime_override: RagRuntimeOverride::Default,
        documents_folder: None,
        vector_db_path: db_path,
        max_retrieved_docs: 2,
        chunk_size: 500,
        embedding_backend: None,
        embedding_model: None,
    });
    let results = rag
        .retrieve_from_embedding("ollama", &[1.0_f32, 0.0_f32], 2)
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].file_path, "/docs/a.md");
    assert!(results[0].score > results[1].score);

    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn rag_keyword_indexing_and_retrieval_skip_embeddings() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("rag-keyword-retrieval");
    let docs_dir = temp_dir.join("docs");
    let db_path = temp_dir.join("rag.sqlite3");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::write(docs_dir.join("alpha.md"), "apple banana apple banana apple").unwrap();
    fs::write(docs_dir.join("beta.md"), "apple banana").unwrap();

    let rag = RagSystem::new(RagConfig {
        enabled: true,
        mode: RagMode::Keyword,
        runtime_override: RagRuntimeOverride::Default,
        documents_folder: Some(docs_dir.clone()),
        vector_db_path: db_path.clone(),
        max_retrieved_docs: 2,
        chunk_size: 500,
        embedding_backend: None,
        embedding_model: None,
    });

    let mut config = Config::default();
    config.rag = RagConfig {
        enabled: true,
        mode: RagMode::Keyword,
        runtime_override: RagRuntimeOverride::Default,
        documents_folder: Some(docs_dir.clone()),
        vector_db_path: db_path,
        max_retrieved_docs: 2,
        chunk_size: 500,
        embedding_backend: None,
        embedding_model: None,
    };

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let stats = runtime
        .block_on(rag.index_documents("unsupported-backend", &config))
        .unwrap();
    assert_eq!(stats.indexed_files, 2);
    assert_eq!(stats.indexed_chunks, 2);

    let results = runtime
        .block_on(rag.retrieve("unsupported-backend", "apple banana", &Config::default()))
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].file_path,
        docs_dir.join("alpha.md").display().to_string()
    );
    assert!(results[0].score >= results[1].score);

    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn rag_hybrid_merge_combines_keyword_and_vector_scores() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("rag-hybrid-retrieval");
    let db_path = temp_dir.join("rag.sqlite3");

    let conn = Connection::open(&db_path).unwrap();
    create_rag_schema(&conn);
    insert_chunk(
        &conn,
        "ollama",
        "/docs/both.md",
        0,
        "alpha beta",
        "[1.0,0.0]",
        true,
    );
    insert_chunk(
        &conn,
        "ollama",
        "/docs/vector.md",
        0,
        "unrelated",
        "[0.0,1.0]",
        true,
    );
    insert_chunk(
        &conn,
        "ollama",
        "/docs/keyword.md",
        0,
        "alpha beta",
        "[0.0,1.0]",
        true,
    );

    let rag = RagSystem::new(RagConfig {
        enabled: true,
        mode: RagMode::Vector,
        runtime_override: RagRuntimeOverride::Default,
        documents_folder: None,
        vector_db_path: db_path,
        max_retrieved_docs: 2,
        chunk_size: 500,
        embedding_backend: None,
        embedding_model: None,
    });

    let results = rag
        .retrieve_hybrid_from_embedding("ollama", "alpha beta", &[1.0_f32, 0.0_f32], 2)
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].file_path, "/docs/both.md");
    assert!(results[0].score > results[1].score);

    support::remove_dir_all_if_exists(&temp_dir);
}

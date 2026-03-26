use crate::rag::IndexStats;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LangChainClientConfig {
    pub base_url: String,
    pub timeout_ms: u64,
    pub retry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangChainConversationTurn {
    pub user_prompt: String,
    pub assistant_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangChainPrepareRequest {
    pub prompt: String,
    pub conversation: Vec<LangChainConversationTurn>,
    pub prompt_mode: String,
    pub active_window_context: Option<String>,
    pub documents_folder: Option<String>,
    pub query_backend: String,
    pub query_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangChainIndexRequest {
    pub documents_folder: String,
    pub force_reindex: bool,
}

#[derive(Debug, Deserialize)]
struct LangChainPrepareResponse {
    prepared_prompt: String,
}

#[derive(Debug, Deserialize)]
struct LangChainIndexResponse {
    indexed_files: usize,
    indexed_chunks: usize,
    total_lines: usize,
}

enum AttemptResult<T> {
    Success(T),
    RetryableError(String),
    FatalError(String),
}

pub async fn prepare_prompt_with_retry(
    config: &LangChainClientConfig,
    request: &LangChainPrepareRequest,
) -> Result<String> {
    let attempts = config.retry_count.saturating_add(1);
    for attempt in 0..attempts {
        match prepare_prompt_once(config, request).await {
            AttemptResult::Success(prepared_prompt) => return Ok(prepared_prompt),
            AttemptResult::RetryableError(_error) if attempt + 1 < attempts => continue,
            AttemptResult::RetryableError(error) | AttemptResult::FatalError(error) => {
                return Err(anyhow!(error));
            }
        }
    }
    Err(anyhow!(
        "LangChain prepare request failed without a concrete error"
    ))
}

pub async fn index_documents_with_retry(
    config: &LangChainClientConfig,
    request: &LangChainIndexRequest,
) -> Result<IndexStats> {
    let attempts = config.retry_count.saturating_add(1);
    for attempt in 0..attempts {
        match index_documents_once(config, request).await {
            AttemptResult::Success(stats) => return Ok(stats),
            AttemptResult::RetryableError(_error) if attempt + 1 < attempts => continue,
            AttemptResult::RetryableError(error) | AttemptResult::FatalError(error) => {
                return Err(anyhow!(error));
            }
        }
    }
    Err(anyhow!(
        "LangChain index request failed without a concrete error"
    ))
}

async fn prepare_prompt_once(
    config: &LangChainClientConfig,
    request: &LangChainPrepareRequest,
) -> AttemptResult<String> {
    let client = match build_client(config.timeout_ms) {
        Ok(client) => client,
        Err(err) => return AttemptResult::FatalError(err.to_string()),
    };
    let endpoint = format!("{}/v1/rag/prepare", normalize_base_url(&config.base_url));
    let response = match client.post(&endpoint).json(request).send().await {
        Ok(response) => response,
        Err(err) => return classify_transport_error("prepare", err),
    };

    if !response.status().is_success() {
        let retryable = response.status().is_server_error();
        let error = build_http_error("prepare", response).await;
        return if retryable {
            AttemptResult::RetryableError(error)
        } else {
            AttemptResult::FatalError(error)
        };
    }

    match response
        .json::<LangChainPrepareResponse>()
        .await
        .context("LangChain prepare response did not match the expected JSON shape")
    {
        Ok(body) => AttemptResult::Success(body.prepared_prompt),
        Err(err) => AttemptResult::FatalError(err.to_string()),
    }
}

async fn index_documents_once(
    config: &LangChainClientConfig,
    request: &LangChainIndexRequest,
) -> AttemptResult<IndexStats> {
    let client = match build_client(config.timeout_ms) {
        Ok(client) => client,
        Err(err) => return AttemptResult::FatalError(err.to_string()),
    };
    let endpoint = format!("{}/v1/rag/index", normalize_base_url(&config.base_url));
    let response = match client.post(&endpoint).json(request).send().await {
        Ok(response) => response,
        Err(err) => return classify_transport_error("index", err),
    };

    if !response.status().is_success() {
        let retryable = response.status().is_server_error();
        let error = build_http_error("index", response).await;
        return if retryable {
            AttemptResult::RetryableError(error)
        } else {
            AttemptResult::FatalError(error)
        };
    }

    match response
        .json::<LangChainIndexResponse>()
        .await
        .context("LangChain index response did not match the expected JSON shape")
    {
        Ok(body) => AttemptResult::Success(IndexStats {
            indexed_files: body.indexed_files,
            indexed_chunks: body.indexed_chunks,
            total_lines: body.total_lines,
        }),
        Err(err) => AttemptResult::FatalError(err.to_string()),
    }
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_string()
}

fn build_client(timeout_ms: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms.max(1)))
        .build()
        .context("Could not create LangChain HTTP client")
}

fn classify_transport_error<T>(operation: &str, err: reqwest::Error) -> AttemptResult<T> {
    let message = format!("LangChain {operation} request failed: {err}");
    if err.is_timeout() || err.is_connect() || err.is_request() {
        AttemptResult::RetryableError(message)
    } else {
        AttemptResult::FatalError(message)
    }
}

async fn build_http_error(operation: &str, response: reqwest::Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let error_message = parse_error_message(&body).unwrap_or_else(|| body.trim().to_string());
    if error_message.is_empty() {
        format!("LangChain {operation} request failed with HTTP {status}")
    } else {
        format!(
            "LangChain {operation} request failed with HTTP {status}: {error_message}"
        )
    }
}

fn parse_error_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    match value.get("error") {
        Some(serde_json::Value::String(message)) => Some(message.trim().to_string()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
    .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    struct MockResponse {
        status: StatusCode,
        body: String,
    }

    fn spawn_mock_server(responses: Vec<MockResponse>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured_requests = requests.clone();

        thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0_u8; 8192];
                let bytes_read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                captured_requests.lock().unwrap().push(request);

                let body = response.body.as_bytes();
                let status_line = format!(
                    "HTTP/1.1 {} {}\r\n",
                    response.status.as_u16(),
                    response.status.canonical_reason().unwrap_or("UNKNOWN")
                );
                let headers = format!(
                    "Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(status_line.as_bytes()).unwrap();
                stream.write_all(headers.as_bytes()).unwrap();
                stream.write_all(body).unwrap();
            }
        });

        (format!("http://{addr}"), requests)
    }

    #[tokio::test]
    async fn prepare_prompt_uses_remote_response() {
        let (base_url, requests) = spawn_mock_server(vec![MockResponse {
            status: StatusCode::OK,
            body: r#"{"prepared_prompt":"remote prompt"}"#.to_string(),
        }]);
        let config = LangChainClientConfig {
            base_url,
            timeout_ms: 2_000,
            retry_count: 1,
        };
        let request = LangChainPrepareRequest {
            prompt: "hello".to_string(),
            conversation: vec![],
            prompt_mode: "generic_question".to_string(),
            active_window_context: None,
            documents_folder: Some("/tmp/docs".to_string()),
            query_backend: "ollama".to_string(),
            query_model: Some("llama3".to_string()),
        };

        let prepared = prepare_prompt_with_retry(&config, &request).await.unwrap();

        assert_eq!(prepared, "remote prompt");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("POST /v1/rag/prepare HTTP/1.1"));
    }

    #[tokio::test]
    async fn prepare_prompt_retries_once_then_succeeds() {
        let (base_url, requests) = spawn_mock_server(vec![
            MockResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: r#"{"error":"temporary error"}"#.to_string(),
            },
            MockResponse {
                status: StatusCode::OK,
                body: r#"{"prepared_prompt":"second try"}"#.to_string(),
            },
        ]);
        let config = LangChainClientConfig {
            base_url,
            timeout_ms: 2_000,
            retry_count: 1,
        };
        let request = LangChainPrepareRequest {
            prompt: "hello".to_string(),
            conversation: vec![],
            prompt_mode: "text_assist".to_string(),
            active_window_context: None,
            documents_folder: None,
            query_backend: "gemini".to_string(),
            query_model: Some("gemini-2.5-flash".to_string()),
        };

        let prepared = prepare_prompt_with_retry(&config, &request).await.unwrap();

        assert_eq!(prepared, "second try");
        assert_eq!(requests.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn index_documents_returns_remote_stats() {
        let (base_url, requests) = spawn_mock_server(vec![MockResponse {
            status: StatusCode::OK,
            body: r#"{"indexed_files":4,"indexed_chunks":18,"total_lines":1200}"#.to_string(),
        }]);
        let config = LangChainClientConfig {
            base_url,
            timeout_ms: 2_000,
            retry_count: 0,
        };
        let request = LangChainIndexRequest {
            documents_folder: "/tmp/docs".to_string(),
            force_reindex: false,
        };

        let stats = index_documents_with_retry(&config, &request).await.unwrap();

        assert_eq!(stats.indexed_files, 4);
        assert_eq!(stats.indexed_chunks, 18);
        assert_eq!(stats.total_lines, 1200);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("POST /v1/rag/index HTTP/1.1"));
    }
}

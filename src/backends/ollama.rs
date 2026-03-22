use crate::backends::{ImageAttachment, ResponseProgress, ResponseProgressSink};
use crate::config::Config;
use anyhow::{anyhow, Result};
use serde_json::json;

pub async fn query(
    prompt: &str,
    images: &[ImageAttachment],
    config: &Config,
    progress: Option<ResponseProgressSink>,
) -> Result<String> {
    let (base_url, model) = if let Some(ref o) = config.ollama {
        (
            o.base_url.trim_end_matches('/').to_string(),
            o.model.clone(),
        )
    } else {
        ("http://localhost:11434".to_string(), "llama3".to_string())
    };

    query_at(&base_url, prompt, images, &model, progress).await
}

pub(crate) async fn query_at(
    base_url: &str,
    prompt: &str,
    images: &[ImageAttachment],
    model: &str,
    progress: Option<ResponseProgressSink>,
) -> Result<String> {
    let url = format!("{base_url}/api/generate");
    let payload = json!({
        "model": model,
        "prompt": prompt,
        "images": images
            .iter()
            .map(|image| image.data_base64.clone())
            .collect::<Vec<_>>(),
        "stream": progress.is_some()
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                anyhow!(
                    "⚠️ Cannot connect to Ollama at {base_url}.\nMake sure Ollama is running: ollama serve\n(Error: {e})"
                )
            } else {
                e.into()
            }
        })?;

    if !response.status().is_success() {
        return Err(anyhow!(ollama_error_message(response.status())));
    }

    if let Some(progress) = progress {
        return stream_ollama_response(response, progress).await;
    }

    let result: serde_json::Value = response.json().await?;
    ollama_response_text(&result)
}

pub(crate) fn ollama_error_message(status: reqwest::StatusCode) -> String {
    format!("Ollama returned API error: HTTP {status}")
}

pub(crate) fn ollama_response_text(result: &serde_json::Value) -> Result<String> {
    if let Some(resp) = result.get("response").and_then(|r| r.as_str()) {
        Ok(resp.to_string())
    } else {
        Err(anyhow!("Invalid response format from Ollama"))
    }
}

async fn stream_ollama_response(
    mut response: reqwest::Response,
    progress: ResponseProgressSink,
) -> Result<String> {
    let mut response_text = String::new();
    let mut buffer = String::new();

    while let Some(chunk) = response.chunk().await? {
        buffer.push_str(
            std::str::from_utf8(&chunk)
                .map_err(|err| anyhow!("Invalid UTF-8 in Ollama stream: {err}"))?,
        );

        while let Some(newline_index) = buffer.find('\n') {
            let line = buffer[..newline_index].trim().to_string();
            buffer.drain(..=newline_index);

            if line.is_empty() {
                continue;
            }

            let value: serde_json::Value = serde_json::from_str(&line)?;
            if let Some(fragment) = value.get("response").and_then(|value| value.as_str()) {
                if !fragment.is_empty() {
                    progress(ResponseProgress::Chunk(fragment.to_string()));
                    response_text.push_str(fragment);
                }
            }

            if value
                .get("done")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                return Ok(response_text);
            }
        }
    }

    let tail = buffer.trim();
    if !tail.is_empty() {
        let value: serde_json::Value = serde_json::from_str(tail)?;
        if let Some(fragment) = value.get("response").and_then(|value| value.as_str()) {
            if !fragment.is_empty() {
                progress(ResponseProgress::Chunk(fragment.to_string()));
                response_text.push_str(fragment);
            }
        }
    }

    Ok(response_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_chunk_text_accumulates_response_fragments() {
        let value = serde_json::json!({
            "response": "partial text",
            "done": false
        });

        assert_eq!(ollama_response_text(&value).unwrap(), "partial text");
    }

    #[test]
    fn stream_chunk_with_empty_response_is_ignored() {
        let value = serde_json::json!({
            "response": "",
            "done": false
        });

        assert_eq!(ollama_response_text(&value).unwrap(), "");
    }
}

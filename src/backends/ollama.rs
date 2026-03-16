use crate::backends::ImageAttachment;
use crate::config::Config;
use anyhow::{anyhow, Result};
use serde_json::json;

pub async fn query(prompt: &str, images: &[ImageAttachment], config: &Config) -> Result<String> {
    let (base_url, model) = if let Some(ref o) = config.ollama {
        (
            o.base_url.trim_end_matches('/').to_string(),
            o.model.clone(),
        )
    } else {
        ("http://localhost:11434".to_string(), "llama3".to_string())
    };

    let url = format!("{}/api/generate", base_url);
    let payload = json!({
        "model": model,
        "prompt": prompt,
        "images": images
            .iter()
            .map(|image| image.data_base64.clone())
            .collect::<Vec<_>>(),
        "stream": false
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
                anyhow!("⚠️ Cannot connect to Ollama at {}.\nMake sure Ollama is running: ollama serve\n(Error: {})", base_url, e)
            } else {
                e.into()
            }
        })?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Ollama returned API error: HTTP {}",
            response.status()
        ));
    }

    let result: serde_json::Value = response.json().await?;
    if let Some(resp) = result.get("response").and_then(|r| r.as_str()) {
        Ok(resp.to_string())
    } else {
        Err(anyhow!("Invalid response format from Ollama"))
    }
}

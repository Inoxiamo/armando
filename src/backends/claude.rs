use crate::backends::ImageAttachment;
use crate::config::Config;
use anyhow::{anyhow, Result};
use serde_json::json;

pub async fn query(prompt: &str, images: &[ImageAttachment], config: &Config) -> Result<String> {
    let (api_key, model) = if let Some(ref c) = config.claude {
        (c.api_key.clone(), c.model.clone())
    } else {
        return Err(anyhow!(
            "⚠️ Claude config section not found in config.yaml."
        ));
    };

    if api_key.is_empty() || api_key == "YOUR_ANTHROPIC_API_KEY" {
        return Err(anyhow!(
            "⚠️ Anthropic API key not configured. Edit config.yaml and set claude.api_key."
        ));
    }

    let mut content = vec![json!({
        "type": "text",
        "text": prompt
    })];
    for image in images {
        content.push(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": image.mime_type,
                "data": image.data_base64
            }
        }));
    }

    let payload = json!({
        "model": model,
        "max_tokens": 2048,
        "messages": [
            {
                "role": "user",
                "content": content
            }
        ]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        let parsed = serde_json::from_str::<serde_json::Value>(&text).ok();
        let message = parsed
            .as_ref()
            .and_then(|value| value.get("error"))
            .and_then(|error| error.get("message"))
            .and_then(|message| message.as_str())
            .unwrap_or(text.trim());

        return Err(anyhow!(
            "Claude API error (HTTP {}): modello `{}`. {}",
            status,
            model,
            message
        ));
    }

    let result: serde_json::Value = response.json().await?;
    let content = result["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow!("Unexpected Claude API response structure"))?;

    Ok(content.to_string())
}

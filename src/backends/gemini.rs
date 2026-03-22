use crate::backends::ImageAttachment;
use crate::config::Config;
use anyhow::{anyhow, Result};
use serde_json::json;

pub async fn query(prompt: &str, images: &[ImageAttachment], config: &Config) -> Result<String> {
    let (api_key, model) = if let Some(ref c) = config.gemini {
        (c.api_key.clone(), c.model.clone())
    } else {
        return Err(anyhow!(
            "⚠️ Gemini config section not found in config.yaml."
        ));
    };

    query_at(
        "https://generativelanguage.googleapis.com/v1beta/models",
        prompt,
        images,
        &api_key,
        &model,
    )
    .await
}

pub(crate) async fn query_at(
    models_base_url: &str,
    prompt: &str,
    images: &[ImageAttachment],
    api_key: &str,
    model: &str,
) -> Result<String> {
    if api_key.is_empty() || api_key == "YOUR_GEMINI_API_KEY" {
        return Err(anyhow!(
            "⚠️ Gemini API key not configured. Edit config.yaml and set gemini.api_key."
        ));
    }

    let url = format!("{models_base_url}/{model}:generateContent?key={api_key}");

    let mut parts = vec![json!({ "text": prompt })];
    for image in images {
        parts.push(json!({
            "inline_data": {
                "mime_type": image.mime_type,
                "data": image.data_base64
            }
        }));
    }

    let payload = json!({
        "contents": [{
            "parts": parts
        }]
    });

    let client = reqwest::Client::new();
    let response = client.post(&url).json(&payload).send().await?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!(gemini_error_message(&text)));
    }

    let result: serde_json::Value = response.json().await?;
    gemini_response_text(&result)
}

pub(crate) fn gemini_error_message(body: &str) -> String {
    format!("Gemini API error: {body}")
}

pub(crate) fn gemini_response_text(result: &serde_json::Value) -> Result<String> {
    let content = result["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow!("Unexpected Gemini API response structure"))?;

    Ok(content.to_string())
}

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

pub async fn embed(text: &str, config: &Config) -> Result<Vec<f32>> {
    embed_with_model(text, config, None).await
}

pub async fn embed_with_model(
    text: &str,
    config: &Config,
    model_override: Option<&str>,
) -> Result<Vec<f32>> {
    let (api_key, model) = config
        .gemini
        .as_ref()
        .map(|cfg| (cfg.api_key.clone(), cfg.model.clone()))
        .ok_or_else(|| anyhow!("⚠️ Gemini config section not found in config.yaml."))?;

    let preferred_model = model_override.unwrap_or(&model);

    let embedding_model = resolve_embedding_model(
        "https://generativelanguage.googleapis.com/v1beta/models",
        &api_key,
        preferred_model,
    )
    .await
    .unwrap_or_else(|_| "text-embedding-004".to_string());

    match embed_at(
        "https://generativelanguage.googleapis.com/v1beta/models",
        &embedding_model,
        text,
        &api_key,
        preferred_model,
    )
    .await
    {
        Ok(embedding) => Ok(embedding),
        Err(_) => {
            embed_at(
                "https://generativelanguage.googleapis.com/v1beta/models",
                "text-embedding-004",
                text,
                &api_key,
                preferred_model,
            )
            .await
        }
    }
}

pub(crate) async fn embed_at(
    models_base_url: &str,
    embedding_model: &str,
    text: &str,
    api_key: &str,
    task_model_hint: &str,
) -> Result<Vec<f32>> {
    if api_key.is_empty() || api_key == "YOUR_GEMINI_API_KEY" {
        return Err(anyhow!(
            "⚠️ Gemini API key not configured. Edit config.yaml and set gemini.api_key."
        ));
    }

    let url = format!("{models_base_url}/{embedding_model}:embedContent?key={api_key}");
    let payload = json!({
        "content": {
            "parts": [{"text": text}]
        },
        "taskType": "RETRIEVAL_DOCUMENT",
        "title": task_model_hint
    });

    let client = reqwest::Client::new();
    let response = client.post(&url).json(&payload).send().await?;
    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!(gemini_error_message(&text)));
    }

    let result: serde_json::Value = response.json().await?;
    let embedding = result["embedding"]["values"]
        .as_array()
        .ok_or_else(|| anyhow!("Unexpected Gemini embeddings API response structure"))?
        .iter()
        .filter_map(|v| v.as_f64())
        .map(|v| v as f32)
        .collect::<Vec<_>>();
    if embedding.is_empty() {
        return Err(anyhow!("Gemini embeddings API returned an empty vector"));
    }
    Ok(embedding)
}

async fn resolve_embedding_model(
    models_base_url: &str,
    api_key: &str,
    preferred_model: &str,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let response = client
        .get(models_base_url)
        .query(&[("key", api_key)])
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Gemini model discovery failed (HTTP {}): {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }

    let value: serde_json::Value = response.json().await?;
    let available = value
        .get("models")
        .and_then(|models| models.as_array())
        .into_iter()
        .flatten()
        .filter(|model| {
            model
                .get("supportedGenerationMethods")
                .and_then(|methods| methods.as_array())
                .into_iter()
                .flatten()
                .filter_map(|method| method.as_str())
                .any(|method| method == "embedContent")
        })
        .filter_map(|model| model.get("name").and_then(|name| name.as_str()))
        .map(|name| name.trim_start_matches("models/").to_string())
        .collect::<Vec<_>>();

    if available.is_empty() {
        return Err(anyhow!(
            "No Gemini models with embedContent support were discovered for this API key"
        ));
    }

    let preferred = preferred_model.trim_start_matches("models/");
    if available.iter().any(|candidate| candidate == preferred) {
        return Ok(preferred.to_string());
    }
    if available
        .iter()
        .any(|candidate| candidate == "text-embedding-004")
    {
        return Ok("text-embedding-004".to_string());
    }
    if available
        .iter()
        .any(|candidate| candidate == "embedding-001")
    {
        return Ok("embedding-001".to_string());
    }

    Ok(available[0].clone())
}

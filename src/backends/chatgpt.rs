use crate::backends::ImageAttachment;
use crate::config::Config;
use anyhow::{anyhow, Result};
use reqwest::multipart;
use serde_json::json;

pub async fn query(prompt: &str, images: &[ImageAttachment], config: &Config) -> Result<String> {
    let (api_key, model) = if let Some(ref c) = config.chatgpt {
        (c.api_key.clone(), c.model.clone())
    } else {
        return Err(anyhow!(
            "⚠️ ChatGPT config section not found in config.yaml."
        ));
    };

    query_at(
        "https://api.openai.com/v1/responses",
        prompt,
        images,
        &api_key,
        &model,
    )
    .await
}

pub(crate) async fn query_at(
    url: &str,
    prompt: &str,
    images: &[ImageAttachment],
    api_key: &str,
    model: &str,
) -> Result<String> {
    if api_key.is_empty() || api_key == "YOUR_OPENAI_API_KEY" {
        return Err(anyhow!(
            "⚠️ OpenAI API key not configured. Edit config.yaml and set chatgpt.api_key."
        ));
    }

    let mut content = vec![json!({
        "type": "input_text",
        "text": prompt
    })];
    for image in images {
        content.push(json!({
            "type": "input_image",
            "image_url": format!("data:{};base64,{}", image.mime_type, image.data_base64)
        }));
    }
    let payload = json!({
        "model": model,
        "input": [{
            "role": "user",
            "content": content
        }]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!(openai_error_message(status, &text, model)));
    }

    let result: serde_json::Value = response.json().await?;
    openai_response_text(&result)
}

pub async fn transcribe_wav_audio(wav_bytes: Vec<u8>, config: &Config) -> Result<String> {
    let api_key = if let Some(ref c) = config.chatgpt {
        c.api_key.clone()
    } else {
        return Err(anyhow!(
            "⚠️ ChatGPT config section not found in config.yaml."
        ));
    };

    transcribe_wav_audio_at(
        "https://api.openai.com/v1/audio/transcriptions",
        wav_bytes,
        &api_key,
    )
    .await
}

pub(crate) async fn transcribe_wav_audio_at(
    url: &str,
    wav_bytes: Vec<u8>,
    api_key: &str,
) -> Result<String> {
    if api_key.is_empty() || api_key == "YOUR_OPENAI_API_KEY" {
        return Err(anyhow!(
            "⚠️ OpenAI API key not configured. Configure it to use voice dictation."
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;

    let part = multipart::Part::bytes(wav_bytes)
        .file_name("dictation.wav")
        .mime_str("audio/wav")?;
    let form = multipart::Form::new()
        .text("model", "whisper-1")
        .text("response_format", "text")
        .part("file", part);

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "OpenAI transcription error (HTTP {status}): {text}"
        ));
    }

    Ok(response.text().await?.trim().to_string())
}

pub(crate) fn openai_error_message(status: reqwest::StatusCode, body: &str, model: &str) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|value| value.get("error"))
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
        .unwrap_or(body.trim());

    if status.as_u16() == 429
        && (message.to_lowercase().contains("quota")
            || message.to_lowercase().contains("billing")
            || message.to_lowercase().contains("rate limit"))
    {
        return format!(
            "Quota OpenAI esaurita o billing non attivo. Il modello configurato e `{model}`. Verifica piano, crediti e fatturazione del progetto API, poi riprova. Dettaglio API: {message}"
        );
    }

    format!("ChatGPT API error (HTTP {status}): modello `{model}`. {message}")
}

pub(crate) fn openai_response_text(result: &serde_json::Value) -> Result<String> {
    if let Some(text) = result.get("output_text").and_then(|value| value.as_str()) {
        return Ok(text.to_string());
    }

    let content = result["output"][0]["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow!("Unexpected ChatGPT API response structure"))?;

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
        .chatgpt
        .as_ref()
        .map(|cfg| (cfg.api_key.clone(), cfg.model.clone()))
        .ok_or_else(|| anyhow!("⚠️ ChatGPT config section not found in config.yaml."))?;

    let preferred_model = model_override.unwrap_or(&model);

    match embed_at(
        "https://api.openai.com/v1/embeddings",
        preferred_model,
        text,
        &api_key,
    )
    .await
    {
        Ok(embedding) => Ok(embedding),
        Err(_) if preferred_model != "text-embedding-3-small" => {
            embed_at(
                "https://api.openai.com/v1/embeddings",
                "text-embedding-3-small",
                text,
                &api_key,
            )
            .await
        }
        Err(err) => Err(err),
    }
}

pub(crate) async fn embed_at(
    url: &str,
    embedding_model: &str,
    text: &str,
    api_key: &str,
) -> Result<Vec<f32>> {
    if api_key.is_empty() || api_key == "YOUR_OPENAI_API_KEY" {
        return Err(anyhow!(
            "⚠️ OpenAI API key not configured. Edit config.yaml and set chatgpt.api_key."
        ));
    }

    let payload = json!({
        "model": embedding_model,
        "input": text
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!(openai_error_message(
            status,
            &text,
            embedding_model
        )));
    }

    let result: serde_json::Value = response.json().await?;
    let embedding = result["data"][0]["embedding"]
        .as_array()
        .ok_or_else(|| anyhow!("Unexpected OpenAI embeddings API response structure"))?
        .iter()
        .filter_map(|v| v.as_f64())
        .map(|v| v as f32)
        .collect::<Vec<_>>();
    if embedding.is_empty() {
        return Err(anyhow!("OpenAI embeddings API returned an empty vector"));
    }
    Ok(embedding)
}

use crate::config::Config;
use anyhow::{anyhow, Result};
use serde_json::json;

pub async fn query(prompt: &str, config: &Config) -> Result<String> {
    let (api_key, model) = if let Some(ref c) = config.chatgpt {
        (c.api_key.clone(), c.model.clone())
    } else {
        return Err(anyhow!(
            "⚠️ ChatGPT config section not found in config.yaml."
        ));
    };

    if api_key.is_empty() || api_key == "YOUR_OPENAI_API_KEY" {
        return Err(anyhow!(
            "⚠️ OpenAI API key not configured. Edit config.yaml and set chatgpt.api_key."
        ));
    }

    let url = "https://api.openai.com/v1/responses";
    let payload = json!({
        "model": model,
        "input": prompt
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
        let parsed = serde_json::from_str::<serde_json::Value>(&text).ok();
        let message = parsed
            .as_ref()
            .and_then(|value| value.get("error"))
            .and_then(|error| error.get("message"))
            .and_then(|message| message.as_str())
            .unwrap_or(text.trim());

        if status.as_u16() == 429
            && (message.to_lowercase().contains("quota")
                || message.to_lowercase().contains("billing")
                || message.to_lowercase().contains("rate limit"))
        {
            return Err(anyhow!(
                "Quota OpenAI esaurita o billing non attivo. Il modello configurato e `{}`. Verifica piano, crediti e fatturazione del progetto API, poi riprova. Dettaglio API: {}",
                model,
                message
            ));
        }

        return Err(anyhow!(
            "ChatGPT API error (HTTP {}): modello `{}`. {}",
            status,
            model,
            message
        ));
    }

    let result: serde_json::Value = response.json().await?;
    if let Some(text) = result.get("output_text").and_then(|value| value.as_str()) {
        return Ok(text.to_string());
    }

    let content = result["output"][0]["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow!("Unexpected ChatGPT API response structure"))?;

    Ok(content.to_string())
}

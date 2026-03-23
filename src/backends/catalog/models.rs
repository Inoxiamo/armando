use crate::config::Config;
use std::time::Duration;

pub async fn fetch_available_models(backend: &str, config: &Config) -> Result<Vec<String>, String> {
    match backend {
        "chatgpt" => fetch_openai_models(config).await,
        "claude" => fetch_claude_models(config).await,
        "gemini" => fetch_gemini_models(config).await,
        "ollama" => fetch_ollama_models(config).await,
        _ => Err(format!("Unsupported backend `{backend}`.")),
    }
}

async fn fetch_openai_models(config: &Config) -> Result<Vec<String>, String> {
    let chatgpt = config
        .chatgpt
        .as_ref()
        .ok_or_else(|| "OpenAI is not configured.".to_string())?;

    fetch_openai_models_at("https://api.openai.com/v1/models", &chatgpt.api_key).await
}

async fn fetch_claude_models(config: &Config) -> Result<Vec<String>, String> {
    let claude = config
        .claude
        .as_ref()
        .ok_or_else(|| "Anthropic is not configured.".to_string())?;

    fetch_claude_models_at("https://api.anthropic.com/v1/models", &claude.api_key).await
}

async fn fetch_gemini_models(config: &Config) -> Result<Vec<String>, String> {
    let gemini = config
        .gemini
        .as_ref()
        .ok_or_else(|| "Gemini is not configured.".to_string())?;

    fetch_gemini_models_at(
        "https://generativelanguage.googleapis.com/v1beta/models",
        &gemini.api_key,
    )
    .await
}

async fn fetch_ollama_models(config: &Config) -> Result<Vec<String>, String> {
    let ollama = config
        .ollama
        .as_ref()
        .ok_or_else(|| "Ollama is not configured.".to_string())?;

    fetch_ollama_models_at(&ollama.base_url).await
}

async fn fetch_openai_models_at(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    if api_key.trim().is_empty() || api_key == "YOUR_OPENAI_API_KEY" {
        return Err(
            "Open Settings, add the OpenAI API key, then click Refresh on the model field."
                .to_string(),
        );
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Could not create HTTP client: {err}"))?;
    let response = client
        .get(base_url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|err| {
            format!(
                "OpenAI model lookup failed: {err}. Check network access, proxy settings, and the API key, then click Refresh."
            )
        })?;

    collect_model_ids(
        response,
        |id| id.starts_with("gpt-") || id.starts_with("o") || id.contains("omni"),
        "OpenAI",
    )
    .await
}

async fn fetch_claude_models_at(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    if api_key.trim().is_empty() || api_key == "YOUR_ANTHROPIC_API_KEY" {
        return Err(
            "Open Settings, add the Anthropic API key, then click Refresh on the model field."
                .to_string(),
        );
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Could not create HTTP client: {err}"))?;
    let response = client
        .get(base_url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|err| {
            format!(
                "Anthropic model lookup failed: {err}. Check network access, proxy settings, and the API key, then click Refresh."
            )
        })?;

    collect_model_ids(response, |id| id.starts_with("claude-"), "Anthropic").await
}

async fn fetch_gemini_models_at(
    models_base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, String> {
    if api_key.trim().is_empty() || api_key == "YOUR_GEMINI_API_KEY" {
        return Err(
            "Open Settings, add the Gemini API key, then click Refresh on the model field."
                .to_string(),
        );
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Could not create HTTP client: {err}"))?;
    let response = client
        .get(models_base_url)
        .query(&[("key", api_key)])
        .send()
        .await
        .map_err(|err| {
            format!(
                "Gemini model lookup failed: {err}. Check network access, proxy settings, and the API key, then click Refresh."
            )
        })?;

    let value = parse_response_json(response).await?;
    let mut models = value
        .get("models")
        .and_then(|items| items.as_array())
        .into_iter()
        .flatten()
        .filter(|model| {
            model
                .get("supportedGenerationMethods")
                .and_then(|methods| methods.as_array())
                .into_iter()
                .flatten()
                .filter_map(|method| method.as_str())
                .any(|method| method == "generateContent")
        })
        .filter_map(|model| model.get("name").and_then(|name| name.as_str()))
        .map(|name| name.trim_start_matches("models/").to_string())
        .collect::<Vec<_>>();

    normalize_models(&mut models);
    if models.is_empty() {
        return Err(
            "Gemini did not return any text-generation models. Verify the API key can access models, then click Refresh."
                .to_string(),
        );
    }
    Ok(models)
}

async fn fetch_ollama_models_at(base_url: &str) -> Result<Vec<String>, String> {
    if base_url.trim().is_empty() {
        return Err(
            "Open Settings, fill the Ollama base URL, then click Refresh on the model field."
                .to_string(),
        );
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Could not create HTTP client: {err}"))?;
    let response = client
        .get(format!("{}/api/tags", base_url.trim_end_matches('/')))
        .send()
        .await
        .map_err(|err| {
            format!(
                "Ollama model lookup failed: {err}. Check the base URL, server reachability, and then click Refresh."
            )
        })?;
    let value = parse_response_json(response).await?;
    let mut models = value
        .get("models")
        .and_then(|items| items.as_array())
        .into_iter()
        .flatten()
        .filter_map(|model| model.get("name").and_then(|name| name.as_str()))
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    normalize_models(&mut models);
    if models.is_empty() {
        return Err(
            "Ollama did not return any models. Verify the server is reachable and that it exposes tags, then click Refresh."
                .to_string(),
        );
    }
    Ok(models)
}

async fn collect_model_ids(
    response: reqwest::Response,
    keep: impl Fn(&str) -> bool,
    provider: &str,
) -> Result<Vec<String>, String> {
    let value = parse_response_json(response).await?;
    collect_model_ids_from_value(&value, keep, provider)
}

pub(crate) fn collect_model_ids_from_value(
    value: &serde_json::Value,
    keep: impl Fn(&str) -> bool,
    provider: &str,
) -> Result<Vec<String>, String> {
    let mut models = value
        .get("data")
        .and_then(|items| items.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id").and_then(|id| id.as_str()))
        .filter(|id| keep(id))
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    normalize_models(&mut models);
    if models.is_empty() {
        return Err(format!(
            "{provider} did not return any compatible models. Verify the API key or account can access models, then click Refresh."
        ));
    }
    Ok(models)
}

async fn parse_response_json(response: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        format!("Could not read provider response: {err}. Check connectivity and retry.")
    })?;

    parse_response_body(status, &body)
}

pub(crate) fn parse_response_body(
    status: reqwest::StatusCode,
    body: &str,
) -> Result<serde_json::Value, String> {
    if !status.is_success() {
        let detail = body.trim();
        return Err(if detail.is_empty() {
            format!(
                "Provider request failed with status {status}. Check credentials, quota, and endpoint settings, then click Refresh."
            )
        } else {
            format!(
                "Provider request failed with status {status}: {detail}. Check credentials, quota, and endpoint settings, then click Refresh."
            )
        });
    }

    serde_json::from_str(body).map_err(|err| {
        format!(
            "Could not parse provider response: {err}. Verify the endpoint and try Refresh again."
        )
    })
}

pub(crate) fn normalize_models(models: &mut Vec<String>) {
    models.retain(|model| !model.trim().is_empty());
    models.sort();
    models.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use serde_json::json;

    #[test]
    fn parse_response_body_http_error_with_body() {
        let err = parse_response_body(StatusCode::UNAUTHORIZED, "invalid_api_key").unwrap_err();

        assert!(err.contains("status 401"));
        assert!(err.contains("invalid_api_key"));
    }

    #[test]
    fn parse_response_body_malformed_json() {
        let err = parse_response_body(StatusCode::OK, "{not-json}").unwrap_err();

        assert!(err.contains("Could not parse provider response"));
    }

    #[test]
    fn collect_model_ids_from_value_empty_compatible_models() {
        let value = json!({
            "data": [
                {"id": "image-gen-model"},
                {"id": "audio-model"}
            ]
        });

        let err = collect_model_ids_from_value(&value, |id| id.starts_with("gpt-"), "OpenAI")
            .unwrap_err();

        assert!(err.contains("OpenAI did not return any compatible models"));
    }

    #[test]
    fn normalize_models_dedup_sort_filter_empty() {
        let mut models = vec![
            "gpt-4o-mini".to_string(),
            "".to_string(),
            "gpt-4o-mini".to_string(),
            "claude-3-5-sonnet".to_string(),
            " ".to_string(),
            "gemini-1.5-flash".to_string(),
            "claude-3-5-sonnet".to_string(),
        ];

        normalize_models(&mut models);

        assert_eq!(
            models,
            vec![
                "claude-3-5-sonnet".to_string(),
                "gemini-1.5-flash".to_string(),
                "gpt-4o-mini".to_string(),
            ]
        );
    }
}

use crate::backends::{chatgpt, claude, gemini, ollama};
use crate::config::Config;

pub async fn embed_text(backend: &str, text: &str, config: &Config) -> Result<Vec<f32>, String> {
    let (embedding_backend, embedding_model) = resolve_embedding_request(backend, config);
    embed_text_with_model(embedding_backend, text, config, embedding_model).await
}

pub async fn embed_text_with_model(
    backend: &str,
    text: &str,
    config: &Config,
    model_override: Option<&str>,
) -> Result<Vec<f32>, String> {
    match backend {
        "chatgpt" => chatgpt::embed_with_model(text, config, model_override)
            .await
            .map_err(|err| err.to_string()),
        "claude" => claude::embed_with_model(text, config, model_override)
            .await
            .map_err(|err| err.to_string()),
        "gemini" => gemini::embed_with_model(text, config, model_override)
            .await
            .map_err(|err| err.to_string()),
        "ollama" => ollama::embed_with_model(text, config, model_override)
            .await
            .map_err(|err| err.to_string()),
        _ => Err(format!("Unsupported backend for embeddings: {backend}")),
    }
}

fn resolve_embedding_request<'a>(
    backend: &'a str,
    config: &'a Config,
) -> (&'a str, Option<&'a str>) {
    let embedding_backend = config
        .rag
        .embedding_backend
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            let selected = backend.trim();
            if selected.is_empty() {
                None
            } else {
                Some(selected)
            }
        })
        .or_else(|| {
            let configured = config.default_backend.trim();
            if configured.is_empty() {
                None
            } else {
                Some(configured)
            }
        })
        .unwrap_or("gemini");
    let embedding_model = config
        .rag
        .embedding_model
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    (embedding_backend, embedding_model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ClaudeConfig, HistoryConfig, LoggingConfig, RagConfig, ThemeConfig, UiConfig, UpdateConfig,
    };
    use std::collections::HashMap;

    fn test_config() -> Config {
        Config {
            aliases: Some(HashMap::from([(
                "TITLE".to_string(),
                "Trasforma il testo in un titolo breve.".to_string(),
            )])),
            auto_read_selection: true,
            default_backend: "gemini".to_string(),
            theme: ThemeConfig::default(),
            ui: UiConfig::default(),
            history: HistoryConfig::default(),
            logging: LoggingConfig::default(),
            update: UpdateConfig::default(),
            rag: RagConfig::default(),
            gemini: None,
            chatgpt: None,
            claude: Some(ClaudeConfig {
                api_key: String::new(),
                model: "claude-3-5-sonnet-latest".to_string(),
            }),
            ollama: None,
            loaded_from: None,
            chatgpt_api_key_from_env: false,
            gemini_api_key_from_env: false,
            claude_api_key_from_env: false,
            rag_documents_folder_from_env: false,
        }
    }

    #[test]
    fn embedding_request_uses_rag_overrides_when_present() {
        let mut config = test_config();
        config.rag.embedding_backend = Some("claude".to_string());
        config.rag.embedding_model = Some("embed-2024".to_string());

        let (backend, model) = resolve_embedding_request("chatgpt", &config);

        assert_eq!(backend, "claude");
        assert_eq!(model, Some("embed-2024"));
    }

    #[test]
    fn embedding_request_falls_back_to_selected_backend_when_overrides_are_missing() {
        let config = test_config();

        let (backend, model) = resolve_embedding_request("gemini", &config);

        assert_eq!(backend, "gemini");
        assert_eq!(model, None);
    }
}

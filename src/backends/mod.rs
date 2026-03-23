#[path = "providers/chatgpt.rs"]
pub mod chatgpt;
#[path = "providers/claude.rs"]
pub mod claude;
#[path = "pipeline/embedding.rs"]
pub mod embedding;
#[path = "providers/gemini.rs"]
pub mod gemini;
#[path = "ops/health.rs"]
pub mod health;
#[path = "catalog/models.rs"]
pub mod models;
#[path = "providers/ollama.rs"]
pub mod ollama;
#[path = "pipeline/prompt.rs"]
pub mod prompt;
#[path = "pipeline/query_flow.rs"]
mod query_flow;

use crate::config::Config;
use crate::history;
use crate::logging;
use crate::prompt_profiles::PromptProfiles;
use crate::rag::RetrievedDocument;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageAttachment {
    pub name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationTurn {
    pub user_prompt: String,
    pub assistant_response: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryInput {
    pub prompt: String,
    pub images: Vec<ImageAttachment>,
    pub conversation: Vec<ConversationTurn>,
    pub active_window_context: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    TextAssist,
    GenericQuestion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthLevel {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub backend: String,
    pub level: HealthLevel,
    pub summary: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseProgress {
    Chunk(String),
    PullStatus(String, Option<f32>),
}

pub type ResponseProgressSink = Arc<dyn Fn(ResponseProgress) + Send + Sync>;

pub async fn query(
    backend: &str,
    input: &QueryInput,
    config: &Config,
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    progress: Option<ResponseProgressSink>,
) -> String {
    let request_id = logging::log_request(config, backend, input);
    let effective_prompt = input.prompt.clone();
    let retrieved_docs = query_flow::retrieve_docs(backend, &effective_prompt, config).await;
    let prepared_prompt = query_flow::build_prepared_prompt(
        input,
        &effective_prompt,
        prompt_profiles,
        mode,
        &retrieved_docs,
    );
    logging::log_prepared_prompt(config, request_id, backend, input, &prepared_prompt);
    let res =
        query_flow::dispatch_backend_query(backend, &prepared_prompt, input, config, progress)
            .await;

    match res {
        Ok(text) => {
            logging::log_success(config, request_id, backend, input, &text);
            if config.history.enabled {
                if let Ok(entry) = history::new_entry(backend, &input.prompt, &text) {
                    let _ = history::append_entry(entry);
                }
            }
            text
        }
        Err(e) => {
            let error_text = e.to_string();
            if error_text.starts_with("Unknown backend: ") {
                return format!("❌ {error_text}");
            }
            log::error!("{backend} error: {e:?}");
            logging::log_error(config, request_id, backend, input, &error_text);
            format!("❌ {backend} error: {e}")
        }
    }
}

pub async fn embed_text(backend: &str, text: &str, config: &Config) -> Result<Vec<f32>, String> {
    embedding::embed_text(backend, text, config).await
}

pub async fn embed_text_with_model(
    backend: &str,
    text: &str,
    config: &Config,
    model_override: Option<&str>,
) -> Result<Vec<f32>, String> {
    embedding::embed_text_with_model(backend, text, config, model_override).await
}

pub async fn transcribe_wav_audio(wav_bytes: Vec<u8>, config: &Config) -> Result<String, String> {
    chatgpt::transcribe_wav_audio(wav_bytes, config)
        .await
        .map_err(|err| err.to_string())
}

pub fn health_checks(config: &Config) -> Vec<HealthCheck> {
    health::health_checks(config)
}

pub fn startup_health_checks(config: &Config, selected_backend: &str) -> Vec<HealthCheck> {
    health::startup_health_checks(config, selected_backend)
}

pub fn startup_dictation_tools_health_check_for(
    ffmpeg_available: bool,
    arecord_available: bool,
) -> HealthCheck {
    health::startup_dictation_tools_health_check_for(ffmpeg_available, arecord_available)
}

pub fn startup_clipboard_tools_health_check_for(
    wl_paste_available: bool,
    xclip_available: bool,
) -> HealthCheck {
    health::startup_clipboard_tools_health_check_for(wl_paste_available, xclip_available)
}

pub async fn fetch_available_models(backend: &str, config: &Config) -> Result<Vec<String>, String> {
    models::fetch_available_models(backend, config).await
}

pub async fn pull_ollama_model(
    model: &str,
    config: &Config,
    progress: ResponseProgressSink,
) -> Result<(), String> {
    let ollama = config
        .ollama
        .as_ref()
        .ok_or_else(|| "Ollama is not configured.".to_string())?;

    ollama::pull_model(&ollama.base_url, model, progress)
        .await
        .map_err(|err| err.to_string())
}

fn prepare_prompt(
    prompt: &str,
    conversation: &[ConversationTurn],
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    has_images: bool,
    active_window_context: Option<&str>,
) -> String {
    prompt::prepare_prompt(
        prompt,
        conversation,
        prompt_profiles,
        mode,
        has_images,
        active_window_context,
    )
}

fn prepare_prompt_with_retrieval(
    prompt: &str,
    conversation: &[ConversationTurn],
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    has_images: bool,
    active_window_context: Option<&str>,
    retrieved_docs: &[RetrievedDocument],
) -> String {
    prompt::prepare_prompt_with_retrieval(
        prompt,
        conversation,
        prompt_profiles,
        mode,
        has_images,
        active_window_context,
        retrieved_docs,
    )
}

#[cfg(test)]
fn expand_tags(
    prompt: &str,
    text_assist_tags: &std::collections::HashMap<String, String>,
    language_tags: &std::collections::HashMap<String, String>,
) -> (String, Vec<String>) {
    prompt::expand_tags(prompt, text_assist_tags, language_tags)
}

#[cfg(test)]
fn parse_header_tags(
    header: &str,
    text_assist_tags: &std::collections::HashMap<String, String>,
    language_tags: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    prompt::parse_header_tags(header, text_assist_tags, language_tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ClaudeConfig, Config, HistoryConfig, LoggingConfig, RagConfig, ThemeConfig, UiConfig,
        UpdateConfig,
    };
    use crate::prompt_profiles::{GenericPromptTag, PromptProfiles};
    use std::collections::HashMap;
    use std::path::PathBuf;

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

    fn test_profiles() -> PromptProfiles {
        let config = test_config();
        let mut profiles = PromptProfiles::default_built_in();
        if let Some(aliases) = config.aliases.as_ref() {
            for (tag, instruction) in aliases {
                profiles
                    .text_assist_tags
                    .insert(tag.to_uppercase(), instruction.clone());
            }
        }
        profiles
    }

    #[test]
    fn text_assist_prompt_keeps_cleanup_instructions() {
        let prompt = prepare_prompt(
            "sistema questo testo",
            &[],
            &test_profiles(),
            PromptMode::TextAssist,
            false,
            None,
        );

        assert!(prompt.contains("Act as a text transformation assistant"));
        assert!(prompt.contains("User request:\nsistema questo testo"));
        assert!(prompt.contains("keep the final output in the same language as the source text"));
        assert!(!prompt.contains("Use clear Markdown formatting"));
    }

    #[test]
    fn generic_question_uses_markdown_by_default() {
        let prompt = prepare_prompt(
            "come funziona docker compose?",
            &[],
            &test_profiles(),
            PromptMode::GenericQuestion,
            false,
            None,
        );

        assert!(prompt.contains("Treat the user's text as a general question or request"));
        assert!(prompt.contains("Use clear Markdown formatting when it helps readability."));
        assert!(prompt.contains("answer in the same language as the user's request"));
        assert!(!prompt.contains("return only the final command"));
    }

    #[test]
    fn generic_question_with_cmd_returns_command_only_instruction() {
        let prompt = prepare_prompt(
            "CMD: dammi il comando per vedere i processi",
            &[],
            &test_profiles(),
            PromptMode::GenericQuestion,
            false,
            None,
        );

        assert!(prompt.contains("return only the final command"));
        assert!(!prompt.contains("Use clear Markdown formatting when it helps readability."));
        assert!(prompt.contains("Automatically apply these context instructions: CMD."));
        assert!(prompt.contains("User request:\ndammi il comando per vedere i processi"));
    }

    #[test]
    fn generic_question_does_not_expand_standard_text_assist_aliases() {
        let prompt = prepare_prompt(
            "TITLE: armando popup ai",
            &[],
            &test_profiles(),
            PromptMode::GenericQuestion,
            false,
            None,
        );

        assert!(!prompt.contains("Trasforma il testo in un titolo breve."));
        assert!(!prompt.contains("Automatically apply these context instructions"));
        assert!(prompt.contains("User request:\nTITLE: armando popup ai"));
    }

    #[test]
    fn expand_tags_applies_builtin_and_custom_aliases() {
        let profiles = test_profiles();
        let (expanded, tags) = expand_tags(
            "CMD TITLE: hello world",
            &profiles.text_assist_tags,
            &profiles.language_tags,
        );

        assert_eq!(tags, vec!["CMD".to_string(), "TITLE".to_string()]);
        assert!(expanded.contains("Shape the final output as an executable command"));
        assert!(expanded.contains("Trasforma il testo in un titolo breve."));
        assert!(expanded.ends_with("hello world"));
    }

    #[test]
    fn unknown_tags_disable_header_expansion() {
        let profiles = test_profiles();
        let (expanded, tags) = expand_tags(
            "NOPE: hello world",
            &profiles.text_assist_tags,
            &profiles.language_tags,
        );

        assert!(tags.is_empty());
        assert_eq!(expanded, "NOPE: hello world");
    }

    #[test]
    fn parse_header_tags_accepts_cmd() {
        let profiles = test_profiles();
        let tags = parse_header_tags(
            "CMD SHORT",
            &profiles.text_assist_tags,
            &profiles.language_tags,
        );
        assert_eq!(tags, vec!["CMD".to_string(), "SHORT".to_string()]);
    }

    #[test]
    fn parse_header_tags_accepts_language_aliases() {
        let profiles = test_profiles();
        let tags = parse_header_tags(
            "WORK ESP",
            &profiles.text_assist_tags,
            &profiles.language_tags,
        );
        assert_eq!(tags, vec!["WORK".to_string(), "ESP".to_string()]);
    }

    #[test]
    fn text_assist_language_tag_forces_requested_language_once() {
        let profiles = test_profiles();
        let prompt = prepare_prompt(
            "WORK FRA: please rewrite this for a customer update",
            &[],
            &profiles,
            PromptMode::TextAssist,
            false,
            None,
        );

        assert!(prompt.contains("Write the final output in French."));
        assert!(!prompt.contains("keep the final output in the same language as the source text"));
        assert!(prompt.contains("Keep the output professional and work-oriented."));
        assert!(prompt.contains("please rewrite this for a customer update"));
    }

    #[test]
    fn generic_question_language_tag_forces_requested_language() {
        let profiles = test_profiles();
        let prompt = prepare_prompt(
            "DEU: explain what docker compose does",
            &[],
            &profiles,
            PromptMode::GenericQuestion,
            false,
            None,
        );

        assert!(prompt.contains("Answer in German."));
        assert!(!prompt.contains("answer in the same language as the user's request"));
        assert!(prompt.contains("User request:\nexplain what docker compose does"));
    }

    #[test]
    fn generic_question_accepts_full_language_name_tags() {
        let profiles = test_profiles();
        let prompt = prepare_prompt(
            "PORTUGUESE: explain what docker compose does",
            &[],
            &profiles,
            PromptMode::GenericQuestion,
            false,
            None,
        );

        assert!(prompt.contains("Answer in Portuguese."));
        assert!(prompt.contains("User request:\nexplain what docker compose does"));
    }

    #[test]
    fn text_assist_accepts_short_language_aliases() {
        let profiles = test_profiles();
        let prompt = prepare_prompt(
            "WORK VI: rewrite this update for the team",
            &[],
            &profiles,
            PromptMode::TextAssist,
            false,
            None,
        );

        assert!(prompt.contains("Write the final output in Vietnamese."));
        assert!(prompt.contains("Keep the output professional and work-oriented."));
    }

    #[test]
    fn generic_question_uses_externalized_custom_tag() {
        let mut profiles = test_profiles();
        profiles.generic_question_tags.insert(
            "SQL".to_string(),
            GenericPromptTag {
                instruction: "Rispondi solo con SQL valido.".to_string(),
                strip_header: true,
            },
        );

        let prompt = prepare_prompt(
            "SQL: elenco utenti ordinati per nome",
            &[],
            &profiles,
            PromptMode::GenericQuestion,
            false,
            None,
        );

        assert!(prompt.contains("Rispondi solo con SQL valido."));
        assert!(prompt.contains("User request:\nelenco utenti ordinati per nome"));
        assert!(!prompt.contains("Use clear Markdown formatting when it helps readability."));
    }

    #[test]
    fn conversation_context_is_embedded_when_present() {
        let prompt = prepare_prompt(
            "continua la conversazione",
            &[ConversationTurn {
                user_prompt: "come stai?".to_string(),
                assistant_response: "bene".to_string(),
            }],
            &test_profiles(),
            PromptMode::GenericQuestion,
            false,
            None,
        );

        assert!(prompt.contains("Current conversation context"));
        assert!(prompt.contains("User:\ncome stai?"));
        assert!(prompt.contains("Assistant:\nbene"));
        assert!(
            prompt.contains("Do not automatically reinterpret the new request as a text cleanup")
        );
    }

    #[test]
    fn active_window_context_is_embedded_as_a_hint_when_present() {
        let prompt = prepare_prompt(
            "riassumi questo testo",
            &[],
            &test_profiles(),
            PromptMode::TextAssist,
            false,
            Some("Firefox - release notes"),
        );

        assert!(prompt.contains("active window context only as a hint"));
        assert!(prompt.contains("Firefox - release notes"));
    }

    #[test]
    fn startup_health_checks_include_config_backend_and_tool_states() {
        let mut config = test_config();
        config.loaded_from = Some(PathBuf::from("/tmp/armando-config.yaml"));
        config.claude = Some(ClaudeConfig {
            api_key: "secret".to_string(),
            model: "claude-3-5-sonnet-latest".to_string(),
        });

        let checks = startup_health_checks(&config, "claude");

        assert_eq!(checks.len(), 4);
        assert_eq!(checks[0].backend, "config");
        assert_eq!(checks[0].level, HealthLevel::Ok);
        assert_eq!(checks[1].backend, "selected-backend");
        assert_eq!(checks[1].level, HealthLevel::Ok);
        assert_eq!(checks[1].summary, "Ready");
        assert_eq!(checks[2].backend, "dictation-tools");
        assert!(matches!(
            checks[2].level,
            HealthLevel::Ok | HealthLevel::Warning
        ));
        assert_eq!(checks[3].backend, "clipboard-tools");
    }

    #[test]
    fn startup_tool_checks_report_missing_helpers_as_limited_or_missing() {
        let dictation = startup_dictation_tools_health_check_for(false, false);
        assert_eq!(dictation.backend, "dictation-tools");
        assert_eq!(dictation.level, HealthLevel::Warning);
        assert!(dictation.detail.contains("Install"));
        assert!(dictation.detail.contains("ffmpeg"));

        let clipboard = startup_clipboard_tools_health_check_for(false, false);
        assert_eq!(clipboard.backend, "clipboard-tools");
        assert_eq!(clipboard.level, HealthLevel::Warning);
        assert!(clipboard.detail.contains("wl-paste"));
        assert!(clipboard.detail.contains("xclip"));
    }

    #[test]
    fn health_checks_point_to_settings_when_provider_setup_is_missing_or_incomplete() {
        let config = test_config();
        let checks = health_checks(&config);

        let chatgpt = checks
            .iter()
            .find(|check| check.backend == "chatgpt")
            .unwrap();
        assert!(matches!(
            chatgpt.level,
            HealthLevel::Warning | HealthLevel::Error
        ));
        assert!(chatgpt.detail.contains("switch"));

        let claude = checks
            .iter()
            .find(|check| check.backend == "claude")
            .unwrap();
        assert_eq!(claude.level, HealthLevel::Error);
        assert!(claude.detail.contains("Settings"));

        let gemini = checks
            .iter()
            .find(|check| check.backend == "gemini")
            .unwrap();
        assert!(matches!(
            gemini.level,
            HealthLevel::Warning | HealthLevel::Error
        ));
        assert!(gemini.detail.contains("switch"));

        let ollama = checks
            .iter()
            .find(|check| check.backend == "ollama")
            .unwrap();
        assert!(matches!(
            ollama.level,
            HealthLevel::Warning | HealthLevel::Error
        ));
        assert!(
            ollama.detail.contains("Settings")
                || ollama.detail.contains("switch to another backend")
        );
    }

    #[test]
    fn response_body_parser_reports_http_error_body() {
        let err = models::parse_response_body(
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"error":{"message":"invalid api key"}}"#,
        )
        .unwrap_err();

        assert!(err.contains("Provider request failed with status 401"));
        assert!(err.contains("invalid api key"));
    }

    #[test]
    fn response_body_parser_reports_malformed_json() {
        let err = models::parse_response_body(reqwest::StatusCode::OK, "not-json").unwrap_err();

        assert!(err.contains("Could not parse provider response"));
    }

    #[test]
    fn openai_model_lookup_reports_empty_model_list() {
        let value = serde_json::json!({"data":[{"id":"text-davinci-003"}]});
        let err =
            models::collect_model_ids_from_value(&value, |id| id.starts_with("gpt-"), "OpenAI")
                .unwrap_err();

        assert!(err.contains("OpenAI did not return any compatible models"));
    }

    #[test]
    fn gemini_model_lookup_reports_empty_text_models() {
        let value = serde_json::json!({
            "models": [
                {
                    "name": "models/gemini-1.5-pro",
                    "supportedGenerationMethods": ["embedContent"]
                }
            ]
        });

        let models = value
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

        assert!(models.is_empty());
    }

    #[test]
    fn ollama_model_lookup_reports_empty_list() {
        let value = serde_json::json!({"models":[]});
        let mut models = value
            .get("models")
            .and_then(|items| items.as_array())
            .into_iter()
            .flatten()
            .filter_map(|model| model.get("name").and_then(|name| name.as_str()))
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        models::normalize_models(&mut models);
        assert!(models.is_empty());
    }

    #[test]
    fn chatgpt_query_maps_quota_style_error_body() {
        let message = chatgpt::openai_error_message(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            r#"{"error":{"message":"rate limit exceeded"}}"#,
            "gpt-4o",
        );

        assert!(message.contains("Quota OpenAI esaurita"));
        assert!(message.contains("rate limit exceeded"));
    }

    #[test]
    fn claude_query_reports_error_body() {
        let message = claude::claude_error_message(
            reqwest::StatusCode::BAD_REQUEST,
            r#"{"error":{"message":"bad prompt"}}"#,
            "claude-3-5-sonnet-latest",
        );

        assert!(message.contains("Claude API error"));
        assert!(message.contains("bad prompt"));
    }

    #[test]
    fn gemini_query_reports_malformed_response_structure() {
        let err = gemini::gemini_response_text(&serde_json::json!({})).unwrap_err();

        assert!(err
            .to_string()
            .contains("Unexpected Gemini API response structure"));
    }

    #[test]
    fn ollama_query_reports_malformed_response_structure() {
        let err = ollama::ollama_response_text(&serde_json::json!({"foo":"bar"})).unwrap_err();

        assert!(err
            .to_string()
            .contains("Invalid response format from Ollama"));
    }
}

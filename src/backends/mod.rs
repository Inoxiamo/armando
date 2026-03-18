pub mod chatgpt;
pub mod claude;
pub mod gemini;
pub mod ollama;

use crate::config::Config;
use crate::history;
use crate::logging;
use crate::prompt_profiles::PromptProfiles;
use std::time::Duration;

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

pub async fn query(
    backend: &str,
    input: &QueryInput,
    config: &Config,
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
) -> String {
    logging::log_request(config, backend, input);
    let prepared_prompt = prepare_prompt(
        &input.prompt,
        &input.conversation,
        prompt_profiles,
        mode,
        !input.images.is_empty(),
    );
    let res = match backend {
        "chatgpt" => chatgpt::query(&prepared_prompt, &input.images, config).await,
        "claude" => claude::query(&prepared_prompt, &input.images, config).await,
        "gemini" => gemini::query(&prepared_prompt, &input.images, config).await,
        "ollama" => ollama::query(&prepared_prompt, &input.images, config).await,
        _ => return format!("❌ Unknown backend: {backend}"),
    };

    match res {
        Ok(text) => {
            logging::log_success(config, backend, input, &text);
            if config.history.enabled {
                if let Ok(entry) = history::new_entry(backend, &input.prompt, &text) {
                    let _ = history::append_entry(entry);
                }
            }
            text
        }
        Err(e) => {
            log::error!("{backend} error: {e:?}");
            logging::log_error(config, backend, input, &e.to_string());
            format!("❌ {backend} error: {e}")
        }
    }
}

pub async fn transcribe_wav_audio(wav_bytes: Vec<u8>, config: &Config) -> Result<String, String> {
    chatgpt::transcribe_wav_audio(wav_bytes, config)
        .await
        .map_err(|err| err.to_string())
}

pub fn health_checks(config: &Config) -> Vec<HealthCheck> {
    vec![
        health_check_openai(config),
        health_check_claude(config),
        health_check_gemini(config),
        health_check_ollama(config),
    ]
}

pub async fn fetch_available_models(backend: &str, config: &Config) -> Result<Vec<String>, String> {
    match backend {
        "chatgpt" => fetch_openai_models(config).await,
        "claude" => fetch_claude_models(config).await,
        "gemini" => fetch_gemini_models(config).await,
        "ollama" => fetch_ollama_models(config).await,
        _ => Err(format!("Unsupported backend `{backend}`.")),
    }
}

fn health_check_openai(config: &Config) -> HealthCheck {
    match &config.chatgpt {
        Some(chatgpt)
            if !chatgpt.api_key.is_empty() && chatgpt.api_key != "YOUR_OPENAI_API_KEY" =>
        {
            if chatgpt.model.trim().is_empty() {
                warning(
                    "chatgpt",
                    "Model missing",
                    "OpenAI API key is set, but the model field is empty.",
                )
            } else {
                ok(
                    "chatgpt",
                    "Ready",
                    format!("Configured with model `{model}`.", model = chatgpt.model),
                )
            }
        }
        Some(_) => error(
            "chatgpt",
            "API key missing",
            "Configure `chatgpt.api_key` to enable OpenAI requests.",
        ),
        None => warning(
            "chatgpt",
            "Not configured",
            "No `chatgpt` section found in config.",
        ),
    }
}

fn health_check_claude(config: &Config) -> HealthCheck {
    match &config.claude {
        Some(claude)
            if !claude.api_key.is_empty() && claude.api_key != "YOUR_ANTHROPIC_API_KEY" =>
        {
            if claude.model.trim().is_empty() {
                warning(
                    "claude",
                    "Model missing",
                    "Anthropic API key is set, but the model field is empty.",
                )
            } else {
                ok(
                    "claude",
                    "Ready",
                    format!("Configured with model `{model}`.", model = claude.model),
                )
            }
        }
        Some(_) => error(
            "claude",
            "API key missing",
            "Configure `claude.api_key` to enable Anthropic requests.",
        ),
        None => warning(
            "claude",
            "Not configured",
            "No `claude` section found in config.",
        ),
    }
}

fn health_check_gemini(config: &Config) -> HealthCheck {
    match &config.gemini {
        Some(gemini) if !gemini.api_key.is_empty() && gemini.api_key != "YOUR_GEMINI_API_KEY" => {
            if gemini.model.trim().is_empty() {
                warning(
                    "gemini",
                    "Model missing",
                    "Gemini API key is set, but the model field is empty.",
                )
            } else {
                ok(
                    "gemini",
                    "Ready",
                    format!("Configured with model `{model}`.", model = gemini.model),
                )
            }
        }
        Some(_) => error(
            "gemini",
            "API key missing",
            "Configure `gemini.api_key` to enable Gemini requests.",
        ),
        None => warning(
            "gemini",
            "Not configured",
            "No `gemini` section found in config.",
        ),
    }
}

fn health_check_ollama(config: &Config) -> HealthCheck {
    match &config.ollama {
        Some(ollama) => {
            if ollama.base_url.trim().is_empty() {
                error(
                    "ollama",
                    "Base URL missing",
                    "Configure `ollama.base_url` to reach the Ollama server.",
                )
            } else if ollama.model.trim().is_empty() {
                warning(
                    "ollama",
                    "Model missing",
                    "Ollama base URL is set, but the model field is empty.",
                )
            } else {
                ok(
                    "ollama",
                    "Ready",
                    format!(
                        "Configured with `{}` on `{}`.",
                        ollama.model, ollama.base_url
                    ),
                )
            }
        }
        None => warning(
            "ollama",
            "Not configured",
            "No `ollama` section found in config.",
        ),
    }
}

async fn fetch_openai_models(config: &Config) -> Result<Vec<String>, String> {
    let chatgpt = config
        .chatgpt
        .as_ref()
        .ok_or_else(|| "OpenAI is not configured.".to_string())?;

    if chatgpt.api_key.trim().is_empty() || chatgpt.api_key == "YOUR_OPENAI_API_KEY" {
        return Err("Set an OpenAI API key before loading available models.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Could not create HTTP client: {err}"))?;
    let response = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(&chatgpt.api_key)
        .send()
        .await
        .map_err(|err| format!("OpenAI model lookup failed: {err}"))?;

    collect_model_ids(response, |id| {
        id.starts_with("gpt-") || id.starts_with("o") || id.contains("omni")
    })
    .await
}

async fn fetch_claude_models(config: &Config) -> Result<Vec<String>, String> {
    let claude = config
        .claude
        .as_ref()
        .ok_or_else(|| "Anthropic is not configured.".to_string())?;

    if claude.api_key.trim().is_empty() || claude.api_key == "YOUR_ANTHROPIC_API_KEY" {
        return Err("Set an Anthropic API key before loading available models.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Could not create HTTP client: {err}"))?;
    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", &claude.api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|err| format!("Anthropic model lookup failed: {err}"))?;

    collect_model_ids(response, |id| id.starts_with("claude-")).await
}

async fn fetch_gemini_models(config: &Config) -> Result<Vec<String>, String> {
    let gemini = config
        .gemini
        .as_ref()
        .ok_or_else(|| "Gemini is not configured.".to_string())?;

    if gemini.api_key.trim().is_empty() || gemini.api_key == "YOUR_GEMINI_API_KEY" {
        return Err("Set a Gemini API key before loading available models.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Could not create HTTP client: {err}"))?;
    let response = client
        .get("https://generativelanguage.googleapis.com/v1beta/models")
        .query(&[("key", gemini.api_key.as_str())])
        .send()
        .await
        .map_err(|err| format!("Gemini model lookup failed: {err}"))?;

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
        return Err("No Gemini text-generation models were returned.".to_string());
    }
    Ok(models)
}

async fn fetch_ollama_models(config: &Config) -> Result<Vec<String>, String> {
    let ollama = config
        .ollama
        .as_ref()
        .ok_or_else(|| "Ollama is not configured.".to_string())?;

    if ollama.base_url.trim().is_empty() {
        return Err("Set an Ollama base URL before loading available models.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Could not create HTTP client: {err}"))?;
    let response = client
        .get(format!(
            "{}/api/tags",
            ollama.base_url.trim_end_matches('/')
        ))
        .send()
        .await
        .map_err(|err| format!("Ollama model lookup failed: {err}"))?;
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
        return Err("No Ollama models were returned by the local server.".to_string());
    }
    Ok(models)
}

async fn collect_model_ids(
    response: reqwest::Response,
    keep: impl Fn(&str) -> bool,
) -> Result<Vec<String>, String> {
    let value = parse_response_json(response).await?;
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
        return Err("No compatible models were returned by the provider.".to_string());
    }
    Ok(models)
}

async fn parse_response_json(response: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| format!("Could not read provider response: {err}"))?;

    if !status.is_success() {
        let detail = body.trim();
        return Err(if detail.is_empty() {
            format!("Provider request failed with status {status}.")
        } else {
            format!("Provider request failed with status {status}: {detail}")
        });
    }

    serde_json::from_str(&body).map_err(|err| format!("Could not parse provider response: {err}"))
}

fn normalize_models(models: &mut Vec<String>) {
    models.retain(|model| !model.trim().is_empty());
    models.sort();
    models.dedup();
}

fn ok(backend: &str, summary: &str, detail: String) -> HealthCheck {
    HealthCheck {
        backend: backend.to_string(),
        level: HealthLevel::Ok,
        summary: summary.to_string(),
        detail,
    }
}

fn warning(backend: &str, summary: &str, detail: &str) -> HealthCheck {
    HealthCheck {
        backend: backend.to_string(),
        level: HealthLevel::Warning,
        summary: summary.to_string(),
        detail: detail.to_string(),
    }
}

fn error(backend: &str, summary: &str, detail: &str) -> HealthCheck {
    HealthCheck {
        backend: backend.to_string(),
        level: HealthLevel::Error,
        summary: summary.to_string(),
        detail: detail.to_string(),
    }
}

fn prepare_prompt(
    prompt: &str,
    conversation: &[ConversationTurn],
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    has_images: bool,
) -> String {
    let (expanded_prompt, detected_tags) = match mode {
        PromptMode::TextAssist => expand_tags(
            prompt,
            &prompt_profiles.text_assist_tags,
            &prompt_profiles.language_tags,
        ),
        PromptMode::GenericQuestion => expand_generic_question_prompt(
            prompt,
            &prompt_profiles.generic_question_tags,
            &prompt_profiles.language_tags,
        ),
    };
    let explicit_language = detected_tags
        .iter()
        .find_map(|tag| prompt_profiles.language_tags.get(tag))
        .cloned();

    let mut instructions = match mode {
        PromptMode::TextAssist => vec![
            "Act as a text transformation assistant focused on rewriting, improving, correcting, translating, and adapting text.".to_string(),
            "Treat the user's input as text to transform unless the user explicitly asks for something else.".to_string(),
            "Produce a final version that is ready to copy and use directly in the target context.".to_string(),
            "Preserve the original meaning while improving clarity, tone, grammar, syntax, and readability.".to_string(),
            "Apply style, tone, and formatting instructions directly in the final text without explaining what you changed.".to_string(),
            "Return only the final requested content.".to_string(),
            "Do not add introductions, commentary, explanations, or closing remarks.".to_string(),
            "Do not add quotation marks or special formatting unless explicitly requested.".to_string(),
        ],
        PromptMode::GenericQuestion => vec![
            "Treat the user's text as a general question or request, not as a text-cleanup task.".to_string(),
            "Answer the user's request directly and accurately.".to_string(),
            "Keep the response useful, concise, and free of unnecessary preambles.".to_string(),
        ],
    };

    match mode {
        PromptMode::TextAssist => {
            if let Some(language) = explicit_language.as_deref() {
                instructions.push(format!("Write the final output in {language}."));
            } else {
                instructions.push(
                    "Unless an explicit language tag is provided, keep the final output in the same language as the source text that follows the tags.".to_string(),
                );
            }
        }
        PromptMode::GenericQuestion => {
            if let Some(language) = explicit_language.as_deref() {
                instructions.push(format!("Answer in {language}."));
            } else {
                instructions.push(
                    "Unless an explicit language tag is provided, answer in the same language as the user's request.".to_string(),
                );
            }
        }
    }

    if has_images {
        instructions.push(
            "If images or screenshots are attached, use them as visual context to read text, understand interfaces, extract details, and improve the answer."
                .to_string(),
        );
    }

    if mode == PromptMode::GenericQuestion {
        let generic_tag_instructions = detected_tags
            .iter()
            .filter_map(|tag| prompt_profiles.generic_question_tags.get(tag))
            .map(|tag| tag.instruction.trim().to_string())
            .filter(|instruction| !instruction.is_empty())
            .collect::<Vec<_>>();

        if generic_tag_instructions.is_empty() {
            instructions
                .push("Use clear Markdown formatting when it helps readability.".to_string());
        } else {
            instructions.extend(generic_tag_instructions);
        }
    }

    if !detected_tags.is_empty() {
        instructions.push(format!(
            "Automatically apply these context instructions: {}.",
            detected_tags.join(", ")
        ));
    }

    let effective_prompt = if expanded_prompt.trim().is_empty() && has_images {
        "Analyze the attached images or screenshots and respond in a useful, direct, and concrete way."
            .to_string()
    } else {
        expanded_prompt.trim().to_string()
    };

    let conversation_block = if conversation.is_empty() {
        String::new()
    } else {
        let turns = conversation
            .iter()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|turn| {
                format!(
                    "User:\n{}\n\nAssistant:\n{}",
                    turn.user_prompt.trim(),
                    turn.assistant_response.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        format!(
            "\n\nCurrent conversation context:\nUse these previous turns only as context for the ongoing conversation. Do not automatically reinterpret the new request as a text cleanup or transformation task unless the user explicitly asks for that.\n\n{turns}"
        )
    };

    format!(
        "{}{}\n\nUser request:\n{effective_prompt}",
        instructions.join("\n"),
        conversation_block,
    )
}

fn expand_generic_question_prompt(
    prompt: &str,
    generic_tags: &std::collections::HashMap<String, crate::prompt_profiles::GenericPromptTag>,
    language_tags: &std::collections::HashMap<String, String>,
) -> (String, Vec<String>) {
    let Some(colon_idx) = prompt.find(':') else {
        return (prompt.trim().to_string(), Vec::new());
    };

    let header = prompt[..colon_idx].trim();
    let body = prompt[colon_idx + 1..].trim_start();
    let tags = parse_known_tags(header, |tag| {
        generic_tags.contains_key(tag) || language_tags.contains_key(tag)
    });
    if tags.is_empty() {
        return (prompt.trim().to_string(), Vec::new());
    }

    let should_strip_header = !body.is_empty()
        && tags.iter().any(|tag| {
            generic_tags
                .get(tag)
                .is_some_and(|tag_config| tag_config.strip_header)
                || language_tags.contains_key(tag)
        });
    let effective_prompt = if should_strip_header {
        body.trim().to_string()
    } else {
        prompt.trim().to_string()
    };

    (effective_prompt, tags)
}

fn expand_tags(
    prompt: &str,
    text_assist_tags: &std::collections::HashMap<String, String>,
    language_tags: &std::collections::HashMap<String, String>,
) -> (String, Vec<String>) {
    let Some(colon_idx) = prompt.find(':') else {
        return (prompt.to_string(), Vec::new());
    };

    let header = prompt[..colon_idx].trim();
    let body = prompt[colon_idx + 1..].trim_start();
    if header.is_empty() || body.is_empty() {
        return (prompt.to_string(), Vec::new());
    }

    let tags = parse_header_tags(header, text_assist_tags, language_tags);
    if tags.is_empty() {
        return (prompt.to_string(), Vec::new());
    }

    let mut instructions = Vec::new();

    for tag in &tags {
        if let Some(custom) = text_assist_tags.get(tag) {
            instructions.push(custom.trim().to_string());
        }
    }

    let expanded = if instructions.is_empty() {
        body.to_string()
    } else {
        format!("{}\n\n{body}", instructions.join("\n"))
    };

    (expanded, tags)
}

fn parse_header_tags(
    header: &str,
    text_assist_tags: &std::collections::HashMap<String, String>,
    language_tags: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    parse_known_tags(header, |tag| {
        text_assist_tags.contains_key(tag) || language_tags.contains_key(tag)
    })
}

fn parse_known_tags<F>(header: &str, is_known: F) -> Vec<String>
where
    F: Fn(&str) -> bool,
{
    let tags: Vec<String> = header
        .split(|c: char| c.is_whitespace() || matches!(c, '-' | '+' | ',' | '/' | '|'))
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }

            let normalized = normalize_tag(part);
            let valid = !normalized.is_empty()
                && normalized
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());

            valid.then_some(normalized)
        })
        .collect();

    let all_known = tags.iter().all(|tag| is_known(tag));

    if all_known {
        tags
    } else {
        Vec::new()
    }
}

fn normalize_tag(tag: &str) -> String {
    tag.trim().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ClaudeConfig, Config, HistoryConfig, LoggingConfig, ThemeConfig, UiConfig,
    };
    use crate::prompt_profiles::{GenericPromptTag, PromptProfiles};
    use std::collections::HashMap;

    fn test_config() -> Config {
        Config {
            aliases: Some(HashMap::from([(
                "TITLE".to_string(),
                "Trasforma il testo in un titolo breve.".to_string(),
            )])),
            auto_read_selection: true,
            default_backend: "ollama".to_string(),
            theme: ThemeConfig::default(),
            ui: UiConfig::default(),
            history: HistoryConfig::default(),
            logging: LoggingConfig::default(),
            gemini: None,
            chatgpt: None,
            claude: Some(ClaudeConfig {
                api_key: String::new(),
                model: "claude-3-5-sonnet-latest".to_string(),
            }),
            ollama: None,
            loaded_from: None,
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
        );

        assert!(prompt.contains("Current conversation context"));
        assert!(prompt.contains("User:\ncome stai?"));
        assert!(prompt.contains("Assistant:\nbene"));
        assert!(
            prompt.contains("Do not automatically reinterpret the new request as a text cleanup")
        );
    }
}

pub mod chatgpt;
pub mod claude;
pub mod gemini;
pub mod ollama;

use crate::config::Config;
use crate::history;
use crate::logging;
use crate::prompt_profiles::PromptProfiles;
use crate::rag::{RagRuntimeOverride, RagSystem, RetrievedDocument};
use std::process::Command;
use std::sync::Arc;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseProgress {
    Chunk(String),
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
    let (effective_prompt, rag_override) = RagSystem::parse_prompt_override(&input.prompt);
    let rag_enabled = match rag_override {
        RagRuntimeOverride::Default => config.rag.enabled,
        RagRuntimeOverride::ForceOn => true,
        RagRuntimeOverride::ForceOff => false,
    };

    let retrieved_docs = if rag_enabled {
        let rag = RagSystem::new(config.rag.clone());
        match rag.retrieve(backend, &effective_prompt, config).await {
            Ok(docs) => docs,
            Err(err) => {
                log::warn!("RAG retrieval failed: {err:#}");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let prepared_prompt = if retrieved_docs.is_empty() {
        prepare_prompt(
            &effective_prompt,
            &input.conversation,
            prompt_profiles,
            mode,
            !input.images.is_empty(),
            input.active_window_context.as_deref(),
        )
    } else {
        prepare_prompt_with_retrieval(
            &effective_prompt,
            &input.conversation,
            prompt_profiles,
            mode,
            !input.images.is_empty(),
            input.active_window_context.as_deref(),
            &retrieved_docs,
        )
    };
    logging::log_prepared_prompt(config, request_id, backend, input, &prepared_prompt);
    let res = match backend {
        "chatgpt" => chatgpt::query(&prepared_prompt, &input.images, config).await,
        "claude" => claude::query(&prepared_prompt, &input.images, config).await,
        "gemini" => gemini::query(&prepared_prompt, &input.images, config).await,
        "ollama" => ollama::query(&prepared_prompt, &input.images, config, progress).await,
        _ => return format!("❌ Unknown backend: {backend}"),
    };

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
            log::error!("{backend} error: {e:?}");
            logging::log_error(config, request_id, backend, input, &e.to_string());
            format!("❌ {backend} error: {e}")
        }
    }
}

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
        .unwrap_or(backend);
    let embedding_model = config
        .rag
        .embedding_model
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    (embedding_backend, embedding_model)
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

pub fn startup_health_checks(config: &Config, selected_backend: &str) -> Vec<HealthCheck> {
    let provider_health_checks = health_checks(config);
    vec![
        startup_config_health_check(config),
        startup_selected_backend_health_check(selected_backend, &provider_health_checks),
        startup_dictation_tools_health_check_for(
            command_exists("ffmpeg"),
            command_exists("arecord"),
        ),
        startup_clipboard_tools_health_check(),
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
                    "Open Settings, fill `chatgpt.model`, then click Refresh on the model field.",
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
            "Open Settings, add `chatgpt.api_key`, then click Refresh or retry the request.",
        ),
        None => warning(
            "chatgpt",
            "Not configured",
            "Add a `chatgpt` section to the config or switch to another backend in the top bar.",
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
                    "Open Settings, fill `claude.model`, then click Refresh on the model field.",
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
            "Open Settings, add `claude.api_key`, then click Refresh or retry the request.",
        ),
        None => warning(
            "claude",
            "Not configured",
            "Add a `claude` section to the config or switch to another backend in the top bar.",
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
                    "Open Settings, fill `gemini.model`, then click Refresh on the model field.",
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
            "Open Settings, add `gemini.api_key`, then click Refresh or retry the request.",
        ),
        None => warning(
            "gemini",
            "Not configured",
            "Add a `gemini` section to the config or switch to another backend in the top bar.",
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
                    "Open Settings, fill `ollama.base_url`, then click Refresh to reach the server.",
                )
            } else if ollama.model.trim().is_empty() {
                warning(
                    "ollama",
                    "Model missing",
                    "Open Settings, fill `ollama.model`, then click Refresh to verify the model list.",
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
            "Add an `ollama` section to the config or switch to another backend in the top bar.",
        ),
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

fn collect_model_ids_from_value(
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

fn parse_response_body(
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

fn startup_config_health_check(config: &Config) -> HealthCheck {
    match &config.loaded_from {
        Some(path) => ok(
            "config",
            "Loaded",
            format!(
                "Loaded from `{}`. If this is not the config you expected, open the right profile and save once.",
                path.display()
            ),
        ),
        None => warning(
            "config",
            "Source missing",
            "No config source was recorded. Save a setting once so the app can remember the active file.",
        ),
    }
}

fn startup_selected_backend_health_check(
    selected_backend: &str,
    provider_health_checks: &[HealthCheck],
) -> HealthCheck {
    provider_health_checks
        .iter()
        .find(|check| check.backend == selected_backend)
        .map(|check| HealthCheck {
            backend: "selected-backend".to_string(),
            level: check.level.clone(),
            summary: check.summary.clone(),
            detail: format!(
                "Selected backend `{selected_backend}`: {}. If this is not what you want, switch it in the top toolbar before sending.",
                check.detail
            ),
        })
        .unwrap_or_else(|| {
            error(
                "selected-backend",
                "Unknown backend",
                &format!(
                    "The selected backend `{selected_backend}` is not supported. Use the top toolbar to choose chatgpt, claude, gemini, or ollama."
                ),
            )
        })
}

pub fn startup_dictation_tools_health_check_for(
    ffmpeg_available: bool,
    arecord_available: bool,
) -> HealthCheck {
    if ffmpeg_available {
        ok(
            "dictation-tools",
            "Ready",
            "Voice dictation will use `ffmpeg` for microphone capture.".to_string(),
        )
    } else if arecord_available {
        ok(
            "dictation-tools",
            "Ready",
            "Voice dictation will use `arecord` for microphone capture.".to_string(),
        )
    } else {
        warning(
            "dictation-tools",
            "Tools missing",
            "Voice dictation needs `ffmpeg` or `arecord` on the system. Install one of them, then reopen dictation.",
        )
    }
}

fn startup_clipboard_tools_health_check() -> HealthCheck {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        startup_clipboard_tools_health_check_for(
            command_exists("wl-paste"),
            command_exists("xclip"),
        )
    }

    #[cfg(not(all(unix, not(target_os = "macos"))))]
    {
        ok(
            "clipboard-tools",
            "Ready",
            "Clipboard image paste uses the platform clipboard integration.".to_string(),
        )
    }
}

pub fn startup_clipboard_tools_health_check_for(
    wl_paste_available: bool,
    xclip_available: bool,
) -> HealthCheck {
    if wl_paste_available {
        ok(
            "clipboard-tools",
            "Ready",
            "Clipboard image paste can use `wl-paste` on Wayland.".to_string(),
        )
    } else if xclip_available {
        ok(
            "clipboard-tools",
            "Ready",
            "Clipboard image paste can use `xclip` on X11.".to_string(),
        )
    } else {
        warning(
            "clipboard-tools",
            "Limited",
            "Clipboard image paste falls back to native clipboard handling on this platform. Install `wl-paste` or `xclip` on Linux if image paste stays unavailable.",
        )
    }
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn prepare_prompt(
    prompt: &str,
    conversation: &[ConversationTurn],
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    has_images: bool,
    active_window_context: Option<&str>,
) -> String {
    prepare_prompt_with_retrieval(
        prompt,
        conversation,
        prompt_profiles,
        mode,
        has_images,
        active_window_context,
        &[],
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

    if let Some(active_window_context) = active_window_context {
        instructions.push(format!(
            "If relevant, use this active window context only as a hint: {active_window_context}."
        ));
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

    if !retrieved_docs.is_empty() {
        instructions.push("Use the retrieved context below as additional grounding instructions when relevant. If retrieved context conflicts with the direct user request, prioritize the user request.".to_string());
        let context = retrieved_docs
            .iter()
            .enumerate()
            .map(|(idx, doc)| {
                format!(
                    "[{}] source={} score={:.4}\n{}",
                    idx + 1,
                    doc.file_path,
                    doc.score,
                    doc.chunk_text.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        instructions.push(format!("Retrieved context:\n{context}"));
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
            default_backend: "ollama".to_string(),
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

        let chatgpt = health_check_openai(&config);
        assert!(matches!(
            chatgpt.level,
            HealthLevel::Warning | HealthLevel::Error
        ));
        assert!(chatgpt.detail.contains("switch"));

        let claude = health_check_claude(&config);
        assert_eq!(claude.level, HealthLevel::Error);
        assert!(claude.detail.contains("Settings"));

        let gemini = health_check_gemini(&config);
        assert!(matches!(
            gemini.level,
            HealthLevel::Warning | HealthLevel::Error
        ));
        assert!(gemini.detail.contains("switch"));

        let ollama = health_check_ollama(&config);
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
        let err = parse_response_body(
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"error":{"message":"invalid api key"}}"#,
        )
        .unwrap_err();

        assert!(err.contains("Provider request failed with status 401"));
        assert!(err.contains("invalid api key"));
    }

    #[test]
    fn response_body_parser_reports_malformed_json() {
        let err = parse_response_body(reqwest::StatusCode::OK, "not-json").unwrap_err();

        assert!(err.contains("Could not parse provider response"));
    }

    #[test]
    fn openai_model_lookup_reports_empty_model_list() {
        let value = serde_json::json!({"data":[{"id":"text-davinci-003"}]});
        let err = collect_model_ids_from_value(&value, |id| id.starts_with("gpt-"), "OpenAI")
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

        normalize_models(&mut models);
        assert!(models.is_empty());
    }

    #[test]
    fn chatgpt_query_maps_quota_style_error_body() {
        let message = crate::backends::chatgpt::openai_error_message(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            r#"{"error":{"message":"rate limit exceeded"}}"#,
            "gpt-4o",
        );

        assert!(message.contains("Quota OpenAI esaurita"));
        assert!(message.contains("rate limit exceeded"));
    }

    #[test]
    fn claude_query_reports_error_body() {
        let message = crate::backends::claude::claude_error_message(
            reqwest::StatusCode::BAD_REQUEST,
            r#"{"error":{"message":"bad prompt"}}"#,
            "claude-3-5-sonnet-latest",
        );

        assert!(message.contains("Claude API error"));
        assert!(message.contains("bad prompt"));
    }

    #[test]
    fn gemini_query_reports_malformed_response_structure() {
        let err =
            crate::backends::gemini::gemini_response_text(&serde_json::json!({})).unwrap_err();

        assert!(err
            .to_string()
            .contains("Unexpected Gemini API response structure"));
    }

    #[test]
    fn ollama_query_reports_malformed_response_structure() {
        let err = crate::backends::ollama::ollama_response_text(&serde_json::json!({"foo":"bar"}))
            .unwrap_err();

        assert!(err
            .to_string()
            .contains("Invalid response format from Ollama"));
    }
}

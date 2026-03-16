pub mod chatgpt;
pub mod claude;
pub mod gemini;
pub mod ollama;

use crate::config::Config;
use crate::history;
use crate::logging;
use std::collections::HashMap;
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

pub async fn query(backend: &str, input: &QueryInput, config: &Config, mode: PromptMode) -> String {
    logging::log_request(config, backend, input);
    let prepared_prompt = prepare_prompt(
        &input.prompt,
        &input.conversation,
        config,
        mode,
        !input.images.is_empty(),
    );
    let res = match backend {
        "chatgpt" => chatgpt::query(&prepared_prompt, &input.images, config).await,
        "claude" => claude::query(&prepared_prompt, &input.images, config).await,
        "gemini" => gemini::query(&prepared_prompt, &input.images, config).await,
        "ollama" => ollama::query(&prepared_prompt, &input.images, config).await,
        _ => return format!("❌ Unknown backend: {}", backend),
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
            log::error!("{} error: {:?}", backend, e);
            logging::log_error(config, backend, input, &e.to_string());
            format!("❌ {} error: {}", backend, e)
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
                    format!("Configured with model `{}`.", chatgpt.model),
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
                    format!("Configured with model `{}`.", claude.model),
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
                    format!("Configured with model `{}`.", gemini.model),
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
    config: &Config,
    mode: PromptMode,
    has_images: bool,
) -> String {
    let (expanded_prompt, detected_tags) = match mode {
        PromptMode::TextAssist => expand_tags(prompt, config.aliases.as_ref()),
        PromptMode::GenericQuestion => expand_generic_question_prompt(prompt),
    };
    let mut instructions = match mode {
        PromptMode::TextAssist => vec![
            "Agisci principalmente come assistente di pulizia, ottimizzazione, correzione, traduzione e adattamento del testo.".to_string(),
            "Tratta l'input dell'utente come testo da sistemare, migliorare, tradurre, accorciare, espandere o rendere piu adatto a un contesto specifico, salvo istruzioni diverse esplicite.".to_string(),
            "Produci una versione finale pronta da copiare e applicare direttamente nell'applicazione di destinazione.".to_string(),
            "Mantieni il senso originale del testo, migliorandone chiarezza, tono, grammatica, sintassi e leggibilita.".to_string(),
            "Se la richiesta implica impostazioni di stile o formato, applicale direttamente nel testo finale senza spiegare cosa hai fatto.".to_string(),
            "Rispondi solo con il contenuto finale richiesto.".to_string(),
            "Niente introduzioni, commenti, spiegazioni, premesse o chiusure.".to_string(),
            "Non usare virgolette o formattazione speciale, a meno che siano richieste esplicitamente.".to_string(),
        ],
        PromptMode::GenericQuestion => vec![
            "Tratta il testo dell'utente come una domanda o richiesta generica, senza reinterpretarlo come compito di formattazione o pulizia del testo.".to_string(),
            "Rispondi esattamente alla domanda o richiesta espressa dall'utente.".to_string(),
            "Mantieni una risposta diretta, utile e senza premesse superflue.".to_string(),
        ],
    };

    if has_images {
        instructions.push(
            "Se sono presenti immagini o screenshot allegati, usali come contesto visivo per leggere testo, capire interfacce, estrarre dettagli e migliorare la risposta."
                .to_string(),
        );
    }

    if mode == PromptMode::GenericQuestion {
        if detected_tags.iter().any(|tag| tag == "CMD") {
            instructions.push(
                "Se la risposta richiesta e un comando o una one-liner da terminale, restituisci solo il comando finale, senza markdown, senza backtick e senza testo aggiuntivo."
                    .to_string(),
            );
        } else {
            instructions.push(
                "Formatta la risposta in Markdown chiaro e leggibile quando utile.".to_string(),
            );
        }
    }

    if !detected_tags.is_empty() {
        instructions.push(format!(
            "Applica automaticamente queste istruzioni di contesto: {}.",
            detected_tags.join(", ")
        ));
    }

    let effective_prompt = if expanded_prompt.trim().is_empty() && has_images {
        "Analizza gli allegati immagine o screenshot e rispondi in modo utile, diretto e concreto."
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
                    "Utente:\n{}\n\nAssistente:\n{}",
                    turn.user_prompt.trim(),
                    turn.assistant_response.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        format!(
            "\n\nContesto conversazione corrente:\nUsa questi turni precedenti solo come contesto della conversazione in corso. Non reinterpretare automaticamente la nuova richiesta come compito di pulizia o trasformazione del testo, a meno che l'utente lo chieda esplicitamente.\n\n{}",
            turns
        )
    };

    format!(
        "{}{}\n\nRichiesta utente:\n{}",
        instructions.join("\n"),
        conversation_block,
        effective_prompt
    )
}

fn expand_generic_question_prompt(prompt: &str) -> (String, Vec<String>) {
    let Some(colon_idx) = prompt.find(':') else {
        return (prompt.trim().to_string(), Vec::new());
    };

    let header = prompt[..colon_idx].trim();
    let body = prompt[colon_idx + 1..].trim_start();
    if header.eq_ignore_ascii_case("CMD") && !body.is_empty() {
        return (body.trim().to_string(), vec!["CMD".to_string()]);
    }

    (prompt.trim().to_string(), Vec::new())
}

fn expand_tags(prompt: &str, aliases: Option<&HashMap<String, String>>) -> (String, Vec<String>) {
    let Some(colon_idx) = prompt.find(':') else {
        return (prompt.to_string(), Vec::new());
    };

    let header = prompt[..colon_idx].trim();
    let body = prompt[colon_idx + 1..].trim_start();
    if header.is_empty() || body.is_empty() {
        return (prompt.to_string(), Vec::new());
    }

    let tags = parse_header_tags(header, aliases);
    if tags.is_empty() {
        return (prompt.to_string(), Vec::new());
    }

    let mut instructions = Vec::new();
    let mut applied_tags = Vec::new();

    for tag in tags {
        if let Some(instruction) = built_in_tag_instruction(&tag) {
            instructions.push(instruction.to_string());
            applied_tags.push(tag);
            continue;
        }

        if let Some(custom) = aliases.and_then(|map| map.get(&tag)) {
            instructions.push(custom.trim().to_string());
            applied_tags.push(tag);
        }
    }

    if instructions.is_empty() {
        return (prompt.to_string(), Vec::new());
    }

    (
        format!("{}\n\n{}", instructions.join("\n"), body),
        applied_tags,
    )
}

fn parse_header_tags(header: &str, aliases: Option<&HashMap<String, String>>) -> Vec<String> {
    let tags: Vec<String> = header
        .split(|c: char| c.is_whitespace() || matches!(c, '-' | '+' | ',' | '/' | '|'))
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }

            let normalized = part.to_uppercase();
            let valid = normalized
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());

            valid.then_some(normalized)
        })
        .collect();

    let all_known = tags.iter().all(|tag| {
        built_in_tag_instruction(tag).is_some()
            || aliases.is_some_and(|map| map.contains_key(tag))
            || matches!(
                tag.as_str(),
                "EMAIL" | "MAIL" | "WORK" | "FORMAL" | "CASUAL" | "SHORT" | "LONG" | "CMD"
            )
    });

    if all_known {
        tags
    } else {
        Vec::new()
    }
}

fn built_in_tag_instruction(tag: &str) -> Option<&'static str> {
    match tag {
        "GMAIL" | "EMAIL" | "MAIL" => {
            Some("Scrivi o riformula il testo come email professionale, chiara e naturale.")
        }
        "SLACK" => {
            Some("Scrivi o riformula il testo come messaggio Slack breve, operativo e naturale.")
        }
        "WHATSAPP" => Some(
            "Scrivi o riformula il testo come messaggio WhatsApp diretto, semplice e colloquiale.",
        ),
        "ITA" => Some("Traduci o riscrivi il risultato finale in italiano."),
        "ENG" => Some("Translate or rewrite the final result in English."),
        "FORMAL" => Some("Usa un tono formale e professionale."),
        "CASUAL" => Some("Usa un tono informale e naturale."),
        "WORK" => Some("Mantieni un contesto professionale e orientato al lavoro."),
        "SHORT" => Some("Mantieni il risultato breve e sintetico."),
        "LONG" => Some("Puoi essere piu completo, ma resta diretto."),
        "CMD" => Some("La risposta finale deve essere orientata a un comando eseguibile."),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ClaudeConfig, Config, ThemeConfig, UiConfig};

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
    fn text_assist_prompt_keeps_cleanup_instructions() {
        let prompt = prepare_prompt(
            "sistema questo testo",
            &[],
            &test_config(),
            PromptMode::TextAssist,
            false,
        );

        assert!(prompt.contains("Agisci principalmente come assistente di pulizia"));
        assert!(prompt.contains("Richiesta utente:\nsistema questo testo"));
        assert!(!prompt.contains("Formatta la risposta in Markdown"));
    }

    #[test]
    fn generic_question_uses_markdown_by_default() {
        let prompt = prepare_prompt(
            "come funziona docker compose?",
            &[],
            &test_config(),
            PromptMode::GenericQuestion,
            false,
        );

        assert!(
            prompt.contains("Tratta il testo dell'utente come una domanda o richiesta generica")
        );
        assert!(prompt.contains("Formatta la risposta in Markdown chiaro e leggibile"));
        assert!(!prompt.contains("solo il comando finale"));
    }

    #[test]
    fn generic_question_with_cmd_returns_command_only_instruction() {
        let prompt = prepare_prompt(
            "CMD: dammi il comando per vedere i processi",
            &[],
            &test_config(),
            PromptMode::GenericQuestion,
            false,
        );

        assert!(prompt.contains("solo il comando finale"));
        assert!(!prompt.contains("Formatta la risposta in Markdown chiaro e leggibile"));
        assert!(prompt.contains("Applica automaticamente queste istruzioni di contesto: CMD."));
        assert!(prompt.contains("Richiesta utente:\ndammi il comando per vedere i processi"));
    }

    #[test]
    fn generic_question_does_not_expand_standard_text_assist_aliases() {
        let prompt = prepare_prompt(
            "TITLE: armando popup ai",
            &[],
            &test_config(),
            PromptMode::GenericQuestion,
            false,
        );

        assert!(!prompt.contains("Trasforma il testo in un titolo breve."));
        assert!(!prompt.contains("Applica automaticamente queste istruzioni di contesto"));
        assert!(prompt.contains("Richiesta utente:\nTITLE: armando popup ai"));
    }

    #[test]
    fn expand_tags_applies_builtin_and_custom_aliases() {
        let (expanded, tags) =
            expand_tags("CMD TITLE: hello world", test_config().aliases.as_ref());

        assert_eq!(tags, vec!["CMD".to_string(), "TITLE".to_string()]);
        assert!(
            expanded.contains("La risposta finale deve essere orientata a un comando eseguibile.")
        );
        assert!(expanded.contains("Trasforma il testo in un titolo breve."));
        assert!(expanded.ends_with("hello world"));
    }

    #[test]
    fn unknown_tags_disable_header_expansion() {
        let (expanded, tags) = expand_tags("NOPE: hello world", test_config().aliases.as_ref());

        assert!(tags.is_empty());
        assert_eq!(expanded, "NOPE: hello world");
    }

    #[test]
    fn parse_header_tags_accepts_cmd() {
        let tags = parse_header_tags("CMD SHORT", test_config().aliases.as_ref());
        assert_eq!(tags, vec!["CMD".to_string(), "SHORT".to_string()]);
    }

    #[test]
    fn conversation_context_is_embedded_when_present() {
        let prompt = prepare_prompt(
            "continua la conversazione",
            &[ConversationTurn {
                user_prompt: "come stai?".to_string(),
                assistant_response: "bene".to_string(),
            }],
            &test_config(),
            PromptMode::GenericQuestion,
            false,
        );

        assert!(prompt.contains("Contesto conversazione corrente"));
        assert!(prompt.contains("Utente:\ncome stai?"));
        assert!(prompt.contains("Assistente:\nbene"));
        assert!(prompt.contains(
            "Non reinterpretare automaticamente la nuova richiesta come compito di pulizia"
        ));
    }
}

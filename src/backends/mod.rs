pub mod chatgpt;
pub mod claude;
pub mod gemini;
pub mod ollama;

use crate::config::Config;
use crate::history;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ImageAttachment {
    pub name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub size_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct QueryInput {
    pub prompt: String,
    pub images: Vec<ImageAttachment>,
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
    let prepared_prompt = prepare_prompt(&input.prompt, config, mode, !input.images.is_empty());
    let res = match backend {
        "chatgpt" => chatgpt::query(&prepared_prompt, &input.images, config).await,
        "claude" => claude::query(&prepared_prompt, &input.images, config).await,
        "gemini" => gemini::query(&prepared_prompt, &input.images, config).await,
        "ollama" => ollama::query(&prepared_prompt, &input.images, config).await,
        _ => return format!("❌ Unknown backend: {}", backend),
    };

    match res {
        Ok(text) => {
            if let Ok(entry) = history::new_entry(backend, &input.prompt, &text) {
                let _ = history::append_entry(entry);
            }
            text
        }
        Err(e) => {
            log::error!("{} error: {:?}", backend, e);
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

fn prepare_prompt(prompt: &str, config: &Config, mode: PromptMode, has_images: bool) -> String {
    let (expanded_prompt, detected_tags) = expand_tags(prompt, config.aliases.as_ref());
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

    format!(
        "{}\n\nRichiesta utente:\n{}",
        instructions.join("\n"),
        effective_prompt
    )
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
            &test_config(),
            PromptMode::TextAssist,
        );

        assert!(prompt.contains("Agisci principalmente come assistente di pulizia"));
        assert!(prompt.contains("Richiesta utente:\nsistema questo testo"));
        assert!(!prompt.contains("Formatta la risposta in Markdown"));
    }

    #[test]
    fn generic_question_uses_markdown_by_default() {
        let prompt = prepare_prompt(
            "come funziona docker compose?",
            &test_config(),
            PromptMode::GenericQuestion,
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
            &test_config(),
            PromptMode::GenericQuestion,
        );

        assert!(prompt.contains("solo il comando finale"));
        assert!(!prompt.contains("Formatta la risposta in Markdown chiaro e leggibile"));
        assert!(prompt.contains("Applica automaticamente queste istruzioni di contesto: CMD."));
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
}

pub mod chatgpt;
pub mod gemini;
pub mod ollama;

use crate::config::Config;
use crate::history;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    TextAssist,
    GenericQuestion,
}

pub async fn query(backend: &str, prompt: &str, config: &Config, mode: PromptMode) -> String {
    let prepared_prompt = prepare_prompt(prompt, config, mode);
    let res = match backend {
        "chatgpt" => chatgpt::query(&prepared_prompt, config).await,
        "gemini" => gemini::query(&prepared_prompt, config).await,
        "ollama" => ollama::query(&prepared_prompt, config).await,
        _ => return format!("❌ Unknown backend: {}", backend),
    };

    match res {
        Ok(text) => {
            if let Ok(entry) = history::new_entry(backend, prompt, &text) {
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

fn prepare_prompt(prompt: &str, config: &Config, mode: PromptMode) -> String {
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

    format!(
        "{}\n\nRichiesta utente:\n{}",
        instructions.join("\n"),
        expanded_prompt.trim()
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
    use crate::config::{Config, ThemeConfig};

    fn test_config() -> Config {
        Config {
            hotkey: "<ctrl>+<space>".to_string(),
            aliases: Some(HashMap::from([(
                "TITLE".to_string(),
                "Trasforma il testo in un titolo breve.".to_string(),
            )])),
            auto_read_selection: true,
            paste_response_shortcut: "<ctrl>+<enter>".to_string(),
            default_backend: "ollama".to_string(),
            theme: ThemeConfig::default(),
            gemini: None,
            chatgpt: None,
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

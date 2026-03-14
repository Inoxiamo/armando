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
                "EMAIL" | "MAIL" | "WORK" | "FORMAL" | "CASUAL" | "SHORT" | "LONG"
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
        _ => None,
    }
}

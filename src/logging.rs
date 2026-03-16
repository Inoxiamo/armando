use std::fs::{self, OpenOptions};
use std::io::Write;

use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::app_paths;
use crate::backends::QueryInput;
use crate::config::Config;

#[derive(Serialize)]
struct DebugLogEvent<'a> {
    timestamp: String,
    event: &'a str,
    backend: &'a str,
    prompt: &'a str,
    image_count: usize,
    image_names: Vec<&'a str>,
    conversation_turns: usize,
    detail: Option<String>,
}

pub fn log_request(config: &Config, backend: &str, input: &QueryInput) {
    if !config.logging.enabled {
        return;
    }

    let event = DebugLogEvent {
        timestamp: now_rfc3339(),
        event: "request",
        backend,
        prompt: &input.prompt,
        image_count: input.images.len(),
        image_names: input
            .images
            .iter()
            .map(|image| image.name.as_str())
            .collect(),
        conversation_turns: input.conversation.len(),
        detail: None,
    };
    let _ = append_event(&event);
}

pub fn log_success(config: &Config, backend: &str, input: &QueryInput, response: &str) {
    if !config.logging.enabled {
        return;
    }

    let preview = response.chars().take(240).collect::<String>();
    let event = DebugLogEvent {
        timestamp: now_rfc3339(),
        event: "success",
        backend,
        prompt: &input.prompt,
        image_count: input.images.len(),
        image_names: input
            .images
            .iter()
            .map(|image| image.name.as_str())
            .collect(),
        conversation_turns: input.conversation.len(),
        detail: Some(preview),
    };
    let _ = append_event(&event);
}

pub fn log_error(config: &Config, backend: &str, input: &QueryInput, error: &str) {
    if !config.logging.enabled {
        return;
    }

    let event = DebugLogEvent {
        timestamp: now_rfc3339(),
        event: "error",
        backend,
        prompt: &input.prompt,
        image_count: input.images.len(),
        image_names: input
            .images
            .iter()
            .map(|image| image.name.as_str())
            .collect(),
        conversation_turns: input.conversation.len(),
        detail: Some(error.to_string()),
    };
    let _ = append_event(&event);
}

fn append_event(event: &DebugLogEvent<'_>) -> anyhow::Result<()> {
    let path = app_paths::debug_log_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(event)?)?;
    Ok(())
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

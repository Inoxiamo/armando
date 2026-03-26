use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::app_paths;
use crate::backends::QueryInput;
use crate::config::Config;

#[derive(Serialize)]
struct DebugLogEvent<'a> {
    timestamp: String,
    request_id: u64,
    event: &'a str,
    backend: &'a str,
    prompt: &'a str,
    image_count: usize,
    image_names: Vec<&'a str>,
    conversation_turns: usize,
    detail: Option<String>,
}

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

pub fn log_request(config: &Config, backend: &str, input: &QueryInput) -> Option<u64> {
    if !config.logging.enabled {
        return None;
    }
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);

    let event = DebugLogEvent {
        timestamp: now_rfc3339(),
        request_id,
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
    let _ = append_readable_event(&event);
    Some(request_id)
}

pub fn log_prepared_prompt(
    config: &Config,
    request_id: Option<u64>,
    backend: &str,
    input: &QueryInput,
    prepared: &str,
) {
    if !config.logging.enabled {
        return;
    }
    let request_id = request_id.unwrap_or_default();

    let event = DebugLogEvent {
        timestamp: now_rfc3339(),
        request_id,
        event: "prepared_prompt",
        backend,
        prompt: &input.prompt,
        image_count: input.images.len(),
        image_names: input
            .images
            .iter()
            .map(|image| image.name.as_str())
            .collect(),
        conversation_turns: input.conversation.len(),
        detail: Some(prepared.to_string()),
    };
    let _ = append_event(&event);
    let _ = append_readable_event(&event);
}

pub fn log_success(
    config: &Config,
    request_id: Option<u64>,
    backend: &str,
    input: &QueryInput,
    response: &str,
) {
    if !config.logging.enabled {
        return;
    }
    let request_id = request_id.unwrap_or_default();

    let preview = response.chars().take(240).collect::<String>();
    let event = DebugLogEvent {
        timestamp: now_rfc3339(),
        request_id,
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
    let _ = append_readable_event(&event);
}

pub fn log_error(
    config: &Config,
    request_id: Option<u64>,
    backend: &str,
    input: &QueryInput,
    error: &str,
) {
    if !config.logging.enabled {
        return;
    }
    let request_id = request_id.unwrap_or_default();

    let event = DebugLogEvent {
        timestamp: now_rfc3339(),
        request_id,
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
    let _ = append_readable_event(&event);
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

fn append_readable_event(event: &DebugLogEvent<'_>) -> anyhow::Result<()> {
    let jsonl_path = app_paths::debug_log_file_path()?;
    let readable_path = jsonl_path
        .parent()
        .map(|parent| parent.join("debug-readable.log"))
        .ok_or_else(|| anyhow::anyhow!("Could not resolve debug-readable.log directory"))?;

    if let Some(parent) = readable_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(readable_path)?;

    writeln!(
        file,
        "===== {} request_id={} event={} backend={} =====",
        event.timestamp, event.request_id, event.event, event.backend
    )?;
    writeln!(
        file,
        "images={} conversation_turns={}",
        event.image_count, event.conversation_turns
    )?;
    writeln!(file, "prompt:\n{}\n", event.prompt)?;

    if let Some(detail) = &event.detail {
        writeln!(file, "detail:\n{detail}\n")?;
    }
    writeln!(file, "----------------------------------------\n")?;
    Ok(())
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

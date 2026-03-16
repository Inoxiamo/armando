use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

use crate::app_paths;

const RETENTION_DAYS: i64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub created_at: String,
    pub backend: String,
    pub prompt: String,
    pub response: String,
}

pub fn append_entry(entry: HistoryEntry) -> anyhow::Result<()> {
    let mut entries = load_entries()?;
    entries.push(entry);
    prune_old_entries(&mut entries);
    write_entries(&entries)
}

pub fn recent_entries() -> anyhow::Result<Vec<HistoryEntry>> {
    let mut entries = load_entries()?;
    prune_old_entries(&mut entries);
    write_entries(&entries)?;
    entries.reverse();
    Ok(entries)
}

pub fn delete_entries(ids: &[String]) -> anyhow::Result<()> {
    let mut entries = load_entries()?;
    entries.retain(|entry| !ids.iter().any(|id| id == &entry_id(entry)));
    write_entries(&entries)
}

pub fn history_file_path() -> anyhow::Result<PathBuf> {
    history_path()
}

pub fn new_entry(backend: &str, prompt: &str, response: &str) -> anyhow::Result<HistoryEntry> {
    Ok(HistoryEntry {
        created_at: OffsetDateTime::now_utc().format(&Rfc3339)?,
        backend: backend.to_string(),
        prompt: prompt.to_string(),
        response: response.to_string(),
    })
}

fn load_entries() -> anyhow::Result<Vec<HistoryEntry>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path)
        .with_context(|| format!("Failed to open history file at {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn write_entries(entries: &[HistoryEntry]) -> anyhow::Result<()> {
    let path = history_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .with_context(|| format!("Failed to write history file at {}", path.display()))?;

    for entry in entries {
        writeln!(file, "{}", serde_json::to_string(entry)?)?;
    }

    Ok(())
}

fn history_path() -> anyhow::Result<PathBuf> {
    app_paths::history_file_path()
}

fn prune_old_entries(entries: &mut Vec<HistoryEntry>) {
    let cutoff = OffsetDateTime::now_utc() - Duration::days(RETENTION_DAYS);
    entries.retain(|entry| {
        OffsetDateTime::parse(&entry.created_at, &Rfc3339)
            .map(|timestamp| timestamp >= cutoff)
            .unwrap_or(false)
    });
}

pub fn entry_id(entry: &HistoryEntry) -> String {
    format!(
        "{}::{}::{}::{}",
        entry.created_at, entry.backend, entry.prompt, entry.response
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_at(offset_days: i64, label: &str) -> HistoryEntry {
        HistoryEntry {
            created_at: (OffsetDateTime::now_utc() - Duration::days(offset_days))
                .format(&Rfc3339)
                .unwrap(),
            backend: "ollama".to_string(),
            prompt: format!("prompt-{label}"),
            response: format!("response-{label}"),
        }
    }

    #[test]
    fn prune_old_entries_keeps_recent_entries() {
        let mut entries = vec![entry_at(1, "recent"), entry_at(RETENTION_DAYS - 1, "edge")];

        prune_old_entries(&mut entries);

        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn prune_old_entries_removes_expired_and_invalid_entries() {
        let mut entries = vec![
            entry_at(1, "recent"),
            entry_at(RETENTION_DAYS + 1, "old"),
            HistoryEntry {
                created_at: "not-a-date".to_string(),
                backend: "ollama".to_string(),
                prompt: "bad".to_string(),
                response: "bad".to_string(),
            },
        ];

        prune_old_entries(&mut entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].prompt, "prompt-recent");
    }

    #[test]
    fn new_entry_preserves_payload() {
        let entry = new_entry("gemini", "prompt", "response").unwrap();

        assert_eq!(entry.backend, "gemini");
        assert_eq!(entry.prompt, "prompt");
        assert_eq!(entry.response, "response");
        assert!(!entry.created_at.is_empty());
    }
}

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

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
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .or_else(dirs::config_dir)
        .ok_or_else(|| {
            anyhow::anyhow!("Could not determine a writable application data directory")
        })?;

    Ok(base.join("test-popup-ai").join("history.jsonl"))
}

fn prune_old_entries(entries: &mut Vec<HistoryEntry>) {
    let cutoff = OffsetDateTime::now_utc() - Duration::days(RETENTION_DAYS);
    entries.retain(|entry| {
        OffsetDateTime::parse(&entry.created_at, &Rfc3339)
            .map(|timestamp| timestamp >= cutoff)
            .unwrap_or(false)
    });
}

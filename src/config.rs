use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatGptConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_name", alias = "preset")]
    pub name: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default = "default_hotkey")]
    pub hotkey: String,

    pub aliases: Option<HashMap<String, String>>,

    // New Settings
    #[serde(default = "default_auto_read_selection")]
    pub auto_read_selection: bool,
    #[serde(default = "default_paste_response_shortcut")]
    pub paste_response_shortcut: String,

    #[serde(default = "default_backend")]
    pub default_backend: String,
    #[serde(default)]
    pub theme: ThemeConfig,
    pub gemini: Option<GeminiConfig>,
    pub chatgpt: Option<ChatGptConfig>,
    pub ollama: Option<OllamaConfig>,
    #[serde(skip)]
    pub loaded_from: Option<PathBuf>,
}

fn default_hotkey() -> String {
    "<ctrl>+<space>".to_string()
}

fn default_auto_read_selection() -> bool {
    true
}

fn default_paste_response_shortcut() -> String {
    "<ctrl>+<enter>".to_string()
}

fn default_backend() -> String {
    "ollama".to_string()
}

fn default_theme_name() -> String {
    "nerv-hud".to_string()
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut paths_to_try = Vec::new();
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                // Prefer a config shipped next to the executable.
                paths_to_try.push(parent.join("config.yaml"));
                // Fallback for cargo run (target/debug/...)
                if let Some(grandparent) = parent.parent().and_then(|p| p.parent()) {
                    paths_to_try.push(grandparent.join("config.yaml"));
                }
            }
        }

        paths_to_try.push(std::env::current_dir()?.join("config.yaml"));

        if let Some(config_dir) = dirs::config_dir() {
            paths_to_try.push(config_dir.join("test-popup-ai").join("config.yaml"));
        }

        for path in paths_to_try {
            if path.exists() {
                log::info!("Loading config from {}", path.display());
                let content = std::fs::read_to_string(&path)?;
                let mut config: Self = serde_yaml::from_str(&content)?;
                config.loaded_from = Some(path);
                return Ok(config);
            }
        }

        anyhow::bail!("config.yaml not found in any of the expected locations.");
    }
}

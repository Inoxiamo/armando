use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::app_paths;

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
pub struct ClaudeConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_name", alias = "preset")]
    pub name: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UiConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_window_height")]
    pub window_height: f32,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct HistoryConfig {
    #[serde(default)]
    pub enabled: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            window_height: default_window_height(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_theme_name(),
            path: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub aliases: Option<HashMap<String, String>>,

    #[serde(default = "default_auto_read_selection")]
    pub auto_read_selection: bool,

    #[serde(default = "default_backend")]
    pub default_backend: String,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub history: HistoryConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub gemini: Option<GeminiConfig>,
    pub chatgpt: Option<ChatGptConfig>,
    pub claude: Option<ClaudeConfig>,
    pub ollama: Option<OllamaConfig>,
    #[serde(skip)]
    pub loaded_from: Option<PathBuf>,
}

fn default_auto_read_selection() -> bool {
    true
}

fn default_backend() -> String {
    "ollama".to_string()
}

fn default_theme_name() -> String {
    "default-dark".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_window_height() -> f32 {
    640.0
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        for path in app_paths::candidate_config_paths()? {
            if path.exists() {
                log::info!("Loading config from {}", path.display());
                let content = std::fs::read_to_string(&path)?;
                let mut config: Self = serde_yaml::from_str(&content)?;
                config.loaded_from = Some(path);
                return Ok(config);
            }
        }

        anyhow::bail!(
            "No configuration file found. Expected `configs/default.yaml` or legacy `config.yaml` in the standard application locations."
        );
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let path = self
            .loaded_from
            .clone()
            .unwrap_or(app_paths::default_config_path()?);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut serializable = self.clone();
        serializable.loaded_from = None;
        let content = serde_yaml::to_string(&serializable)?;
        std::fs::write(&path, content)?;
        self.loaded_from = Some(path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_config_defaults_to_default_dark() {
        let theme = ThemeConfig::default();
        assert_eq!(theme.name, "default-dark");
        assert!(theme.path.is_none());
    }

    #[test]
    fn theme_config_supports_legacy_preset_alias() {
        let yaml = "theme:\n  preset: nerv-magi-system\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.theme.name, "nerv-magi-system");
    }

    #[test]
    fn config_defaults_are_applied_when_fields_are_missing() {
        let yaml = "{}";
        let config: Config = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.default_backend, "ollama");
        assert!(config.auto_read_selection);
        assert_eq!(config.theme.name, "default-dark");
        assert_eq!(config.ui.language, "en");
        assert_eq!(config.ui.window_height, 640.0);
        assert!(!config.history.enabled);
        assert!(!config.logging.enabled);
    }
}

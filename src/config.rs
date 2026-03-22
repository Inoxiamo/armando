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

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RagMode {
    Keyword,
    #[default]
    Vector,
    Hybrid,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RagRuntimeOverride {
    #[default]
    Default,
    ForceOn,
    ForceOff,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RagConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub mode: RagMode,
    #[serde(default)]
    pub runtime_override: RagRuntimeOverride,
    pub documents_folder: Option<PathBuf>,
    #[serde(default = "default_rag_vector_db_path")]
    pub vector_db_path: PathBuf,
    #[serde(default = "default_rag_max_retrieved_docs")]
    pub max_retrieved_docs: usize,
    #[serde(default = "default_rag_chunk_size")]
    pub chunk_size: usize,
    #[serde(default)]
    pub embedding_backend: Option<String>,
    #[serde(default)]
    pub embedding_model: Option<String>,
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

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct UpdateConfig {
    #[serde(default)]
    pub beta: bool,
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
    #[serde(default)]
    pub update: UpdateConfig,
    #[serde(default)]
    pub rag: RagConfig,
    pub gemini: Option<GeminiConfig>,
    pub chatgpt: Option<ChatGptConfig>,
    pub claude: Option<ClaudeConfig>,
    pub ollama: Option<OllamaConfig>,
    #[serde(skip)]
    pub loaded_from: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aliases: None,
            auto_read_selection: default_auto_read_selection(),
            default_backend: default_backend(),
            theme: ThemeConfig::default(),
            ui: UiConfig::default(),
            history: HistoryConfig::default(),
            logging: LoggingConfig::default(),
            update: UpdateConfig::default(),
            rag: RagConfig::default(),
            gemini: None,
            chatgpt: None,
            claude: None,
            ollama: None,
            loaded_from: None,
        }
    }
}

fn default_auto_read_selection() -> bool {
    true
}

fn default_backend() -> String {
    "gemini".to_string()
}

fn default_theme_name() -> String {
    "default-dark".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_window_height() -> f32 {
    600.0
}

fn default_rag_vector_db_path() -> PathBuf {
    PathBuf::from(".armando-rag.sqlite3")
}

fn default_rag_max_retrieved_docs() -> usize {
    4
}

fn default_rag_chunk_size() -> usize {
    1200
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: RagMode::Vector,
            runtime_override: RagRuntimeOverride::Default,
            documents_folder: None,
            vector_db_path: default_rag_vector_db_path(),
            max_retrieved_docs: default_rag_max_retrieved_docs(),
            chunk_size: default_rag_chunk_size(),
            embedding_backend: None,
            embedding_model: None,
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();

        for path in app_paths::candidate_config_paths()? {
            if path.exists() {
                log::info!("Loading config from {}", path.display());
                let content = std::fs::read_to_string(&path)?;
                let mut config: Self = serde_yaml::from_str(&content)?;
                apply_env_overrides(&mut config);
                config.loaded_from = Some(path);
                return Ok(config);
            }
        }

        if let Some(template_path) = app_paths::bundled_default_config_template_path()? {
            log::warn!(
                "No config file found, falling back to bundled defaults from {}",
                template_path.display()
            );

            match std::fs::read_to_string(&template_path)
                .ok()
                .and_then(|content| serde_yaml::from_str::<Self>(&content).ok())
            {
                Some(mut config) => {
                    apply_env_overrides(&mut config);
                    return Ok(config);
                }
                None => {
                    log::warn!(
                        "Bundled default config at {} could not be loaded, falling back to built-in defaults",
                        template_path.display()
                    );
                }
            }
        } else {
            log::warn!("No config file found and no bundled defaults were available");
        }

        let mut config = Self::default();
        apply_env_overrides(&mut config);
        Ok(config)
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

    pub fn load_template(template_name: &str) -> anyhow::Result<Option<Self>> {
        let Some(path) = app_paths::bundled_config_template_path(template_name)? else {
            return Ok(None);
        };

        let content = std::fs::read_to_string(&path)?;
        let mut config: Self = serde_yaml::from_str(&content)?;
        config.loaded_from = Some(path);
        Ok(Some(config))
    }
}

fn first_env(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| !is_placeholder_env_value(value))
}

fn is_placeholder_env_value(value: &str) -> bool {
    let normalized = value.trim().trim_matches('"').to_ascii_uppercase();
    normalized.starts_with("YOUR_")
}

fn apply_env_overrides(config: &mut Config) {
    if let Some(api_key) = first_env(&["ARMANDO_GEMINI_API_KEY", "GEMINI_API_KEY"]) {
        let section = config.gemini.get_or_insert(GeminiConfig {
            api_key: String::new(),
            model: "gemini-1.5-flash".to_string(),
        });
        section.api_key = api_key;
    }

    if let Some(api_key) = first_env(&["ARMANDO_OPENAI_API_KEY", "OPENAI_API_KEY"]) {
        let section = config.chatgpt.get_or_insert(ChatGptConfig {
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
        });
        section.api_key = api_key;
    }

    if let Some(api_key) = first_env(&["ARMANDO_ANTHROPIC_API_KEY", "ANTHROPIC_API_KEY"]) {
        let section = config.claude.get_or_insert(ClaudeConfig {
            api_key: String::new(),
            model: "claude-3-5-sonnet-latest".to_string(),
        });
        section.api_key = api_key;
    }

    if let Some(documents_folder) = first_env(&["ARMANDO_RAG_DOCUMENTS_FOLDER"]) {
        config.rag.documents_folder = Some(PathBuf::from(documents_folder));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

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

        assert_eq!(config.default_backend, "gemini");
        assert!(config.auto_read_selection);
        assert_eq!(config.theme.name, "default-dark");
        assert_eq!(config.ui.language, "en");
        assert_eq!(config.ui.window_height, 600.0);
        assert!(!config.history.enabled);
        assert!(!config.logging.enabled);
        assert!(!config.update.beta);
        assert!(!config.rag.enabled);
        assert_eq!(config.rag.mode, RagMode::Vector);
        assert_eq!(config.rag.runtime_override, RagRuntimeOverride::Default);
        assert!(config.rag.documents_folder.is_none());
        assert_eq!(
            config.rag.vector_db_path,
            PathBuf::from(".armando-rag.sqlite3")
        );
        assert_eq!(config.rag.max_retrieved_docs, 4);
        assert_eq!(config.rag.chunk_size, 1200);
        assert!(config.rag.embedding_backend.is_none());
        assert!(config.rag.embedding_model.is_none());
    }

    #[test]
    fn rag_config_deserializes_mode_and_embedding_overrides() {
        let yaml = r#"
rag:
  mode: hybrid
  runtime_override: force_on
  embedding_backend: chatgpt
  embedding_model: text-embedding-3-large
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.rag.mode, RagMode::Hybrid);
        assert_eq!(config.rag.runtime_override, RagRuntimeOverride::ForceOn);
        assert_eq!(config.rag.embedding_backend.as_deref(), Some("chatgpt"));
        assert_eq!(
            config.rag.embedding_model.as_deref(),
            Some("text-embedding-3-large")
        );
    }

    #[test]
    fn load_template_returns_none_when_template_is_missing() {
        let template = Config::load_template("missing-template").unwrap();
        assert!(template.is_none());
    }

    #[test]
    fn env_overrides_are_applied_to_api_keys() {
        let _guard = env_lock();
        std::env::set_var("ARMANDO_OPENAI_API_KEY", "from-env-openai");
        std::env::set_var("ARMANDO_GEMINI_API_KEY", "from-env-gemini");
        std::env::set_var("ARMANDO_ANTHROPIC_API_KEY", "from-env-claude");

        let mut config = Config::default();
        apply_env_overrides(&mut config);

        assert_eq!(
            config.chatgpt.as_ref().map(|cfg| cfg.api_key.as_str()),
            Some("from-env-openai")
        );
        assert_eq!(
            config.gemini.as_ref().map(|cfg| cfg.api_key.as_str()),
            Some("from-env-gemini")
        );
        assert_eq!(
            config.claude.as_ref().map(|cfg| cfg.api_key.as_str()),
            Some("from-env-claude")
        );

        std::env::remove_var("ARMANDO_OPENAI_API_KEY");
        std::env::remove_var("ARMANDO_GEMINI_API_KEY");
        std::env::remove_var("ARMANDO_ANTHROPIC_API_KEY");
    }

    #[test]
    fn env_overrides_ignore_placeholder_values() {
        let _guard = env_lock();
        std::env::set_var("ARMANDO_OPENAI_API_KEY", "YOUR_OPENAI_API_KEY");
        std::env::set_var("ARMANDO_GEMINI_API_KEY", "YOUR_GEMINI_API_KEY");
        std::env::set_var("ARMANDO_ANTHROPIC_API_KEY", "YOUR_ANTHROPIC_API_KEY");

        let mut config = Config::default();
        config.chatgpt = Some(ChatGptConfig {
            api_key: "keep-openai".to_string(),
            model: "gpt-4o-mini".to_string(),
        });
        config.gemini = Some(GeminiConfig {
            api_key: "keep-gemini".to_string(),
            model: "gemini-1.5-flash".to_string(),
        });
        config.claude = Some(ClaudeConfig {
            api_key: "keep-claude".to_string(),
            model: "claude-3-5-sonnet-latest".to_string(),
        });

        apply_env_overrides(&mut config);

        assert_eq!(
            config.chatgpt.as_ref().map(|value| value.api_key.as_str()),
            Some("keep-openai")
        );
        assert_eq!(
            config.gemini.as_ref().map(|value| value.api_key.as_str()),
            Some("keep-gemini")
        );
        assert_eq!(
            config.claude.as_ref().map(|value| value.api_key.as_str()),
            Some("keep-claude")
        );

        std::env::remove_var("ARMANDO_OPENAI_API_KEY");
        std::env::remove_var("ARMANDO_GEMINI_API_KEY");
        std::env::remove_var("ARMANDO_ANTHROPIC_API_KEY");
    }
}

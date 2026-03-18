use crate::app_paths;
use crate::config::Config;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericPromptTag {
    pub instruction: String,
    pub strip_header: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptProfiles {
    pub text_assist_tags: HashMap<String, String>,
    pub generic_question_tags: HashMap<String, GenericPromptTag>,
}

#[derive(Debug, Default, Deserialize)]
struct PromptTagsFile {
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
struct GenericPromptsFile {
    #[serde(default)]
    tags: HashMap<String, GenericPromptTagDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum GenericPromptTagDef {
    Simple(String),
    Structured {
        instruction: String,
        #[serde(default)]
        strip_header: bool,
    },
}

impl GenericPromptTagDef {
    fn into_runtime(self) -> GenericPromptTag {
        match self {
            Self::Simple(instruction) => GenericPromptTag {
                instruction,
                strip_header: false,
            },
            Self::Structured {
                instruction,
                strip_header,
            } => GenericPromptTag {
                instruction,
                strip_header,
            },
        }
    }
}

impl PromptProfiles {
    pub fn load(config: &Config) -> anyhow::Result<Self> {
        let mut text_assist_tags = built_in_text_assist_tags();
        merge_legacy_aliases(&mut text_assist_tags, config.aliases.as_ref());

        if let Some(path) = find_existing_path(app_paths::candidate_prompt_tags_paths(
            config.loaded_from.as_deref(),
        )?) {
            let content = std::fs::read_to_string(&path)?;
            let parsed: PromptTagsFile = serde_yaml::from_str(&content).map_err(|err| {
                anyhow::anyhow!("Could not parse prompt tags file {}: {err}", path.display())
            })?;
            merge_text_assist_tags(&mut text_assist_tags, parsed.tags);
        }

        let mut generic_question_tags = built_in_generic_question_tags();
        if let Some(path) = find_existing_path(app_paths::candidate_generic_prompt_paths(
            config.loaded_from.as_deref(),
        )?) {
            let content = std::fs::read_to_string(&path)?;
            let parsed: GenericPromptsFile = serde_yaml::from_str(&content).map_err(|err| {
                anyhow::anyhow!(
                    "Could not parse generic prompt presets file {}: {err}",
                    path.display()
                )
            })?;
            merge_generic_question_tags(&mut generic_question_tags, parsed.tags);
        }

        Ok(Self {
            text_assist_tags,
            generic_question_tags,
        })
    }

    pub fn default_built_in() -> Self {
        Self {
            text_assist_tags: built_in_text_assist_tags(),
            generic_question_tags: built_in_generic_question_tags(),
        }
    }
}

fn merge_legacy_aliases(
    target: &mut HashMap<String, String>,
    aliases: Option<&HashMap<String, String>>,
) {
    for (tag, instruction) in aliases.into_iter().flatten() {
        let normalized = normalize_tag(tag);
        if normalized.is_empty() {
            continue;
        }
        let instruction = instruction.trim();
        if instruction.is_empty() {
            continue;
        }
        target.insert(normalized, instruction.to_string());
    }
}

fn merge_text_assist_tags(target: &mut HashMap<String, String>, new_tags: HashMap<String, String>) {
    for (tag, instruction) in new_tags {
        let normalized = normalize_tag(&tag);
        if normalized.is_empty() {
            continue;
        }
        let instruction = instruction.trim();
        if instruction.is_empty() {
            continue;
        }
        target.insert(normalized, instruction.to_string());
    }
}

fn merge_generic_question_tags(
    target: &mut HashMap<String, GenericPromptTag>,
    new_tags: HashMap<String, GenericPromptTagDef>,
) {
    for (tag, preset) in new_tags {
        let normalized = normalize_tag(&tag);
        if normalized.is_empty() {
            continue;
        }
        let runtime = preset.into_runtime();
        if runtime.instruction.trim().is_empty() {
            continue;
        }
        target.insert(
            normalized,
            GenericPromptTag {
                instruction: runtime.instruction.trim().to_string(),
                strip_header: runtime.strip_header,
            },
        );
    }
}

fn normalize_tag(tag: &str) -> String {
    tag.trim().to_uppercase()
}

fn find_existing_path(paths: Vec<std::path::PathBuf>) -> Option<std::path::PathBuf> {
    paths.into_iter().find(|path| path.exists())
}

fn built_in_text_assist_tags() -> HashMap<String, String> {
    HashMap::from([
        (
            "GMAIL".to_string(),
            "Scrivi o riformula il testo come email professionale, chiara e naturale.".to_string(),
        ),
        (
            "EMAIL".to_string(),
            "Scrivi o riformula il testo come email professionale, chiara e naturale.".to_string(),
        ),
        (
            "MAIL".to_string(),
            "Scrivi o riformula il testo come email professionale, chiara e naturale.".to_string(),
        ),
        (
            "SLACK".to_string(),
            "Scrivi o riformula il testo come messaggio Slack breve, operativo e naturale."
                .to_string(),
        ),
        (
            "WHATSAPP".to_string(),
            "Scrivi o riformula il testo come messaggio WhatsApp diretto, semplice e colloquiale."
                .to_string(),
        ),
        (
            "ITA".to_string(),
            "Traduci o riscrivi il risultato finale in italiano.".to_string(),
        ),
        (
            "ENG".to_string(),
            "Translate or rewrite the final result in English.".to_string(),
        ),
        (
            "FORMAL".to_string(),
            "Usa un tono formale e professionale.".to_string(),
        ),
        (
            "CASUAL".to_string(),
            "Usa un tono informale e naturale.".to_string(),
        ),
        (
            "WORK".to_string(),
            "Mantieni un contesto professionale e orientato al lavoro.".to_string(),
        ),
        (
            "SHORT".to_string(),
            "Mantieni il risultato breve e sintetico.".to_string(),
        ),
        (
            "LONG".to_string(),
            "Puoi essere piu completo, ma resta diretto.".to_string(),
        ),
        (
            "CMD".to_string(),
            "La risposta finale deve essere orientata a un comando eseguibile.".to_string(),
        ),
    ])
}

fn built_in_generic_question_tags() -> HashMap<String, GenericPromptTag> {
    HashMap::from([(
        "CMD".to_string(),
        GenericPromptTag {
            instruction: "Se la risposta richiesta e un comando o una one-liner da terminale, restituisci solo il comando finale, senza markdown, senza backtick e senza testo aggiuntivo.".to_string(),
            strip_header: true,
        },
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, HistoryConfig, LoggingConfig, ThemeConfig, UiConfig};
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

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
            history: HistoryConfig::default(),
            logging: LoggingConfig::default(),
            gemini: None,
            chatgpt: None,
            claude: None,
            ollama: None,
            loaded_from: None,
        }
    }

    fn write_yaml(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "armando-{label}-{}-{timestamp}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn default_builtins_include_existing_tags() {
        let profiles = PromptProfiles::default_built_in();
        assert!(profiles.text_assist_tags.contains_key("ITA"));
        assert!(profiles.text_assist_tags.contains_key("EMAIL"));
        assert!(profiles.generic_question_tags.contains_key("CMD"));
    }

    #[test]
    fn load_uses_legacy_aliases_when_no_dedicated_file_exists() {
        let profiles = PromptProfiles::load(&test_config()).unwrap();
        assert_eq!(
            profiles.text_assist_tags.get("TITLE"),
            Some(&"Trasforma il testo in un titolo breve.".to_string())
        );
    }

    #[test]
    fn dedicated_prompt_tags_override_legacy_aliases() {
        let temp_dir = unique_temp_dir("prompt-tags");
        std::fs::create_dir_all(temp_dir.join("configs")).unwrap();
        write_yaml(
            &temp_dir.join("configs/default.yaml"),
            "default_backend: ollama\n",
        );
        write_yaml(
            &temp_dir.join("prompt-tags.yaml"),
            "tags:\n  TITLE: \"Nuovo titolo\"\n  EXTRA: \"Extra\"\n",
        );

        let mut config = test_config();
        config.loaded_from = Some(temp_dir.join("configs/default.yaml"));

        let profiles = PromptProfiles::load(&config).unwrap();
        assert_eq!(
            profiles.text_assist_tags.get("TITLE"),
            Some(&"Nuovo titolo".to_string())
        );
        assert_eq!(
            profiles.text_assist_tags.get("EXTRA"),
            Some(&"Extra".to_string())
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn generic_prompt_file_can_override_cmd_and_add_new_tag() {
        let temp_dir = unique_temp_dir("generic-prompts");
        std::fs::create_dir_all(temp_dir.join("configs")).unwrap();
        write_yaml(
            &temp_dir.join("configs/default.yaml"),
            "default_backend: ollama\n",
        );
        write_yaml(
            &temp_dir.join("generic-prompts.yaml"),
            "tags:\n  CMD:\n    instruction: \"Solo comando shell\"\n    strip_header: true\n  SQL:\n    instruction: \"Rispondi con SQL puro\"\n    strip_header: true\n",
        );

        let mut config = test_config();
        config.loaded_from = Some(temp_dir.join("configs/default.yaml"));

        let profiles = PromptProfiles::load(&config).unwrap();
        assert_eq!(
            profiles
                .generic_question_tags
                .get("CMD")
                .unwrap()
                .instruction,
            "Solo comando shell"
        );
        assert!(
            profiles
                .generic_question_tags
                .get("CMD")
                .unwrap()
                .strip_header
        );
        assert_eq!(
            profiles.generic_question_tags.get("SQL").unwrap(),
            &GenericPromptTag {
                instruction: "Rispondi con SQL puro".to_string(),
                strip_header: true,
            }
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn invalid_prompt_tags_yaml_returns_error() {
        let temp_dir = unique_temp_dir("invalid-prompt-tags");
        std::fs::create_dir_all(temp_dir.join("configs")).unwrap();
        write_yaml(
            &temp_dir.join("configs/default.yaml"),
            "default_backend: ollama\n",
        );
        write_yaml(&temp_dir.join("prompt-tags.yaml"), "tags: [broken");

        let mut config = test_config();
        config.loaded_from = Some(temp_dir.join("configs/default.yaml"));

        let err = PromptProfiles::load(&config).unwrap_err();
        assert!(err.to_string().contains("Could not parse prompt tags file"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

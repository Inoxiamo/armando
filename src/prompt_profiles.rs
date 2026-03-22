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
    pub language_tags: HashMap<String, String>,
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
            language_tags: built_in_language_tags(),
        })
    }

    pub fn default_built_in() -> Self {
        Self {
            text_assist_tags: built_in_text_assist_tags(),
            generic_question_tags: built_in_generic_question_tags(),
            language_tags: built_in_language_tags(),
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
            "Write or rewrite the text as a professional, clear, and natural email.".to_string(),
        ),
        (
            "EMAIL".to_string(),
            "Write or rewrite the text as a professional, clear, and natural email.".to_string(),
        ),
        (
            "MAIL".to_string(),
            "Write or rewrite the text as a professional, clear, and natural email.".to_string(),
        ),
        (
            "SLACK".to_string(),
            "Write or rewrite the text as a short, natural, action-oriented Slack message."
                .to_string(),
        ),
        (
            "WHATSAPP".to_string(),
            "Write or rewrite the text as a direct, simple, conversational WhatsApp message."
                .to_string(),
        ),
        (
            "FORMAL".to_string(),
            "Use a formal and professional tone.".to_string(),
        ),
        (
            "CASUAL".to_string(),
            "Use an informal and natural tone.".to_string(),
        ),
        (
            "WORK".to_string(),
            "Keep the output professional and work-oriented.".to_string(),
        ),
        (
            "SHORT".to_string(),
            "Keep the final result short and concise.".to_string(),
        ),
        (
            "LONG".to_string(),
            "You may be more complete, but stay direct and useful.".to_string(),
        ),
        (
            "CMD".to_string(),
            "Shape the final output as an executable command or command-oriented result."
                .to_string(),
        ),
    ])
}

fn built_in_generic_question_tags() -> HashMap<String, GenericPromptTag> {
    HashMap::from([(
        "CMD".to_string(),
        GenericPromptTag {
            instruction: "If the requested answer is a shell command or terminal one-liner, return only the final command, with no Markdown, no backticks, and no extra text.".to_string(),
            strip_header: true,
        },
    )])
}

fn built_in_language_tags() -> HashMap<String, String> {
    HashMap::from([
        ("EN".to_string(), "English".to_string()),
        ("ENG".to_string(), "English".to_string()),
        ("ENGLISH".to_string(), "English".to_string()),
        ("IT".to_string(), "Italian".to_string()),
        ("ITA".to_string(), "Italian".to_string()),
        ("ITALIAN".to_string(), "Italian".to_string()),
        ("ES".to_string(), "Spanish".to_string()),
        ("ESP".to_string(), "Spanish".to_string()),
        ("SPA".to_string(), "Spanish".to_string()),
        ("SPANISH".to_string(), "Spanish".to_string()),
        ("FR".to_string(), "French".to_string()),
        ("FRA".to_string(), "French".to_string()),
        ("FRE".to_string(), "French".to_string()),
        ("FRENCH".to_string(), "French".to_string()),
        ("DE".to_string(), "German".to_string()),
        ("DEU".to_string(), "German".to_string()),
        ("GER".to_string(), "German".to_string()),
        ("GERMAN".to_string(), "German".to_string()),
        ("JA".to_string(), "Japanese".to_string()),
        ("JP".to_string(), "Japanese".to_string()),
        ("JPN".to_string(), "Japanese".to_string()),
        ("JAP".to_string(), "Japanese".to_string()),
        ("JAPANESE".to_string(), "Japanese".to_string()),
        ("PT".to_string(), "Portuguese".to_string()),
        ("POR".to_string(), "Portuguese".to_string()),
        ("PORTUGUESE".to_string(), "Portuguese".to_string()),
        ("BR".to_string(), "Brazilian Portuguese".to_string()),
        ("PTBR".to_string(), "Brazilian Portuguese".to_string()),
        ("BRAZILIAN".to_string(), "Brazilian Portuguese".to_string()),
        (
            "BRAZILIANPORTUGUESE".to_string(),
            "Brazilian Portuguese".to_string(),
        ),
        ("NL".to_string(), "Dutch".to_string()),
        ("NLD".to_string(), "Dutch".to_string()),
        ("DUT".to_string(), "Dutch".to_string()),
        ("DUTCH".to_string(), "Dutch".to_string()),
        ("PL".to_string(), "Polish".to_string()),
        ("POL".to_string(), "Polish".to_string()),
        ("POLISH".to_string(), "Polish".to_string()),
        ("RU".to_string(), "Russian".to_string()),
        ("RUS".to_string(), "Russian".to_string()),
        ("RUSSIAN".to_string(), "Russian".to_string()),
        ("UK".to_string(), "Ukrainian".to_string()),
        ("UKR".to_string(), "Ukrainian".to_string()),
        ("UKRAINIAN".to_string(), "Ukrainian".to_string()),
        ("CS".to_string(), "Czech".to_string()),
        ("CES".to_string(), "Czech".to_string()),
        ("CZE".to_string(), "Czech".to_string()),
        ("CZECH".to_string(), "Czech".to_string()),
        ("HU".to_string(), "Hungarian".to_string()),
        ("HUN".to_string(), "Hungarian".to_string()),
        ("HUNGARIAN".to_string(), "Hungarian".to_string()),
        ("RO".to_string(), "Romanian".to_string()),
        ("RON".to_string(), "Romanian".to_string()),
        ("RUM".to_string(), "Romanian".to_string()),
        ("ROMANIAN".to_string(), "Romanian".to_string()),
        ("EL".to_string(), "Greek".to_string()),
        ("GRE".to_string(), "Greek".to_string()),
        ("ELL".to_string(), "Greek".to_string()),
        ("GREEK".to_string(), "Greek".to_string()),
        ("TR".to_string(), "Turkish".to_string()),
        ("TUR".to_string(), "Turkish".to_string()),
        ("TURKISH".to_string(), "Turkish".to_string()),
        ("SV".to_string(), "Swedish".to_string()),
        ("SWE".to_string(), "Swedish".to_string()),
        ("SWEDISH".to_string(), "Swedish".to_string()),
        ("DA".to_string(), "Danish".to_string()),
        ("DAN".to_string(), "Danish".to_string()),
        ("DANISH".to_string(), "Danish".to_string()),
        ("NO".to_string(), "Norwegian".to_string()),
        ("NOR".to_string(), "Norwegian".to_string()),
        ("NORWEGIAN".to_string(), "Norwegian".to_string()),
        ("FI".to_string(), "Finnish".to_string()),
        ("FIN".to_string(), "Finnish".to_string()),
        ("FINNISH".to_string(), "Finnish".to_string()),
        ("AR".to_string(), "Arabic".to_string()),
        ("ARA".to_string(), "Arabic".to_string()),
        ("ARABIC".to_string(), "Arabic".to_string()),
        ("HE".to_string(), "Hebrew".to_string()),
        ("HEB".to_string(), "Hebrew".to_string()),
        ("HEBREW".to_string(), "Hebrew".to_string()),
        ("HI".to_string(), "Hindi".to_string()),
        ("HIN".to_string(), "Hindi".to_string()),
        ("HINDI".to_string(), "Hindi".to_string()),
        ("ZH".to_string(), "Chinese".to_string()),
        ("ZHO".to_string(), "Chinese".to_string()),
        ("CHI".to_string(), "Chinese".to_string()),
        ("CHINESE".to_string(), "Chinese".to_string()),
        ("ZHCN".to_string(), "Simplified Chinese".to_string()),
        (
            "SIMPLIFIEDCHINESE".to_string(),
            "Simplified Chinese".to_string(),
        ),
        ("ZHTW".to_string(), "Traditional Chinese".to_string()),
        (
            "TRADITIONALCHINESE".to_string(),
            "Traditional Chinese".to_string(),
        ),
        ("KO".to_string(), "Korean".to_string()),
        ("KOR".to_string(), "Korean".to_string()),
        ("KOREAN".to_string(), "Korean".to_string()),
        ("TH".to_string(), "Thai".to_string()),
        ("THA".to_string(), "Thai".to_string()),
        ("THAI".to_string(), "Thai".to_string()),
        ("VI".to_string(), "Vietnamese".to_string()),
        ("VIE".to_string(), "Vietnamese".to_string()),
        ("VIETNAMESE".to_string(), "Vietnamese".to_string()),
        ("ID".to_string(), "Indonesian".to_string()),
        ("IND".to_string(), "Indonesian".to_string()),
        ("INDONESIAN".to_string(), "Indonesian".to_string()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Config, HistoryConfig, LoggingConfig, RagConfig, ThemeConfig, UiConfig, UpdateConfig,
    };
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_config() -> Config {
        Config {
            aliases: Some(HashMap::from([(
                "TITLE".to_string(),
                "Trasforma il testo in un titolo breve.".to_string(),
            )])),
            auto_read_selection: true,
            default_backend: "gemini".to_string(),
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
            chatgpt_api_key_from_env: false,
            gemini_api_key_from_env: false,
            claude_api_key_from_env: false,
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
        assert!(profiles.text_assist_tags.contains_key("EMAIL"));
        assert!(profiles.generic_question_tags.contains_key("CMD"));
        assert_eq!(
            profiles.language_tags.get("ITA"),
            Some(&"Italian".to_string())
        );
        assert_eq!(
            profiles.language_tags.get("ESP"),
            Some(&"Spanish".to_string())
        );
        assert_eq!(
            profiles.language_tags.get("FRA"),
            Some(&"French".to_string())
        );
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

use std::collections::HashMap;

use serde::Deserialize;

use crate::app_paths;

#[derive(Debug, Clone, Deserialize)]
pub struct LocaleDefinition {
    pub code: String,
    pub name: String,
    pub strings: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct I18n {
    locale: LocaleDefinition,
    fallback: LocaleDefinition,
}

impl I18n {
    pub fn load(code: &str) -> anyhow::Result<Self> {
        let fallback = load_locale_file("en")?;
        let locale = if code == "en" {
            fallback.clone()
        } else {
            load_locale_file(code).unwrap_or_else(|_| fallback.clone())
        };

        Ok(Self { locale, fallback })
    }

    pub fn tr(&self, key: &str) -> String {
        self.locale
            .strings
            .get(key)
            .or_else(|| self.fallback.strings.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    pub fn code(&self) -> &str {
        &self.locale.code
    }

    pub fn language_name(&self) -> &str {
        &self.locale.name
    }
}

pub fn available_locales() -> anyhow::Result<Vec<LocaleDefinition>> {
    let mut locales = Vec::new();
    for code in app_paths::discover_named_files("locales", "yaml")? {
        if let Ok(locale) = load_locale_file(&code) {
            locales.push(locale);
        }
    }
    locales.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(locales)
}

fn load_locale_file(code: &str) -> anyhow::Result<LocaleDefinition> {
    for path in app_paths::candidate_locale_paths(code)? {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let locale: LocaleDefinition = serde_yaml::from_str(&content)?;
            return Ok(locale);
        }
    }

    anyhow::bail!("Locale '{}' not found.", code)
}

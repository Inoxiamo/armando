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
        let fallback = load_builtin_locale("en")?;
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
    locales.extend(load_builtin_locales()?);

    for code in app_paths::discover_named_files("locales", "yaml")? {
        if let Ok(locale) = load_locale_file(&code) {
            if let Some(existing) = locales
                .iter_mut()
                .find(|current| current.code == locale.code)
            {
                *existing = locale;
            } else {
                locales.push(locale);
            }
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

    load_builtin_locale(code)
}

fn load_builtin_locales() -> anyhow::Result<Vec<LocaleDefinition>> {
    Ok(vec![load_builtin_locale("en")?, load_builtin_locale("it")?])
}

fn load_builtin_locale(code: &str) -> anyhow::Result<LocaleDefinition> {
    let raw = match code {
        "en" => include_str!("../../locales/en.yaml"),
        "it" => include_str!("../../locales/it.yaml"),
        _ => anyhow::bail!("Locale '{code}' not found."),
    };

    let locale: LocaleDefinition = serde_yaml::from_str(raw)?;
    Ok(locale)
}

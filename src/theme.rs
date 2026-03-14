use std::path::{Path, PathBuf};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeDefinition {
    pub name: String,
    pub window_fill: String,
    pub panel_fill: String,
    pub panel_fill_soft: String,
    pub panel_fill_raised: String,
    pub accent_color: String,
    pub accent_hover_color: String,
    pub accent_text_color: String,
    pub text_color: String,
    pub weak_text_color: String,
    pub border_color: String,
    pub danger_color: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    pub window_fill: egui::Color32,
    pub panel_fill: egui::Color32,
    pub panel_fill_soft: egui::Color32,
    pub panel_fill_raised: egui::Color32,
    pub accent_color: egui::Color32,
    pub accent_hover_color: egui::Color32,
    pub accent_text_color: egui::Color32,
    pub text_color: egui::Color32,
    pub weak_text_color: egui::Color32,
    pub border_color: egui::Color32,
    pub danger_color: egui::Color32,
}

pub fn load_theme(config: &Config) -> anyhow::Result<ResolvedTheme> {
    let theme_cfg = &config.theme;

    if let Some(path) = &theme_cfg.path {
        let resolved = resolve_relative_to_config(path, config.loaded_from.as_deref());
        return load_theme_file(&resolved);
    }

    for path in candidate_theme_paths(&theme_cfg.name, config.loaded_from.as_deref()) {
        if path.exists() {
            return load_theme_file(&path);
        }
    }

    anyhow::bail!(
        "Theme '{}' not found. Looked for an external theme file in the standard themes directories.",
        theme_cfg.name
    )
}

fn load_theme_file(path: &Path) -> anyhow::Result<ResolvedTheme> {
    let content = std::fs::read_to_string(path)?;
    let theme: ThemeDefinition = serde_yaml::from_str(&content)?;
    theme.resolve()
}

fn resolve_relative_to_config(path: &Path, loaded_from: Option<&Path>) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    if let Some(config_path) = loaded_from {
        if let Some(parent) = config_path.parent() {
            return parent.join(path);
        }
    }

    path.to_path_buf()
}

fn candidate_theme_paths(name: &str, loaded_from: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let file_name = format!("{}.yaml", name);

    if let Some(config_path) = loaded_from {
        if let Some(parent) = config_path.parent() {
            paths.push(parent.join("themes").join(&file_name));
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            paths.push(parent.join("themes").join(&file_name));
            if let Some(grandparent) = parent.parent().and_then(|p| p.parent()) {
                paths.push(grandparent.join("themes").join(&file_name));
            }
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        paths.push(current_dir.join("themes").join(&file_name));
    }

    if let Some(config_dir) = dirs::config_dir() {
        paths.push(
            config_dir
                .join("test-popup-ai")
                .join("themes")
                .join(&file_name),
        );
    }

    paths
}

impl ThemeDefinition {
    fn resolve(self) -> anyhow::Result<ResolvedTheme> {
        Ok(ResolvedTheme {
            window_fill: parse_hex_color(&self.window_fill)?,
            panel_fill: parse_hex_color(&self.panel_fill)?,
            panel_fill_soft: parse_hex_color(&self.panel_fill_soft)?,
            panel_fill_raised: parse_hex_color(&self.panel_fill_raised)?,
            accent_color: parse_hex_color(&self.accent_color)?,
            accent_hover_color: parse_hex_color(&self.accent_hover_color)?,
            accent_text_color: parse_hex_color(&self.accent_text_color)?,
            text_color: parse_hex_color(&self.text_color)?,
            weak_text_color: parse_hex_color(&self.weak_text_color)?,
            border_color: parse_hex_color(&self.border_color)?,
            danger_color: parse_hex_color(&self.danger_color)?,
        })
    }
}

fn parse_hex_color(value: &str) -> anyhow::Result<egui::Color32> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 {
        anyhow::bail!("Invalid theme color '{}': expected 6 hex characters", value);
    }

    let r = u8::from_str_radix(&value[0..2], 16)?;
    let g = u8::from_str_radix(&value[2..4], 16)?;
    let b = u8::from_str_radix(&value[4..6], 16)?;
    Ok(egui::Color32::from_rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ThemeConfig;

    #[test]
    fn resolve_theme_definition_parses_all_colors() {
        let resolved = ThemeDefinition {
            name: "Test".to_string(),
            window_fill: "#010203".to_string(),
            panel_fill: "#111213".to_string(),
            panel_fill_soft: "#212223".to_string(),
            panel_fill_raised: "#313233".to_string(),
            accent_color: "#414243".to_string(),
            accent_hover_color: "#515253".to_string(),
            accent_text_color: "#616263".to_string(),
            text_color: "#717273".to_string(),
            weak_text_color: "#818283".to_string(),
            border_color: "#919293".to_string(),
            danger_color: "#A1A2A3".to_string(),
        }
        .resolve()
        .unwrap();

        assert_eq!(resolved.window_fill, egui::Color32::from_rgb(1, 2, 3));
        assert_eq!(
            resolved.accent_text_color,
            egui::Color32::from_rgb(97, 98, 99)
        );
        assert_eq!(
            resolved.danger_color,
            egui::Color32::from_rgb(161, 162, 163)
        );
    }

    #[test]
    fn parse_hex_color_rejects_invalid_length() {
        let err = parse_hex_color("#12345").unwrap_err().to_string();
        assert!(err.contains("expected 6 hex characters"));
    }

    #[test]
    fn resolve_relative_to_config_uses_config_parent() {
        let config_path = Path::new("/tmp/test-popup-ai/config.yaml");
        let resolved =
            resolve_relative_to_config(Path::new("themes/custom.yaml"), Some(config_path));

        assert_eq!(
            resolved,
            PathBuf::from("/tmp/test-popup-ai/themes/custom.yaml")
        );
    }

    #[test]
    fn candidate_theme_paths_include_config_relative_location() {
        let config_path = Path::new("/tmp/test-popup-ai/config.yaml");
        let paths = candidate_theme_paths("nerv-hud", Some(config_path));

        assert!(paths.contains(&PathBuf::from("/tmp/test-popup-ai/themes/nerv-hud.yaml")));
    }

    #[test]
    fn load_theme_uses_explicit_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("test-popup-ai-theme-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let theme_path = temp_dir.join("custom.yaml");
        std::fs::write(
            &theme_path,
            "\
name: Custom\n\
window_fill: \"#010203\"\n\
panel_fill: \"#111213\"\n\
panel_fill_soft: \"#212223\"\n\
panel_fill_raised: \"#313233\"\n\
accent_color: \"#414243\"\n\
accent_hover_color: \"#515253\"\n\
accent_text_color: \"#616263\"\n\
text_color: \"#717273\"\n\
weak_text_color: \"#818283\"\n\
border_color: \"#919293\"\n\
danger_color: \"#A1A2A3\"\n",
        )
        .unwrap();

        let config = Config {
            hotkey: "<ctrl>+<space>".to_string(),
            aliases: None,
            auto_read_selection: true,
            paste_response_shortcut: "<ctrl>+<enter>".to_string(),
            default_backend: "ollama".to_string(),
            theme: ThemeConfig {
                name: "ignored".to_string(),
                path: Some(theme_path.clone()),
            },
            gemini: None,
            chatgpt: None,
            ollama: None,
            loaded_from: None,
        };

        let theme = load_theme(&config).unwrap();
        assert_eq!(theme.panel_fill, egui::Color32::from_rgb(17, 18, 19));

        let _ = std::fs::remove_file(theme_path);
        let _ = std::fs::remove_dir(temp_dir);
    }
}

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

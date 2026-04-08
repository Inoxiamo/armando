use std::collections::HashSet;

use eframe::egui;

use crate::history::{self, HistoryEntry};
use crate::theme::ResolvedTheme;

use super::media_io;

#[allow(clippy::too_many_arguments)]
pub(super) fn history_entry_card(
    copy_label: &str,
    reuse_label: &str,
    select_label: &str,
    prompt_label: &str,
    response_label: &str,
    selectable: bool,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    theme: &ResolvedTheme,
    entry: &HistoryEntry,
    selected_history_entries: &mut HashSet<String>,
    prompt: &mut String,
    response: &mut String,
    show_history: &mut bool,
    prompt_focus_initialized: &mut bool,
    history_action_error: &mut Option<String>,
) {
    let entry_id = history::entry_id(entry);
    super::card_frame(ctx, theme.panel_fill_raised, theme.border_color).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if selectable {
                let mut is_selected = selected_history_entries.contains(&entry_id);
                if ui
                    .checkbox(&mut is_selected, "")
                    .on_hover_text(select_label)
                    .changed()
                {
                    if is_selected {
                        selected_history_entries.insert(entry_id.clone());
                    } else {
                        selected_history_entries.remove(&entry_id);
                    }
                }
            }
            ui.label(
                egui::RichText::new(entry.backend.to_uppercase())
                    .strong()
                    .monospace()
                    .color(ctx.style().visuals.hyperlink_color),
            );
            ui.label(
                egui::RichText::new(trim_timestamp(&entry.created_at))
                    .small()
                    .color(theme.weak_text_color),
            );
        });

        ui.add_space(6.0);
        ui.label(super::muted_label(prompt_label, theme.weak_text_color));
        ui.label(
            egui::RichText::new(trim_for_preview(&entry.prompt, 180))
                .strong()
                .small(),
        );
        ui.add_space(6.0);
        ui.label(super::muted_label(response_label, theme.weak_text_color));
        ui.label(
            egui::RichText::new(trim_for_preview(&entry.response, 260))
                .small()
                .color(theme.weak_text_color),
        );
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            let copy_button = super::secondary_action_button(copy_label, theme.panel_fill_soft);
            if ui.add(copy_button).clicked() {
                media_io::copy_markdown_rendered_text_to_clipboard(&entry.response);
                *history_action_error = None;
            }

            let reuse_button = super::primary_action_button(
                reuse_label,
                theme.accent_color,
                theme.accent_text_color,
            );
            if ui.add(reuse_button).clicked() {
                *prompt = entry.prompt.clone();
                *response = entry.response.clone();
                *show_history = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                    620.0, 420.0,
                )));
                *prompt_focus_initialized = false;
                *history_action_error = None;
                ctx.request_repaint();
            }
        });
    });
}

pub(super) fn open_history_file() -> anyhow::Result<()> {
    let path = history::history_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        std::fs::File::create(&path)?;
    }
    super::open_path_in_file_manager(&path)
}

fn trim_for_preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let mut result = String::new();
    for (index, ch) in trimmed.chars().enumerate() {
        if index >= max_chars {
            result.push_str("...");
            return result;
        }
        result.push(ch);
    }
    result
}

fn trim_timestamp(value: &str) -> String {
    value.replace('T', " ").replace('Z', "")
}

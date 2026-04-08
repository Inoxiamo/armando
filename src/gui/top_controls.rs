use eframe::egui;

use super::{layout, AiPopupApp, ToolbarIcon};

pub(super) fn render_top_controls(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let backend_label = app.tr("app.backend");
    let generic_mode_label = app.tr("app.generic_mode");
    let session_chat_label = app.tr("app.session_chat_mode");
    let settings_open_label = app.tr("app.settings");

    ui.horizontal(|ui| {
        ui.label(super::muted_label(
            &backend_label,
            app.theme.weak_text_color,
        ));
        let backend_button = super::dropdown_button_text(&app.selected_backend, &app.theme);
        super::dropdown_box_scope(ui, &app.theme, |ui| {
            egui::ComboBox::from_id_source("backend_combo")
                .selected_text(backend_button)
                .width(148.0)
                .show_ui(ui, |ui| {
                    super::apply_dropdown_menu_style(ui, &app.theme);
                    super::dropdown_option(ui, &mut app.selected_backend, "ollama", &app.theme);
                    super::dropdown_option(ui, &mut app.selected_backend, "chatgpt", &app.theme);
                    super::dropdown_option(ui, &mut app.selected_backend, "claude", &app.theme);
                    super::dropdown_option(ui, &mut app.selected_backend, "gemini", &app.theme);
                });
        });
        ui.add_space(6.0);
        ui.checkbox(&mut app.generic_question_mode, generic_mode_label);
        ui.add_space(6.0);
        ui.checkbox(&mut app.session_chat_enabled, session_chat_label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(layout::section_actions_right_inset());
            let gear = super::icon_action_button(
                app,
                ToolbarIcon::Settings,
                app.theme.panel_fill_soft,
                app.theme.text_color,
            );
            if ui.add(gear).on_hover_text(settings_open_label).clicked() {
                app.set_settings_visibility(ctx, !app.show_settings);
            }
        });
    });
}

use eframe::egui;

use super::AiPopupApp;

pub(super) fn render_settings_panel(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(super::section_label(
                &app.tr("app.settings"),
                app.theme.text_color,
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(super::icon_action_button(
                        app,
                        super::ToolbarIcon::Close,
                        app.theme.panel_fill_soft,
                        app.theme.text_color,
                    ))
                    .on_hover_text(app.tr("settings.close"))
                    .clicked()
                {
                    app.set_settings_visibility(ctx, false);
                }
            });
        });
        if let Some(path) = &app.config.loaded_from {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    app.tr_with("app.config_path", &[("path", path.display().to_string())]),
                )
                .small()
                .color(app.theme.weak_text_color),
            );
        }
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                super::render_general_settings_section(app, ctx, ui);
                settings_section_break(ui, app.theme.border_color);
                super::render_provider_settings_sections(app, ctx, ui);
                settings_section_break(ui, app.theme.border_color);
                super::render_startup_settings_section(app, ui);
                settings_section_break(ui, app.theme.border_color);
                super::render_history_debug_settings_section(app, ui);
                settings_section_break(ui, app.theme.border_color);
                super::render_rag_settings_section(app, ctx, ui);

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(super::muted_label(
                    &app.tr("settings.saved"),
                    app.theme.weak_text_color,
                ));
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} v{}",
                            app.tr("settings.version"),
                            super::display_version()
                        ))
                        .small()
                        .color(app.theme.weak_text_color),
                    );
                    ui.add_space(8.0);
                    super::render_update_status(app, ctx, ui);
                });
            });
    });
}

fn settings_section_break(ui: &mut egui::Ui, color: egui::Color32) {
    ui.add_space(2.0);
    let width = ui.available_width().max(12.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 2.0), egui::Sense::hover());
    let y = rect.center().y;
    let x_padding = 8.0;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + x_padding, y),
            egui::pos2(rect.right() - x_padding, y),
        ],
        egui::Stroke::new(1.0, color.gamma_multiply(0.35)),
    );
    ui.add_space(2.0);
}

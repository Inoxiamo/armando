use eframe::egui;

use super::{muted_label, AiPopupApp};

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
                render_general_settings_section(app, ctx, ui);
                settings_section_break(ui, app.theme.border_color);
                super::provider_settings::render_provider_settings_sections(app, ctx, ui);
                settings_section_break(ui, app.theme.border_color);
                render_startup_settings_section(app, ui);
                settings_section_break(ui, app.theme.border_color);
                render_history_debug_settings_section(app, ui);
                settings_section_break(ui, app.theme.border_color);
                super::rag_settings::render_rag_settings_section(app, ctx, ui);

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
                    super::update_status::render_update_status(app, ctx, ui);
                });
            });
    });
}

fn render_startup_settings_section(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(
        egui::RichText::new(app.tr("settings.health"))
            .color(app.theme.text_color)
            .strong(),
    )
    .id_source("settings_startup_health")
    .default_open(false)
    .show(ui, |ui| {
        super::startup_health::render_startup_health_section(app, ui);

        if app.config.loaded_from.is_none() {
            ui.add_space(12.0);
            super::startup_health::render_first_run_setup_section(app, ui);
        }
    });
}

fn render_general_settings_section(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(
        egui::RichText::new(app.tr("settings.general"))
            .color(app.theme.text_color)
            .strong(),
    )
    .id_source("settings_general")
    .default_open(false)
    .show(ui, |ui| {
        ui.label(muted_label(
            &app.tr("settings.language"),
            app.theme.weak_text_color,
        ));
        let current_language = app
            .available_locales
            .iter()
            .find(|locale| locale.code == app.i18n.code())
            .map(|locale| locale.name.clone())
            .unwrap_or_else(|| app.i18n.language_name().to_string());
        let dropdown_theme = app.theme.clone();
        super::dropdown_box_scope(ui, &dropdown_theme, |ui| {
            egui::ComboBox::from_id_source("settings_language")
                .selected_text(super::dropdown_button_text(
                    &current_language,
                    &dropdown_theme,
                ))
                .width(220.0)
                .show_ui(ui, |ui| {
                    super::apply_dropdown_menu_style(ui, &dropdown_theme);
                    let locales: Vec<(String, String)> = app
                        .available_locales
                        .iter()
                        .map(|locale| (locale.code.clone(), locale.name.clone()))
                        .collect();
                    for (code, name) in locales {
                        if ui
                            .selectable_label(
                                app.config.ui.language == code,
                                super::dropdown_item_text(&name, &dropdown_theme),
                            )
                            .clicked()
                        {
                            app.apply_language(&code);
                            app.persist_settings();
                        }
                    }
                });
        });

        ui.add_space(8.0);
        ui.label(muted_label(
            &app.tr("settings.theme"),
            app.theme.weak_text_color,
        ));
        let current_theme = app.config.theme.name.clone();
        let dropdown_theme = app.theme.clone();
        super::dropdown_box_scope(ui, &dropdown_theme, |ui| {
            egui::ComboBox::from_id_source("settings_theme")
                .selected_text(super::dropdown_button_text(&current_theme, &dropdown_theme))
                .width(220.0)
                .show_ui(ui, |ui| {
                    super::apply_dropdown_menu_style(ui, &dropdown_theme);
                    let themes = app.available_themes.clone();
                    for theme_name in themes {
                        if ui
                            .selectable_label(
                                app.config.theme.name == theme_name,
                                super::dropdown_item_text(&theme_name, &dropdown_theme),
                            )
                            .clicked()
                        {
                            app.apply_theme_by_name(ctx, &theme_name);
                            app.persist_settings();
                        }
                    }
                });
        });

        ui.add_space(8.0);
        ui.label(muted_label(
            &app.tr("settings.default_backend"),
            app.theme.weak_text_color,
        ));
        let dropdown_theme = app.theme.clone();
        super::dropdown_box_scope(ui, &dropdown_theme, |ui| {
            egui::ComboBox::from_id_source("settings_default_backend")
                .selected_text(super::dropdown_button_text(
                    &app.config.default_backend,
                    &dropdown_theme,
                ))
                .width(220.0)
                .show_ui(ui, |ui| {
                    super::apply_dropdown_menu_style(ui, &dropdown_theme);
                    for backend in ["ollama", "chatgpt", "claude", "gemini"] {
                        if ui
                            .selectable_label(
                                app.config.default_backend == backend,
                                super::dropdown_item_text(backend, &dropdown_theme),
                            )
                            .clicked()
                        {
                            app.config.default_backend = backend.to_string();
                            app.selected_backend = backend.to_string();
                            app.persist_settings();
                        }
                    }
                });
        });

        ui.add_space(8.0);
        let mut auto_read = app.config.auto_read_selection;
        if ui
            .checkbox(&mut auto_read, app.tr("settings.auto_read_selection"))
            .changed()
        {
            app.config.auto_read_selection = auto_read;
            app.persist_settings();
        }

        ui.add_space(8.0);
        let mut update_beta = app.config.update.beta;
        if ui
            .checkbox(&mut update_beta, app.tr("settings.update_beta_channel"))
            .changed()
        {
            app.config.update.beta = update_beta;
            app.persist_settings();
            app.start_release_check(ctx);
        }
        ui.label(muted_label(
            &app.tr("settings.update_beta_channel_hint"),
            app.theme.weak_text_color,
        ));
    });
}

fn render_history_debug_settings_section(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(
        egui::RichText::new(app.tr("settings.history_debug"))
            .color(app.theme.text_color)
            .strong(),
    )
    .id_source("settings_history_debug")
    .default_open(false)
    .show(ui, |ui| {
        let mut history_enabled = app.config.history.enabled;
        if ui
            .checkbox(&mut history_enabled, app.tr("settings.history_enabled"))
            .changed()
        {
            app.config.history.enabled = history_enabled;
            if !history_enabled {
                app.show_history = false;
            }
            app.reload_history();
            app.persist_settings();
        }
        ui.label(muted_label(
            &app.tr("settings.history_warning"),
            app.theme.weak_text_color,
        ));

        ui.add_space(8.0);
        let mut debug_logging = app.config.logging.enabled;
        if ui
            .checkbox(&mut debug_logging, app.tr("settings.debug_logging"))
            .changed()
        {
            app.config.logging.enabled = debug_logging;
            app.persist_settings();
        }
        ui.label(muted_label(
            &app.tr("settings.debug_logging_warning"),
            app.theme.weak_text_color,
        ));
    });
}

fn settings_section_break(ui: &mut egui::Ui, color: egui::Color32) {
    ui.add_space(1.0);
    let width = ui.available_width().max(12.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    let y = rect.center().y;
    let x_padding = 6.0;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + x_padding, y),
            egui::pos2(rect.right() - x_padding, y),
        ],
        egui::Stroke::new(1.0, color.gamma_multiply(0.48)),
    );
    ui.add_space(1.0);
}

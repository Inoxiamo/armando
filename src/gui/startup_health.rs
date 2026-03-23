use std::path::PathBuf;

use eframe::egui;

use crate::app_paths;
use crate::backends::{self, HealthCheck, HealthLevel};

use super::AiPopupApp;

pub(super) fn render_startup_health_section(app: &AiPopupApp, ui: &mut egui::Ui) {
    let diagnostics = backends::startup_health_checks(&app.config, &app.selected_backend);

    super::card_frame(
        ui.ctx(),
        app.theme.panel_fill_soft,
        app.theme.border_color.gamma_multiply(0.65),
    )
    .show(ui, |ui| {
        for (index, diagnostic) in diagnostics.iter().enumerate() {
            render_startup_health_row(app, ui, diagnostic);
            if index + 1 < diagnostics.len() {
                ui.add_space(8.0);
            }
        }
    });
}

pub(super) fn render_first_run_setup_section(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    let config_path = app_paths::default_config_path();
    let create_enabled = config_path.is_ok();
    let template_names = app_paths::discover_config_template_names().unwrap_or_default();

    super::card_frame(
        ui.ctx(),
        app.theme.panel_fill_soft,
        app.theme.border_color.gamma_multiply(0.65),
    )
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(super::section_label(
                &app.tr("startup.first_run_setup"),
                app.theme.text_color,
            ));
        });
        ui.add_space(6.0);

        match &config_path {
            Ok(path) => {
                ui.label(
                    egui::RichText::new(app.tr_with(
                        "startup.config_destination",
                        &[("path", path.display().to_string())],
                    ))
                    .small()
                    .color(app.theme.weak_text_color),
                );
            }
            Err(err) => {
                ui.colored_label(
                    app.theme.danger_color,
                    app.tr_with(
                        "startup.config_destination_error",
                        &[("error", err.to_string())],
                    ),
                );
            }
        }

        if !template_names.is_empty() {
            ui.add_space(6.0);
            ui.label(super::muted_label(
                &app.tr("startup.config_templates"),
                app.theme.weak_text_color,
            ));

            let template_theme = app.theme.clone();
            super::dropdown_box_scope(ui, &template_theme, |ui| {
                egui::ComboBox::from_id_source("startup_config_template")
                    .selected_text(super::dropdown_button_text(
                        &app.first_run_template,
                        &template_theme,
                    ))
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        super::apply_dropdown_menu_style(ui, &template_theme);
                        for template_name in template_names.iter() {
                            if ui
                                .selectable_label(
                                    app.first_run_template == *template_name,
                                    super::dropdown_item_text(template_name, &template_theme),
                                )
                                .clicked()
                            {
                                app.first_run_template = template_name.clone();
                            }
                        }
                    });
            });
        }

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    create_enabled,
                    super::primary_action_button(
                        &app.tr("startup.create_config_from_template"),
                        app.theme.accent_color,
                        app.theme.accent_text_color,
                    ),
                )
                .clicked()
            {
                let template_name = app.first_run_template.clone();
                app.create_config_from_template(ui.ctx(), &template_name);
            }

            if ui
                .add_enabled(
                    create_enabled,
                    super::secondary_action_button(
                        &app.tr("startup.open_config_folder"),
                        app.theme.panel_fill_soft,
                    ),
                )
                .clicked()
            {
                if let Ok(path) = &config_path {
                    if let Some(parent) = path.parent() {
                        app.settings_error =
                            super::open_path_in_file_manager(parent).err().map(|err| {
                                app.tr_with(
                                    "startup.open_config_folder_error",
                                    &[("error", err.to_string())],
                                )
                            });
                    }
                }
            }
        });

        if let Some(hint) = startup_first_run_hint(app, &config_path) {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(hint)
                    .small()
                    .color(app.theme.weak_text_color),
            );
        }
    });
}

fn render_startup_health_row(app: &AiPopupApp, ui: &mut egui::Ui, diagnostic: &HealthCheck) {
    let label = startup_health_label(app, diagnostic);
    let status_color = health_level_color(app, &diagnostic.level);

    ui.horizontal_wrapped(|ui| {
        ui.label(super::muted_label(&label, app.theme.weak_text_color));
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(&diagnostic.summary)
                .strong()
                .color(status_color),
        );
    });
    ui.label(
        egui::RichText::new(&diagnostic.detail)
            .small()
            .color(app.theme.weak_text_color),
    );
    if let Some(hint) = startup_recovery_hint(app, diagnostic) {
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(hint)
                .small()
                .color(app.theme.weak_text_color),
        );
    }
}

fn startup_health_label(app: &AiPopupApp, diagnostic: &HealthCheck) -> String {
    match diagnostic.backend.as_str() {
        "config" => app.tr("startup.config_source"),
        "selected-backend" => app.tr("startup.selected_backend"),
        "dictation-tools" => app.tr("startup.dictation_tools"),
        "clipboard-tools" => app.tr("startup.clipboard_tools"),
        _ => diagnostic.backend.clone(),
    }
}

fn startup_recovery_hint(app: &AiPopupApp, diagnostic: &HealthCheck) -> Option<String> {
    if matches!(diagnostic.level, HealthLevel::Ok) {
        return None;
    }

    let hint = match diagnostic.backend.as_str() {
        "config" => app.tr("startup.config_recovery_hint"),
        "selected-backend" => app.tr("startup.selected_backend_recovery_hint"),
        "dictation-tools" => app.tr("startup.dictation_tools_recovery_hint"),
        "clipboard-tools" => app.tr("startup.clipboard_tools_recovery_hint"),
        _ => return None,
    };

    Some(hint)
}

fn startup_first_run_hint(
    app: &AiPopupApp,
    config_path: &anyhow::Result<PathBuf>,
) -> Option<String> {
    match config_path {
        Ok(path) => Some(app.tr_with(
            "startup.first_run_hint",
            &[("path", path.display().to_string())],
        )),
        Err(err) => Some(app.tr_with(
            "startup.first_run_hint_error",
            &[("error", err.to_string())],
        )),
    }
}

fn health_level_color(app: &AiPopupApp, level: &HealthLevel) -> egui::Color32 {
    match level {
        HealthLevel::Ok => app.theme.accent_color,
        HealthLevel::Warning => egui::Color32::from_rgb(227, 177, 76),
        HealthLevel::Error => app.theme.danger_color,
    }
}

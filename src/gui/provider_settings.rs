use eframe::egui;

use super::{AiPopupApp, Config, ProviderModelState, ResolvedTheme};
use crate::backends::{HealthCheck, HealthLevel};

pub(super) fn render_provider_settings_sections(
    app: &mut AiPopupApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    egui::CollapsingHeader::new(
        egui::RichText::new(app.tr("settings.models"))
            .color(app.theme.text_color)
            .strong(),
    )
    .id_source("settings_models_keys")
    .default_open(false)
    .show(ui, |ui| {
        let health_checks = crate::backends::health_checks(&app.config);
        let settings_theme = app.theme.clone();

        render_provider_config_section(
            app,
            ctx,
            ui,
            &settings_theme,
            ProviderConfigSection {
                id: "settings_provider_gemini",
                provider: "gemini",
                health_check: health_check_for(&health_checks, "gemini"),
                primary_label: app.tr("settings.gemini_key"),
                secondary_label: app.tr("settings.gemini_model"),
            },
            |config| {
                config
                    .gemini
                    .as_ref()
                    .map(|value| (value.api_key.clone(), value.model.clone()))
            },
            |config, primary, secondary| {
                if let Some(value) = config.gemini.as_mut() {
                    value.api_key = primary;
                    value.model = secondary;
                }
            },
        );

        render_provider_config_section(
            app,
            ctx,
            ui,
            &settings_theme,
            ProviderConfigSection {
                id: "settings_provider_chatgpt",
                provider: "chatgpt",
                health_check: health_check_for(&health_checks, "chatgpt"),
                primary_label: app.tr("settings.chatgpt_key"),
                secondary_label: app.tr("settings.chatgpt_model"),
            },
            |config| {
                config
                    .chatgpt
                    .as_ref()
                    .map(|value| (value.api_key.clone(), value.model.clone()))
            },
            |config, primary, secondary| {
                if let Some(value) = config.chatgpt.as_mut() {
                    value.api_key = primary;
                    value.model = secondary;
                }
            },
        );

        render_provider_config_section(
            app,
            ctx,
            ui,
            &settings_theme,
            ProviderConfigSection {
                id: "settings_provider_claude",
                provider: "claude",
                health_check: health_check_for(&health_checks, "claude"),
                primary_label: app.tr("settings.claude_key"),
                secondary_label: app.tr("settings.claude_model"),
            },
            |config| {
                config
                    .claude
                    .as_ref()
                    .map(|value| (value.api_key.clone(), value.model.clone()))
            },
            |config, primary, secondary| {
                if let Some(value) = config.claude.as_mut() {
                    value.api_key = primary;
                    value.model = secondary;
                }
            },
        );

        render_provider_config_section(
            app,
            ctx,
            ui,
            &settings_theme,
            ProviderConfigSection {
                id: "settings_provider_ollama",
                provider: "ollama",
                health_check: health_check_for(&health_checks, "ollama"),
                primary_label: app.tr("settings.ollama_url"),
                secondary_label: app.tr("settings.ollama_model"),
            },
            |config| {
                config
                    .ollama
                    .as_ref()
                    .map(|value| (value.base_url.clone(), value.model.clone()))
            },
            |config, primary, secondary| {
                if let Some(value) = config.ollama.as_mut() {
                    value.base_url = primary;
                    value.model = secondary;
                }
            },
        );
    });
}

fn health_check_for(health_checks: &[HealthCheck], backend_name: &str) -> HealthCheck {
    health_checks
        .iter()
        .find(|check| check.backend == backend_name)
        .cloned()
        .unwrap_or(HealthCheck {
            backend: backend_name.to_string(),
            level: HealthLevel::Warning,
            summary: "Unknown".to_string(),
            detail: "No health information available.".to_string(),
        })
}

struct ProviderConfigSection {
    id: &'static str,
    provider: &'static str,
    health_check: HealthCheck,
    primary_label: String,
    secondary_label: String,
}

fn render_provider_config_section<FGet, FSet>(
    app: &mut AiPopupApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    theme: &ResolvedTheme,
    section: ProviderConfigSection,
    get_values: FGet,
    set_values: FSet,
) where
    FGet: Fn(&Config) -> Option<(String, String)>,
    FSet: Fn(&mut Config, String, String),
{
    let Some((mut primary_value, mut secondary_value)) = get_values(&app.config) else {
        return;
    };

    if provider_settings_section(
        app,
        ctx,
        ui,
        theme,
        section.id,
        section.provider,
        &section.health_check,
        &section.primary_label,
        &section.secondary_label,
        &mut primary_value,
        &mut secondary_value,
    ) {
        set_values(&mut app.config, primary_value, secondary_value);
        app.persist_settings();
    }
}

#[allow(clippy::too_many_arguments)]
fn provider_settings_section(
    app: &mut AiPopupApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    theme: &ResolvedTheme,
    id: &str,
    provider: &str,
    health_check: &HealthCheck,
    primary_label: &str,
    secondary_label: &str,
    primary_value: &mut String,
    secondary_value: &mut String,
) -> bool {
    let mut changed = false;
    let mut should_fetch_models = false;
    let available_models_label = app.tr("settings.available_models");
    let refresh_models_label = app.tr("settings.refresh_models");
    let loading_models_label = app.tr("settings.loading_models");
    let select_model_label = app.tr("settings.select_model");
    let models_hint_label = app.tr("settings.models_hint");
    let color = match health_check.level {
        HealthLevel::Ok => theme.accent_color,
        HealthLevel::Warning => egui::Color32::from_rgb(227, 177, 76),
        HealthLevel::Error => theme.danger_color,
    };
    let header = egui::RichText::new(format!(
        "{} · {}",
        provider.to_uppercase(),
        health_check.summary
    ))
    .color(color);

    egui::CollapsingHeader::new(header)
        .id_source(id)
        .default_open(false)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(&health_check.detail)
                    .small()
                    .color(theme.weak_text_color),
            );
            ui.add_space(8.0);
            ui.label(super::muted_label(
                &app.tr("settings.model_credits"),
                theme.weak_text_color,
            ));
            ui.label(
                egui::RichText::new(provider_credit_label(app, provider))
                    .color(color)
                    .strong(),
            );
            ui.label(super::muted_label(
                &provider_credit_note(app, provider),
                theme.weak_text_color,
            ));

            let primary_changed =
                super::settings_text_field(ui, theme, primary_label, primary_value, true);
            if primary_changed {
                app.invalidate_provider_models(provider);
            }
            changed |= primary_changed;

            if provider == "ollama" {
                ui.label(super::muted_label(
                    &app.tr("settings.ollama_url_hint"),
                    theme.weak_text_color,
                ));
            }

            let is_pulling = app.async_pull_status.lock().unwrap().contains_key(provider);
            let pull_status = app.async_pull_status.lock().unwrap().get(provider).cloned();
            let pulling_label = app.tr("settings.pulling");

            let model_state = app
                .provider_model_states
                .entry(provider.to_string())
                .or_default();

            let (model_changed, model_interacted, should_pull) = settings_model_field(
                ui,
                theme,
                provider,
                secondary_label,
                secondary_value,
                model_state,
                &available_models_label,
                &refresh_models_label,
                &loading_models_label,
                &select_model_label,
                &models_hint_label,
                is_pulling,
                pull_status,
                &pulling_label,
            );
            changed |= model_changed;
            should_fetch_models = model_interacted && model_state.models.is_empty();
            if should_pull {
                app.request_ollama_model_pull(ctx, secondary_value);
            }
        });
    if should_fetch_models {
        app.request_provider_models(ctx, provider);
    }
    changed
}

#[allow(clippy::too_many_arguments)]
fn settings_model_field(
    ui: &mut egui::Ui,
    theme: &ResolvedTheme,
    provider: &str,
    label: &str,
    value: &mut String,
    state: &mut ProviderModelState,
    available_models_label: &str,
    refresh_label: &str,
    loading_label: &str,
    select_model_label: &str,
    models_hint_label: &str,
    is_pulling: bool,
    pull_status: Option<(String, Option<f32>)>,
    pulling_label: &str,
) -> (bool, bool, bool) {
    let mut changed = false;
    let mut should_fetch = false;
    let mut should_pull = false;

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(super::muted_label(label, theme.weak_text_color));
        if provider == "ollama" {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button_label = if state.is_loading {
                    loading_label
                } else {
                    refresh_label
                };
                if ui
                    .add_enabled(
                        !state.is_loading,
                        super::secondary_action_button(button_label, theme.panel_fill_soft),
                    )
                    .clicked()
                {
                    state.models.clear();
                    state.last_error = None;
                    should_fetch = true;
                }
            });
        }
    });

    let response = ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
    changed |= response.changed();
    should_fetch |= response.clicked() || response.gained_focus() || response.has_focus();

    if provider != "ollama" {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(super::muted_label(
                available_models_label,
                theme.weak_text_color,
            ));
            let button_label = if state.is_loading {
                loading_label
            } else {
                refresh_label
            };
            if ui
                .add_enabled(
                    !state.is_loading,
                    super::secondary_action_button(button_label, theme.panel_fill_soft),
                )
                .clicked()
            {
                state.models.clear();
                state.last_error = None;
                should_fetch = true;
            }
        });
    }

    if provider == "ollama" {
        if is_pulling {
            let (status, percentage) = pull_status.unwrap_or_default();
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(pulling_label)
                        .small()
                        .color(theme.accent_color),
                );
                ui.label(
                    egui::RichText::new(&status)
                        .small()
                        .color(theme.weak_text_color),
                );
            });
            if let Some(p) = percentage {
                ui.add(egui::ProgressBar::new(p));
            }
        }
    }

    if state.is_loading {
        ui.label(super::muted_label(loading_label, theme.weak_text_color));
    } else if !state.models.is_empty() || provider == "ollama" {
        let selected_text = if value.trim().is_empty() {
            select_model_label.to_string()
        } else {
            value.clone()
        };

        let filter = value.to_lowercase();
        let mut suggestions = Vec::new();

        // Add local models
        for m in &state.models {
            if m.to_lowercase().contains(&filter) || filter.is_empty() {
                suggestions.push((m.clone(), false));
            }
        }

        // Add remote models if Ollama
        if provider == "ollama" {
            for &m in super::POPULAR_OLLAMA_MODELS {
                if (m.to_lowercase().contains(&filter) || filter.is_empty())
                    && !state.models.iter().any(|local| local == m)
                {
                    suggestions.push((m.to_string(), true));
                }
            }
        }

        if !suggestions.is_empty() {
            super::dropdown_box_scope(ui, theme, |ui| {
                egui::ComboBox::from_id_source(format!("{provider}_available_models"))
                    .selected_text(super::dropdown_button_text(&selected_text, theme))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        super::apply_dropdown_menu_style(ui, theme);
                        for (model_name, is_remote) in suggestions {
                            let label = if is_remote {
                                format!("{model_name} ↓")
                            } else {
                                model_name.clone()
                            };
                            if ui
                                .selectable_value(value, model_name.clone(), label)
                                .changed()
                            {
                                changed = true;
                                if is_remote {
                                    should_pull = true;
                                }
                            }
                        }
                    });
            });
        }
    } else {
        if provider != "ollama" {
            ui.label(super::muted_label(models_hint_label, theme.weak_text_color));
        }
    }

    if let Some(error) = &state.last_error {
        ui.label(egui::RichText::new(error).small().color(theme.danger_color));
        ui.label(super::muted_label(models_hint_label, theme.weak_text_color));
    }

    (changed, should_fetch, should_pull)
}

fn provider_credit_label(app: &AiPopupApp, provider: &str) -> String {
    match provider {
        "ollama" => app.tr("settings.credits_infinite"),
        _ => app.tr("settings.credits_unknown"),
    }
}

fn provider_credit_note(app: &AiPopupApp, provider: &str) -> String {
    match provider {
        "ollama" => app.tr("settings.model_credits_local"),
        _ => app.tr("settings.model_credits_unavailable"),
    }
}

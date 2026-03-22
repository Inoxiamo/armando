use crate::config::{RagMode, RagRuntimeOverride};
use std::path::PathBuf;

use super::{
    apply_dropdown_menu_style, default_rag_embedding_model, dropdown_box_scope,
    dropdown_button_text, dropdown_item_text, egui, muted_label, rag_embedding_backend,
    rag_embedding_model, settings_text_field, AiPopupApp,
};

pub(super) fn render_rag_settings_section(
    app: &mut AiPopupApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    egui::CollapsingHeader::new(
        egui::RichText::new(app.tr("settings.rag"))
            .color(app.theme.text_color)
            .strong(),
    )
    .id_source("settings_rag")
    .default_open(false)
    .show(ui, |ui| {
        let mut enabled = app.config.rag.enabled;
        if ui
            .checkbox(&mut enabled, app.tr("settings.rag_enabled"))
            .changed()
        {
            app.config.rag.enabled = enabled;
            app.persist_settings();
        }

        ui.add_space(8.0);
        ui.label(muted_label(
            &app.tr("settings.rag_retrieval_mode"),
            app.theme.weak_text_color,
        ));
        let retrieval_mode_label = match app.config.rag.mode {
            RagMode::Keyword => app.tr("settings.rag_mode_keyword"),
            RagMode::Vector => app.tr("settings.rag_mode_vector"),
            RagMode::Hybrid => app.tr("settings.rag_mode_hybrid"),
        };
        let retrieval_mode_theme = app.theme.clone();
        dropdown_box_scope(ui, &retrieval_mode_theme, |ui| {
            egui::ComboBox::from_id_source("settings_rag_retrieval_mode")
                .selected_text(dropdown_button_text(
                    &retrieval_mode_label,
                    &retrieval_mode_theme,
                ))
                .width(220.0)
                .show_ui(ui, |ui| {
                    apply_dropdown_menu_style(ui, &retrieval_mode_theme);
                    let options = [
                        (RagMode::Keyword, "settings.rag_mode_keyword"),
                        (RagMode::Vector, "settings.rag_mode_vector"),
                        (RagMode::Hybrid, "settings.rag_mode_hybrid"),
                    ];
                    for (mode, label_key) in options {
                        if ui
                            .selectable_label(
                                app.config.rag.mode == mode,
                                dropdown_item_text(&app.tr(label_key), &retrieval_mode_theme),
                            )
                            .clicked()
                        {
                            app.config.rag.mode = mode;
                            app.persist_settings();
                        }
                    }
                });
        });

        ui.add_space(8.0);
        ui.label(muted_label(
            &app.tr("settings.rag_runtime_override"),
            app.theme.weak_text_color,
        ));
        let runtime_override_label = match app.config.rag.runtime_override {
            RagRuntimeOverride::Default => app.tr("settings.rag_runtime_default"),
            RagRuntimeOverride::ForceOn => app.tr("settings.rag_runtime_force_on"),
            RagRuntimeOverride::ForceOff => app.tr("settings.rag_runtime_force_off"),
        };
        let runtime_override_theme = app.theme.clone();
        dropdown_box_scope(ui, &runtime_override_theme, |ui| {
            egui::ComboBox::from_id_source("settings_rag_runtime_override")
                .selected_text(dropdown_button_text(
                    &runtime_override_label,
                    &runtime_override_theme,
                ))
                .width(220.0)
                .show_ui(ui, |ui| {
                    apply_dropdown_menu_style(ui, &runtime_override_theme);
                    let options = [
                        (RagRuntimeOverride::Default, "settings.rag_runtime_default"),
                        (RagRuntimeOverride::ForceOn, "settings.rag_runtime_force_on"),
                        (
                            RagRuntimeOverride::ForceOff,
                            "settings.rag_runtime_force_off",
                        ),
                    ];
                    for (mode, label_key) in options {
                        if ui
                            .selectable_label(
                                app.config.rag.runtime_override == mode,
                                dropdown_item_text(&app.tr(label_key), &runtime_override_theme),
                            )
                            .clicked()
                        {
                            app.config.rag.runtime_override = mode;
                            app.persist_settings();
                        }
                    }
                });
        });

        if app.config.rag.mode != RagMode::Keyword {
            ui.add_space(8.0);
            ui.label(muted_label(
                &app.tr("settings.rag_embedding_backend"),
                app.theme.weak_text_color,
            ));
            let current_embedding_backend = rag_embedding_backend(&app.config);
            let backend_theme = app.theme.clone();
            dropdown_box_scope(ui, &backend_theme, |ui| {
                egui::ComboBox::from_id_source("settings_rag_embedding_backend")
                    .selected_text(dropdown_button_text(
                        &current_embedding_backend,
                        &backend_theme,
                    ))
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        apply_dropdown_menu_style(ui, &backend_theme);
                        for backend in ["ollama", "chatgpt", "claude", "gemini"] {
                            if ui
                                .selectable_label(
                                    current_embedding_backend == backend,
                                    dropdown_item_text(backend, &backend_theme),
                                )
                                .clicked()
                            {
                                app.config.rag.embedding_backend = Some(backend.to_string());
                                let has_model = app
                                    .config
                                    .rag
                                    .embedding_model
                                    .as_deref()
                                    .is_some_and(|value| !value.trim().is_empty());
                                if !has_model {
                                    app.config.rag.embedding_model =
                                        Some(default_rag_embedding_model(&app.config, backend));
                                }
                                app.persist_settings();
                            }
                        }
                    });
            });

            ui.add_space(8.0);
            let mut embedding_model = rag_embedding_model(&app.config, &current_embedding_backend);
            if settings_text_field(
                ui,
                &app.theme,
                &app.tr("settings.rag_embedding_model"),
                &mut embedding_model,
                false,
            ) {
                let value = embedding_model.trim();
                app.config.rag.embedding_model = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
                app.persist_settings();
            }
        }

        ui.add_space(8.0);
        ui.label(muted_label(
            &app.tr("settings.rag_documents_folder"),
            app.theme.weak_text_color,
        ));
        ui.horizontal(|ui| {
            let mut documents_folder = app
                .config
                .rag
                .documents_folder
                .as_ref()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_default();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut documents_folder)
                        .hint_text(app.tr("settings.rag_documents_folder_hint")),
                )
                .changed()
            {
                let value = documents_folder.trim();
                app.config.rag.documents_folder = if value.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(value))
                };
                app.persist_settings();
            }

            if ui.button(app.tr("settings.rag_browse")).clicked() {
                let mut dialog = rfd::FileDialog::new();
                if let Some(current) = &app.config.rag.documents_folder {
                    dialog = dialog.set_directory(current);
                }
                if let Some(path) = dialog.pick_folder() {
                    app.config.rag.documents_folder = Some(path);
                    app.persist_settings();
                }
            }
        });

        ui.add_space(8.0);
        ui.label(muted_label(
            &app.tr("settings.rag_vector_db_path"),
            app.theme.weak_text_color,
        ));
        let mut vector_db_path = app.config.rag.vector_db_path.to_string_lossy().to_string();
        if ui
            .add(egui::TextEdit::singleline(&mut vector_db_path))
            .changed()
        {
            let value = vector_db_path.trim();
            if !value.is_empty() {
                app.config.rag.vector_db_path = PathBuf::from(value);
                app.persist_settings();
            }
        }

        ui.add_space(8.0);
        egui::CollapsingHeader::new(
            egui::RichText::new(app.tr("settings.rag_advanced"))
                .color(app.theme.text_color)
                .strong(),
        )
        .id_source("settings_rag_advanced")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(muted_label(
                    &app.tr("settings.rag_chunk_size"),
                    app.theme.weak_text_color,
                ));
                let mut chunk_size = app.config.rag.chunk_size as u64;
                if ui
                    .add(egui::DragValue::new(&mut chunk_size).clamp_range(128..=20_000))
                    .changed()
                {
                    app.config.rag.chunk_size = chunk_size as usize;
                    app.persist_settings();
                }
            });

            ui.horizontal(|ui| {
                ui.label(muted_label(
                    &app.tr("settings.rag_top_n"),
                    app.theme.weak_text_color,
                ));
                let mut top_n = app.config.rag.max_retrieved_docs as u64;
                if ui
                    .add(egui::DragValue::new(&mut top_n).clamp_range(1..=20))
                    .changed()
                {
                    app.config.rag.max_retrieved_docs = top_n as usize;
                    app.persist_settings();
                }
            });
        });

        ui.add_space(8.0);
        let can_index = app.config.rag.documents_folder.is_some();
        let index_button = ui.add_enabled(
            can_index && !app.rag_index_in_progress,
            egui::Button::new(app.tr("settings.rag_index_now")),
        );
        if index_button.clicked() {
            app.start_rag_index(ctx);
        }
        if app.rag_index_in_progress {
            ui.label(muted_label(
                &app.tr("settings.rag_indexing"),
                app.theme.weak_text_color,
            ));
        } else if !can_index {
            ui.label(muted_label(
                &app.tr("settings.rag_index_disabled_hint"),
                app.theme.weak_text_color,
            ));
        }

        ui.label(muted_label(
            &app.tr("settings.rag_runtime_hint"),
            app.theme.weak_text_color,
        ));
    });
}

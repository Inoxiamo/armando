use crate::config::{RagEngine, RagMode};
use crate::rag::RagCorpusStats;
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
        // Temporary guard: keep LangChain unavailable from settings until re-enabled.
        let mut langchain_forced_to_simple = false;
        if app.config.rag.engine == RagEngine::Langchain {
            app.config.rag.engine = RagEngine::Simple;
            app.persist_settings();
            langchain_forced_to_simple = true;
        }

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
            &app.tr("settings.rag_engine"),
            app.theme.weak_text_color,
        ));
        let rag_engine_label = match app.config.rag.engine {
            RagEngine::Simple => app.tr("settings.rag_engine_simple"),
            RagEngine::Langchain => app.tr("settings.rag_engine_langchain"),
        };
        let rag_engine_theme = app.theme.clone();
        dropdown_box_scope(ui, &rag_engine_theme, |ui| {
            egui::ComboBox::from_id_source("settings_rag_engine")
                .selected_text(dropdown_button_text(&rag_engine_label, &rag_engine_theme))
                .width(220.0)
                .show_ui(ui, |ui| {
                    apply_dropdown_menu_style(ui, &rag_engine_theme);
                    if ui
                        .selectable_label(
                            app.config.rag.engine == RagEngine::Simple,
                            dropdown_item_text(
                                &app.tr("settings.rag_engine_simple"),
                                &rag_engine_theme,
                            ),
                        )
                        .clicked()
                    {
                        app.config.rag.engine = RagEngine::Simple;
                        app.persist_settings();
                    }

                    ui.add_enabled_ui(false, |ui| {
                        let _ = ui.selectable_label(
                            false,
                            dropdown_item_text(
                                &format!(
                                    "{} ({})",
                                    app.tr("settings.rag_engine_langchain"),
                                    app.tr("settings.rag_temporarily_disabled_short")
                                ),
                                &rag_engine_theme,
                            ),
                        );
                    });
                });
        });

        let use_langchain = app.config.rag.engine == RagEngine::Langchain;
        if langchain_forced_to_simple {
            ui.add_space(8.0);
            ui.label(muted_label(
                &app.tr("settings.rag_langchain_temporarily_disabled_hint"),
                app.theme.weak_text_color,
            ));
        }

        ui.add_space(8.0);
        ui.label(muted_label(
            &app.tr("settings.rag_langchain_temporarily_disabled_hint"),
            app.theme.weak_text_color,
        ));
        if use_langchain {
            ui.add_space(8.0);
            ui.label(muted_label(
                &app.tr("settings.rag_langchain_managed_hint"),
                app.theme.weak_text_color,
            ));
        }

        ui.add_space(8.0);
        ui.add_enabled_ui(!use_langchain, |ui| {
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

            if app.config.rag.mode != RagMode::Keyword {
                ui.add_space(8.0);
                ui.label(muted_label(
                    &app.tr("settings.rag_tuning_preset"),
                    app.theme.weak_text_color,
                ));
                if let Some(stats) = &app.rag_corpus_stats {
                    ui.label(muted_label(
                        &app.tr_with(
                            "settings.rag_corpus_summary",
                            &[
                                ("files", stats.file_count.to_string()),
                                ("lines", stats.total_lines.to_string()),
                            ],
                        ),
                        app.theme.weak_text_color,
                    ));
                } else {
                    ui.label(muted_label(
                        &app.tr("settings.rag_preset_from_index_hint"),
                        app.theme.weak_text_color,
                    ));
                }
                let active_mode = app.config.rag.mode;
                let inferred_preset = detect_selected_preset(
                    active_mode,
                    app.config.rag.chunk_size,
                    app.config.rag.max_retrieved_docs,
                    app.rag_corpus_stats.as_ref(),
                );
                let preset_theme = app.theme.clone();
                dropdown_box_scope(ui, &preset_theme, |ui| {
                    egui::ComboBox::from_id_source("settings_rag_tuning_preset")
                        .selected_text(dropdown_button_text(
                            &app.tr(inferred_preset.label_key()),
                            &preset_theme,
                        ))
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            apply_dropdown_menu_style(ui, &preset_theme);
                            for preset in [
                                RagTuningPreset::Compact,
                                RagTuningPreset::Balanced,
                                RagTuningPreset::Deep,
                                RagTuningPreset::Custom,
                            ] {
                                if ui
                                    .selectable_label(
                                        inferred_preset == preset,
                                        dropdown_item_text(
                                            &app.tr(preset.label_key()),
                                            &preset_theme,
                                        ),
                                    )
                                    .clicked()
                                {
                                    if preset != RagTuningPreset::Custom {
                                        let (chunk_size, top_n) = preset_values_for_mode(
                                            preset,
                                            active_mode,
                                            app.rag_corpus_stats.as_ref(),
                                        );
                                        app.config.rag.chunk_size = chunk_size;
                                        app.config.rag.max_retrieved_docs = top_n;
                                        app.persist_settings();
                                    }
                                }
                            }
                        });
                });

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
                let mut embedding_model =
                    rag_embedding_model(&app.config, &current_embedding_backend);
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
        });

        if use_langchain {
            ui.add_space(8.0);
            let mut base_url = app.config.rag.langchain_base_url.clone();
            if settings_text_field(
                ui,
                &app.theme,
                &app.tr("settings.rag_langchain_endpoint"),
                &mut base_url,
                false,
            ) {
                let value = base_url.trim();
                if !value.is_empty() {
                    app.config.rag.langchain_base_url = value.to_string();
                    app.persist_settings();
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(muted_label(
                    &app.tr("settings.rag_langchain_timeout_ms"),
                    app.theme.weak_text_color,
                ));
                let mut timeout = app.config.rag.langchain_timeout_ms;
                if ui
                    .add(egui::DragValue::new(&mut timeout).clamp_range(100..=120_000))
                    .changed()
                {
                    app.config.rag.langchain_timeout_ms = timeout;
                    app.persist_settings();
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(muted_label(
                    &app.tr("settings.rag_langchain_retry_count"),
                    app.theme.weak_text_color,
                ));
                let mut retries = app.config.rag.langchain_retry_count as u64;
                if ui
                    .add(egui::DragValue::new(&mut retries).clamp_range(0..=5))
                    .changed()
                {
                    app.config.rag.langchain_retry_count = retries as usize;
                    app.persist_settings();
                }
            });

            ui.add_space(8.0);
            ui.label(muted_label(
                &app.tr("settings.rag_langchain_setup"),
                app.theme.weak_text_color,
            ));
            ui.label(
                egui::RichText::new(app.tr("settings.rag_langchain_setup_cmds"))
                    .monospace()
                    .small()
                    .color(app.theme.text_color),
            );
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
                app.rag_corpus_stats = None;
                app.persist_settings();
            }

            if ui.button(app.tr("settings.rag_browse")).clicked() {
                let mut dialog = rfd::FileDialog::new();
                if let Some(current) = &app.config.rag.documents_folder {
                    dialog = dialog.set_directory(current);
                }
                if let Some(path) = dialog.pick_folder() {
                    app.config.rag.documents_folder = Some(path);
                    app.rag_corpus_stats = None;
                    app.persist_settings();
                }
            }
        });

        if !use_langchain {
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
        }

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
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RagTuningPreset {
    Compact,
    Balanced,
    Deep,
    Custom,
}

impl RagTuningPreset {
    fn label_key(self) -> &'static str {
        match self {
            RagTuningPreset::Compact => "settings.rag_preset_compact",
            RagTuningPreset::Balanced => "settings.rag_preset_balanced",
            RagTuningPreset::Deep => "settings.rag_preset_deep",
            RagTuningPreset::Custom => "settings.rag_preset_custom",
        }
    }
}

fn detect_selected_preset(
    mode: RagMode,
    chunk_size: usize,
    top_n: usize,
    stats: Option<&RagCorpusStats>,
) -> RagTuningPreset {
    for preset in [
        RagTuningPreset::Compact,
        RagTuningPreset::Balanced,
        RagTuningPreset::Deep,
    ] {
        let (preset_chunk, preset_top_n) = preset_values_for_mode(preset, mode, stats);
        if preset_chunk == chunk_size && preset_top_n == top_n {
            return preset;
        }
    }
    RagTuningPreset::Custom
}

fn preset_values_for_mode(
    preset: RagTuningPreset,
    mode: RagMode,
    stats: Option<&RagCorpusStats>,
) -> (usize, usize) {
    let default_stats = RagCorpusStats::default();
    let stats = stats.unwrap_or(&default_stats);
    let files = stats.file_count.max(1);
    let lines = stats.total_lines.max(files);
    let avg_lines_per_file = lines / files;

    let line_bucket = match lines {
        0..=5_000 => 0usize,
        5_001..=25_000 => 1usize,
        25_001..=100_000 => 2usize,
        _ => 3usize,
    };
    let file_bucket = match files {
        0..=10 => 0usize,
        11..=40 => 1usize,
        41..=120 => 2usize,
        _ => 3usize,
    };

    let (base_chunk, base_top_n) = match preset {
        RagTuningPreset::Compact => (700usize, 3usize),
        RagTuningPreset::Balanced => (1_150usize, 5usize),
        RagTuningPreset::Deep => (1_600usize, 7usize),
        RagTuningPreset::Custom => (1_200usize, 4usize),
    };

    let avg_adjustment = (avg_lines_per_file / 40).min(320);
    let mut chunk_size =
        base_chunk + (line_bucket * 150) + avg_adjustment - (file_bucket.saturating_mul(40));
    let mut top_n = base_top_n + line_bucket + usize::from(file_bucket >= 2);

    if mode == RagMode::Hybrid {
        chunk_size = chunk_size.saturating_sub(100);
        top_n = top_n.saturating_sub(1).max(2);
    }

    (chunk_size.clamp(256, 4_000), top_n.clamp(1, 20))
}

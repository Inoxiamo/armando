use eframe::egui;

use super::{AiPopupApp, ResolvedTheme};

pub(super) fn render_session_history_section(
    app: &mut AiPopupApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(super::section_label(
            &app.tr("app.session_history"),
            app.theme.text_color,
        ));
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(app.tr("app.session_history_note"))
                .small()
                .color(app.theme.weak_text_color),
        );
    });
    ui.add_space(8.0);

    super::card_frame(ctx, app.theme.panel_fill, app.theme.border_color).show(ui, |ui| {
        render_session_history_entries(app, ctx, ui);
    });
}

pub(super) fn render_persistent_history_section(
    app: &mut AiPopupApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(super::section_label(
            &app.tr("app.saved_history"),
            app.theme.text_color,
        ));
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(app.tr("app.saved_history_note"))
                .small()
                .color(app.theme.weak_text_color),
        );
    });
    ui.add_space(8.0);

    super::card_frame(ctx, app.theme.panel_fill, app.theme.border_color).show(ui, |ui| {
        render_persistent_history_actions(app, ui);

        if let Some(error) = &app.history_error {
            ui.add_space(8.0);
            ui.colored_label(app.theme.danger_color, error);
        } else if let Some(error) = &app.history_action_error {
            ui.add_space(8.0);
            ui.colored_label(app.theme.danger_color, error);
        }

        ui.add_space(12.0);
        render_saved_history_entries(app, ctx, ui);
    });
}

fn render_persistent_history_actions(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    let all_backends = app.tr("app.all_backends");
    let history_search_hint = app.tr("app.search_history");
    let open_history_label = app.tr("app.open_saved_history_file");
    let select_all_label = app.tr("app.select_all");
    let delete_all_label = app.tr("app.delete_all");
    let delete_selected_label = app.tr_with(
        "app.delete_selected",
        &[("count", app.selected_history_entries.len().to_string())],
    );

    ui.horizontal_wrapped(|ui| {
        super::dropdown_box_scope(ui, &app.theme, |ui| {
            egui::ComboBox::from_id_source("history_backend_filter")
                .selected_text(selected_history_backend_label(
                    &app.history_filter_backend,
                    &all_backends,
                    &app.theme,
                ))
                .width(150.0)
                .show_ui(ui, |ui| {
                    super::apply_dropdown_menu_style(ui, &app.theme);
                    ui.selectable_value(
                        &mut app.history_filter_backend,
                        "all".to_string(),
                        super::dropdown_item_text(all_backends.as_str(), &app.theme),
                    );
                    super::dropdown_option(
                        ui,
                        &mut app.history_filter_backend,
                        "chatgpt",
                        &app.theme,
                    );
                    super::dropdown_option(
                        ui,
                        &mut app.history_filter_backend,
                        "claude",
                        &app.theme,
                    );
                    super::dropdown_option(
                        ui,
                        &mut app.history_filter_backend,
                        "gemini",
                        &app.theme,
                    );
                    super::dropdown_option(
                        ui,
                        &mut app.history_filter_backend,
                        "ollama",
                        &app.theme,
                    );
                });
        });
        ui.add(
            egui::TextEdit::singleline(&mut app.history_filter_query)
                .hint_text(history_search_hint)
                .desired_width(280.0),
        );
        if ui
            .add(super::secondary_action_button(
                &open_history_label,
                app.theme.panel_fill_soft,
            ))
            .clicked()
        {
            app.history_action_error = super::history_entry::open_history_file()
                .err()
                .map(|err| err.to_string());
        }
        if ui
            .add(super::secondary_action_button(
                &select_all_label,
                app.theme.panel_fill_soft,
            ))
            .clicked()
        {
            app.select_all_visible_history_entries();
        }
        if ui
            .add_enabled(
                !app.selected_history_entries.is_empty(),
                super::secondary_action_button(&delete_selected_label, app.theme.panel_fill_soft),
            )
            .clicked()
        {
            app.delete_selected_history_entries();
        }
        if ui
            .add(super::secondary_action_button(
                &delete_all_label,
                app.theme.panel_fill_soft,
            ))
            .clicked()
        {
            app.delete_all_visible_history_entries();
        }
    });
}

fn render_session_history_entries(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    if app.session_history_entries.is_empty() {
        return;
    }

    ui.label(
        egui::RichText::new(app.tr("app.session_history"))
            .strong()
            .color(app.theme.text_color),
    );
    ui.add_space(8.0);

    for entry in app.session_history_entries.iter().take(5) {
        super::history_entry::history_entry_card(
            &app.tr("app.copy_result"),
            &app.tr("app.reuse_entry"),
            &app.tr("app.select_entry"),
            &app.tr("app.history_prompt"),
            &app.tr("app.history_response"),
            false,
            ui,
            ctx,
            &app.theme,
            entry,
            &mut app.selected_history_entries,
            &mut app.prompt,
            &mut app.response,
            &mut app.show_history,
            &mut app.prompt_focus_initialized,
            &mut app.history_action_error,
        );
        ui.add_space(8.0);
    }

    ui.separator();
    ui.add_space(12.0);
}

fn render_saved_history_entries(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let entries = app.filtered_history_entries();
    if entries.is_empty() {
        ui.label(
            egui::RichText::new(app.tr("app.no_saved_history")).color(app.theme.weak_text_color),
        );
        return;
    }

    let history_height = ui.available_height().clamp(240.0, 380.0);
    egui::ScrollArea::vertical()
        .id_source("history_entries_scroll")
        .auto_shrink([false; 2])
        .max_height(history_height)
        .show(ui, |ui| {
            for (index, entry) in entries.iter().enumerate() {
                super::history_entry::history_entry_card(
                    &app.tr("app.copy_result"),
                    &app.tr("app.reuse_entry"),
                    &app.tr("app.select_entry"),
                    &app.tr("app.history_prompt"),
                    &app.tr("app.history_response"),
                    true,
                    ui,
                    ctx,
                    &app.theme,
                    entry,
                    &mut app.selected_history_entries,
                    &mut app.prompt,
                    &mut app.response,
                    &mut app.show_history,
                    &mut app.prompt_focus_initialized,
                    &mut app.history_action_error,
                );
                if index + 1 < entries.len() {
                    ui.add_space(10.0);
                }
            }
        });
}

fn selected_history_backend_label<'a>(
    selected: &'a str,
    all_backends_label: &'a str,
    theme: &ResolvedTheme,
) -> egui::RichText {
    match selected {
        "chatgpt" => super::dropdown_button_text("chatgpt", theme),
        "claude" => super::dropdown_button_text("claude", theme),
        "gemini" => super::dropdown_button_text("gemini", theme),
        "ollama" => super::dropdown_button_text("ollama", theme),
        _ => super::dropdown_button_text(all_backends_label, theme),
    }
}

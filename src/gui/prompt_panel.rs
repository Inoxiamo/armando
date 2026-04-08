use eframe::egui;
use egui::text::{CCursor, CCursorRange};

use super::{layout, media_io, AiPopupApp, ToolbarIcon};

pub(super) fn render_prompt_section(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let prompt_section_label = app.tr("app.prompt");
    let prompt_id = ui.make_persistent_id("prompt_input");

    ui.horizontal(|ui| {
        ui.label(super::section_label(
            &prompt_section_label,
            app.theme.text_color,
        ));
        ui.add_space(8.0);
        let can_toggle_rag = super::rag_toggle_available(&app.config);
        let rag_button = ui
            .add_enabled(
                can_toggle_rag,
                egui::Button::new(egui::RichText::new("RAG").strong())
                    .fill(if app.config.rag.enabled {
                        app.theme.accent_color
                    } else {
                        app.theme.panel_fill_soft
                    })
                    .stroke(egui::Stroke::new(1.0, app.theme.border_color)),
            )
            .on_hover_text(if can_toggle_rag {
                "Toggle RAG"
            } else {
                "RAG requires an existing SQLite DB file"
            });
        if rag_button.clicked() {
            app.config.rag.enabled = !app.config.rag.enabled;
            app.persist_settings();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(layout::section_actions_right_inset());
            let send_clicked = ui
                .add_enabled(
                    !app.is_loading,
                    super::icon_action_button(
                        app,
                        ToolbarIcon::Send,
                        app.theme.accent_color,
                        app.theme.accent_text_color,
                    ),
                )
                .on_hover_text(app.tr("app.send"))
                .clicked();
            if send_clicked {
                app.submit_prompt(ctx);
            }

            if !app.attachments.is_empty()
                && ui
                    .add(super::icon_action_button(
                        app,
                        ToolbarIcon::Clear,
                        app.theme.panel_fill_soft,
                        app.theme.text_color,
                    ))
                    .on_hover_text(app.tr("app.clear_images"))
                    .clicked()
            {
                app.clear_attachments();
            }

            let voice_icon = if app.voice_recording.is_some() {
                ToolbarIcon::Stop
            } else {
                ToolbarIcon::Mic
            };
            let voice_label = if app.voice_recording.is_some() {
                app.tr("app.voice_stop")
            } else {
                app.tr("app.voice_start")
            };
            if ui
                .add(super::icon_action_button(
                    app,
                    voice_icon,
                    app.theme.panel_fill_soft,
                    app.theme.text_color,
                ))
                .on_hover_text(voice_label)
                .clicked()
            {
                app.toggle_dictation(ctx);
            }

            if ui
                .add(super::icon_action_button(
                    app,
                    ToolbarIcon::PasteImage,
                    app.theme.panel_fill_soft,
                    app.theme.text_color,
                ))
                .on_hover_text(app.tr("app.paste_image"))
                .clicked()
            {
                app.attach_image_from_clipboard();
            }

            if ui
                .add(super::icon_action_button(
                    app,
                    ToolbarIcon::AttachImage,
                    app.theme.panel_fill_soft,
                    app.theme.text_color,
                ))
                .on_hover_text(app.tr("app.attach_image"))
                .clicked()
            {
                app.attach_image_from_file();
            }
        });
    });
    ui.add_space(4.0);

    let prompt_hint = app.tr("app.prompt_hint");
    let prompt_max_height = super::editor_max_height(ctx, 88.0);
    app.prompt_editor_height = app.prompt_editor_height.clamp(88.0, prompt_max_height);
    let prompt_before_edit = app.prompt.clone();
    let input_output = super::input_frame(ctx, app.theme.panel_fill).show(ui, |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), app.prompt_editor_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_height(app.prompt_editor_height);
                egui::TextEdit::multiline(&mut app.prompt)
                    .id(prompt_id)
                    .hint_text(prompt_hint)
                    .desired_width(f32::INFINITY)
                    .show(ui)
            },
        )
        .inner
    });
    let input_output = input_output.inner;
    let input_resp = &input_output.response;

    if !app.prompt_focus_initialized {
        input_resp.request_focus();

        let mut state = input_output.state.clone();
        state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(0))));
        state.store(ctx, prompt_id);

        app.prompt_focus_initialized = true;
        ctx.request_repaint();
    }

    let prompt_has_focus = input_resp.has_focus() || ctx.memory(|mem| mem.has_focus(prompt_id));

    let submit_with_shift_enter = prompt_has_focus
        && ctx.input(|i| {
            i.key_pressed(egui::Key::Enter)
                && i.modifiers.shift
                && !i.modifiers.ctrl
                && !i.modifiers.alt
                && !i.modifiers.command
        });

    if submit_with_shift_enter {
        app.prompt = prompt_before_edit.clone();
        app.submit_prompt(ctx);
    }

    let submit_with_copy_close = ctx.input(|i| {
        i.key_pressed(egui::Key::Enter)
            && (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.shift
            && !i.modifiers.alt
    });

    if submit_with_copy_close {
        app.prompt = prompt_before_edit.clone();
        app.auto_copy_close_after_response = true;
        app.submit_prompt(ctx);
    }

    let paste_shortcut_pressed = ctx.input(|i| {
        i.key_pressed(egui::Key::V)
            && (i.modifiers.ctrl || i.modifiers.command)
            && !i.modifiers.shift
            && !i.modifiers.alt
    });
    let paste_action = ctx.input(|i| media_io::classify_prompt_paste_events(&i.events));

    if prompt_has_focus {
        if let Some(path) = paste_action.image_path_from_paste {
            if let Ok(attachment) = media_io::load_image_attachment_from_path(&path) {
                app.attachments.push(attachment);
                app.prompt = prompt_before_edit;
            }
        } else if media_io::should_attach_clipboard_image_from_shortcut(
            paste_shortcut_pressed,
            &prompt_before_edit,
            &app.prompt,
        ) {
            let _ = app.try_attach_image_from_clipboard(false);
        }
    }

    let prompt_helper_text = if app.is_loading {
        app.tr_with(
            "app.helper_waiting",
            &[("backend", app.selected_backend.clone())],
        )
    } else {
        app.tr("app.helper_ready")
    };
    super::editor_resize_row(
        ui,
        &app.theme,
        &mut app.prompt_editor_height,
        88.0,
        prompt_max_height,
        Some(prompt_helper_text.as_str()),
    );
}

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};

use super::markdown::{classify_line, is_code_fence, MarkdownLine};
use super::{layout, media_io, AiPopupApp, ToolbarIcon};

pub(super) fn render_response_section(
    app: &mut AiPopupApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    let response_section_label = app.tr("app.response");

    ui.horizontal(|ui| {
        ui.label(super::section_label(
            &response_section_label,
            app.theme.text_color,
        ));
        if app.is_loading {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(app.tr("app.generating"))
                    .small()
                    .color(app.theme.weak_text_color),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(layout::section_actions_right_inset());
            let history_count = app.history_entries.len();
            let history_label = if app.show_history {
                app.tr_with("app.hide_history", &[("count", history_count.to_string())])
            } else {
                app.tr_with("app.show_history", &[("count", history_count.to_string())])
            };

            if ui
                .add_enabled(
                    app.config.history.enabled,
                    super::icon_action_button(
                        app,
                        if app.show_history {
                            ToolbarIcon::HistoryOpen
                        } else {
                            ToolbarIcon::History
                        },
                        if app.show_history {
                            app.theme.panel_fill_soft
                        } else {
                            app.theme.panel_fill
                        },
                        app.theme.text_color,
                    ),
                )
                .on_hover_text(history_label)
                .clicked()
            {
                app.set_history_visibility(ctx, !app.show_history);
            }

            if ui
                .add_enabled(
                    !app.response.is_empty(),
                    super::icon_action_button(
                        app,
                        ToolbarIcon::Copy,
                        app.theme.panel_fill_soft,
                        app.theme.text_color,
                    ),
                )
                .on_hover_text(app.tr("app.copy_response"))
                .clicked()
            {
                media_io::copy_markdown_rendered_text_to_clipboard(&app.response);
            }
        });
    });
    ui.add_space(4.0);

    let response_max_height = super::editor_max_height(ctx, 140.0);
    app.response_editor_height = app.response_editor_height.clamp(84.0, response_max_height);
    super::input_frame(ctx, app.theme.panel_fill).show(ui, |ui| {
        ui.set_min_height(app.response_editor_height);
        egui::ScrollArea::vertical()
            .max_height(app.response_editor_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                render_markdown_like_response(app, ui);
            });
    });
    super::editor_resize_row(
        ui,
        &app.theme,
        &mut app.response_editor_height,
        84.0,
        response_max_height,
        None,
    );
}

fn render_markdown_like_response(app: &AiPopupApp, ui: &mut egui::Ui) {
    let response = app.response.trim_end();
    if response.is_empty() {
        return;
    }

    let mut in_code_block = false;
    let mut code_block = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim_end();

        if is_code_fence(trimmed) {
            if in_code_block {
                render_code_block(app, ui, &code_block.join("\n"));
                code_block.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_block.push(trimmed.to_string());
            continue;
        }

        render_markdown_line(app, ui, trimmed);
    }

    if in_code_block {
        render_code_block(app, ui, &code_block.join("\n"));
    }
}

fn render_markdown_line(app: &AiPopupApp, ui: &mut egui::Ui, line: &str) {
    match classify_line(line) {
        MarkdownLine::Empty => {
            ui.add_space(4.0);
        }
        MarkdownLine::Heading { level, content } => {
            let size = match level {
                1 => 20.0,
                2 => 18.0,
                _ => 16.0,
            };
            render_inline_markdown_label(ui, app, content, size, app.theme.text_color, false);
        }
        MarkdownLine::Bullet { content } => {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("•").color(app.theme.text_color));
                render_inline_markdown_label(ui, app, content, 14.0, app.theme.text_color, false);
            });
        }
        MarkdownLine::Numbered { index, content } => {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("{index}."))
                        .size(14.0)
                        .color(app.theme.text_color),
                );
                render_inline_markdown_label(ui, app, content, 14.0, app.theme.text_color, false);
            });
        }
        MarkdownLine::Quote { content } => {
            render_inline_markdown_label(ui, app, content, 14.0, app.theme.weak_text_color, true);
        }
        MarkdownLine::SingleLink { text, url } => {
            ui.hyperlink_to(text, url);
        }
        MarkdownLine::Plain { content } => {
            render_inline_markdown_label(ui, app, content, 14.0, app.theme.text_color, false);
        }
    }
}

fn render_code_block(app: &AiPopupApp, ui: &mut egui::Ui, code: &str) {
    if code.trim().is_empty() {
        return;
    }

    egui::Frame::none()
        .fill(app.theme.panel_fill_soft)
        .stroke(egui::Stroke::new(
            1.0,
            app.theme.border_color.gamma_multiply(0.25),
        ))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .rounding(egui::Rounding::same(8.0))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(code)
                        .monospace()
                        .size(13.0)
                        .color(app.theme.text_color),
                )
                .wrap(true),
            );
        });
}

fn render_inline_markdown_label(
    ui: &mut egui::Ui,
    app: &AiPopupApp,
    text: &str,
    size: f32,
    color: egui::Color32,
    base_italics: bool,
) {
    ui.label(inline_markdown_job(app, text, size, color, base_italics));
}

fn inline_markdown_job(
    app: &AiPopupApp,
    text: &str,
    size: f32,
    color: egui::Color32,
    base_italics: bool,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut plain = String::new();
    let mut rest = text;

    while !rest.is_empty() {
        if let Some(content) = parse_inline_segment(&mut rest, "**", "**") {
            flush_plain_segment(
                &mut job,
                &mut plain,
                size,
                color,
                false,
                base_italics,
                false,
            );
            append_inline_segment(
                &mut job,
                content,
                size,
                color,
                true,
                base_italics,
                false,
                app.theme.panel_fill_soft,
            );
            continue;
        }

        if let Some(content) = parse_inline_segment(&mut rest, "`", "`") {
            flush_plain_segment(
                &mut job,
                &mut plain,
                size,
                color,
                false,
                base_italics,
                false,
            );
            append_inline_segment(
                &mut job,
                content,
                size,
                color,
                false,
                false,
                true,
                app.theme.panel_fill_soft,
            );
            continue;
        }

        if let Some(content) = parse_inline_segment(&mut rest, "*", "*") {
            flush_plain_segment(
                &mut job,
                &mut plain,
                size,
                color,
                false,
                base_italics,
                false,
            );
            append_inline_segment(
                &mut job,
                content,
                size,
                color,
                false,
                true,
                false,
                app.theme.panel_fill_soft,
            );
            continue;
        }

        if let Some(ch) = rest.chars().next() {
            plain.push(ch);
            rest = &rest[ch.len_utf8()..];
        } else {
            break;
        }
    }

    flush_plain_segment(
        &mut job,
        &mut plain,
        size,
        color,
        false,
        base_italics,
        false,
    );
    job
}

fn parse_inline_segment<'a>(rest: &mut &'a str, start: &str, end: &str) -> Option<&'a str> {
    if !rest.starts_with(start) {
        return None;
    }

    let after_start = &rest[start.len()..];
    let end_index = after_start.find(end)?;
    if end_index == 0 {
        return None;
    }

    let content = &after_start[..end_index];
    *rest = &after_start[end_index + end.len()..];
    Some(content)
}

fn flush_plain_segment(
    job: &mut LayoutJob,
    plain: &mut String,
    size: f32,
    color: egui::Color32,
    bold: bool,
    italics: bool,
    code: bool,
) {
    if plain.is_empty() {
        return;
    }
    let text = std::mem::take(plain);
    append_inline_segment(
        job,
        &text,
        size,
        color,
        bold,
        italics,
        code,
        egui::Color32::TRANSPARENT,
    );
}

fn append_inline_segment(
    job: &mut LayoutJob,
    text: &str,
    size: f32,
    color: egui::Color32,
    bold: bool,
    italics: bool,
    code: bool,
    code_background: egui::Color32,
) {
    if text.is_empty() {
        return;
    }

    let mut format = TextFormat {
        font_id: if code {
            egui::FontId::monospace((size - 1.0).max(11.0))
        } else {
            egui::FontId::proportional(size)
        },
        color,
        italics,
        ..Default::default()
    };

    if bold {
        format.font_id = egui::FontId::proportional(size + 0.8);
    }

    if code {
        format.background = code_background;
    }

    job.append(text, 0.0, format);
}

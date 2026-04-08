use eframe::egui;

use super::{media_io, AiPopupApp};

pub(super) fn status_section_has_content(app: &AiPopupApp) -> bool {
    status_section_has_content_state(
        !app.attachments.is_empty(),
        app.dictation_status.is_some(),
        app.attachment_notice.is_some(),
        app.attachment_error.is_some(),
        app.settings_notice.is_some(),
        app.settings_error.is_some(),
    )
}

pub(super) fn status_section_has_content_state(
    has_attachments: bool,
    has_dictation_status: bool,
    has_attachment_notice: bool,
    has_attachment_error: bool,
    has_settings_notice: bool,
    has_settings_error: bool,
) -> bool {
    has_attachments
        || has_dictation_status
        || has_attachment_notice
        || has_attachment_error
        || has_settings_notice
        || has_settings_error
}

pub(super) fn render_status_section(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    super::card_frame(
        ui.ctx(),
        app.theme.panel_fill_soft,
        app.theme.border_color.gamma_multiply(0.65),
    )
    .show(ui, |ui| {
        if !app.attachments.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(app.tr_with(
                        "app.images_attached",
                        &[("count", app.attachments.len().to_string())],
                    ))
                    .small()
                    .color(app.theme.weak_text_color),
                );
                for image in &app.attachments {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} ({})",
                            image.name,
                            media_io::format_size(image.size_bytes)
                        ))
                        .small()
                        .color(app.theme.text_color),
                    );
                }
            });
            ui.add_space(6.0);
        }

        if let Some(status) = &app.dictation_status {
            ui.label(super::muted_label(status, app.theme.weak_text_color));
        }
        if let Some(notice) = &app.attachment_notice {
            ui.add_space(4.0);
            ui.label(super::muted_label(notice, app.theme.weak_text_color));
        }
        if let Some(notice) = &app.settings_notice {
            ui.add_space(4.0);
            ui.label(super::muted_label(notice, app.theme.weak_text_color));
        }
        if let Some(error) = &app.attachment_error {
            ui.add_space(4.0);
            ui.colored_label(app.theme.danger_color, error);
        }
        if let Some(error) = &app.settings_error {
            ui.add_space(4.0);
            ui.colored_label(app.theme.danger_color, error);
        }
    });
}

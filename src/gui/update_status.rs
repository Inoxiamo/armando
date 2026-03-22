use super::{egui, AiPopupApp, ReleaseCheckState, ToolbarIcon};

pub(super) fn render_update_status(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    match &app.release_check_state {
        ReleaseCheckState::Checking => {
            ui.label(
                egui::RichText::new(app.tr("settings.update_checking"))
                    .small()
                    .color(app.theme.weak_text_color),
            );
        }
        ReleaseCheckState::UpToDate { latest_version } => {
            ui.label(
                egui::RichText::new(format!(
                    "{} (v{})",
                    app.tr("settings.update_current"),
                    latest_version
                ))
                .small()
                .color(app.theme.weak_text_color),
            );
        }
        ReleaseCheckState::UpdateAvailable(release) => {
            let guide = crate::update::current_platform_update_guide();
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(app.tr_with(
                            "settings.update_available",
                            &[("version", release.version.clone())],
                        ))
                        .small()
                        .color(app.theme.accent_color),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(app.tr_with(
                            "settings.update_guided_platform",
                            &[("platform", guide.platform_label.clone())],
                        ))
                        .small()
                        .color(app.theme.weak_text_color),
                    );
                });
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(guide.detail.clone())
                        .small()
                        .color(app.theme.weak_text_color),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add(super::icon_action_button_sized(
                            app,
                            ToolbarIcon::Update,
                            app.theme.accent_color,
                            app.theme.accent_text_color,
                            egui::vec2(20.0, 20.0),
                        ))
                        .on_hover_text(app.tr("settings.update_open_download"))
                        .clicked()
                    {
                        super::open_url(ctx, release.release_url.clone());
                    }

                    match &guide.action {
                        crate::update::UpdateAction::CopyCommand { command } => {
                            if ui
                                .add(super::secondary_action_button(
                                    &app.tr("settings.update_copy_command"),
                                    app.theme.panel_fill_soft,
                                ))
                                .clicked()
                            {
                                super::media_io::copy_text_to_clipboard(command);
                            }
                        }
                        crate::update::UpdateAction::OpenReleasePage => {
                            if ui
                                .add(super::secondary_action_button(
                                    &app.tr("settings.update_open_release_page"),
                                    app.theme.panel_fill_soft,
                                ))
                                .clicked()
                            {
                                super::open_url(ctx, release.release_url.clone());
                            }
                        }
                    }
                });
            });
        }
        ReleaseCheckState::Error(error) => {
            ui.label(
                egui::RichText::new(app.tr("settings.update_error"))
                    .small()
                    .color(app.theme.weak_text_color),
            )
            .on_hover_text(error);
        }
    }
}

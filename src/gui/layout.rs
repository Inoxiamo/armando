use eframe::egui;

pub(super) fn sync_main_viewport(
    ctx: &egui::Context,
    show_history: bool,
    show_settings: bool,
    window_height: f32,
) {
    let desired_size = main_viewport_min_size(show_history, show_settings, window_height);
    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(desired_size));

    if let Some(next_size) = requested_viewport_inner_size(
        ctx.input(|i| i.viewport().inner_rect.map(|rect| rect.size())),
        desired_size,
    ) {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(next_size));
    }
}

pub(super) fn default_prompt_editor_height(window_height: f32) -> f32 {
    (window_height * 0.16).clamp(88.0, 136.0)
}

pub(super) fn default_response_editor_height(window_height: f32) -> f32 {
    (window_height * 0.18).clamp(96.0, 156.0)
}

pub(super) fn main_viewport_min_size(
    show_history: bool,
    show_settings: bool,
    window_height: f32,
) -> egui::Vec2 {
    const BASE_MIN_WIDTH: f32 = 820.0;
    const BASE_MIN_HEIGHT: f32 = 500.0;
    const SETTINGS_MIN_WIDTH: f32 = 1320.0;
    const SETTINGS_MIN_HEIGHT: f32 = 600.0;
    const HISTORY_MIN_HEIGHT: f32 = 620.0;
    const MAX_DEFAULT_HEIGHT: f32 = 680.0;

    let preferred_height = window_height.clamp(BASE_MIN_HEIGHT, MAX_DEFAULT_HEIGHT);

    let min_width = if show_settings {
        SETTINGS_MIN_WIDTH
    } else {
        BASE_MIN_WIDTH
    };
    let min_height = if show_settings {
        preferred_height
            .max(SETTINGS_MIN_HEIGHT)
            .max(if show_history {
                HISTORY_MIN_HEIGHT
            } else {
                BASE_MIN_HEIGHT
            })
    } else if show_history {
        preferred_height.max(HISTORY_MIN_HEIGHT)
    } else {
        preferred_height
    };

    egui::vec2(min_width, min_height)
}

pub(super) fn editor_max_height_for_viewport(viewport_height: f32, min_height: f32) -> f32 {
    (viewport_height / 3.0).max(min_height)
}

pub(super) fn requested_viewport_inner_size(
    current_size: Option<egui::Vec2>,
    desired_size: egui::Vec2,
) -> Option<egui::Vec2> {
    current_size.and_then(|current_size| {
        if current_size.x < desired_size.x || current_size.y < desired_size.y {
            Some(egui::vec2(
                current_size.x.max(desired_size.x),
                current_size.y.max(desired_size.y),
            ))
        } else {
            None
        }
    })
}

pub(super) fn section_actions_right_inset() -> f32 {
    10.0
}

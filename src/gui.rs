use base64::Engine as _;
use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::app_paths;
use crate::backends;
use crate::backends::PromptMode;
use crate::backends::ResponseProgress;
use crate::backends::ResponseProgressSink;
use crate::backends::{ConversationTurn, HealthCheck, HealthLevel, ImageAttachment, QueryInput};
use crate::config::Config;
use crate::history::{self, HistoryEntry};
use crate::i18n::{available_locales, I18n, LocaleDefinition};
use crate::prompt_profiles::PromptProfiles;
use crate::theme::{available_theme_names, load_theme_by_name, ResolvedTheme};
use crate::update::{self, ReleaseInfo, UpdateAction};
use crate::window_context;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn display_version() -> String {
    APP_VERSION.to_string()
}

fn load_toolbar_icon_textures(ctx: &egui::Context) -> HashMap<ToolbarIcon, egui::TextureHandle> {
    let mut textures = HashMap::new();
    for (icon, name, source) in [
        (
            ToolbarIcon::Settings,
            "toolbar_settings",
            IconTextureSource::Png(include_bytes!("../assets/icons/settings.png")),
        ),
        (
            ToolbarIcon::Send,
            "toolbar_send",
            IconTextureSource::Png(include_bytes!("../assets/icons/send.png")),
        ),
        (
            ToolbarIcon::Clear,
            "toolbar_clear",
            IconTextureSource::Svg(include_str!("../assets/icons/close.svg")),
        ),
        (
            ToolbarIcon::Mic,
            "toolbar_mic",
            IconTextureSource::Png(include_bytes!("../assets/icons/mic.png")),
        ),
        (
            ToolbarIcon::Stop,
            "toolbar_stop",
            IconTextureSource::Svg(include_str!("../assets/icons/stop.svg")),
        ),
        (
            ToolbarIcon::PasteImage,
            "toolbar_paste_image",
            IconTextureSource::Png(include_bytes!("../assets/icons/screenshot.png")),
        ),
        (
            ToolbarIcon::AttachImage,
            "toolbar_attach_image",
            IconTextureSource::Png(include_bytes!("../assets/icons/attach-image.png")),
        ),
        (
            ToolbarIcon::History,
            "toolbar_history",
            IconTextureSource::Svg(include_str!("../assets/icons/history.svg")),
        ),
        (
            ToolbarIcon::HistoryOpen,
            "toolbar_history_open",
            IconTextureSource::Svg(include_str!("../assets/icons/history-open.svg")),
        ),
        (
            ToolbarIcon::Copy,
            "toolbar_copy",
            IconTextureSource::Svg(include_str!("../assets/icons/copy.svg")),
        ),
        (
            ToolbarIcon::Close,
            "toolbar_close",
            IconTextureSource::Svg(include_str!("../assets/icons/close.svg")),
        ),
    ] {
        let rendered = match source {
            IconTextureSource::Png(bytes) => render_png_icon(bytes),
            IconTextureSource::Svg(svg) => render_svg_icon(svg, ctx.pixels_per_point()),
        };

        match rendered {
            Ok(image) => {
                let texture = ctx.load_texture(name, image, egui::TextureOptions::LINEAR);
                textures.insert(icon, texture);
            }
            Err(err) => {
                log::error!("Could not render toolbar icon `{name}`: {err}");
            }
        }
    }
    textures
}

enum IconTextureSource {
    Svg(&'static str),
    Png(&'static [u8]),
}

fn render_png_icon(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    const PNG_ALPHA_THRESHOLD: u8 = 8;

    let image = image::load_from_memory(bytes)
        .map_err(|err| format!("Invalid PNG: {err}"))?
        .to_rgba8();
    let mut rgba = image.clone().into_raw();
    let width = image.width() as usize;
    let height = image.height() as usize;

    strip_edge_frame(&mut rgba, width, height);
    let cropped = crop_and_center_icon(&rgba, width, height, PNG_ALPHA_THRESHOLD);

    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [width, height],
        &cropped,
    ))
}

fn strip_edge_frame(rgba: &mut [u8], width: usize, height: usize) {
    const EDGE_ALPHA_THRESHOLD: u8 = 96;
    const EDGE_WHITE_THRESHOLD: u8 = 240;

    let mut queue = VecDeque::new();
    let mut visited = vec![false; width * height];

    let mut enqueue = |x: usize, y: usize, queue: &mut VecDeque<(usize, usize)>| {
        let index = y * width + x;
        if !visited[index] {
            visited[index] = true;
            queue.push_back((x, y));
        }
    };

    for x in 0..width {
        enqueue(x, 0, &mut queue);
        enqueue(x, height - 1, &mut queue);
    }
    for y in 0..height {
        enqueue(0, y, &mut queue);
        enqueue(width - 1, y, &mut queue);
    }

    while let Some((x, y)) = queue.pop_front() {
        let pixel_index = (y * width + x) * 4;
        let r = rgba[pixel_index];
        let g = rgba[pixel_index + 1];
        let b = rgba[pixel_index + 2];
        let a = rgba[pixel_index + 3];

        let is_edge_frame = a >= EDGE_ALPHA_THRESHOLD
            && r >= EDGE_WHITE_THRESHOLD
            && g >= EDGE_WHITE_THRESHOLD
            && b >= EDGE_WHITE_THRESHOLD;

        if !is_edge_frame {
            continue;
        }

        rgba[pixel_index..pixel_index + 4].fill(0);

        if x > 0 {
            enqueue(x - 1, y, &mut queue);
        }
        if x + 1 < width {
            enqueue(x + 1, y, &mut queue);
        }
        if y > 0 {
            enqueue(x, y - 1, &mut queue);
        }
        if y + 1 < height {
            enqueue(x, y + 1, &mut queue);
        }
    }
}

fn render_svg_icon(svg: &str, pixels_per_point: f32) -> Result<egui::ColorImage, String> {
    const ICON_DRAW_SIZE_POINTS: f32 = 32.0;
    const ICON_OVERSAMPLE: f32 = 16.0;
    const MIN_TARGET_SIZE: u32 = 768;
    const ICON_CANVAS_PADDING: f32 = 0.03;
    const ALPHA_CROP_THRESHOLD: u8 = 2;

    let target_size = ((ICON_DRAW_SIZE_POINTS * pixels_per_point * ICON_OVERSAMPLE).ceil() as u32)
        .max(MIN_TARGET_SIZE);

    let options = resvg::usvg::Options::default();
    let tree =
        resvg::usvg::Tree::from_str(svg, &options).map_err(|err| format!("Invalid SVG: {err}"))?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(target_size, target_size)
        .ok_or_else(|| "Could not allocate pixmap for SVG icon.".to_string())?;

    let usable_size = target_size as f32 * (1.0 - ICON_CANVAS_PADDING * 2.0);
    let scale = (usable_size / size.width() as f32).min(usable_size / size.height() as f32);
    let rendered_width = size.width() as f32 * scale;
    let rendered_height = size.height() as f32 * scale;
    let translate_x = (target_size as f32 - rendered_width) * 0.5;
    let translate_y = (target_size as f32 - rendered_height) * 0.5;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_translate(translate_x, translate_y)
            .post_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let rgba = pixmap.data().to_vec();
    let cropped = crop_and_center_icon(
        &rgba,
        target_size as usize,
        target_size as usize,
        ALPHA_CROP_THRESHOLD,
    );

    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [target_size as usize, target_size as usize],
        &cropped,
    ))
}

fn crop_and_center_icon(rgba: &[u8], width: usize, height: usize, alpha_threshold: u8) -> Vec<u8> {
    let Some((min_x, min_y, max_x, max_y)) = visible_bounds(rgba, width, height, alpha_threshold)
    else {
        return rgba.to_vec();
    };

    let content_width = max_x - min_x + 1;
    let content_height = max_y - min_y + 1;
    let side = content_width.max(content_height);
    let offset_x = (width.saturating_sub(side)) / 2;
    let offset_y = (height.saturating_sub(side)) / 2;
    let inset_x = (side.saturating_sub(content_width)) / 2;
    let inset_y = (side.saturating_sub(content_height)) / 2;

    let mut normalized = vec![0_u8; rgba.len()];
    for y in 0..content_height {
        for x in 0..content_width {
            let src_x = min_x + x;
            let src_y = min_y + y;
            let dst_x = offset_x + inset_x + x;
            let dst_y = offset_y + inset_y + y;

            let src_index = (src_y * width + src_x) * 4;
            let dst_index = (dst_y * width + dst_x) * 4;
            normalized[dst_index..dst_index + 4].copy_from_slice(&rgba[src_index..src_index + 4]);
        }
    }

    normalized
}

fn visible_bounds(
    rgba: &[u8],
    width: usize,
    height: usize,
    alpha_threshold: u8,
) -> Option<(usize, usize, usize, usize)> {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for y in 0..height {
        for x in 0..width {
            let alpha = rgba[(y * width + x) * 4 + 3];
            if alpha > alpha_threshold {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }

    found.then_some((min_x, min_y, max_x, max_y))
}

struct VoiceRecording {
    child: Child,
    path: PathBuf,
}

type AsyncAvailableModels = Arc<Mutex<Vec<(String, Result<Vec<String>, String>)>>>;
type AsyncReleaseCheck = Arc<Mutex<Option<Result<ReleaseInfo, String>>>>;

#[derive(Clone, PartialEq, Eq)]
struct RequestFingerprint {
    backend: String,
    prompt: String,
    images: Vec<ImageAttachment>,
    mode: PromptMode,
    session_chat_enabled: bool,
    conversation: Vec<ConversationTurn>,
    active_window_context: Option<String>,
}

#[derive(Default)]
struct ProviderModelState {
    models: Vec<String>,
    is_loading: bool,
    last_error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ToolbarIcon {
    Settings,
    Update,
    Send,
    Clear,
    Mic,
    Stop,
    PasteImage,
    AttachImage,
    History,
    HistoryOpen,
    Copy,
    Close,
}

#[derive(Clone)]
enum ReleaseCheckState {
    Checking,
    UpToDate { latest_version: String },
    UpdateAvailable(ReleaseInfo),
    Error(String),
}

const SESSION_HISTORY_LIMIT: usize = 24;

pub struct AiPopupApp {
    config: Config,
    prompt_profiles: PromptProfiles,
    theme: ResolvedTheme,
    runtime: Arc<Runtime>,

    // UI State
    prompt: String,
    response: String,
    attachments: Vec<ImageAttachment>,
    attachment_notice: Option<String>,
    attachment_error: Option<String>,
    selected_backend: String,
    is_loading: bool,
    auto_copy_close_after_response: bool,
    dictation_status: Option<String>,
    voice_recording: Option<VoiceRecording>,
    prompt_focus_initialized: bool,
    prompt_editor_height: f32,
    response_editor_height: f32,
    generic_question_mode: bool,
    session_chat_enabled: bool,
    session_conversation: Vec<ConversationTurn>,
    session_history_entries: Vec<HistoryEntry>,
    history_entries: Vec<HistoryEntry>,
    history_error: Option<String>,
    show_history: bool,
    selected_history_entries: HashSet<String>,
    history_filter_backend: String,
    history_filter_query: String,
    history_action_error: Option<String>,
    settings_error: Option<String>,
    settings_notice: Option<String>,
    show_settings: bool,
    pending_submission: Option<(String, String)>,
    last_submitted_request: Option<RequestFingerprint>,
    first_run_template: String,
    available_themes: Vec<String>,
    available_locales: Vec<LocaleDefinition>,
    i18n: I18n,
    provider_model_states: HashMap<String, ProviderModelState>,
    toolbar_icon_textures: HashMap<ToolbarIcon, egui::TextureHandle>,
    toolbar_icon_scale: f32,

    // For tokio to update UI when done
    async_response: Arc<Mutex<Option<Result<String, String>>>>,
    async_response_chunks: Arc<Mutex<Vec<String>>>,
    async_dictation: Arc<Mutex<Option<Result<String, String>>>>,
    async_available_models: AsyncAvailableModels,
    async_release_check: AsyncReleaseCheck,
    release_check_state: ReleaseCheckState,
    last_completed_request: Option<RequestFingerprint>,
}

impl AiPopupApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config: Config,
        prompt_profiles: PromptProfiles,
        theme: ResolvedTheme,
        runtime: Arc<Runtime>,
    ) -> Self {
        let style = build_style(&theme);
        cc.egui_ctx.set_style(style);
        let i18n = I18n::load(&config.ui.language).unwrap_or_else(|_| I18n::load("en").unwrap());

        let default_backend = config.default_backend.clone();

        // Read PRIMARY selection if enabled
        let mut initial_prompt = String::new();
        if config.auto_read_selection {
            if let Ok(text) = get_primary_selection() {
                if !text.trim().is_empty() {
                    initial_prompt = text;
                }
            }
        }

        let (history_entries, history_error) = if config.history.enabled {
            match history::recent_entries() {
                Ok(entries) => (entries, None),
                Err(err) => (Vec::new(), Some(err.to_string())),
            }
        } else {
            (Vec::new(), None)
        };

        let fallback_theme_name = config.theme.name.clone();
        let prompt_editor_height = default_prompt_editor_height(config.ui.window_height);
        let response_editor_height = default_response_editor_height(config.ui.window_height);
        let first_run_template = app_paths::discover_config_template_names()
            .ok()
            .and_then(|names| {
                if names.iter().any(|name| name == "default") {
                    Some("default".to_string())
                } else {
                    names.into_iter().next()
                }
            })
            .unwrap_or_else(|| "default".to_string());

        let mut app = Self {
            config,
            prompt_profiles,
            theme,
            runtime,
            prompt: initial_prompt,
            response: String::new(),
            attachments: Vec::new(),
            attachment_notice: None,
            attachment_error: None,
            selected_backend: default_backend,
            is_loading: false,
            auto_copy_close_after_response: false,
            dictation_status: None,
            voice_recording: None,
            prompt_focus_initialized: false,
            prompt_editor_height,
            response_editor_height,
            generic_question_mode: false,
            session_chat_enabled: false,
            session_conversation: Vec::new(),
            session_history_entries: Vec::new(),
            history_entries,
            history_error,
            show_history: false,
            selected_history_entries: HashSet::new(),
            history_filter_backend: "all".to_string(),
            history_filter_query: String::new(),
            history_action_error: None,
            settings_error: None,
            settings_notice: None,
            show_settings: false,
            pending_submission: None,
            last_submitted_request: None,
            first_run_template,
            available_themes: available_theme_names().unwrap_or_else(|_| vec![fallback_theme_name]),
            available_locales: available_locales().unwrap_or_default(),
            i18n,
            provider_model_states: HashMap::new(),
            toolbar_icon_textures: load_toolbar_icon_textures(&cc.egui_ctx),
            toolbar_icon_scale: cc.egui_ctx.pixels_per_point(),
            async_response: Arc::new(Mutex::new(None)),
            async_response_chunks: Arc::new(Mutex::new(Vec::new())),
            async_dictation: Arc::new(Mutex::new(None)),
            async_available_models: Arc::new(Mutex::new(Vec::new())),
            async_release_check: Arc::new(Mutex::new(None)),
            release_check_state: ReleaseCheckState::Checking,
            last_completed_request: None,
        };
        app.start_release_check(&cc.egui_ctx);
        app
    }

    fn check_async_response(&mut self, _ctx: &egui::Context) {
        let response_chunks = {
            let mut chunk_lock = self.async_response_chunks.lock().unwrap();
            std::mem::take(&mut *chunk_lock)
        };

        for chunk in response_chunks {
            let had_placeholder = self.response.starts_with("⏳ Querying ");
            if had_placeholder {
                self.response.clear();
            }
            self.response.push_str(&chunk);
        }

        let res = {
            let mut resp_lock = self.async_response.lock().unwrap();
            resp_lock.take()
        };

        if let Some(res) = res {
            self.is_loading = false;
            match res {
                Ok(text) => {
                    if !text.is_empty() {
                        self.response = text;
                    }
                    if let Some((backend, prompt)) = self.pending_submission.take() {
                        if let Ok(entry) = history::new_entry(&backend, &prompt, &self.response) {
                            push_session_history_entry(
                                &mut self.session_history_entries,
                                entry.clone(),
                            );
                            if self.config.history.enabled {
                                let _ = history::append_entry(entry);
                            }
                        }
                        if self.session_chat_enabled {
                            self.session_conversation.push(ConversationTurn {
                                user_prompt: prompt,
                                assistant_response: self.response.clone(),
                            });
                        }
                        self.last_completed_request = self.last_submitted_request.take();
                    }
                    self.reload_history();
                    if self.auto_copy_close_after_response {
                        self.auto_copy_close_after_response = false;
                        self.copy_response_and_close(_ctx);
                    }
                }
                Err(e) => {
                    self.response = format!("Error: {e}");
                    self.pending_submission = None;
                    self.last_submitted_request = None;
                    self.auto_copy_close_after_response = false;
                }
            }
        }

        let dictation = {
            let mut dictation_lock = self.async_dictation.lock().unwrap();
            dictation_lock.take()
        };

        if let Some(res) = dictation {
            match res {
                Ok(text) => {
                    if !text.trim().is_empty() {
                        if !self.prompt.trim().is_empty() && !self.prompt.ends_with('\n') {
                            self.prompt.push('\n');
                        }
                        self.prompt.push_str(text.trim());
                    }
                    self.dictation_status = Some(self.tr("app.voice_ready"));
                    self.attachment_error = None;
                }
                Err(error) => {
                    self.dictation_status = Some(error);
                }
            }
        }

        let available_models = {
            let mut models_lock = self.async_available_models.lock().unwrap();
            std::mem::take(&mut *models_lock)
        };

        for (provider, result) in available_models {
            let state = self.provider_model_states.entry(provider).or_default();
            state.is_loading = false;
            match result {
                Ok(models) => {
                    state.models = models;
                    state.last_error = None;
                }
                Err(error) => {
                    state.last_error = Some(error);
                }
            }
        }

        let release_check = {
            let mut release_lock = self.async_release_check.lock().unwrap();
            release_lock.take()
        };

        if let Some(result) = release_check {
            self.release_check_state = match result {
                Ok(release) if update::update_available(APP_VERSION, &release.version) => {
                    ReleaseCheckState::UpdateAvailable(release)
                }
                Ok(release) => ReleaseCheckState::UpToDate {
                    latest_version: release.version,
                },
                Err(error) => ReleaseCheckState::Error(error),
            };
        }
    }

    fn start_release_check(&mut self, ctx: &egui::Context) {
        self.release_check_state = ReleaseCheckState::Checking;
        let async_release_check = self.async_release_check.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let result = update::fetch_latest_release().await;
            *async_release_check.lock().unwrap() = Some(result);
            ctx.request_repaint();
        });
    }

    fn submit_prompt(&mut self, ctx: &egui::Context) {
        if (self.prompt.trim().is_empty() && self.attachments.is_empty()) || self.is_loading {
            return;
        }

        let active_window_context = window_context::current_active_window_context();
        let current_request = self.current_request_fingerprint(active_window_context.as_deref());
        if self
            .last_completed_request
            .as_ref()
            .is_some_and(|last| *last == current_request)
            && !self.response.trim().is_empty()
        {
            if self.auto_copy_close_after_response {
                self.auto_copy_close_after_response = false;
                self.copy_response_and_close(ctx);
            }
            return;
        }

        self.is_loading = true;
        self.response = format!("⏳ Querying {}…", self.selected_backend);

        let prompt = self.prompt.clone();
        let images = self.attachments.clone();
        let conversation = if self.session_chat_enabled {
            self.session_conversation.clone()
        } else {
            Vec::new()
        };
        let backend = self.selected_backend.clone();
        let mode = if self.generic_question_mode {
            PromptMode::GenericQuestion
        } else {
            PromptMode::TextAssist
        };
        let config = self.config.clone();
        let prompt_profiles = self.prompt_profiles.clone();
        let async_response = self.async_response.clone();
        let async_response_chunks = self.async_response_chunks.clone();
        let progress_ctx = ctx.clone();
        self.pending_submission = Some((backend.clone(), prompt.clone()));
        self.last_submitted_request = Some(current_request);
        self.attachment_notice = None;
        self.attachment_error = None;
        self.async_response_chunks.lock().unwrap().clear();

        let response_progress: Option<ResponseProgressSink> =
            Some(Arc::new(move |event: ResponseProgress| match event {
                ResponseProgress::Chunk(chunk) => {
                    async_response_chunks.lock().unwrap().push(chunk);
                    progress_ctx.request_repaint();
                }
            }));

        // Spawn async task
        let ctx = ctx.clone();
        self.runtime.spawn(async move {
            let res = backends::query(
                &backend,
                &QueryInput {
                    prompt,
                    images,
                    conversation,
                    active_window_context,
                },
                &config,
                &prompt_profiles,
                mode,
                response_progress,
            )
            .await;

            // Store result
            *async_response.lock().unwrap() = Some(Ok(res));

            // Request UI repaint since we updated state from background thread
            ctx.request_repaint();
        });
    }

    fn reload_history(&mut self) {
        if !self.config.history.enabled {
            self.history_entries.clear();
            self.selected_history_entries.clear();
            self.history_error = None;
            self.history_action_error = None;
            return;
        }

        match history::recent_entries() {
            Ok(entries) => {
                self.history_entries = entries;
                self.selected_history_entries.retain(|id| {
                    self.history_entries
                        .iter()
                        .any(|entry| history::entry_id(entry) == *id)
                });
                self.history_error = None;
                self.history_action_error = None;
            }
            Err(err) => {
                self.history_error = Some(err.to_string());
            }
        }
    }

    fn delete_selected_history_entries(&mut self) {
        if !self.config.history.enabled {
            return;
        }
        if self.selected_history_entries.is_empty() {
            return;
        }

        let ids: Vec<String> = self.selected_history_entries.iter().cloned().collect();
        match history::delete_entries(&ids) {
            Ok(()) => {
                self.selected_history_entries.clear();
                self.reload_history();
            }
            Err(err) => {
                self.history_action_error = Some(err.to_string());
            }
        }
    }

    fn select_all_visible_history_entries(&mut self) {
        if !self.config.history.enabled {
            return;
        }
        for entry in &self.session_history_entries {
            self.selected_history_entries
                .insert(history::entry_id(entry));
        }
        for entry in self.filtered_history_entries() {
            self.selected_history_entries
                .insert(history::entry_id(&entry));
        }
    }

    fn delete_all_visible_history_entries(&mut self) {
        if !self.config.history.enabled {
            return;
        }
        let mut ids: Vec<String> = self
            .filtered_history_entries()
            .iter()
            .map(history::entry_id)
            .collect();
        ids.sort();
        ids.dedup();

        if ids.is_empty() {
            return;
        }

        match history::delete_entries(&ids) {
            Ok(()) => {
                self.selected_history_entries.clear();
                self.reload_history();
            }
            Err(err) => {
                self.history_action_error = Some(err.to_string());
            }
        }
    }

    fn filtered_history_entries(&self) -> Vec<HistoryEntry> {
        if !self.config.history.enabled {
            return Vec::new();
        }
        let query = self.history_filter_query.trim().to_lowercase();
        self.history_entries
            .iter()
            .filter(|entry| {
                self.history_filter_backend == "all" || entry.backend == self.history_filter_backend
            })
            .filter(|entry| {
                query.is_empty()
                    || entry.prompt.to_lowercase().contains(&query)
                    || entry.response.to_lowercase().contains(&query)
                    || entry.created_at.to_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    fn set_history_visibility(&mut self, ctx: &egui::Context, visible: bool) {
        self.show_history = self.config.history.enabled && visible;
        if self.show_history {
            self.reload_history();
        }
        sync_main_viewport(
            ctx,
            self.show_history,
            self.show_settings,
            self.config.ui.window_height,
        );
        ctx.request_repaint();
    }

    fn set_settings_visibility(&mut self, ctx: &egui::Context, visible: bool) {
        self.show_settings = visible;
        sync_main_viewport(
            ctx,
            self.show_history,
            self.show_settings,
            self.config.ui.window_height,
        );
        ctx.request_repaint();
    }

    fn tr(&self, key: &str) -> String {
        self.i18n.tr(key)
    }

    fn tr_with(&self, key: &str, pairs: &[(&str, String)]) -> String {
        let mut value = self.tr(key);
        for (name, replacement) in pairs {
            value = value.replace(&format!("{{{name}}}"), replacement);
        }
        value
    }

    fn ensure_config_sections(&mut self) {
        if self.config.gemini.is_none() {
            self.config.gemini = Some(crate::config::GeminiConfig {
                api_key: String::new(),
                model: "gemini-flash-latest".to_string(),
            });
        }
        if self.config.chatgpt.is_none() {
            self.config.chatgpt = Some(crate::config::ChatGptConfig {
                api_key: String::new(),
                model: "gpt-4o-mini".to_string(),
            });
        }
        if self.config.claude.is_none() {
            self.config.claude = Some(crate::config::ClaudeConfig {
                api_key: String::new(),
                model: "claude-3-5-sonnet-latest".to_string(),
            });
        }
        if self.config.ollama.is_none() {
            self.config.ollama = Some(crate::config::OllamaConfig {
                base_url: "http://localhost:11434".to_string(),
                model: "gemma3:1b".to_string(),
            });
        }
    }

    fn apply_theme_by_name(&mut self, ctx: &egui::Context, name: &str) {
        match load_theme_by_name(name, self.config.loaded_from.as_deref()) {
            Ok(theme) => {
                self.config.theme.name = name.to_string();
                self.config.theme.path = None;
                self.theme = theme.clone();
                ctx.set_style(build_style(&theme));
                self.settings_error = None;
            }
            Err(err) => {
                self.settings_error =
                    Some(self.tr_with("app.settings_save_error", &[("error", err.to_string())]));
                self.settings_notice = None;
            }
        }
    }

    fn apply_language(&mut self, language: &str) {
        match I18n::load(language) {
            Ok(i18n) => {
                self.config.ui.language = language.to_string();
                self.i18n = i18n;
                self.settings_error = None;
            }
            Err(err) => {
                self.settings_error =
                    Some(self.tr_with("app.settings_save_error", &[("error", err.to_string())]));
                self.settings_notice = None;
            }
        }
    }

    fn persist_settings(&mut self) {
        match self.config.save() {
            Ok(()) => {
                self.settings_notice = Some(self.tr("app.settings_save_ok"));
                self.settings_error = None;
            }
            Err(err) => {
                let save_path = self.config.loaded_from.clone().unwrap_or_else(|| {
                    app_paths::default_config_path()
                        .unwrap_or_else(|_| PathBuf::from("config.yaml"))
                });
                self.settings_error = Some(self.tr_with(
                    "app.settings_save_error_with_path",
                    &[
                        ("path", save_path.display().to_string()),
                        ("error", err.to_string()),
                    ],
                ));
                self.settings_notice = None;
            }
        }
    }

    fn create_config_from_template(&mut self, ctx: &egui::Context, template_name: &str) {
        let path = match app_paths::default_config_path() {
            Ok(path) => path,
            Err(err) => {
                self.settings_error = Some(self.tr_with(
                    "startup.config_destination_error",
                    &[("error", err.to_string())],
                ));
                self.settings_notice = None;
                return;
            }
        };

        let template_config = match Config::load_template(template_name) {
            Ok(Some(mut config)) => {
                config.loaded_from = None;
                config
            }
            Ok(None) => {
                self.settings_error = Some(self.tr_with(
                    "startup.config_template_missing",
                    &[("template", template_name.to_string())],
                ));
                self.settings_notice = None;
                return;
            }
            Err(err) => {
                self.settings_error = Some(self.tr_with(
                    "startup.config_template_load_error",
                    &[
                        ("template", template_name.to_string()),
                        ("error", err.to_string()),
                    ],
                ));
                self.settings_notice = None;
                return;
            }
        };

        if let Some(parent) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                self.settings_error = Some(self.tr_with(
                    "startup.config_template_save_error",
                    &[
                        ("template", template_name.to_string()),
                        ("error", err.to_string()),
                    ],
                ));
                self.settings_notice = None;
                return;
            }
        }

        let serialized = match serde_yaml::to_string(&template_config) {
            Ok(content) => content,
            Err(err) => {
                self.settings_error = Some(self.tr_with(
                    "startup.config_template_save_error",
                    &[
                        ("template", template_name.to_string()),
                        ("error", err.to_string()),
                    ],
                ));
                self.settings_notice = None;
                return;
            }
        };

        if let Err(err) = std::fs::write(&path, serialized) {
            self.settings_error = Some(self.tr_with(
                "startup.config_template_save_error",
                &[
                    ("template", template_name.to_string()),
                    ("error", err.to_string()),
                ],
            ));
            self.settings_notice = None;
            return;
        }

        match Config::load() {
            Ok(config) => {
                self.config = config;
                self.selected_backend = self.config.default_backend.clone();
                self.prompt_profiles = PromptProfiles::load(&self.config)
                    .unwrap_or_else(|_| PromptProfiles::default_built_in());
                if let Ok(theme) =
                    load_theme_by_name(&self.config.theme.name, self.config.loaded_from.as_deref())
                {
                    self.theme = theme.clone();
                    ctx.set_style(build_style(&theme));
                }
                if let Ok(i18n) = I18n::load(&self.config.ui.language) {
                    self.i18n = i18n;
                }
                self.prompt_focus_initialized = false;
                self.settings_error = None;
                self.settings_notice = Some(self.tr_with(
                    "startup.config_template_created",
                    &[("template", template_name.to_string())],
                ));
            }
            Err(err) => {
                self.settings_error = Some(self.tr_with(
                    "startup.config_template_load_error",
                    &[
                        ("template", template_name.to_string()),
                        ("error", err.to_string()),
                    ],
                ));
                self.settings_notice = None;
            }
        }
    }

    fn attach_image_from_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
            .pick_file()
        {
            match load_image_attachment_from_path(&path) {
                Ok(image) => {
                    self.attachments.push(image);
                    self.attachment_notice = Some(self.tr_with(
                        "app.images_attached",
                        &[("count", self.attachments.len().to_string())],
                    ));
                    self.attachment_error = None;
                }
                Err(err) => {
                    self.attachment_error = Some(err);
                }
            }
        }
    }

    fn attach_image_from_clipboard(&mut self) {
        if let Err(err) = self.try_attach_image_from_clipboard(true) {
            self.attachment_error = Some(err);
        }
    }

    fn try_attach_image_from_clipboard(&mut self, report_errors: bool) -> Result<bool, String> {
        match load_image_attachment_from_clipboard() {
            Ok(image) => {
                self.attachments.push(image);
                self.attachment_notice = Some(self.tr_with(
                    "app.images_attached",
                    &[("count", self.attachments.len().to_string())],
                ));
                self.attachment_error = None;
                Ok(true)
            }
            Err(err) => {
                if report_errors {
                    Err(err)
                } else {
                    Ok(false)
                }
            }
        }
    }

    fn clear_attachments(&mut self) {
        self.attachments.clear();
        self.attachment_notice = None;
        self.attachment_error = None;
    }

    fn toggle_dictation(&mut self, ctx: &egui::Context) {
        if self.voice_recording.is_some() {
            self.stop_dictation(ctx);
        } else {
            self.start_dictation();
        }
    }

    fn start_dictation(&mut self) {
        match begin_voice_recording() {
            Ok(recording) => {
                self.voice_recording = Some(recording);
                self.dictation_status = Some(self.tr("app.voice_recording"));
            }
            Err(err) => {
                self.dictation_status = Some(err);
            }
        }
    }

    fn stop_dictation(&mut self, ctx: &egui::Context) {
        let Some(recording) = self.voice_recording.take() else {
            return;
        };

        let wav_bytes = match finish_voice_recording(recording) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.dictation_status = Some(if err.is_empty() {
                    self.tr("app.voice_error_capture")
                } else {
                    err
                });
                return;
            }
        };

        self.dictation_status = Some(self.tr("app.voice_transcribing"));
        let async_dictation = self.async_dictation.clone();
        let config = self.config.clone();
        let ctx = ctx.clone();
        self.runtime.spawn(async move {
            let result = backends::transcribe_wav_audio(wav_bytes, &config).await;
            *async_dictation.lock().unwrap() = Some(result);
            ctx.request_repaint();
        });
    }

    fn copy_response_and_close(&mut self, ctx: &egui::Context) {
        if self.response.trim().is_empty() {
            return;
        }

        copy_text_to_clipboard(&self.response);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn current_request_fingerprint(
        &self,
        active_window_context: Option<&str>,
    ) -> RequestFingerprint {
        RequestFingerprint {
            backend: self.selected_backend.clone(),
            prompt: self.prompt.clone(),
            images: self.attachments.clone(),
            mode: if self.generic_question_mode {
                PromptMode::GenericQuestion
            } else {
                PromptMode::TextAssist
            },
            session_chat_enabled: self.session_chat_enabled,
            conversation: if self.session_chat_enabled {
                self.session_conversation.clone()
            } else {
                Vec::new()
            },
            active_window_context: active_window_context.map(|value| value.to_string()),
        }
    }

    fn request_provider_models(&mut self, ctx: &egui::Context, provider: &str) {
        let state = self
            .provider_model_states
            .entry(provider.to_string())
            .or_default();
        if state.is_loading {
            return;
        }
        state.is_loading = true;
        state.last_error = None;

        let config = self.config.clone();
        let provider_name = provider.to_string();
        let async_available_models = self.async_available_models.clone();
        let ctx = ctx.clone();

        self.runtime.spawn(async move {
            let result = backends::fetch_available_models(&provider_name, &config).await;
            async_available_models
                .lock()
                .unwrap()
                .push((provider_name, result));
            ctx.request_repaint();
        });
    }

    fn invalidate_provider_models(&mut self, provider: &str) {
        if let Some(state) = self.provider_model_states.get_mut(provider) {
            state.models.clear();
            state.is_loading = false;
            state.last_error = None;
        }
    }

    fn refresh_toolbar_icon_textures_if_needed(&mut self, ctx: &egui::Context) {
        let scale = ctx.pixels_per_point();
        if (self.toolbar_icon_scale - scale).abs() > f32::EPSILON {
            self.toolbar_icon_textures = load_toolbar_icon_textures(ctx);
            self.toolbar_icon_scale = scale;
        }
    }
}

impl eframe::App for AiPopupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_toolbar_icon_textures_if_needed(ctx);
        self.check_async_response(ctx);
        self.ensure_config_sections();
        sync_main_viewport(
            ctx,
            self.show_history,
            self.show_settings,
            self.config.ui.window_height,
        );

        // Handle global Esc to close
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.show_settings {
                self.set_settings_visibility(ctx, false);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        let frame = egui::Frame::none()
            .fill(ctx.style().visuals.window_fill)
            .inner_margin(egui::Margin::same(14.0));

        show_settings_side_panel(self, ctx);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            render_main_panel(self, ctx, ui);
        });
    }
}

fn show_settings_side_panel(app: &mut AiPopupApp, ctx: &egui::Context) {
    if !app.show_settings {
        return;
    }

    egui::SidePanel::right("settings_panel")
        .resizable(false)
        .default_width(380.0)
        .frame(settings_panel_frame(
            ctx,
            app.theme.panel_fill_raised,
            app.theme.border_color,
        ))
        .show(ctx, |ui| {
            render_settings_panel(app, ctx, ui);
        });
}

fn render_main_panel(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_source("main_content_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                render_top_controls(app, ctx, ui);
                ui.add_space(12.0);
                render_prompt_section(app, ctx, ui);
                if status_section_has_content(app) {
                    ui.add_space(10.0);
                    render_status_section(app, ui);
                    ui.add_space(16.0);
                } else {
                    ui.add_space(12.0);
                }
                render_response_section(app, ctx, ui);

                if !app.session_history_entries.is_empty() {
                    ui.add_space(16.0);
                    render_session_history_section(app, ctx, ui);
                }

                if app.show_history {
                    ui.add_space(16.0);
                    render_persistent_history_section(app, ctx, ui);
                }
            });
        });
}

fn status_section_has_content(app: &AiPopupApp) -> bool {
    status_section_has_content_state(
        !app.attachments.is_empty(),
        app.dictation_status.is_some(),
        app.attachment_notice.is_some(),
        app.attachment_error.is_some(),
        app.settings_notice.is_some(),
        app.settings_error.is_some(),
    )
}

fn status_section_has_content_state(
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

fn render_startup_health_section(app: &AiPopupApp, ui: &mut egui::Ui) {
    let diagnostics = backends::startup_health_checks(&app.config, &app.selected_backend);

    card_frame(
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

fn render_first_run_setup_section(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    let config_path = app_paths::default_config_path();
    let create_enabled = config_path.is_ok();
    let template_names = app_paths::discover_config_template_names().unwrap_or_default();

    card_frame(
        ui.ctx(),
        app.theme.panel_fill_soft,
        app.theme.border_color.gamma_multiply(0.65),
    )
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(section_label(
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
            ui.label(muted_label(
                &app.tr("startup.config_templates"),
                app.theme.weak_text_color,
            ));

            let template_theme = app.theme.clone();
            dropdown_box_scope(ui, &template_theme, |ui| {
                egui::ComboBox::from_id_source("startup_config_template")
                    .selected_text(dropdown_button_text(
                        &app.first_run_template,
                        &template_theme,
                    ))
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        apply_dropdown_menu_style(ui, &template_theme);
                        for template_name in template_names.iter() {
                            if ui
                                .selectable_label(
                                    app.first_run_template == *template_name,
                                    dropdown_item_text(template_name, &template_theme),
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
                    primary_action_button(
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
                    secondary_action_button(
                        &app.tr("startup.open_config_folder"),
                        app.theme.panel_fill_soft,
                    ),
                )
                .clicked()
            {
                if let Ok(path) = &config_path {
                    if let Some(parent) = path.parent() {
                        app.settings_error = open_path_in_file_manager(parent).err().map(|err| {
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
        ui.label(muted_label(&label, app.theme.weak_text_color));
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

fn render_top_controls(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let backend_label = app.tr("app.backend");
    let generic_mode_label = app.tr("app.generic_mode");
    let session_chat_label = app.tr("app.session_chat_mode");
    let settings_open_label = app.tr("app.settings");

    ui.horizontal(|ui| {
        ui.label(muted_label(&backend_label, app.theme.weak_text_color));
        let backend_button = dropdown_button_text(&app.selected_backend, &app.theme);
        dropdown_box_scope(ui, &app.theme, |ui| {
            egui::ComboBox::from_id_source("backend_combo")
                .selected_text(backend_button)
                .width(148.0)
                .show_ui(ui, |ui| {
                    apply_dropdown_menu_style(ui, &app.theme);
                    dropdown_option(ui, &mut app.selected_backend, "ollama", &app.theme);
                    dropdown_option(ui, &mut app.selected_backend, "chatgpt", &app.theme);
                    dropdown_option(ui, &mut app.selected_backend, "claude", &app.theme);
                    dropdown_option(ui, &mut app.selected_backend, "gemini", &app.theme);
                });
        });
        ui.add_space(6.0);
        ui.checkbox(&mut app.generic_question_mode, generic_mode_label);
        ui.add_space(6.0);
        ui.checkbox(&mut app.session_chat_enabled, session_chat_label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(section_actions_right_inset());
            let gear = icon_action_button(
                app,
                ToolbarIcon::Settings,
                app.theme.panel_fill_soft,
                app.theme.text_color,
            );
            if ui.add(gear).on_hover_text(settings_open_label).clicked() {
                app.set_settings_visibility(ctx, !app.show_settings);
            }
        });
    });
}

fn render_prompt_section(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let prompt_section_label = app.tr("app.prompt");
    let prompt_id = ui.make_persistent_id("prompt_input");

    ui.horizontal(|ui| {
        ui.label(section_label(&prompt_section_label, app.theme.text_color));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(section_actions_right_inset());
            let send_clicked = ui
                .add_enabled(
                    !app.is_loading,
                    icon_action_button(
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
                    .add(icon_action_button(
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
                .add(icon_action_button(
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
                .add(icon_action_button(
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
                .add(icon_action_button(
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
    let prompt_max_height = editor_max_height(ctx, 88.0);
    app.prompt_editor_height = app.prompt_editor_height.clamp(88.0, prompt_max_height);
    let prompt_before_edit = app.prompt.clone();
    let input_output = input_frame(ctx, app.theme.panel_fill).show(ui, |ui| {
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
    let paste_action = ctx.input(|i| classify_prompt_paste_events(&i.events));

    if prompt_has_focus {
        if let Some(path) = paste_action.image_path_from_paste {
            if let Ok(attachment) = load_image_attachment_from_path(&path) {
                app.attachments.push(attachment);
                app.prompt = prompt_before_edit;
            }
        } else if should_attach_clipboard_image_from_shortcut(
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
    editor_resize_row(
        ui,
        &app.theme,
        &mut app.prompt_editor_height,
        88.0,
        prompt_max_height,
        Some(prompt_helper_text.as_str()),
    );
}

fn render_status_section(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    card_frame(
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
                            format_size(image.size_bytes)
                        ))
                        .small()
                        .color(app.theme.text_color),
                    );
                }
            });
            ui.add_space(6.0);
        }

        if let Some(status) = &app.dictation_status {
            ui.label(muted_label(status, app.theme.weak_text_color));
        }
        if let Some(notice) = &app.attachment_notice {
            ui.add_space(4.0);
            ui.label(muted_label(notice, app.theme.weak_text_color));
        }
        if let Some(notice) = &app.settings_notice {
            ui.add_space(4.0);
            ui.label(muted_label(notice, app.theme.weak_text_color));
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

fn health_level_color(app: &AiPopupApp, level: &HealthLevel) -> egui::Color32 {
    match level {
        HealthLevel::Ok => app.theme.accent_color,
        HealthLevel::Warning => egui::Color32::from_rgb(227, 177, 76),
        HealthLevel::Error => app.theme.danger_color,
    }
}

fn render_response_section(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let response_section_label = app.tr("app.response");

    ui.horizontal(|ui| {
        ui.label(section_label(&response_section_label, app.theme.text_color));
        if app.is_loading {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(app.tr("app.generating"))
                    .small()
                    .color(app.theme.weak_text_color),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(section_actions_right_inset());
            let history_count = app.history_entries.len();
            let history_label = if app.show_history {
                app.tr_with("app.hide_history", &[("count", history_count.to_string())])
            } else {
                app.tr_with("app.show_history", &[("count", history_count.to_string())])
            };

            if ui
                .add_enabled(
                    app.config.history.enabled,
                    icon_action_button(
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
                    icon_action_button(
                        app,
                        ToolbarIcon::Copy,
                        app.theme.panel_fill_soft,
                        app.theme.text_color,
                    ),
                )
                .on_hover_text(app.tr("app.copy_response"))
                .clicked()
            {
                copy_text_to_clipboard(&app.response);
            }
        });
    });
    ui.add_space(4.0);

    let response_max_height = editor_max_height(ctx, 140.0);
    app.response_editor_height = app.response_editor_height.clamp(84.0, response_max_height);
    input_frame(ctx, app.theme.panel_fill).show(ui, |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), app.response_editor_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_height(app.response_editor_height);
                ui.add_sized(
                    egui::vec2(ui.available_width(), app.response_editor_height),
                    egui::TextEdit::multiline(&mut app.response.as_str())
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace),
                );
            },
        );
    });
    editor_resize_row(
        ui,
        &app.theme,
        &mut app.response_editor_height,
        84.0,
        response_max_height,
        None,
    );
}

fn render_session_history_section(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label(section_label(
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

    card_frame(ctx, app.theme.panel_fill, app.theme.border_color).show(ui, |ui| {
        render_session_history_entries(app, ctx, ui);
    });
}

fn render_persistent_history_section(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label(section_label(
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

    card_frame(ctx, app.theme.panel_fill, app.theme.border_color).show(ui, |ui| {
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
        dropdown_box_scope(ui, &app.theme, |ui| {
            egui::ComboBox::from_id_source("history_backend_filter")
                .selected_text(selected_history_backend_label(
                    &app.history_filter_backend,
                    &all_backends,
                    &app.theme,
                ))
                .width(150.0)
                .show_ui(ui, |ui| {
                    apply_dropdown_menu_style(ui, &app.theme);
                    ui.selectable_value(
                        &mut app.history_filter_backend,
                        "all".to_string(),
                        dropdown_item_text(all_backends.as_str(), &app.theme),
                    );
                    dropdown_option(ui, &mut app.history_filter_backend, "chatgpt", &app.theme);
                    dropdown_option(ui, &mut app.history_filter_backend, "claude", &app.theme);
                    dropdown_option(ui, &mut app.history_filter_backend, "gemini", &app.theme);
                    dropdown_option(ui, &mut app.history_filter_backend, "ollama", &app.theme);
                });
        });
        ui.add(
            egui::TextEdit::singleline(&mut app.history_filter_query)
                .hint_text(history_search_hint)
                .desired_width(280.0),
        );
        if ui
            .add(secondary_action_button(
                &open_history_label,
                app.theme.panel_fill_soft,
            ))
            .clicked()
        {
            app.history_action_error = open_history_file().err().map(|err| err.to_string());
        }
        if ui
            .add(secondary_action_button(
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
                secondary_action_button(&delete_selected_label, app.theme.panel_fill_soft),
            )
            .clicked()
        {
            app.delete_selected_history_entries();
        }
        if ui
            .add(secondary_action_button(
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
        history_entry_card(
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
                history_entry_card(
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
        "chatgpt" => dropdown_button_text("chatgpt", theme),
        "claude" => dropdown_button_text("claude", theme),
        "gemini" => dropdown_button_text("gemini", theme),
        "ollama" => dropdown_button_text("ollama", theme),
        _ => dropdown_button_text(all_backends_label, theme),
    }
}

fn section_label(text: &str, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text).strong().size(15.5).color(color)
}

fn muted_label(text: &str, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text).size(13.5).color(color)
}

fn dropdown_button_text(text: &str, theme: &ResolvedTheme) -> egui::RichText {
    egui::RichText::new(text).color(theme.text_color).strong()
}

fn dropdown_item_text(text: &str, theme: &ResolvedTheme) -> egui::RichText {
    egui::RichText::new(text).color(theme.text_color)
}

fn dropdown_option(
    ui: &mut egui::Ui,
    selected: &mut String,
    value: &str,
    theme: &ResolvedTheme,
) -> egui::Response {
    ui.selectable_value(
        selected,
        value.to_string(),
        dropdown_item_text(value, theme),
    )
}

fn apply_dropdown_menu_style(ui: &mut egui::Ui, theme: &ResolvedTheme) {
    let visuals = ui.visuals_mut();
    visuals.selection.bg_fill = theme.accent_color.gamma_multiply(0.92);
    visuals.selection.stroke = egui::Stroke::new(1.0, theme.accent_color);
    visuals.widgets.inactive.bg_fill = theme.panel_fill_raised;
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, theme.border_color.gamma_multiply(0.14));
    visuals.widgets.hovered.bg_fill = lighten(theme.panel_fill_raised, 0.03);
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, theme.border_color.gamma_multiply(0.22));
    visuals.widgets.active.bg_fill = lighten(theme.panel_fill_raised, 0.05);
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(1.0, theme.border_color.gamma_multiply(0.28));
    visuals.widgets.open = visuals.widgets.hovered;
}

fn apply_dropdown_button_style(ui: &mut egui::Ui, theme: &ResolvedTheme) {
    let visuals = ui.visuals_mut();
    visuals.widgets.inactive.bg_fill = theme.panel_fill_soft;
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, theme.border_color.gamma_multiply(0.26));
    visuals.widgets.hovered.bg_fill = lighten(theme.panel_fill_soft, 0.05);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme.accent_color);
    visuals.widgets.active.bg_fill = lighten(theme.panel_fill_soft, 0.08);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, theme.accent_color);
    visuals.widgets.open.bg_fill = lighten(theme.panel_fill_soft, 0.05);
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, theme.accent_color);
    visuals.widgets.inactive.fg_stroke.color = theme.text_color;
    visuals.widgets.hovered.fg_stroke.color = theme.text_color;
    visuals.widgets.active.fg_stroke.color = theme.text_color;
    visuals.widgets.open.fg_stroke.color = theme.text_color;
}

fn dropdown_box_scope<R>(
    ui: &mut egui::Ui,
    theme: &ResolvedTheme,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.scope(|ui| {
        apply_dropdown_button_style(ui, theme);
        add_contents(ui)
    })
    .inner
}

fn primary_action_button<'a>(
    label: &'a str,
    fill: egui::Color32,
    text_color: egui::Color32,
) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).strong().color(text_color))
        .fill(fill)
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(8.0))
        .min_size(egui::vec2(100.0, 28.0))
}

fn secondary_action_button<'a>(label: &'a str, fill: egui::Color32) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).strong())
        .fill(fill)
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(8.0))
        .min_size(egui::vec2(94.0, 28.0))
}

fn icon_action_button(
    app: &AiPopupApp,
    icon: ToolbarIcon,
    fill: egui::Color32,
    stroke_color: egui::Color32,
) -> impl egui::Widget {
    icon_action_button_sized(app, icon, fill, stroke_color, egui::vec2(31.0, 31.0))
}

fn icon_action_button_sized(
    app: &AiPopupApp,
    icon: ToolbarIcon,
    fill: egui::Color32,
    stroke_color: egui::Color32,
    size: egui::Vec2,
) -> impl egui::Widget {
    IconActionButton {
        icon,
        texture: app.toolbar_icon_textures.get(&icon).cloned(),
        fill,
        stroke_color,
        size,
    }
}

struct IconActionButton {
    icon: ToolbarIcon,
    texture: Option<egui::TextureHandle>,
    fill: egui::Color32,
    stroke_color: egui::Color32,
    size: egui::Vec2,
}

impl egui::Widget for IconActionButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let sense = egui::Sense::click();
        let (rect, response) = ui.allocate_exact_size(self.size, sense);
        let visuals = ui.style().interact(&response);

        let fill = if !ui.is_enabled() {
            self.fill.gamma_multiply(0.45)
        } else if response.is_pointer_button_down_on() {
            lighten(self.fill, 0.06)
        } else if response.hovered() {
            lighten(self.fill, 0.04)
        } else {
            self.fill
        };
        let stroke = egui::Stroke::new(
            1.8,
            if !ui.is_enabled() {
                self.stroke_color.gamma_multiply(0.35)
            } else {
                self.stroke_color
            },
        );
        let prefer_vector = matches!(self.icon, ToolbarIcon::History | ToolbarIcon::HistoryOpen);

        ui.painter()
            .rect(rect, egui::Rounding::same(10.0), fill, egui::Stroke::NONE);
        if let Some(texture) = self.texture.filter(|_| !prefer_vector) {
            let (padding, offset) = icon_texture_layout(self.icon, self.size);
            let image_rect = rect.shrink(padding).translate(offset);
            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                if ui.is_enabled() {
                    self.stroke_color
                } else {
                    self.stroke_color.gamma_multiply(0.35)
                },
            );
        } else {
            paint_toolbar_icon(
                ui.painter(),
                rect.shrink(8.0),
                self.icon,
                stroke,
                visuals.fg_stroke.color,
            );
        }
        response
    }
}

fn icon_texture_layout(icon: ToolbarIcon, size: egui::Vec2) -> (f32, egui::Vec2) {
    let base_padding = (size.x.min(size.y) * 0.09).clamp(2.2, 3.2);
    match icon {
        ToolbarIcon::PasteImage => (base_padding, egui::vec2(-0.7, 0.0)),
        ToolbarIcon::History => (base_padding, egui::vec2(0.6, 0.0)),
        ToolbarIcon::HistoryOpen => (base_padding, egui::vec2(0.4, 0.0)),
        _ => (base_padding, egui::Vec2::ZERO),
    }
}

fn paint_toolbar_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: ToolbarIcon,
    stroke: egui::Stroke,
    fill_color: egui::Color32,
) {
    match icon {
        ToolbarIcon::Settings => paint_settings_icon(painter, rect, stroke),
        ToolbarIcon::Update => paint_update_icon(painter, rect, stroke),
        ToolbarIcon::Send => paint_send_icon(painter, rect, stroke, fill_color),
        ToolbarIcon::Clear | ToolbarIcon::Close => paint_close_icon(painter, rect, stroke),
        ToolbarIcon::Mic => paint_mic_icon(painter, rect, stroke),
        ToolbarIcon::Stop => paint_stop_icon(painter, rect, fill_color),
        ToolbarIcon::PasteImage => paint_image_icon(painter, rect, stroke, true),
        ToolbarIcon::AttachImage => paint_attach_icon(painter, rect, stroke),
        ToolbarIcon::History => paint_history_icon(painter, rect, stroke, false),
        ToolbarIcon::HistoryOpen => paint_history_icon(painter, rect, stroke, true),
        ToolbarIcon::Copy => paint_copy_icon(painter, rect, stroke),
    }
}

fn paint_update_icon(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let center_x = rect.center().x;
    let arrow_tip_y = rect.top() + 1.5;
    let arrow_base_y = rect.top() + rect.height() * 0.42;
    let shaft_bottom = rect.bottom() - 1.5;
    let half_head = rect.width() * 0.22;

    painter.line_segment(
        [
            egui::pos2(center_x, shaft_bottom),
            egui::pos2(center_x, arrow_base_y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center_x, arrow_tip_y),
            egui::pos2(center_x - half_head, arrow_base_y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center_x, arrow_tip_y),
            egui::pos2(center_x + half_head, arrow_base_y),
        ],
        stroke,
    );
}

fn open_url(ctx: &egui::Context, url: String) {
    ctx.output_mut(|output| {
        output.open_url = Some(egui::output::OpenUrl { url, new_tab: true });
    });
}

fn paint_close_icon(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
}

fn paint_stop_icon(painter: &egui::Painter, rect: egui::Rect, fill: egui::Color32) {
    painter.rect_filled(rect.shrink(1.5), egui::Rounding::same(2.0), fill);
}

fn paint_send_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    fill: egui::Color32,
) {
    let left = egui::pos2(rect.left(), rect.top() + rect.height() * 0.2);
    let tip = egui::pos2(rect.right(), rect.center().y);
    let bottom = egui::pos2(rect.left(), rect.bottom() - rect.height() * 0.2);
    painter.add(egui::Shape::convex_polygon(
        vec![left, tip, bottom],
        fill,
        stroke,
    ));
}

fn paint_mic_icon(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let capsule = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.center().y - 2.0),
        egui::vec2(rect.width() * 0.42, rect.height() * 0.62),
    );
    painter.rect_stroke(capsule, egui::Rounding::same(5.0), stroke);
    painter.line_segment(
        [
            egui::pos2(rect.center().x, capsule.bottom()),
            egui::pos2(rect.center().x, rect.bottom() - 3.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(rect.center().x - 4.5, rect.bottom() - 3.0),
            egui::pos2(rect.center().x + 4.5, rect.bottom() - 3.0),
        ],
        stroke,
    );
}

fn paint_attach_icon(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let x = rect.center().x;
    let top = rect.top() + 1.5;
    let bottom = rect.bottom() - 1.5;
    painter.line_segment(
        [
            egui::pos2(x + 3.0, top + 2.0),
            egui::pos2(x - 1.5, top + 6.5),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x - 1.5, top + 6.5),
            egui::pos2(x - 1.5, bottom - 3.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x - 1.5, bottom - 3.0),
            egui::pos2(x + 4.0, bottom - 8.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x + 4.0, bottom - 8.0),
            egui::pos2(x + 4.0, top + 7.5),
        ],
        stroke,
    );
}

fn paint_image_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    with_plus: bool,
) {
    painter.rect_stroke(rect.shrink(1.0), egui::Rounding::same(3.0), stroke);
    painter.circle_stroke(egui::pos2(rect.left() + 5.0, rect.top() + 5.0), 1.8, stroke);
    painter.line_segment(
        [
            egui::pos2(rect.left() + 3.0, rect.bottom() - 4.0),
            egui::pos2(rect.center().x - 1.0, rect.center().y + 1.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(rect.center().x - 1.0, rect.center().y + 1.0),
            egui::pos2(rect.right() - 3.0, rect.bottom() - 6.0),
        ],
        stroke,
    );
    if with_plus {
        painter.line_segment(
            [
                egui::pos2(rect.right() - 5.0, rect.top() + 2.5),
                egui::pos2(rect.right() - 5.0, rect.top() + 8.5),
            ],
            stroke,
        );
        painter.line_segment(
            [
                egui::pos2(rect.right() - 8.0, rect.top() + 5.5),
                egui::pos2(rect.right() - 2.0, rect.top() + 5.5),
            ],
            stroke,
        );
    }
}

fn paint_history_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    stroke: egui::Stroke,
    active: bool,
) {
    for offset in [0.0_f32, 4.5, 9.0] {
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.top() + 2.5 + offset),
                egui::pos2(rect.right(), rect.top() + 2.5 + offset),
            ],
            stroke,
        );
    }
    if active {
        painter.circle_filled(
            egui::pos2(rect.right() - 1.5, rect.top() + 2.5),
            2.0,
            stroke.color,
        );
    }
}

fn paint_copy_icon(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let softened = egui::Stroke::new(stroke.width, stroke.color.gamma_multiply(0.88));
    let size = egui::vec2(rect.width() * 0.42, rect.height() * 0.46);
    let back = egui::Rect::from_center_size(rect.center() + egui::vec2(-1.8, -1.6), size);
    let front = egui::Rect::from_center_size(rect.center() + egui::vec2(1.2, 1.0), size);
    painter.rect_stroke(back, egui::Rounding::same(1.5), softened);
    painter.rect_stroke(front, egui::Rounding::same(1.5), softened);
    painter.line_segment(
        [
            egui::pos2(front.left(), front.top()),
            egui::pos2(front.right(), front.top()),
        ],
        egui::Stroke::new(softened.width * 0.8, softened.color.gamma_multiply(0.55)),
    );
}

fn paint_settings_icon(painter: &egui::Painter, rect: egui::Rect, stroke: egui::Stroke) {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.23;
    for angle in [0.0_f32, 45.0, 90.0, 135.0] {
        let radians = angle.to_radians();
        let dir = egui::vec2(radians.cos(), radians.sin());
        painter.line_segment(
            [center + dir * (radius + 1.0), center + dir * (radius + 4.5)],
            stroke,
        );
        painter.line_segment(
            [center - dir * (radius + 1.0), center - dir * (radius + 4.5)],
            stroke,
        );
    }
    painter.circle_stroke(center, radius + 1.0, stroke);
    painter.circle_stroke(center, radius * 0.45, stroke);
}

fn editor_resize_row(
    ui: &mut egui::Ui,
    theme: &ResolvedTheme,
    height: &mut f32,
    min_height: f32,
    max_height: f32,
    helper_text: Option<&str>,
) {
    if let Some(helper_text) = helper_text {
        ui.label(muted_label(helper_text, theme.weak_text_color));
        ui.add_space(1.0);
        editor_resize_handle(ui, theme, height, min_height, max_height);
    } else {
        editor_resize_handle(ui, theme, height, min_height, max_height);
    }
}

fn editor_resize_handle(
    ui: &mut egui::Ui,
    theme: &ResolvedTheme,
    height: &mut f32,
    min_height: f32,
    max_height: f32,
) {
    ui.add_space(1.0);

    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 10.0),
        egui::Sense::click_and_drag(),
    );
    let is_active = response.hovered() || response.dragged();
    if is_active {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    if response.dragged() {
        *height = (*height + ui.input(|i| i.pointer.delta().y)).clamp(min_height, max_height);
        ui.ctx().request_repaint();
    }

    let stroke_color = if response.dragged() {
        theme.accent_color
    } else if response.hovered() {
        theme.border_color.gamma_multiply(0.85)
    } else {
        theme.border_color.gamma_multiply(0.55)
    };
    let handle_width = rect.width().min(42.0);
    let handle_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(handle_width, 3.0));
    ui.painter().rect(
        handle_rect,
        egui::Rounding::same(999.0),
        stroke_color.gamma_multiply(if is_active { 0.24 } else { 0.18 }),
        egui::Stroke::new(1.0, stroke_color),
    );
}

fn card_frame(ctx: &egui::Context, fill: egui::Color32, stroke: egui::Color32) -> egui::Frame {
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke.gamma_multiply(0.08)))
        .rounding(egui::Rounding::same(14.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 6.0),
            blur: 18.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(if ctx.style().visuals.dark_mode {
                24
            } else {
                10
            }),
        })
        .inner_margin(egui::Margin::same(11.0))
}

fn settings_panel_frame(
    ctx: &egui::Context,
    fill: egui::Color32,
    stroke: egui::Color32,
) -> egui::Frame {
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke.gamma_multiply(0.08)))
        .rounding(egui::Rounding::same(14.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 6.0),
            blur: 18.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(if ctx.style().visuals.dark_mode {
                24
            } else {
                10
            }),
        })
        .inner_margin(egui::Margin {
            left: 16.0,
            right: 12.0,
            top: 11.0,
            bottom: 11.0,
        })
}

fn input_frame(ctx: &egui::Context, fill: egui::Color32) -> egui::Frame {
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, fill))
        .rounding(egui::Rounding::same(12.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(if ctx.style().visuals.dark_mode {
                8
            } else {
                4
            }),
        })
        .inner_margin(egui::Margin::same(9.0))
}

fn render_settings_panel(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(section_label(&app.tr("app.settings"), app.theme.text_color));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(icon_action_button(
                        app,
                        ToolbarIcon::Close,
                        app.theme.panel_fill_soft,
                        app.theme.text_color,
                    ))
                    .on_hover_text(app.tr("settings.close"))
                    .clicked()
                {
                    app.set_settings_visibility(ctx, false);
                }
            });
        });
        if let Some(path) = &app.config.loaded_from {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    app.tr_with("app.config_path", &[("path", path.display().to_string())]),
                )
                .small()
                .color(app.theme.weak_text_color),
            );
        }
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                render_startup_settings_section(app, ui);
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                render_general_settings_section(app, ctx, ui);
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                render_history_debug_settings_section(app, ui);
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(12.0);
                render_provider_settings_sections(app, ctx, ui);

                ui.add_space(8.0);
                ui.label(muted_label(
                    &app.tr("settings.saved"),
                    app.theme.weak_text_color,
                ));
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} v{}",
                            app.tr("settings.version"),
                            display_version()
                        ))
                        .small()
                        .color(app.theme.weak_text_color),
                    );
                    ui.add_space(8.0);
                    render_update_status(app, ctx, ui);
                });
            });
    });
}

fn render_startup_settings_section(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(
        egui::RichText::new(app.tr("settings.health"))
            .color(app.theme.text_color)
            .strong(),
    )
    .id_source("settings_startup_health")
    .default_open(false)
    .show(ui, |ui| {
        render_startup_health_section(app, ui);

        if app.config.loaded_from.is_none() {
            ui.add_space(12.0);
            render_first_run_setup_section(app, ui);
        }
    });
}

fn render_update_status(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
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
            let guide = update::current_platform_update_guide();
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
                        .add(icon_action_button_sized(
                            app,
                            ToolbarIcon::Update,
                            app.theme.accent_color,
                            app.theme.accent_text_color,
                            egui::vec2(20.0, 20.0),
                        ))
                        .on_hover_text(app.tr("settings.update_open_download"))
                        .clicked()
                    {
                        open_url(ctx, release.release_url.clone());
                    }

                    match &guide.action {
                        UpdateAction::CopyCommand { command } => {
                            if ui
                                .add(secondary_action_button(
                                    &app.tr("settings.update_copy_command"),
                                    app.theme.panel_fill_soft,
                                ))
                                .clicked()
                            {
                                copy_text_to_clipboard(command);
                            }
                        }
                        UpdateAction::OpenReleasePage => {
                            if ui
                                .add(secondary_action_button(
                                    &app.tr("settings.update_open_release_page"),
                                    app.theme.panel_fill_soft,
                                ))
                                .clicked()
                            {
                                open_url(ctx, release.release_url.clone());
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

fn render_general_settings_section(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.label(section_label(
        &app.tr("settings.general"),
        app.theme.text_color,
    ));

    ui.label(muted_label(
        &app.tr("settings.language"),
        app.theme.weak_text_color,
    ));
    let current_language = app
        .available_locales
        .iter()
        .find(|locale| locale.code == app.i18n.code())
        .map(|locale| locale.name.clone())
        .unwrap_or_else(|| app.i18n.language_name().to_string());
    let dropdown_theme = app.theme.clone();
    dropdown_box_scope(ui, &dropdown_theme, |ui| {
        egui::ComboBox::from_id_source("settings_language")
            .selected_text(dropdown_button_text(&current_language, &dropdown_theme))
            .width(220.0)
            .show_ui(ui, |ui| {
                apply_dropdown_menu_style(ui, &dropdown_theme);
                let locales: Vec<(String, String)> = app
                    .available_locales
                    .iter()
                    .map(|locale| (locale.code.clone(), locale.name.clone()))
                    .collect();
                for (code, name) in locales {
                    if ui
                        .selectable_label(
                            app.config.ui.language == code,
                            dropdown_item_text(&name, &dropdown_theme),
                        )
                        .clicked()
                    {
                        app.apply_language(&code);
                        app.persist_settings();
                    }
                }
            });
    });

    ui.add_space(8.0);
    ui.label(muted_label(
        &app.tr("settings.theme"),
        app.theme.weak_text_color,
    ));
    let current_theme = app.config.theme.name.clone();
    let dropdown_theme = app.theme.clone();
    dropdown_box_scope(ui, &dropdown_theme, |ui| {
        egui::ComboBox::from_id_source("settings_theme")
            .selected_text(dropdown_button_text(&current_theme, &dropdown_theme))
            .width(220.0)
            .show_ui(ui, |ui| {
                apply_dropdown_menu_style(ui, &dropdown_theme);
                let themes = app.available_themes.clone();
                for theme_name in themes {
                    if ui
                        .selectable_label(
                            app.config.theme.name == theme_name,
                            dropdown_item_text(&theme_name, &dropdown_theme),
                        )
                        .clicked()
                    {
                        app.apply_theme_by_name(ctx, &theme_name);
                        app.persist_settings();
                    }
                }
            });
    });

    ui.add_space(8.0);
    ui.label(muted_label(
        &app.tr("settings.default_backend"),
        app.theme.weak_text_color,
    ));
    let dropdown_theme = app.theme.clone();
    dropdown_box_scope(ui, &dropdown_theme, |ui| {
        egui::ComboBox::from_id_source("settings_default_backend")
            .selected_text(dropdown_button_text(
                &app.config.default_backend,
                &dropdown_theme,
            ))
            .width(220.0)
            .show_ui(ui, |ui| {
                apply_dropdown_menu_style(ui, &dropdown_theme);
                for backend in ["ollama", "chatgpt", "claude", "gemini"] {
                    if ui
                        .selectable_label(
                            app.config.default_backend == backend,
                            dropdown_item_text(backend, &dropdown_theme),
                        )
                        .clicked()
                    {
                        app.config.default_backend = backend.to_string();
                        app.selected_backend = backend.to_string();
                        app.persist_settings();
                    }
                }
            });
    });

    ui.add_space(8.0);
    let mut auto_read = app.config.auto_read_selection;
    if ui
        .checkbox(&mut auto_read, app.tr("settings.auto_read_selection"))
        .changed()
    {
        app.config.auto_read_selection = auto_read;
        app.persist_settings();
    }
}

fn render_history_debug_settings_section(app: &mut AiPopupApp, ui: &mut egui::Ui) {
    ui.label(section_label(
        &app.tr("settings.history_debug"),
        app.theme.text_color,
    ));

    let mut history_enabled = app.config.history.enabled;
    if ui
        .checkbox(&mut history_enabled, app.tr("settings.history_enabled"))
        .changed()
    {
        app.config.history.enabled = history_enabled;
        if !history_enabled {
            app.show_history = false;
        }
        app.reload_history();
        app.persist_settings();
    }
    ui.label(muted_label(
        &app.tr("settings.history_warning"),
        app.theme.weak_text_color,
    ));

    ui.add_space(8.0);
    let mut debug_logging = app.config.logging.enabled;
    if ui
        .checkbox(&mut debug_logging, app.tr("settings.debug_logging"))
        .changed()
    {
        app.config.logging.enabled = debug_logging;
        app.persist_settings();
    }
    ui.label(muted_label(
        &app.tr("settings.debug_logging_warning"),
        app.theme.weak_text_color,
    ));
}

fn render_provider_settings_sections(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.label(section_label(
        &app.tr("settings.models"),
        app.theme.text_color,
    ));

    let health_checks = backends::health_checks(&app.config);
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
            ui.label(muted_label(
                &app.tr("settings.model_credits"),
                theme.weak_text_color,
            ));
            ui.label(
                egui::RichText::new(provider_credit_label(app, provider))
                    .color(color)
                    .strong(),
            );
            ui.label(muted_label(
                &provider_credit_note(app, provider),
                theme.weak_text_color,
            ));

            let primary_changed =
                settings_text_field(ui, theme, primary_label, primary_value, true);
            if primary_changed {
                app.invalidate_provider_models(provider);
            }
            changed |= primary_changed;

            if provider == "ollama" {
                ui.label(muted_label(
                    &app.tr("settings.ollama_url_hint"),
                    theme.weak_text_color,
                ));
            }

            let model_state = app
                .provider_model_states
                .entry(provider.to_string())
                .or_default();
            let (model_changed, model_interacted) = settings_model_field(
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
            );
            changed |= model_changed;
            should_fetch_models = model_interacted && model_state.models.is_empty();
        });
    if should_fetch_models {
        app.request_provider_models(ctx, provider);
    }
    changed
}

fn settings_text_field(
    ui: &mut egui::Ui,
    theme: &ResolvedTheme,
    label: &str,
    value: &mut String,
    password: bool,
) -> bool {
    ui.add_space(8.0);
    ui.label(muted_label(label, theme.weak_text_color));
    let mut edit = egui::TextEdit::singleline(value).desired_width(f32::INFINITY);
    if password {
        edit = edit.password(true);
    }
    ui.add(edit).changed()
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
) -> (bool, bool) {
    let mut changed = false;
    let mut should_fetch = false;

    ui.add_space(8.0);
    ui.label(muted_label(label, theme.weak_text_color));
    let response = ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
    changed |= response.changed();
    should_fetch |= response.clicked() || response.gained_focus() || response.has_focus();

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(muted_label(available_models_label, theme.weak_text_color));
        let button_label = if state.is_loading {
            loading_label
        } else {
            refresh_label
        };
        if ui
            .add_enabled(
                !state.is_loading,
                secondary_action_button(button_label, theme.panel_fill_soft),
            )
            .clicked()
        {
            state.models.clear();
            state.last_error = None;
            should_fetch = true;
        }
    });

    if state.is_loading {
        ui.label(muted_label(loading_label, theme.weak_text_color));
    } else if !state.models.is_empty() {
        let selected_text = if value.trim().is_empty() {
            select_model_label.to_string()
        } else {
            value.clone()
        };
        dropdown_box_scope(ui, theme, |ui| {
            egui::ComboBox::from_id_source(format!("{provider}_available_models"))
                .selected_text(dropdown_button_text(&selected_text, theme))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    apply_dropdown_menu_style(ui, theme);
                    for model in &state.models {
                        if ui.selectable_value(value, model.clone(), model).changed() {
                            changed = true;
                        }
                    }
                });
        });
    } else {
        ui.label(muted_label(models_hint_label, theme.weak_text_color));
    }

    if let Some(error) = &state.last_error {
        ui.label(egui::RichText::new(error).small().color(theme.danger_color));
        ui.label(muted_label(models_hint_label, theme.weak_text_color));
    }

    (changed, should_fetch)
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

fn load_image_attachment_from_path(path: &Path) -> Result<ImageAttachment, String> {
    let bytes = std::fs::read(path)
        .map_err(|err| format!("Could not read image file `{}`: {err}", path.display()))?;
    let mime_type = infer_image_mime(path)
        .ok_or_else(|| "Unsupported image format. Use PNG, JPG, JPEG, WEBP, or GIF.".to_string())?;

    Ok(image_attachment_from_bytes(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("image")
            .to_string(),
        mime_type.to_string(),
        bytes,
    ))
}

fn load_image_attachment_from_clipboard() -> Result<ImageAttachment, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|err| format!("Clipboard not available: {err}"))?;
    if let Ok(image) = clipboard.get_image() {
        let mut png_bytes = Vec::new();
        PngEncoder::new(&mut png_bytes)
            .write_image(
                image.bytes.as_ref(),
                image.width as u32,
                image.height as u32,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|err| format!("Could not encode clipboard image: {err}"))?;

        return Ok(image_attachment_from_bytes(
            "clipboard-screenshot.png".to_string(),
            "image/png".to_string(),
            png_bytes,
        ));
    }

    if let Ok(text) = clipboard.get_text() {
        if let Some(path) = extract_image_path_from_clipboard_text(&text) {
            return load_image_attachment_from_path(&path);
        }
    }

    if let Some(attachment) = load_image_attachment_from_system_clipboard_commands() {
        return Ok(attachment);
    }

    Err("Clipboard does not currently contain an image.".to_string())
}

fn image_attachment_from_bytes(name: String, mime_type: String, bytes: Vec<u8>) -> ImageAttachment {
    ImageAttachment {
        name,
        mime_type,
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes.as_slice()),
        size_bytes: bytes.len(),
    }
}

fn extract_image_path_from_clipboard_text(text: &str) -> Option<PathBuf> {
    for line in text.lines() {
        let candidate = line.trim().trim_matches('\0');
        if candidate.is_empty() {
            continue;
        }

        let normalized = candidate
            .strip_prefix("file://")
            .unwrap_or(candidate)
            .replace("%20", " ");
        let path = PathBuf::from(normalized);
        if path.exists() && infer_image_mime(&path).is_some() {
            return Some(path);
        }
    }
    None
}

#[derive(Debug, Default)]
struct PromptPasteAction {
    saw_text_paste_event: bool,
    image_path_from_paste: Option<PathBuf>,
}

fn classify_prompt_paste_events(events: &[egui::Event]) -> PromptPasteAction {
    let mut action = PromptPasteAction::default();

    for event in events {
        let egui::Event::Paste(text) = event else {
            continue;
        };

        action.saw_text_paste_event = true;
        if action.image_path_from_paste.is_none() {
            action.image_path_from_paste = extract_image_path_from_clipboard_text(text);
        }
    }

    action
}

fn should_attach_clipboard_image_from_shortcut(
    paste_shortcut_pressed: bool,
    prompt_before_edit: &str,
    prompt_after_edit: &str,
) -> bool {
    paste_shortcut_pressed && prompt_before_edit == prompt_after_edit
}

fn load_image_attachment_from_system_clipboard_commands() -> Option<ImageAttachment> {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for (mime, extension) in [
            ("image/png", "png"),
            ("image/jpeg", "jpg"),
            ("image/webp", "webp"),
            ("image/gif", "gif"),
        ] {
            if let Some(bytes) = try_read_wayland_clipboard_image(mime)
                .or_else(|| try_read_x11_clipboard_image(mime))
            {
                return Some(image_attachment_from_bytes(
                    format!("clipboard-image.{extension}"),
                    mime.to_string(),
                    bytes,
                ));
            }
        }
    }

    None
}

#[cfg(all(unix, not(target_os = "macos")))]
fn try_read_wayland_clipboard_image(mime_type: &str) -> Option<Vec<u8>> {
    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", mime_type])
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }
    Some(output.stdout)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn try_read_x11_clipboard_image(mime_type: &str) -> Option<Vec<u8>> {
    let output = Command::new("xclip")
        .args(["-selection", "clipboard", "-t", mime_type, "-o"])
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }
    Some(output.stdout)
}

fn infer_image_mime(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .as_deref()
    {
        Some("png") => Some("image/png"),
        Some("jpg") | Some("jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        Some("gif") => Some("image/gif"),
        _ => None,
    }
}

fn format_size(bytes: usize) -> String {
    const KB: f32 = 1024.0;
    const MB: f32 = 1024.0 * 1024.0;
    if bytes as f32 >= MB {
        format!("{:.1} MB", bytes as f32 / MB)
    } else if bytes as f32 >= KB {
        format!("{:.0} KB", bytes as f32 / KB)
    } else {
        format!("{bytes} B")
    }
}

fn copy_text_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text.to_string());
    }
}

fn begin_voice_recording() -> Result<VoiceRecording, String> {
    let path = std::env::temp_dir().join(format!(
        "armando-dictation-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0)
    ));

    let mut command = if command_exists("ffmpeg") {
        let mut command = Command::new("ffmpeg");
        command.args([
            "-y",
            "-f",
            "pulse",
            "-i",
            "default",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-acodec",
            "pcm_s16le",
        ]);
        command.arg(&path);
        command
    } else if command_exists("arecord") {
        let mut command = Command::new("arecord");
        command.args(["-q", "-f", "S16_LE", "-r", "16000", "-c", "1", "-t", "wav"]);
        command.arg(&path);
        command
    } else {
        return Err(
            "Voice dictation requires `ffmpeg` or `arecord` to be installed on the system."
                .to_string(),
        );
    };

    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("Could not start microphone recording: {err}"))?;

    Ok(VoiceRecording { child, path })
}

fn finish_voice_recording(mut recording: VoiceRecording) -> Result<Vec<u8>, String> {
    let pid = recording.child.id().to_string();
    let _ = Command::new("kill")
        .args(["-INT", &pid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = recording.child.wait();

    let bytes = std::fs::read(&recording.path)
        .map_err(|err| format!("Could not read recorded dictation audio: {err}"))?;
    let _ = std::fs::remove_file(&recording.path);
    if bytes.is_empty() {
        return Err(String::new());
    }
    Ok(bytes)
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn history_entry_card(
    copy_label: &str,
    reuse_label: &str,
    select_label: &str,
    prompt_label: &str,
    response_label: &str,
    selectable: bool,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    theme: &ResolvedTheme,
    entry: &HistoryEntry,
    selected_history_entries: &mut HashSet<String>,
    prompt: &mut String,
    response: &mut String,
    show_history: &mut bool,
    prompt_focus_initialized: &mut bool,
    history_action_error: &mut Option<String>,
) {
    let entry_id = history::entry_id(entry);
    card_frame(ctx, theme.panel_fill_raised, theme.border_color).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if selectable {
                let mut is_selected = selected_history_entries.contains(&entry_id);
                if ui
                    .checkbox(&mut is_selected, "")
                    .on_hover_text(select_label)
                    .changed()
                {
                    if is_selected {
                        selected_history_entries.insert(entry_id.clone());
                    } else {
                        selected_history_entries.remove(&entry_id);
                    }
                }
            }
            ui.label(
                egui::RichText::new(entry.backend.to_uppercase())
                    .strong()
                    .monospace()
                    .color(ctx.style().visuals.hyperlink_color),
            );
            ui.label(
                egui::RichText::new(trim_timestamp(&entry.created_at))
                    .small()
                    .color(theme.weak_text_color),
            );
        });

        ui.add_space(6.0);
        ui.label(muted_label(prompt_label, theme.weak_text_color));
        ui.label(
            egui::RichText::new(trim_for_preview(&entry.prompt, 180))
                .strong()
                .small(),
        );
        ui.add_space(6.0);
        ui.label(muted_label(response_label, theme.weak_text_color));
        ui.label(
            egui::RichText::new(trim_for_preview(&entry.response, 260))
                .small()
                .color(theme.weak_text_color),
        );
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            let copy_button = secondary_action_button(copy_label, theme.panel_fill_soft);
            if ui.add(copy_button).clicked() {
                copy_text_to_clipboard(&entry.response);
                *history_action_error = None;
            }

            let reuse_button =
                primary_action_button(reuse_label, theme.accent_color, theme.accent_text_color);
            if ui.add(reuse_button).clicked() {
                *prompt = entry.prompt.clone();
                *response = entry.response.clone();
                *show_history = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                    620.0, 420.0,
                )));
                *prompt_focus_initialized = false;
                *history_action_error = None;
                ctx.request_repaint();
            }
        });
    });
}

fn push_session_history_entry(
    session_history_entries: &mut Vec<HistoryEntry>,
    entry: HistoryEntry,
) {
    session_history_entries.insert(0, entry);
    session_history_entries.truncate(SESSION_HISTORY_LIMIT);
}

fn sync_main_viewport(
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

fn trim_for_preview(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let mut result = String::new();
    for (index, ch) in trimmed.chars().enumerate() {
        if index >= max_chars {
            result.push_str("...");
            return result;
        }
        result.push(ch);
    }
    result
}

fn default_prompt_editor_height(window_height: f32) -> f32 {
    (window_height * 0.16).clamp(88.0, 136.0)
}

fn default_response_editor_height(window_height: f32) -> f32 {
    (window_height * 0.18).clamp(96.0, 156.0)
}

fn main_viewport_min_size(
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

fn editor_max_height(ctx: &egui::Context, min_height: f32) -> f32 {
    let viewport_height = ctx
        .input(|i| i.viewport().inner_rect.map(|rect| rect.height()))
        .unwrap_or(600.0);
    editor_max_height_for_viewport(viewport_height, min_height)
}

fn editor_max_height_for_viewport(viewport_height: f32, min_height: f32) -> f32 {
    (viewport_height / 3.0).max(min_height)
}

fn requested_viewport_inner_size(
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

fn section_actions_right_inset() -> f32 {
    10.0
}

fn trim_timestamp(value: &str) -> String {
    value.replace('T', " ").replace('Z', "")
}

fn open_path_in_file_manager(path: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(anyhow::anyhow!(
        "Opening files or folders in the system file manager is not supported on this platform"
    ))
}

fn open_history_file() -> anyhow::Result<()> {
    let path = history::history_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        std::fs::File::create(&path)?;
    }
    open_path_in_file_manager(&path)
}

fn get_primary_selection() -> Result<String, String> {
    // Try wayland first
    if let Ok(output) = std::process::Command::new("wl-paste").arg("-p").output() {
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // Fallback to x11
    if let Ok(output) = std::process::Command::new("xclip")
        .args(["-o", "-selection", "primary"])
        .output()
    {
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // Fallback to pure clipboard via arboard if both fail
    if let Ok(mut board) = arboard::Clipboard::new() {
        if let Ok(text) = board.get_text() {
            return Ok(text);
        }
    }

    Err("Could not read selection".to_string())
}

fn build_style(theme: &ResolvedTheme) -> egui::Style {
    let mut style = egui::Style::default();
    let mut visuals = egui::Visuals::dark();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(24.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(15.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(14.5, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(14.0, egui::FontFamily::Monospace),
    );

    visuals.window_fill = theme.window_fill;
    visuals.panel_fill = theme.panel_fill;
    visuals.faint_bg_color = theme.panel_fill_soft;
    visuals.extreme_bg_color = darken(theme.panel_fill_raised, 0.04);
    visuals.code_bg_color = darken(theme.panel_fill_soft, 0.03);
    visuals.hyperlink_color = theme.accent_color;
    visuals.selection.bg_fill = theme.accent_hover_color;
    visuals.selection.stroke.color = theme.text_color;
    visuals.override_text_color = Some(theme.text_color);
    visuals.window_stroke.color = theme.border_color.gamma_multiply(0.06);
    visuals.window_stroke.width = 0.6;
    visuals.widgets.noninteractive.fg_stroke.color = theme.text_color;
    visuals.widgets.noninteractive.bg_fill = theme.panel_fill;
    visuals.widgets.noninteractive.bg_stroke.color = theme.panel_fill;
    visuals.widgets.inactive.bg_fill = theme.panel_fill;
    visuals.widgets.inactive.bg_stroke.color = theme.panel_fill;
    visuals.widgets.inactive.fg_stroke.color = theme.text_color;
    visuals.widgets.hovered.bg_fill = lighten(theme.panel_fill, 0.03);
    visuals.widgets.hovered.bg_stroke.color = lighten(theme.panel_fill, 0.03);
    visuals.widgets.hovered.fg_stroke.color = theme.text_color;
    visuals.widgets.active.bg_fill = lighten(theme.panel_fill, 0.04);
    visuals.widgets.active.bg_stroke.color = lighten(theme.panel_fill, 0.04);
    visuals.widgets.active.fg_stroke.color = theme.text_color;
    visuals.widgets.open = visuals.widgets.inactive;

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 9.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.5);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.indent = 10.0;
    style.spacing.scroll = egui::style::ScrollStyle {
        floating: true,
        bar_width: 8.0,
        handle_min_length: 24.0,
        bar_inner_margin: 2.0,
        bar_outer_margin: 2.0,
        floating_width: 3.0,
        floating_allocated_width: 0.0,
        foreground_color: false,
        dormant_background_opacity: 0.0,
        active_background_opacity: 0.0,
        interact_background_opacity: 0.0,
        dormant_handle_opacity: 0.10,
        active_handle_opacity: 0.20,
        interact_handle_opacity: 0.32,
    };
    style.visuals.window_rounding = egui::Rounding::same(16.0);
    style.visuals.menu_rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.noninteractive.bg_fill = darken(style.visuals.panel_fill, 0.01);
    style.visuals.widgets.noninteractive.bg_stroke.width = 0.0;
    style.visuals.widgets.inactive.bg_fill = style.visuals.panel_fill;
    style.visuals.widgets.inactive.bg_stroke.width = 0.0;
    style.visuals.widgets.hovered.bg_stroke.width = 0.0;
    style.visuals.widgets.active.bg_stroke.width = 0.0;
    style.visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 10.0),
        blur: 24.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(36),
    };
    style.visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 14.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(24),
    };
    style
}

fn darken(color: egui::Color32, amount: f32) -> egui::Color32 {
    let scale = (1.0 - amount).clamp(0.0, 1.0);
    color32(
        ((color.r() as f32) * scale).round() as u8,
        ((color.g() as f32) * scale).round() as u8,
        ((color.b() as f32) * scale).round() as u8,
    )
}

fn lighten(color: egui::Color32, amount: f32) -> egui::Color32 {
    let amount = amount.clamp(0.0, 1.0);
    color32(
        (color.r() as f32 + (255.0 - color.r() as f32) * amount).round() as u8,
        (color.g() as f32 + (255.0 - color.g() as f32) * amount).round() as u8,
        (color.b() as f32 + (255.0 - color.b() as f32) * amount).round() as u8,
    )
}

fn color32(r: u8, g: u8, b: u8) -> egui::Color32 {
    egui::Color32::from_rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_prompt_paste_events_detects_plain_text_paste() {
        let action =
            classify_prompt_paste_events(&[egui::Event::Paste("hello from clipboard".to_string())]);

        assert!(action.saw_text_paste_event);
        assert!(action.image_path_from_paste.is_none());
    }

    #[test]
    fn classify_prompt_paste_events_detects_image_path_paste() {
        let temp_dir = std::env::temp_dir().join(format!("armando-paste-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let image_path = temp_dir.join("clipboard-image.png");
        std::fs::write(&image_path, b"png").unwrap();

        let action = classify_prompt_paste_events(&[egui::Event::Paste(
            image_path.to_string_lossy().into_owned(),
        )]);

        assert!(action.saw_text_paste_event);
        assert_eq!(action.image_path_from_paste, Some(image_path.clone()));

        let _ = std::fs::remove_file(&image_path);
        let _ = std::fs::remove_dir(&temp_dir);
    }

    #[test]
    fn should_attach_clipboard_image_when_shortcut_does_not_change_prompt() {
        assert!(should_attach_clipboard_image_from_shortcut(
            true,
            "existing prompt",
            "existing prompt",
        ));
    }

    #[test]
    fn should_not_attach_clipboard_image_when_prompt_changed() {
        assert!(!should_attach_clipboard_image_from_shortcut(
            true,
            "before",
            "before pasted text",
        ));
    }

    #[test]
    fn push_session_history_entry_keeps_newest_first_and_caps_length() {
        let mut session_history_entries = Vec::new();

        for index in 0..(SESSION_HISTORY_LIMIT + 3) {
            push_session_history_entry(
                &mut session_history_entries,
                HistoryEntry {
                    created_at: format!("2026-03-20T00:00:{index:02}Z"),
                    backend: "ollama".to_string(),
                    prompt: format!("prompt-{index}"),
                    response: format!("response-{index}"),
                },
            );
        }

        assert_eq!(session_history_entries.len(), SESSION_HISTORY_LIMIT);
        assert_eq!(
            session_history_entries[0].prompt,
            format!("prompt-{}", SESSION_HISTORY_LIMIT + 2)
        );
        assert_eq!(session_history_entries.last().unwrap().prompt, "prompt-3");
    }

    #[test]
    fn editor_max_height_stays_within_a_third_of_the_viewport() {
        assert_eq!(editor_max_height_for_viewport(900.0, 96.0), 300.0);
        assert_eq!(editor_max_height_for_viewport(1200.0, 84.0), 400.0);
    }

    #[test]
    fn editor_max_height_respects_minimum_when_viewport_is_small() {
        assert_eq!(editor_max_height_for_viewport(240.0, 96.0), 96.0);
        assert_eq!(editor_max_height_for_viewport(180.0, 84.0), 84.0);
    }

    #[test]
    fn default_editor_heights_use_compact_startup_sizes() {
        assert!((default_prompt_editor_height(600.0) - 96.0).abs() < f32::EPSILON);
        assert!((default_response_editor_height(600.0) - 108.0).abs() < 0.001);
        assert!((default_prompt_editor_height(1200.0) - 136.0).abs() < f32::EPSILON);
        assert!((default_response_editor_height(1200.0) - 156.0).abs() < f32::EPSILON);
    }

    #[test]
    fn main_viewport_min_size_respects_base_history_and_settings_modes() {
        assert_eq!(
            main_viewport_min_size(false, false, 540.0),
            egui::vec2(820.0, 540.0)
        );
        assert_eq!(
            main_viewport_min_size(true, false, 540.0),
            egui::vec2(820.0, 620.0)
        );
        assert_eq!(
            main_viewport_min_size(false, true, 540.0),
            egui::vec2(1320.0, 600.0)
        );
        assert_eq!(
            main_viewport_min_size(true, true, 540.0),
            egui::vec2(1320.0, 620.0)
        );
    }

    #[test]
    fn main_viewport_min_size_clamps_extreme_heights() {
        assert_eq!(
            main_viewport_min_size(false, false, 320.0),
            egui::vec2(820.0, 500.0)
        );
        assert_eq!(
            main_viewport_min_size(false, false, 900.0),
            egui::vec2(820.0, 680.0)
        );
        assert_eq!(
            main_viewport_min_size(true, true, 900.0),
            egui::vec2(1320.0, 680.0)
        );
    }

    #[test]
    fn requested_viewport_inner_size_expands_only_smaller_axes() {
        let desired_size = egui::vec2(820.0, 620.0);

        assert_eq!(
            requested_viewport_inner_size(Some(egui::vec2(640.0, 700.0)), desired_size),
            Some(egui::vec2(820.0, 700.0))
        );
        assert_eq!(
            requested_viewport_inner_size(Some(egui::vec2(900.0, 540.0)), desired_size),
            Some(egui::vec2(900.0, 620.0))
        );
        assert_eq!(
            requested_viewport_inner_size(Some(egui::vec2(900.0, 700.0)), desired_size),
            None
        );
        assert_eq!(requested_viewport_inner_size(None, desired_size), None);
    }

    #[derive(Debug)]
    struct VisualLayoutState {
        name: &'static str,
        window_height: f32,
        show_history: bool,
        show_settings: bool,
        session_history_entries: usize,
        attachments: usize,
        dictation_status: bool,
        attachment_notice: bool,
        attachment_error: bool,
        settings_notice: bool,
        settings_error: bool,
        current_inner_size: Option<egui::Vec2>,
    }

    fn format_vec2(value: egui::Vec2) -> String {
        format!("{:.0}x{:.0}", value.x, value.y)
    }

    fn format_optional_vec2(value: Option<egui::Vec2>) -> String {
        value.map(format_vec2).unwrap_or_else(|| "none".to_string())
    }

    fn visual_layout_snapshot(state: &VisualLayoutState) -> String {
        let min_inner_size =
            main_viewport_min_size(state.show_history, state.show_settings, state.window_height);
        let prompt_default = default_prompt_editor_height(state.window_height);
        let response_default = default_response_editor_height(state.window_height);
        let prompt_max = editor_max_height_for_viewport(state.window_height, 88.0);
        let response_max = editor_max_height_for_viewport(state.window_height, 84.0);
        let requested_inner_size =
            requested_viewport_inner_size(state.current_inner_size, min_inner_size);

        format!(
            "{name}\n  min_inner={min_inner}\n  requested_inner={requested_inner}\n  prompt_default={prompt_default:.1}\n  response_default={response_default:.1}\n  prompt_max={prompt_max:.1}\n  response_max={response_max:.1}\n  startup_health=false\n  first_run_setup=false\n  status_section={status}\n  session_history={session_history}\n  saved_history={saved_history}\n  toolbar_right_inset=10.0\n",
            name = state.name,
            min_inner = format_vec2(min_inner_size),
            requested_inner = format_optional_vec2(requested_inner_size),
            status = status_section_has_content_state(
                state.attachments > 0,
                state.dictation_status,
                state.attachment_notice,
                state.attachment_error,
                state.settings_notice,
                state.settings_error,
            ),
            session_history = state.session_history_entries > 0,
            saved_history = state.show_history,
            prompt_default = prompt_default,
            response_default = response_default,
            prompt_max = prompt_max,
            response_max = response_max,
        )
    }

    #[test]
    fn visual_layout_snapshot_matrix_matches_expected_summary() {
        let actual = [
            visual_layout_snapshot(&VisualLayoutState {
                name: "startup_compact",
                window_height: 540.0,
                show_history: false,
                show_settings: false,
                session_history_entries: 0,
                attachments: 0,
                dictation_status: false,
                attachment_notice: false,
                attachment_error: false,
                settings_notice: false,
                settings_error: false,
                current_inner_size: Some(egui::vec2(640.0, 520.0)),
            }),
            visual_layout_snapshot(&VisualLayoutState {
                name: "settings_open",
                window_height: 540.0,
                show_history: false,
                show_settings: true,
                session_history_entries: 0,
                attachments: 0,
                dictation_status: false,
                attachment_notice: false,
                attachment_error: false,
                settings_notice: false,
                settings_error: false,
                current_inner_size: Some(egui::vec2(1200.0, 560.0)),
            }),
            visual_layout_snapshot(&VisualLayoutState {
                name: "history_open_with_feedback",
                window_height: 700.0,
                show_history: true,
                show_settings: false,
                session_history_entries: 3,
                attachments: 1,
                dictation_status: true,
                attachment_notice: false,
                attachment_error: false,
                settings_notice: false,
                settings_error: true,
                current_inner_size: Some(egui::vec2(780.0, 610.0)),
            }),
        ]
        .join("\n");

        let expected = "\
startup_compact
  min_inner=820x540
  requested_inner=820x540
  prompt_default=88.0
  response_default=97.2
  prompt_max=180.0
  response_max=180.0
  startup_health=false
  first_run_setup=false
  status_section=false
  session_history=false
  saved_history=false
  toolbar_right_inset=10.0

settings_open
  min_inner=1320x600
  requested_inner=1320x600
  prompt_default=88.0
  response_default=97.2
  prompt_max=180.0
  response_max=180.0
  startup_health=false
  first_run_setup=false
  status_section=false
  session_history=false
  saved_history=false
  toolbar_right_inset=10.0

history_open_with_feedback
  min_inner=820x680
  requested_inner=820x680
  prompt_default=112.0
  response_default=126.0
  prompt_max=233.3
  response_max=233.3
  startup_health=false
  first_run_setup=false
  status_section=true
  session_history=true
  saved_history=true
  toolbar_right_inset=10.0
";

        assert_eq!(actual, expected);
    }

    #[test]
    fn status_section_visibility_reacts_to_any_message_or_error_state() {
        assert!(!status_section_has_content_state(
            false, false, false, false, false, false
        ));
        assert!(status_section_has_content_state(
            true, false, false, false, false, false
        ));
        assert!(status_section_has_content_state(
            false, true, false, false, false, false
        ));
        assert!(status_section_has_content_state(
            false, false, true, false, false, false
        ));
        assert!(status_section_has_content_state(
            false, false, false, true, false, false
        ));
        assert!(status_section_has_content_state(
            false, false, false, false, true, false
        ));
        assert!(status_section_has_content_state(
            false, false, false, false, false, true
        ));
    }
}

use base64::Engine as _;
use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::backends;
use crate::backends::PromptMode;
use crate::backends::{HealthCheck, HealthLevel, ImageAttachment, QueryInput};
use crate::config::Config;
use crate::history::{self, HistoryEntry};
use crate::i18n::{available_locales, I18n, LocaleDefinition};
use crate::theme::{available_theme_names, load_theme_by_name, ResolvedTheme};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn display_version() -> String {
    let mut parts = APP_VERSION.split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    let patch = parts
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    format!("{major}.{minor}.{patch:02}")
}

struct VoiceRecording {
    child: Child,
    path: PathBuf,
}

#[derive(Clone, PartialEq, Eq)]
struct RequestFingerprint {
    backend: String,
    prompt: String,
    images: Vec<ImageAttachment>,
    mode: PromptMode,
}

pub struct AiPopupApp {
    config: Config,
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
    generic_question_mode: bool,
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
    available_themes: Vec<String>,
    available_locales: Vec<LocaleDefinition>,
    i18n: I18n,

    // For tokio to update UI when done
    async_response: Arc<Mutex<Option<Result<String, String>>>>,
    async_dictation: Arc<Mutex<Option<Result<String, String>>>>,
    last_completed_request: Option<RequestFingerprint>,
}

impl AiPopupApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config: Config,
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

        let (history_entries, history_error) = match history::recent_entries() {
            Ok(entries) => (entries, None),
            Err(err) => (Vec::new(), Some(err.to_string())),
        };

        let fallback_theme_name = config.theme.name.clone();

        Self {
            config,
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
            generic_question_mode: false,
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
            available_themes: available_theme_names().unwrap_or_else(|_| vec![fallback_theme_name]),
            available_locales: available_locales().unwrap_or_default(),
            i18n,
            async_response: Arc::new(Mutex::new(None)),
            async_dictation: Arc::new(Mutex::new(None)),
            last_completed_request: None,
        }
    }

    fn check_async_response(&mut self, _ctx: &egui::Context) {
        let res = {
            let mut resp_lock = self.async_response.lock().unwrap();
            resp_lock.take()
        };

        if let Some(res) = res {
            self.is_loading = false;
            match res {
                Ok(text) => {
                    self.response = text;
                    if let Some((backend, prompt)) = self.pending_submission.take() {
                        if let Ok(entry) = history::new_entry(&backend, &prompt, &self.response) {
                            self.session_history_entries.insert(0, entry);
                        }
                    }
                    self.last_completed_request = Some(self.current_request_fingerprint());
                    self.reload_history();
                    if self.auto_copy_close_after_response {
                        self.auto_copy_close_after_response = false;
                        self.copy_response_and_close(_ctx);
                    }
                }
                Err(e) => {
                    self.response = format!("Error: {}", e);
                    self.pending_submission = None;
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
    }

    fn submit_prompt(&mut self, ctx: &egui::Context) {
        if (self.prompt.trim().is_empty() && self.attachments.is_empty()) || self.is_loading {
            return;
        }

        let current_request = self.current_request_fingerprint();
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
        let backend = self.selected_backend.clone();
        let mode = if self.generic_question_mode {
            PromptMode::GenericQuestion
        } else {
            PromptMode::TextAssist
        };
        let config = self.config.clone();
        let async_response = self.async_response.clone();
        let ctx = ctx.clone();
        self.pending_submission = Some((backend.clone(), prompt.clone()));
        self.attachment_notice = None;
        self.attachment_error = None;

        // Spawn async task
        self.runtime.spawn(async move {
            let res =
                backends::query(&backend, &QueryInput { prompt, images }, &config, mode).await;

            // Store result
            *async_response.lock().unwrap() = Some(Ok(res));

            // Request UI repaint since we updated state from background thread
            ctx.request_repaint();
        });
    }

    fn reload_history(&mut self) {
        match history::recent_entries() {
            Ok(entries) => {
                self.history_entries = entries;
                self.session_history_entries.retain(|entry| {
                    let entry_id = history::entry_id(entry);
                    self.history_entries
                        .iter()
                        .any(|persisted| history::entry_id(persisted) == entry_id)
                });
                self.selected_history_entries.retain(|id| {
                    self.history_entries
                        .iter()
                        .any(|entry| history::entry_id(entry) == *id)
                        || self
                            .session_history_entries
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
        if self.selected_history_entries.is_empty() {
            return;
        }

        let ids: Vec<String> = self.selected_history_entries.iter().cloned().collect();
        match history::delete_entries(&ids) {
            Ok(()) => {
                self.session_history_entries
                    .retain(|entry| !ids.iter().any(|id| id == &history::entry_id(entry)));
                self.selected_history_entries.clear();
                self.reload_history();
            }
            Err(err) => {
                self.history_action_error = Some(err.to_string());
            }
        }
    }

    fn select_all_visible_history_entries(&mut self) {
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
        let mut ids: Vec<String> = self
            .session_history_entries
            .iter()
            .map(history::entry_id)
            .collect();
        ids.extend(
            self.filtered_history_entries()
                .iter()
                .map(history::entry_id),
        );
        ids.sort();
        ids.dedup();

        if ids.is_empty() {
            return;
        }

        match history::delete_entries(&ids) {
            Ok(()) => {
                self.session_history_entries
                    .retain(|entry| !ids.iter().any(|id| id == &history::entry_id(entry)));
                self.selected_history_entries.clear();
                self.reload_history();
            }
            Err(err) => {
                self.history_action_error = Some(err.to_string());
            }
        }
    }

    fn filtered_history_entries(&self) -> Vec<HistoryEntry> {
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
        self.show_history = visible;
        if self.show_history {
            self.reload_history();
        }
        sync_history_viewport(ctx, self.show_history);
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
                self.settings_error =
                    Some(self.tr_with("app.settings_save_error", &[("error", err.to_string())]));
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
        match load_image_attachment_from_clipboard() {
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

        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(self.response.clone());
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn current_request_fingerprint(&self) -> RequestFingerprint {
        RequestFingerprint {
            backend: self.selected_backend.clone(),
            prompt: self.prompt.clone(),
            images: self.attachments.clone(),
            mode: if self.generic_question_mode {
                PromptMode::GenericQuestion
            } else {
                PromptMode::TextAssist
            },
        }
    }
}

impl eframe::App for AiPopupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_async_response(ctx);
        self.ensure_config_sections();

        // Handle global Esc to close
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.show_settings {
                self.show_settings = false;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        let frame = egui::Frame::none()
            .fill(ctx.style().visuals.window_fill)
            .inner_margin(egui::Margin::same(14.0));

        if self.show_settings {
            egui::SidePanel::right("settings_panel")
                .resizable(false)
                .default_width(320.0)
                .frame(card_frame(
                    ctx,
                    self.theme.panel_fill_raised,
                    self.theme.border_color,
                ))
                .show(ctx, |ui| {
                    render_settings_panel(self, ctx, ui);
                });
        }

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_source("main_content_scroll")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let all_backends = self.tr("app.all_backends");
                        let backend_label = self.tr("app.backend");
                        let generic_mode_label = self.tr("app.generic_mode");
                        let settings_open_label = self.tr("app.settings_open");

                        ui.horizontal(|ui| {
                            ui.label(muted_label(&backend_label, self.theme.weak_text_color));
                            let backend_button =
                                dropdown_button_text(&self.selected_backend, &self.theme);
                            egui::ComboBox::from_id_source("backend_combo")
                                .selected_text(backend_button)
                                .width(148.0)
                                .show_ui(ui, |ui| {
                                    apply_dropdown_menu_style(ui, &self.theme);
                                    dropdown_option(
                                        ui,
                                        &mut self.selected_backend,
                                        "ollama",
                                        &self.theme,
                                    );
                                    dropdown_option(
                                        ui,
                                        &mut self.selected_backend,
                                        "chatgpt",
                                        &self.theme,
                                    );
                                    dropdown_option(
                                        ui,
                                        &mut self.selected_backend,
                                        "claude",
                                        &self.theme,
                                    );
                                    dropdown_option(
                                        ui,
                                        &mut self.selected_backend,
                                        "gemini",
                                        &self.theme,
                                    );
                                });
                            ui.add_space(6.0);
                            ui.checkbox(&mut self.generic_question_mode, generic_mode_label);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let gear =
                                        secondary_action_button("⚙", self.theme.panel_fill_soft)
                                            .min_size(egui::vec2(34.0, 34.0));
                                    if ui.add(gear).on_hover_text(settings_open_label).clicked() {
                                        self.show_settings = !self.show_settings;
                                    }
                                },
                            );
                        });
                        ui.add_space(10.0);

                        let prompt_id = ui.make_persistent_id("prompt_input");
                        let prompt_hint = self.tr("app.prompt_hint");
                        let input_output = input_frame(ctx, self.theme.panel_fill).show(ui, |ui| {
                            egui::TextEdit::multiline(&mut self.prompt)
                                .id(prompt_id)
                                .hint_text(prompt_hint)
                                .desired_width(f32::INFINITY)
                                .desired_rows(4)
                                .show(ui)
                        });
                        let input_output = input_output.inner;
                        let input_resp = &input_output.response;

                        if !self.prompt_focus_initialized {
                            input_resp.request_focus();

                            let mut state = input_output.state.clone();
                            state.cursor.set_char_range(Some(CCursorRange::two(
                                CCursor::new(0),
                                CCursor::new(0),
                            )));
                            state.store(ctx, prompt_id);

                            self.prompt_focus_initialized = true;
                            ctx.request_repaint();
                        }

                        if input_resp.has_focus()
                            && ctx.input(|i| {
                                i.key_pressed(egui::Key::Enter)
                                    && !i.modifiers.shift
                                    && !i.modifiers.ctrl
                                    && !i.modifiers.alt
                                    && !i.modifiers.command
                            })
                        {
                            self.submit_prompt(ctx);
                        }

                        if ctx.input(|i| {
                            i.key_pressed(egui::Key::Enter)
                                && (i.modifiers.ctrl || i.modifiers.command)
                                && !i.modifiers.shift
                                && !i.modifiers.alt
                        }) {
                            self.auto_copy_close_after_response = true;
                            self.submit_prompt(ctx);
                        }

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add(secondary_action_button(
                                    &self.tr("app.attach_image"),
                                    self.theme.panel_fill_soft,
                                ))
                                .clicked()
                            {
                                self.attach_image_from_file();
                            }

                            if ui
                                .add(secondary_action_button(
                                    &self.tr("app.paste_image"),
                                    self.theme.panel_fill_soft,
                                ))
                                .clicked()
                            {
                                self.attach_image_from_clipboard();
                            }

                            let voice_label = if self.voice_recording.is_some() {
                                self.tr("app.voice_stop")
                            } else {
                                self.tr("app.voice_start")
                            };
                            if ui
                                .add(secondary_action_button(
                                    &voice_label,
                                    self.theme.panel_fill_soft,
                                ))
                                .clicked()
                            {
                                self.toggle_dictation(ctx);
                            }

                            if !self.attachments.is_empty()
                                && ui
                                    .add(secondary_action_button(
                                        &self.tr("app.clear_images"),
                                        self.theme.panel_fill,
                                    ))
                                    .clicked()
                            {
                                self.clear_attachments();
                            }
                        });

                        if !self.attachments.is_empty() {
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(self.tr_with(
                                        "app.images_attached",
                                        &[("count", self.attachments.len().to_string())],
                                    ))
                                    .small()
                                    .color(self.theme.weak_text_color),
                                );
                                for image in &self.attachments {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} ({})",
                                            image.name,
                                            format_size(image.size_bytes)
                                        ))
                                        .small()
                                        .color(self.theme.text_color),
                                    );
                                }
                            });
                        }

                        if let Some(status) = &self.dictation_status {
                            ui.add_space(4.0);
                            ui.label(muted_label(status, self.theme.weak_text_color));
                        }
                        if let Some(notice) = &self.attachment_notice {
                            ui.label(muted_label(notice, self.theme.weak_text_color));
                        }
                        if let Some(error) = &self.attachment_error {
                            ui.colored_label(self.theme.danger_color, error);
                        }

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            let helper_text = if self.is_loading {
                                self.tr_with(
                                    "app.helper_waiting",
                                    &[("backend", self.selected_backend.clone())],
                                )
                            } else {
                                self.tr("app.helper_ready")
                            };
                            ui.label(
                                egui::RichText::new(helper_text)
                                    .small()
                                    .color(self.theme.weak_text_color),
                            );
                        });

                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            let history_count = self.history_entries.len();
                            let history_label = if self.show_history {
                                self.tr_with(
                                    "app.hide_history",
                                    &[("count", history_count.to_string())],
                                )
                            } else {
                                self.tr_with(
                                    "app.show_history",
                                    &[("count", history_count.to_string())],
                                )
                            };

                            if ui
                                .add_enabled(
                                    !self.is_loading,
                                    primary_action_button(
                                        &self.tr("app.send"),
                                        self.theme.accent_color,
                                        self.theme.accent_text_color,
                                    ),
                                )
                                .clicked()
                            {
                                self.submit_prompt(ctx);
                            }

                            if ui
                                .add_enabled(
                                    !self.response.is_empty(),
                                    secondary_action_button(
                                        &self.tr("app.copy_response"),
                                        self.theme.panel_fill,
                                    ),
                                )
                                .clicked()
                            {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(self.response.clone());
                                }
                            }

                            if ui
                                .add(toggle_action_button(
                                    &history_label,
                                    if self.show_history {
                                        self.theme.panel_fill_soft
                                    } else {
                                        self.theme.panel_fill
                                    },
                                ))
                                .clicked()
                            {
                                self.set_history_visibility(ctx, !self.show_history);
                            }
                        });

                        if let Some(path) = &self.config.loaded_from {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(self.tr_with(
                                    "app.config_path",
                                    &[("path", path.display().to_string())],
                                ))
                                .small()
                                .color(self.theme.weak_text_color),
                            );
                        }

                        if let Some(notice) = &self.settings_notice {
                            ui.label(muted_label(notice, self.theme.weak_text_color));
                        }
                        if let Some(error) = &self.settings_error {
                            ui.colored_label(self.theme.danger_color, error);
                        }

                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.label(section_label(
                                &self.tr("app.response"),
                                self.theme.text_color,
                            ));
                            if self.is_loading {
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(self.tr("app.generating"))
                                        .small()
                                        .color(self.theme.weak_text_color),
                                );
                            }
                        });
                        ui.add_space(6.0);

                        input_frame(ctx, self.theme.panel_fill).show(ui, |ui| {
                            let response_height = if self.show_history { 160.0 } else { 230.0 };
                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .max_height(response_height)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.response.as_str())
                                            .desired_width(f32::INFINITY)
                                            .font(egui::TextStyle::Monospace),
                                    );
                                });
                        });

                        if self.show_history {
                            ui.add_space(14.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(section_label(
                                    &self.tr("app.history"),
                                    self.theme.text_color,
                                ));
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new(self.tr("app.last_7_days"))
                                        .small()
                                        .color(self.theme.weak_text_color),
                                );
                            });
                            ui.add_space(6.0);

                            card_frame(ctx, self.theme.panel_fill, self.theme.border_color).show(
                                ui,
                                |ui| {
                                    let history_search_hint = self.tr("app.search_history");
                                    let open_history_label = self.tr("app.open_history_file");
                                    let select_all_label = self.tr("app.select_all");
                                    let delete_all_label = self.tr("app.delete_all");
                                    let delete_selected_label = self.tr_with(
                                        "app.delete_selected",
                                        &[(
                                            "count",
                                            self.selected_history_entries.len().to_string(),
                                        )],
                                    );
                                    ui.horizontal_wrapped(|ui| {
                                        egui::ComboBox::from_id_source("history_backend_filter")
                                            .selected_text(
                                                match self.history_filter_backend.as_str() {
                                                    "all" => dropdown_button_text(
                                                        all_backends.as_str(),
                                                        &self.theme,
                                                    ),
                                                    "chatgpt" => {
                                                        dropdown_button_text("chatgpt", &self.theme)
                                                    }
                                                    "claude" => {
                                                        dropdown_button_text("claude", &self.theme)
                                                    }
                                                    "gemini" => {
                                                        dropdown_button_text("gemini", &self.theme)
                                                    }
                                                    "ollama" => {
                                                        dropdown_button_text("ollama", &self.theme)
                                                    }
                                                    _ => dropdown_button_text(
                                                        all_backends.as_str(),
                                                        &self.theme,
                                                    ),
                                                },
                                            )
                                            .width(150.0)
                                            .show_ui(ui, |ui| {
                                                apply_dropdown_menu_style(ui, &self.theme);
                                                ui.selectable_value(
                                                    &mut self.history_filter_backend,
                                                    "all".to_string(),
                                                    dropdown_item_text(
                                                        all_backends.as_str(),
                                                        &self.theme,
                                                    ),
                                                );
                                                dropdown_option(
                                                    ui,
                                                    &mut self.history_filter_backend,
                                                    "chatgpt",
                                                    &self.theme,
                                                );
                                                dropdown_option(
                                                    ui,
                                                    &mut self.history_filter_backend,
                                                    "claude",
                                                    &self.theme,
                                                );
                                                dropdown_option(
                                                    ui,
                                                    &mut self.history_filter_backend,
                                                    "gemini",
                                                    &self.theme,
                                                );
                                                dropdown_option(
                                                    ui,
                                                    &mut self.history_filter_backend,
                                                    "ollama",
                                                    &self.theme,
                                                );
                                            });
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.history_filter_query,
                                            )
                                            .hint_text(history_search_hint)
                                            .desired_width(280.0),
                                        );
                                        if ui
                                            .add(secondary_action_button(
                                                &open_history_label,
                                                self.theme.panel_fill_soft,
                                            ))
                                            .clicked()
                                        {
                                            self.history_action_error = open_history_file()
                                                .err()
                                                .map(|err| err.to_string());
                                        }
                                        if ui
                                            .add(secondary_action_button(
                                                &select_all_label,
                                                self.theme.panel_fill_soft,
                                            ))
                                            .clicked()
                                        {
                                            self.select_all_visible_history_entries();
                                        }
                                        if ui
                                            .add_enabled(
                                                !self.selected_history_entries.is_empty(),
                                                secondary_action_button(
                                                    &delete_selected_label,
                                                    self.theme.panel_fill_soft,
                                                ),
                                            )
                                            .clicked()
                                        {
                                            self.delete_selected_history_entries();
                                        }
                                        if ui
                                            .add(secondary_action_button(
                                                &delete_all_label,
                                                self.theme.panel_fill_soft,
                                            ))
                                            .clicked()
                                        {
                                            self.delete_all_visible_history_entries();
                                        }
                                    });

                                    if let Some(error) = &self.history_error {
                                        ui.add_space(8.0);
                                        ui.colored_label(self.theme.danger_color, error);
                                    } else if let Some(error) = &self.history_action_error {
                                        ui.add_space(8.0);
                                        ui.colored_label(self.theme.danger_color, error);
                                    }

                                    ui.add_space(10.0);

                                    if !self.session_history_entries.is_empty() {
                                        ui.label(
                                            egui::RichText::new(self.tr("app.session_history"))
                                                .strong()
                                                .color(self.theme.text_color),
                                        );
                                        ui.add_space(6.0);

                                        for entry in self.session_history_entries.iter().take(5) {
                                            history_entry_card(
                                                &self.tr("app.copy_result"),
                                                &self.tr("app.reuse_entry"),
                                                &self.tr("app.select_entry"),
                                                &self.tr("app.history_prompt"),
                                                &self.tr("app.history_response"),
                                                ui,
                                                ctx,
                                                &self.theme,
                                                entry,
                                                &mut self.selected_history_entries,
                                                &mut self.prompt,
                                                &mut self.response,
                                                &mut self.show_history,
                                                &mut self.prompt_focus_initialized,
                                                &mut self.history_action_error,
                                            );
                                            ui.add_space(8.0);
                                        }

                                        ui.separator();
                                        ui.add_space(10.0);
                                    }

                                    let entries = self.filtered_history_entries();
                                    if entries.is_empty() {
                                        ui.label(
                                            egui::RichText::new(self.tr("app.no_history"))
                                                .color(self.theme.weak_text_color),
                                        );
                                    } else {
                                        let history_height =
                                            ui.available_height().clamp(240.0, 380.0);
                                        egui::ScrollArea::vertical()
                                            .id_source("history_entries_scroll")
                                            .auto_shrink([false; 2])
                                            .max_height(history_height)
                                            .show(ui, |ui| {
                                                for (index, entry) in entries.iter().enumerate() {
                                                    history_entry_card(
                                                        &self.tr("app.copy_result"),
                                                        &self.tr("app.reuse_entry"),
                                                        &self.tr("app.select_entry"),
                                                        &self.tr("app.history_prompt"),
                                                        &self.tr("app.history_response"),
                                                        ui,
                                                        ctx,
                                                        &self.theme,
                                                        entry,
                                                        &mut self.selected_history_entries,
                                                        &mut self.prompt,
                                                        &mut self.response,
                                                        &mut self.show_history,
                                                        &mut self.prompt_focus_initialized,
                                                        &mut self.history_action_error,
                                                    );
                                                    if index + 1 < entries.len() {
                                                        ui.add_space(10.0);
                                                    }
                                                }
                                            });
                                    }
                                },
                            );
                        }
                    });
                });
        });
    }
}

fn section_label(text: &str, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text).strong().size(15.0).color(color)
}

fn muted_label(text: &str, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text).small().color(color)
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
    visuals.selection.bg_fill = lighten(theme.panel_fill_soft, 0.06);
    visuals.selection.stroke = egui::Stroke::new(1.0, theme.border_color.gamma_multiply(0.35));
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

fn primary_action_button<'a>(
    label: &'a str,
    fill: egui::Color32,
    text_color: egui::Color32,
) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).strong().color(text_color))
        .fill(fill)
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(10.0))
        .min_size(egui::vec2(126.0, 34.0))
}

fn secondary_action_button<'a>(label: &'a str, fill: egui::Color32) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).strong())
        .fill(fill)
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(10.0))
        .min_size(egui::vec2(118.0, 34.0))
}

fn toggle_action_button<'a>(label: &'a str, fill: egui::Color32) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).strong())
        .fill(fill)
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(10.0))
        .min_size(egui::vec2(136.0, 34.0))
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
        .inner_margin(egui::Margin::same(10.0))
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
        .inner_margin(egui::Margin::same(8.0))
}

fn render_settings_panel(app: &mut AiPopupApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(section_label(&app.tr("app.settings"), app.theme.text_color));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(secondary_action_button(
                        &app.tr("settings.close"),
                        app.theme.panel_fill_soft,
                    ))
                    .clicked()
                {
                    app.show_settings = false;
                }
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(muted_label(
                &app.tr("settings.version"),
                app.theme.weak_text_color,
            ));
            ui.label(
                egui::RichText::new(format!("v{}", display_version()))
                    .small()
                    .color(app.theme.weak_text_color),
            );
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
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
                egui::ComboBox::from_id_source("settings_language")
                    .selected_text(dropdown_button_text(&current_language, &app.theme))
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        apply_dropdown_menu_style(ui, &app.theme);
                        let locales: Vec<(String, String)> = app
                            .available_locales
                            .iter()
                            .map(|locale| (locale.code.clone(), locale.name.clone()))
                            .collect();
                        for (code, name) in locales {
                            if ui
                                .selectable_label(
                                    app.config.ui.language == code,
                                    dropdown_item_text(&name, &app.theme),
                                )
                                .clicked()
                            {
                                app.apply_language(&code);
                                app.persist_settings();
                            }
                        }
                    });

                ui.add_space(8.0);
                ui.label(muted_label(
                    &app.tr("settings.theme"),
                    app.theme.weak_text_color,
                ));
                let current_theme = app.config.theme.name.clone();
                egui::ComboBox::from_id_source("settings_theme")
                    .selected_text(dropdown_button_text(&current_theme, &app.theme))
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        apply_dropdown_menu_style(ui, &app.theme);
                        let themes = app.available_themes.clone();
                        for theme_name in themes {
                            if ui
                                .selectable_label(
                                    app.config.theme.name == theme_name,
                                    dropdown_item_text(&theme_name, &app.theme),
                                )
                                .clicked()
                            {
                                app.apply_theme_by_name(ctx, &theme_name);
                                app.persist_settings();
                            }
                        }
                    });

                ui.add_space(8.0);
                ui.label(muted_label(
                    &app.tr("settings.default_backend"),
                    app.theme.weak_text_color,
                ));
                egui::ComboBox::from_id_source("settings_default_backend")
                    .selected_text(dropdown_button_text(
                        &app.config.default_backend,
                        &app.theme,
                    ))
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        apply_dropdown_menu_style(ui, &app.theme);
                        for backend in ["ollama", "chatgpt", "claude", "gemini"] {
                            if ui
                                .selectable_label(
                                    app.config.default_backend == backend,
                                    dropdown_item_text(backend, &app.theme),
                                )
                                .clicked()
                            {
                                app.config.default_backend = backend.to_string();
                                app.selected_backend = backend.to_string();
                                app.persist_settings();
                            }
                        }
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

                ui.add_space(12.0);
                ui.label(section_label(
                    &app.tr("settings.models"),
                    app.theme.text_color,
                ));

                let health_checks = backends::health_checks(&app.config);
                let find_health = |backend_name: &str| {
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
                };

                let gemini_key_label = app.tr("settings.gemini_key");
                let gemini_model_label = app.tr("settings.gemini_model");
                if let Some(gemini) = app.config.gemini.as_mut() {
                    if provider_settings_section(
                        ui,
                        &app.theme,
                        "settings_provider_gemini",
                        "gemini",
                        &find_health("gemini"),
                        &gemini_key_label,
                        &gemini_model_label,
                        &mut gemini.api_key,
                        &mut gemini.model,
                    ) {
                        app.persist_settings();
                    }
                }

                let chatgpt_key_label = app.tr("settings.chatgpt_key");
                let chatgpt_model_label = app.tr("settings.chatgpt_model");
                if let Some(chatgpt) = app.config.chatgpt.as_mut() {
                    if provider_settings_section(
                        ui,
                        &app.theme,
                        "settings_provider_chatgpt",
                        "chatgpt",
                        &find_health("chatgpt"),
                        &chatgpt_key_label,
                        &chatgpt_model_label,
                        &mut chatgpt.api_key,
                        &mut chatgpt.model,
                    ) {
                        app.persist_settings();
                    }
                }

                let claude_key_label = app.tr("settings.claude_key");
                let claude_model_label = app.tr("settings.claude_model");
                if let Some(claude) = app.config.claude.as_mut() {
                    if provider_settings_section(
                        ui,
                        &app.theme,
                        "settings_provider_claude",
                        "claude",
                        &find_health("claude"),
                        &claude_key_label,
                        &claude_model_label,
                        &mut claude.api_key,
                        &mut claude.model,
                    ) {
                        app.persist_settings();
                    }
                }

                let ollama_url_label = app.tr("settings.ollama_url");
                let ollama_model_label = app.tr("settings.ollama_model");
                if let Some(ollama) = app.config.ollama.as_mut() {
                    if provider_settings_section(
                        ui,
                        &app.theme,
                        "settings_provider_ollama",
                        "ollama",
                        &find_health("ollama"),
                        &ollama_url_label,
                        &ollama_model_label,
                        &mut ollama.base_url,
                        &mut ollama.model,
                    ) {
                        app.persist_settings();
                    }
                }

                ui.add_space(8.0);
                ui.label(muted_label(
                    &app.tr("settings.saved"),
                    app.theme.weak_text_color,
                ));
            });
    });
}

fn provider_settings_section(
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
        .default_open(provider == "chatgpt")
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(&health_check.detail)
                    .small()
                    .color(theme.weak_text_color),
            );
            changed |= settings_text_field(ui, theme, primary_label, primary_value, true);
            changed |= settings_text_field(ui, theme, secondary_label, secondary_value, false);
        });
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

fn load_image_attachment_from_path(path: &Path) -> Result<ImageAttachment, String> {
    let bytes = std::fs::read(path)
        .map_err(|err| format!("Could not read image file `{}`: {}", path.display(), err))?;
    let mime_type = infer_image_mime(path)
        .ok_or_else(|| "Unsupported image format. Use PNG, JPG, JPEG, WEBP, or GIF.".to_string())?;

    Ok(ImageAttachment {
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("image")
            .to_string(),
        mime_type: mime_type.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes.as_slice()),
        size_bytes: bytes.len(),
    })
}

fn load_image_attachment_from_clipboard() -> Result<ImageAttachment, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|err| format!("Clipboard not available: {}", err))?;
    let image = clipboard
        .get_image()
        .map_err(|_| "Clipboard does not currently contain an image.".to_string())?;

    let mut png_bytes = Vec::new();
    PngEncoder::new(&mut png_bytes)
        .write_image(
            image.bytes.as_ref(),
            image.width as u32,
            image.height as u32,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|err| format!("Could not encode clipboard image: {}", err))?;

    Ok(ImageAttachment {
        name: "clipboard-screenshot.png".to_string(),
        mime_type: "image/png".to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(png_bytes.as_slice()),
        size_bytes: png_bytes.len(),
    })
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
        format!("{} B", bytes)
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
        .map_err(|err| format!("Could not start microphone recording: {}", err))?;

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
        .map_err(|err| format!("Could not read recorded dictation audio: {}", err))?;
    let _ = std::fs::remove_file(&recording.path);
    if bytes.is_empty() {
        return Err(String::new());
    }
    Ok(bytes)
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", name))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn history_entry_card(
    copy_label: &str,
    reuse_label: &str,
    select_label: &str,
    prompt_label: &str,
    response_label: &str,
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
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(entry.response.clone());
                }
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

fn sync_history_viewport(ctx: &egui::Context, show_history: bool) {
    const BASE_MIN_WIDTH: f32 = 620.0;
    const BASE_MIN_HEIGHT: f32 = 420.0;
    const HISTORY_MIN_HEIGHT: f32 = 620.0;

    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(if show_history {
        egui::vec2(BASE_MIN_WIDTH, HISTORY_MIN_HEIGHT)
    } else {
        egui::vec2(BASE_MIN_WIDTH, BASE_MIN_HEIGHT)
    }));
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

fn trim_timestamp(value: &str) -> String {
    value.replace('T', " ").replace('Z', "")
}

fn open_history_file() -> anyhow::Result<()> {
    let path = history::history_file_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        std::fs::File::create(&path)?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(&path).spawn()?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(&path).spawn()?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(anyhow::anyhow!(
        "Opening the history file is not supported on this platform"
    ))
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
        .args(&["-o", "-selection", "primary"])
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
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.indent = 10.0;
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

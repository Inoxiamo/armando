use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::backends;
use crate::config::Config;
use crate::history::{self, HistoryEntry};
use crate::theme::ResolvedTheme;

enum PasteOutcome {
    Applied,
    ClipboardOnly,
    Unavailable,
}

pub struct AiPopupApp {
    config: Config,
    theme: ResolvedTheme,
    runtime: Arc<Runtime>,

    // UI State
    prompt: String,
    response: String,
    selected_backend: String,
    is_loading: bool,
    auto_paste_on_finish: bool,
    prompt_focus_initialized: bool,
    history_entries: Vec<HistoryEntry>,
    history_error: Option<String>,
    show_history: bool,
    history_filter_backend: String,
    history_filter_query: String,
    history_action_error: Option<String>,

    // For tokio to update UI when done
    async_response: Arc<Mutex<Option<Result<String, String>>>>,
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

        Self {
            config,
            theme,
            runtime,
            prompt: initial_prompt,
            response: String::new(),
            selected_backend: default_backend,
            is_loading: false,
            auto_paste_on_finish: false,
            prompt_focus_initialized: false,
            history_entries,
            history_error,
            show_history: false,
            history_filter_backend: "all".to_string(),
            history_filter_query: String::new(),
            history_action_error: None,
            async_response: Arc::new(Mutex::new(None)),
        }
    }

    fn check_async_response(&mut self, ctx: &egui::Context) {
        let res = {
            let mut resp_lock = self.async_response.lock().unwrap();
            resp_lock.take()
        };

        if let Some(res) = res {
            self.is_loading = false;
            match res {
                Ok(text) => {
                    self.response = text;
                    self.reload_history();
                    if self.auto_paste_on_finish {
                        match close_and_paste(ctx, self.response.clone()) {
                            PasteOutcome::Applied => {}
                            PasteOutcome::ClipboardOnly => {
                                self.response.push_str(
                                    "\n\n[nota] Risposta copiata negli appunti. Per applicarla automaticamente su Linux serve `wtype` (Wayland) oppure `xdotool` (X11).",
                                );
                            }
                            PasteOutcome::Unavailable => {
                                self.response.push_str(
                                    "\n\n[nota] Auto-apply non disponibile: installa `wl-copy` + `wtype` su Wayland oppure `xclip` + `xdotool` su X11.",
                                );
                            }
                        }
                        self.auto_paste_on_finish = false;
                    }
                }
                Err(e) => {
                    self.response = format!("Error: {}", e);
                    self.auto_paste_on_finish = false;
                }
            }
        }
    }

    fn submit_prompt(&mut self, ctx: &egui::Context) {
        if self.prompt.trim().is_empty() || self.is_loading {
            return;
        }

        self.is_loading = true;
        self.response = format!("⏳ Querying {}…", self.selected_backend);

        let prompt = self.prompt.clone();
        let backend = self.selected_backend.clone();
        let config = self.config.clone();
        let async_response = self.async_response.clone();
        let ctx = ctx.clone();

        // Spawn async task
        self.runtime.spawn(async move {
            let res = backends::query(&backend, &prompt, &config).await;

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
                self.history_error = None;
                self.history_action_error = None;
            }
            Err(err) => {
                self.history_error = Some(err.to_string());
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
}

impl eframe::App for AiPopupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_async_response(ctx);

        // Handle global Esc to close
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Handle Paste & Close hotkey (default: Ctrl+Enter)
        // We parse our standard config <ctrl>+<enter> into egui checking
        let paste_pressed = {
            let s = self.config.paste_response_shortcut.to_lowercase();
            let mut mods = egui::Modifiers::NONE;
            if s.contains("ctrl") {
                mods.ctrl = true;
            }
            if s.contains("shift") {
                mods.shift = true;
            }
            if s.contains("alt") {
                mods.alt = true;
            }

            let key = if s.contains("enter") {
                egui::Key::Enter
            } else if s.contains("space") {
                egui::Key::Space
            } else {
                egui::Key::Enter
            }; // fallback

            shortcut_pressed(ctx, mods, key)
        };

        if paste_pressed {
            if !self.response.is_empty() && !self.is_loading {
                match close_and_paste(ctx, self.response.clone()) {
                    PasteOutcome::Applied => {}
                    PasteOutcome::ClipboardOnly => {
                        self.response.push_str(
                            "\n\n[nota] Risposta copiata negli appunti. Per applicarla automaticamente su Linux serve `wtype` (Wayland) oppure `xdotool` (X11).",
                        );
                    }
                    PasteOutcome::Unavailable => {
                        self.response.push_str(
                            "\n\n[nota] Auto-apply non disponibile: installa `wl-copy` + `wtype` su Wayland oppure `xclip` + `xdotool` su X11.",
                        );
                    }
                }
            } else if !self.prompt.trim().is_empty() {
                self.auto_paste_on_finish = true;
                if !self.is_loading {
                    self.submit_prompt(ctx);
                }
            }
        }

        let frame = egui::Frame::none()
            .fill(ctx.style().visuals.window_fill)
            .stroke(ctx.style().visuals.window_stroke)
            .rounding(ctx.style().visuals.window_rounding)
            .inner_margin(egui::Margin::same(0.0));

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let panel_frame = card_frame(
                ctx,
                self.theme.panel_fill_raised,
                self.theme.border_color,
            );

            panel_frame
                .fill(self.theme.panel_fill_raised)
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("AI Assistant")
                                        .strong()
                                        .size(21.0)
                                        .color(ctx.style().visuals.hyperlink_color),
                                );
                                ui.label(
                                    egui::RichText::new("Fast prompts, polished history, instant apply.")
                                        .small()
                                        .color(self.theme.weak_text_color),
                                );
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                egui::ComboBox::from_id_source("backend_combo")
                                    .selected_text(&self.selected_backend)
                                    .width(132.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_backend, "ollama".to_string(), "ollama");
                                        ui.selectable_value(&mut self.selected_backend, "chatgpt".to_string(), "chatgpt");
                                        ui.selectable_value(&mut self.selected_backend, "gemini".to_string(), "gemini");
                                    });
                                ui.label(egui::RichText::new("Backend").strong());
                            });
                        });

                        ui.add_space(10.0);

                        let prompt_id = ui.make_persistent_id("prompt_input");
                        let input_output = card_frame(
                            ctx,
                            darken(ctx.style().visuals.panel_fill, 0.08),
                            ctx.style().visuals.hyperlink_color,
                        )
                        .show(ui, |ui| {
                            egui::TextEdit::multiline(&mut self.prompt)
                                .id(prompt_id)
                                .hint_text("Write your prompt here...")
                                .desired_width(f32::INFINITY)
                                .desired_rows(4)
                                .show(ui)
                        });
                        let input_output = input_output.inner;
                        let input_resp = &input_output.response;

                        if !self.prompt_focus_initialized {
                            input_resp.request_focus();

                            let mut state = input_output.state.clone();
                            state
                                .cursor
                                .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(0))));
                            state.store(ctx, prompt_id);

                            self.prompt_focus_initialized = true;
                            ctx.request_repaint();
                        }

                        if input_resp.has_focus() {
                            if ctx.input(|i| {
                                i.key_pressed(egui::Key::Enter)
                                    && !i.modifiers.shift
                                    && !i.modifiers.ctrl
                                    && !i.modifiers.alt
                                    && !i.modifiers.command
                            }) {
                                self.submit_prompt(ctx);
                            }
                        }

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            let helper_text = if self.is_loading {
                                format!("Waiting for {}...", self.selected_backend)
                            } else {
                                format!(
                                    "Enter sends, Shift+Enter adds a newline, Esc closes, {} pastes and closes.",
                                    self.config.paste_response_shortcut
                                )
                            };
                            ui.label(
                                egui::RichText::new(helper_text)
                                    .small()
                                    .color(self.theme.weak_text_color),
                            );
                        });

                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            let history_count = self.history_entries.len();
                            let history_label = if self.show_history {
                                format!("Hide History ({})", history_count)
                            } else {
                                format!("Show History ({})", history_count)
                            };
                            let primary_button = primary_action_button(
                                "Send Prompt",
                                self.theme.accent_color,
                                self.theme.border_color,
                                self.theme.accent_text_color,
                            );
                            let copy_button = secondary_action_button(
                                "Copy Response",
                                self.theme.panel_fill_soft,
                                self.theme.border_color,
                            );
                            let history_button = toggle_action_button(
                                &history_label,
                                if self.show_history {
                                    self.theme.panel_fill_soft
                                } else {
                                    self.theme.panel_fill
                                },
                                self.theme.accent_color,
                            );

                            if ui
                                .add_enabled(
                                    !self.is_loading,
                                    primary_button,
                                )
                                .clicked()
                            {
                                self.submit_prompt(ctx);
                            }

                            if ui
                                .add_enabled(!self.response.is_empty(), copy_button)
                                .clicked()
                            {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(self.response.clone());
                                }
                            }

                            if ui.add(history_button).clicked() {
                                self.set_history_visibility(ctx, !self.show_history);
                            }
                        });

                        if let Some(path) = &self.config.loaded_from {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("Config: {}", path.display()))
                                    .small()
                                    .weak(),
                            );
                        }

                        ui.add_space(10.0);

                        ui.label(section_label(ctx, "Response"));
                        ui.add_space(4.0);
                        card_frame(
                            ctx,
                            self.theme.panel_fill_soft,
                            self.theme.border_color,
                        )
                        .show(ui, |ui| {
                            let response_height = if self.show_history { 150.0 } else { 250.0 };
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
                            ui.add_space(12.0);
                            ui.label(section_label(ctx, "History"));
                            ui.add_space(6.0);

                            card_frame(
                                ctx,
                                self.theme.panel_fill_soft,
                                self.theme.border_color,
                            )
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new("Last 7 days")
                                            .small()
                                            .color(self.theme.weak_text_color),
                                    );

                                    let open_button = secondary_action_button(
                                        "Open History File",
                                        self.theme.panel_fill_raised,
                                        self.theme.border_color,
                                    );
                                    if ui.add(open_button).clicked() {
                                        self.history_action_error =
                                            open_history_file().err().map(|err| err.to_string());
                                    }
                                });

                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    egui::ComboBox::from_id_source("history_backend_filter")
                                        .selected_text(match self.history_filter_backend.as_str() {
                                            "all" => "All backends",
                                            "chatgpt" => "chatgpt",
                                            "gemini" => "gemini",
                                            "ollama" => "ollama",
                                            _ => "All backends",
                                        })
                                        .width(150.0)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut self.history_filter_backend, "all".to_string(), "All backends");
                                            ui.selectable_value(&mut self.history_filter_backend, "chatgpt".to_string(), "chatgpt");
                                            ui.selectable_value(&mut self.history_filter_backend, "gemini".to_string(), "gemini");
                                            ui.selectable_value(&mut self.history_filter_backend, "ollama".to_string(), "ollama");
                                        });
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.history_filter_query)
                                            .hint_text("Search prompt or response...")
                                            .desired_width(260.0),
                                    );
                                });

                                if let Some(error) = &self.history_error {
                                    ui.add_space(4.0);
                                    ui.colored_label(self.theme.danger_color, error);
                                } else if let Some(error) = &self.history_action_error {
                                    ui.add_space(4.0);
                                    ui.colored_label(self.theme.danger_color, error);
                                }

                                ui.add_space(8.0);

                                let entries = self.filtered_history_entries();
                                if entries.is_empty() {
                                    ui.label("No recent history yet. Send a prompt and it will appear here.");
                                } else {
                                    let history_height = ui.available_height().clamp(240.0, 360.0);
                                    egui::ScrollArea::vertical()
                                        .id_source("history_entries_scroll")
                                        .auto_shrink([false; 2])
                                        .max_height(history_height)
                                        .show(ui, |ui| {
                                            for (index, entry) in entries.iter().enumerate() {
                                                history_entry_card(
                                                    ui,
                                                    ctx,
                                                    &self.theme,
                                                    entry,
                                                    &mut self.prompt,
                                                    &mut self.response,
                                                    &mut self.show_history,
                                                    &mut self.prompt_focus_initialized,
                                                    &mut self.history_action_error,
                                                );
                                                if index + 1 < entries.len() {
                                                    ui.add_space(8.0);
                                                }
                                            }
                                        });
                                }
                            });
                        }
                    });
                });
        });
    }
}

fn section_label(ctx: &egui::Context, text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .strong()
        .size(15.0)
        .color(ctx.style().visuals.hyperlink_color)
}

fn primary_action_button<'a>(
    label: &'a str,
    fill: egui::Color32,
    stroke: egui::Color32,
    text_color: egui::Color32,
) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).strong().color(text_color))
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .rounding(egui::Rounding::same(7.0))
        .min_size(egui::vec2(138.0, 34.0))
}

fn secondary_action_button<'a>(
    label: &'a str,
    fill: egui::Color32,
    stroke: egui::Color32,
) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).strong())
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .rounding(egui::Rounding::same(7.0))
        .min_size(egui::vec2(120.0, 34.0))
}

fn toggle_action_button<'a>(
    label: &'a str,
    fill: egui::Color32,
    stroke: egui::Color32,
) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).strong())
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .rounding(egui::Rounding::same(7.0))
        .min_size(egui::vec2(148.0, 34.0))
}

fn card_frame(ctx: &egui::Context, fill: egui::Color32, stroke: egui::Color32) -> egui::Frame {
    egui::Frame::none()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .rounding(egui::Rounding::same(10.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 8.0),
            blur: 18.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(if ctx.style().visuals.dark_mode {
                72
            } else {
                36
            }),
        })
        .inner_margin(egui::Margin::same(10.0))
}

fn history_entry_card(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    theme: &ResolvedTheme,
    entry: &HistoryEntry,
    prompt: &mut String,
    response: &mut String,
    show_history: &mut bool,
    prompt_focus_initialized: &mut bool,
    history_action_error: &mut Option<String>,
) {
    card_frame(ctx, theme.panel_fill_raised, theme.border_color).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
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
        ui.label(
            egui::RichText::new(trim_for_preview(&entry.prompt, 180))
                .strong()
                .small(),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(trim_for_preview(&entry.response, 260))
                .small()
                .color(theme.weak_text_color),
        );
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            let copy_button =
                secondary_action_button("Copy Result", theme.panel_fill_soft, theme.border_color);
            if ui.add(copy_button).clicked() {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(entry.response.clone());
                }
                *history_action_error = None;
            }

            let reuse_button = primary_action_button(
                "Reuse Entry",
                theme.accent_color,
                theme.border_color,
                theme.accent_text_color,
            );
            if ui.add(reuse_button).clicked() {
                *prompt = entry.prompt.clone();
                *response = entry.response.clone();
                *show_history = false;
                sync_history_viewport(ctx, false);
                *prompt_focus_initialized = false;
                *history_action_error = None;
            }
        });
    });
}

fn sync_history_viewport(ctx: &egui::Context, show_history: bool) {
    const BASE_MIN_WIDTH: f32 = 520.0;
    const BASE_MIN_HEIGHT: f32 = 360.0;
    const HISTORY_MIN_HEIGHT: f32 = 620.0;
    const HISTORY_GROWTH_DELTA: f32 = 220.0;
    const SCREEN_PADDING: f32 = 80.0;

    let (current_size, monitor_size) = ctx.input(|i| {
        (
            i.viewport()
                .inner_rect
                .map(|rect| rect.size())
                .unwrap_or(egui::vec2(640.0, 480.0)),
            i.viewport().monitor_size,
        )
    });

    let min_size = if show_history {
        egui::vec2(BASE_MIN_WIDTH, HISTORY_MIN_HEIGHT)
    } else {
        egui::vec2(BASE_MIN_WIDTH, BASE_MIN_HEIGHT)
    };
    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min_size));

    if show_history {
        let max_usable_height = monitor_size
            .map(|size| (size.y - SCREEN_PADDING).max(HISTORY_MIN_HEIGHT))
            .unwrap_or(HISTORY_MIN_HEIGHT + HISTORY_GROWTH_DELTA);
        let target_height = (current_size.y + HISTORY_GROWTH_DELTA)
            .max(HISTORY_MIN_HEIGHT)
            .min(max_usable_height);

        if target_height > current_size.y + 1.0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                current_size.x.max(BASE_MIN_WIDTH),
                target_height,
            )));
        }
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

fn simulate_paste(text: String) -> PasteOutcome {
    // We must spawn a detached process because this process is about to exit
    // and would take down X11/Wayland clipboard ownership with it if we used internal clipboards.

    let has_wayland_pair = command_exists("wl-copy") && command_exists("wtype");
    let has_x11_pair = command_exists("xclip") && command_exists("xdotool");
    let has_clipboard_only = command_exists("wl-copy") || command_exists("xclip");

    if !has_wayland_pair && !has_x11_pair && !has_clipboard_only {
        return PasteOutcome::Unavailable;
    }

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("popup_ai_paste.txt");
    if let Err(_) = std::fs::write(&temp_file, &text) {
        return PasteOutcome::Unavailable;
    }

    let temp_file_str = temp_file.to_string_lossy().to_string();

    // Command sequence:
    // 1. Sleep slightly to allow the popup window to close and previous window to regain focus
    // 2. Prefer direct text injection when available
    // 3. Fallback to clipboard priming
    // 4. Clean up temp file
    let script = format!(
        "sleep 0.55; \
        if command -v wl-copy >/dev/null 2>&1 && command -v wtype >/dev/null 2>&1; then \
            wtype -d 1 - < '{0}' 2>/dev/null || (wl-copy --paste-once < '{0}' && sleep 0.20 && wtype -M ctrl -s 120 -k v); \
        elif command -v xclip >/dev/null 2>&1 && command -v xdotool >/dev/null 2>&1; then \
            xdotool type --clearmodifiers --delay 1 --file '{0}' 2>/dev/null || (xclip -selection clipboard < '{0}' && sleep 0.20 && xdotool key --clearmodifiers ctrl+v); \
        elif command -v wl-copy >/dev/null 2>&1; then \
            wl-copy < '{0}'; \
        elif command -v xclip >/dev/null 2>&1; then \
            xclip -selection clipboard < '{0}'; \
        fi; \
        rm -f '{0}'",
        temp_file_str
    );

    // Spawn detached shell process
    let mut cmd = if command_exists("setsid") {
        let mut cmd = std::process::Command::new("setsid");
        cmd.arg("sh").arg("-c").arg(&script);
        cmd
    } else {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(&script);
        cmd
    };

    let _ = cmd.spawn();

    if has_wayland_pair || has_x11_pair {
        PasteOutcome::Applied
    } else {
        PasteOutcome::ClipboardOnly
    }
}

fn close_and_paste(ctx: &egui::Context, text: String) -> PasteOutcome {
    if command_exists("wtype")
        || command_exists("xdotool")
        || command_exists("wl-copy") && command_exists("wtype")
        || command_exists("xclip") && command_exists("xdotool")
    {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    simulate_paste(text)
}

fn shortcut_pressed(ctx: &egui::Context, required_mods: egui::Modifiers, key: egui::Key) -> bool {
    ctx.input(|i| {
        let mods = i.modifiers;
        let ctrl_ok = !required_mods.ctrl || mods.ctrl;
        let shift_ok = !required_mods.shift || mods.shift;
        let alt_ok = !required_mods.alt || mods.alt;
        let command_ok = !required_mods.command || mods.command;
        ctrl_ok && shift_ok && alt_ok && command_ok && i.key_pressed(key)
    })
}

fn command_exists(name: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", name))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn build_style(theme: &ResolvedTheme) -> egui::Style {
    let mut style = egui::Style::default();
    let mut visuals = egui::Visuals::dark();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(20.0, egui::FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(15.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(14.0, egui::FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(14.0, egui::FontFamily::Monospace),
    );

    visuals.window_fill = theme.window_fill;
    visuals.panel_fill = theme.panel_fill;
    visuals.faint_bg_color = theme.panel_fill_soft;
    visuals.extreme_bg_color = darken(theme.panel_fill_raised, 0.08);
    visuals.code_bg_color = darken(theme.panel_fill_soft, 0.08);
    visuals.hyperlink_color = theme.accent_color;
    visuals.selection.bg_fill = theme.accent_hover_color;
    visuals.selection.stroke.color = egui::Color32::BLACK;
    visuals.override_text_color = Some(theme.text_color);
    visuals.window_stroke.color = theme.border_color;
    visuals.window_stroke.width = 1.2;
    visuals.widgets.noninteractive.fg_stroke.color = theme.text_color;
    visuals.widgets.noninteractive.bg_fill = theme.panel_fill_raised;
    visuals.widgets.noninteractive.bg_stroke.color = theme.border_color;
    visuals.widgets.inactive.bg_fill = theme.panel_fill_soft;
    visuals.widgets.inactive.bg_stroke.color = theme.border_color;
    visuals.widgets.inactive.fg_stroke.color = theme.text_color;
    visuals.widgets.hovered.bg_fill = theme.accent_hover_color;
    visuals.widgets.hovered.bg_stroke.color = theme.border_color;
    visuals.widgets.hovered.fg_stroke.color = theme.accent_text_color;
    visuals.widgets.active.bg_fill = theme.accent_color;
    visuals.widgets.active.bg_stroke.color = theme.border_color;
    visuals.widgets.active.fg_stroke.color = theme.accent_text_color;
    visuals.widgets.open = visuals.widgets.active;

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.indent = 12.0;
    style.visuals.window_rounding = egui::Rounding::same(10.0);
    style.visuals.menu_rounding = egui::Rounding::same(8.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(7.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(7.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(7.0);
    style.visuals.widgets.noninteractive.bg_fill = darken(style.visuals.panel_fill, 0.03);
    style.visuals.widgets.noninteractive.bg_stroke.width = 1.0;
    style.visuals.widgets.inactive.bg_fill = lighten(style.visuals.panel_fill, 0.03);
    style.visuals.widgets.inactive.bg_stroke.width = 0.9;
    style.visuals.widgets.hovered.bg_stroke.width = 1.1;
    style.visuals.widgets.active.bg_stroke.width = 1.1;
    style.visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 10.0),
        blur: 22.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(96),
    };
    style.visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 18.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(86),
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

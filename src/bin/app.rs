use armando::{backends, config, gui, prompt_profiles, theme};
use eframe::{egui, Theme};
use std::sync::Arc;
use tokio::runtime::Runtime;

use theme::load_theme;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let cfg = config::Config::load()?;
    if args.iter().any(|arg| arg == "--rag-index") {
        return run_rag_index(cfg);
    }
    let prompt_profiles = prompt_profiles::PromptProfiles::load(&cfg)?;
    run_ui(cfg, prompt_profiles)
}

fn run_rag_index(cfg: config::Config) -> anyhow::Result<()> {
    let runtime = Runtime::new()?;
    let backend = cfg.default_backend.clone();
    let stats = runtime
        .block_on(backends::index_rag_documents(&backend, &cfg))
        .map_err(anyhow::Error::msg)?;
    println!(
        "RAG indexing completed: {} files, {} chunks (backend: {}).",
        stats.indexed_files, stats.indexed_chunks, backend
    );
    Ok(())
}

fn run_ui(
    cfg: config::Config,
    prompt_profiles: prompt_profiles::PromptProfiles,
) -> anyhow::Result<()> {
    // Create tokio runtime for async backend queries
    let rt = Arc::new(Runtime::new()?);
    let theme = load_theme(&cfg)?;
    let initial_window_height = cfg.ui.window_height.clamp(500.0, 620.0);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, initial_window_height])
            .with_min_inner_size([820.0, 500.0])
            .with_app_id("armando")
            .with_always_on_top()
            .with_resizable(true)
            .with_decorations(true)
            .with_title("armando")
            .with_icon(build_app_icon()),
        follow_system_theme: false,
        default_theme: Theme::Dark,
        ..Default::default()
    };

    run_native_with_display_fallback("armando", options, cfg, prompt_profiles, theme, rt)
}

fn run_native_with_display_fallback(
    title: &str,
    options: eframe::NativeOptions,
    cfg: config::Config,
    prompt_profiles: prompt_profiles::PromptProfiles,
    theme: theme::ResolvedTheme,
    rt: Arc<Runtime>,
) -> anyhow::Result<()> {
    match eframe::run_native(
        title,
        options,
        Box::new(move |cc| Box::new(gui::AiPopupApp::new(cc, cfg, prompt_profiles, theme, rt))),
    ) {
        Ok(()) => Ok(()),
        Err(err) => {
            #[cfg(target_os = "linux")]
            {
                let message = format!("{err:?}");
                let has_no_compositor = message.contains("WaylandError(Connection(NoCompositor))");
                let x11_backend_forced = std::env::var("WINIT_UNIX_BACKEND")
                    .map(|value| value.eq_ignore_ascii_case("x11"))
                    .unwrap_or(false);
                let has_x11_display = std::env::var("DISPLAY")
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false);

                if has_no_compositor && !x11_backend_forced && has_x11_display {
                    log::warn!(
                        "Wayland compositor unavailable, retrying startup with WINIT_UNIX_BACKEND=x11"
                    );
                    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

                    let retry_cfg = config::Config::load()?;
                    let retry_profiles = prompt_profiles::PromptProfiles::load(&retry_cfg)?;
                    let retry_theme = load_theme(&retry_cfg)?;
                    let retry_rt = Arc::new(Runtime::new()?);
                    let retry_options = eframe::NativeOptions {
                        viewport: egui::ViewportBuilder::default()
                            .with_inner_size([
                                980.0,
                                retry_cfg.ui.window_height.clamp(500.0, 620.0),
                            ])
                            .with_min_inner_size([820.0, 500.0])
                            .with_app_id("armando")
                            .with_always_on_top()
                            .with_resizable(true)
                            .with_decorations(true)
                            .with_title("armando")
                            .with_icon(build_app_icon()),
                        follow_system_theme: false,
                        default_theme: Theme::Dark,
                        ..Default::default()
                    };

                    return eframe::run_native(
                        title,
                        retry_options,
                        Box::new(move |cc| {
                            Box::new(gui::AiPopupApp::new(
                                cc,
                                retry_cfg,
                                retry_profiles,
                                retry_theme,
                                retry_rt,
                            ))
                        }),
                    )
                    .map_err(|retry_err| anyhow::anyhow!("eframe error: {retry_err:?}"));
                }
            }

            Err(anyhow::anyhow!("eframe error: {err:?}"))
        }
    }
}

fn build_app_icon() -> egui::IconData {
    let width = 64;
    let height = 64;
    let mut rgba = vec![0_u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let background = if x < 8 || x >= width - 8 || y < 8 || y >= height - 8 {
                [31, 24, 48, 255]
            } else {
                [60, 45, 92, 255]
            };
            set_icon_pixel(&mut rgba, width, x, y, background);
        }
    }

    for y in 10..54 {
        for x in 10..54 {
            if x + y < 40 || x + y > 86 {
                continue;
            }

            let edge_distance = (x.min(width - 1 - x)).min(y.min(height - 1 - y));
            let glow = if edge_distance < 4 { 18 } else { 0 };
            set_icon_pixel(
                &mut rgba,
                width,
                x,
                y,
                [94 + glow, 232, 186 + glow / 2, 255],
            );
        }
    }

    for y in 26..34 {
        for x in 20..44 {
            set_icon_pixel(&mut rgba, width, x, y, [28, 22, 43, 255]);
        }
    }

    for y in 36..50 {
        for x in 28..36 {
            set_icon_pixel(&mut rgba, width, x, y, [28, 22, 43, 255]);
        }
    }

    egui::IconData {
        rgba,
        width: width as u32,
        height: height as u32,
    }
}

fn set_icon_pixel(rgba: &mut [u8], width: usize, x: usize, y: usize, color: [u8; 4]) {
    let index = (y * width + x) * 4;
    rgba[index..index + 4].copy_from_slice(&color);
}

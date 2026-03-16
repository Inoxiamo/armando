mod app_paths;
mod backends;
mod config;
mod gui;
mod history;
mod i18n;
mod logging;
mod theme;

use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::theme::load_theme;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let _args: Vec<String> = std::env::args().collect();
    let cfg = config::Config::load()?;
    run_ui(cfg)
}

fn run_ui(cfg: config::Config) -> anyhow::Result<()> {
    // Create tokio runtime for async backend queries
    let rt = Arc::new(Runtime::new()?);
    let theme = load_theme(&cfg)?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 560.0])
            .with_min_inner_size([620.0, 420.0])
            .with_app_id("armando")
            .with_always_on_top()
            .with_resizable(true)
            .with_decorations(true)
            .with_title("armando")
            .with_icon(build_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "armando",
        options,
        Box::new(move |cc| Box::new(gui::AiPopupApp::new(cc, cfg, theme, rt))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {:?}", e))
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

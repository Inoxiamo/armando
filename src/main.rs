mod backends;
mod config;
mod daemon;
mod gui;
mod history;

use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cfg = config::Config::load()?;

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--ui") {
        run_ui(cfg)
    } else {
        daemon::run(cfg)
    }
}

fn run_ui(cfg: config::Config) -> anyhow::Result<()> {
    // Create tokio runtime for async backend queries
    let rt = Arc::new(Runtime::new()?);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 480.0])
            .with_min_inner_size([520.0, 360.0])
            .with_always_on_top()
            .with_resizable(true)
            .with_decorations(true)
            .with_title("AI Assistant"),
        ..Default::default()
    };

    eframe::run_native(
        "AI Assistant",
        options,
        Box::new(move |cc| Box::new(gui::AiPopupApp::new(cc, cfg, rt))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {:?}", e))
}

use armando::{backends, config, gui, prompt_profiles, theme};
use backends::{PromptMode, QueryInput};
use eframe::{egui, Theme};
use std::io::Read;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Runtime;

use theme::load_theme;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = config::Config::load()?;
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--rag-index") {
        return run_rag_index(cfg);
    }
    let prompt_profiles = prompt_profiles::PromptProfiles::load(&cfg)?;
    if let Some(cli) = parse_cli_query_args(&args, &cfg)? {
        return run_cli_query(cfg, prompt_profiles, cli);
    }
    run_ui(cfg, prompt_profiles)
}

#[derive(Debug)]
struct CliQuery {
    prompt: String,
    backend: String,
    mode: PromptMode,
    output_json: bool,
    include_request: bool,
}

fn parse_cli_query_args(args: &[String], cfg: &config::Config) -> anyhow::Result<Option<CliQuery>> {
    parse_cli_query_args_with_reader(args, cfg, read_prompt_from_stdin)
}

fn parse_cli_query_args_with_reader<F>(
    args: &[String],
    cfg: &config::Config,
    stdin_reader: F,
) -> anyhow::Result<Option<CliQuery>>
where
    F: FnOnce() -> anyhow::Result<String>,
{
    let mut ask: Option<String> = None;
    let mut use_stdin = false;
    let mut backend: Option<String> = None;
    let mut force_generic = false;
    let mut force_text_assist = false;
    let mut output_json = false;
    let mut include_request = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--ask" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(anyhow::anyhow!("--ask requires a prompt value"));
                };
                ask = Some(value.to_string());
                index += 2;
            }
            "--stdin" => {
                use_stdin = true;
                index += 1;
            }
            "--backend" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(anyhow::anyhow!("--backend requires a backend name"));
                };
                backend = Some(value.trim().to_string());
                index += 2;
            }
            "--generic" => {
                force_generic = true;
                index += 1;
            }
            "--text-assist" => {
                force_text_assist = true;
                index += 1;
            }
            "--json" => {
                output_json = true;
                index += 1;
            }
            "--request" => {
                include_request = true;
                index += 1;
            }
            _ => {
                index += 1;
            }
        }
    }

    if force_generic && force_text_assist {
        return Err(anyhow::anyhow!(
            "Use either --generic or --text-assist, not both"
        ));
    }

    if ask.is_none() && !use_stdin {
        return Ok(None);
    }
    if ask.is_some() && use_stdin {
        return Err(anyhow::anyhow!("Use either --ask or --stdin, not both"));
    }

    let prompt = if use_stdin {
        stdin_reader()?
    } else {
        ask.unwrap_or_default()
    };
    if prompt.trim().is_empty() {
        return Err(anyhow::anyhow!("Prompt cannot be empty"));
    }

    let mode = if force_text_assist {
        PromptMode::TextAssist
    } else {
        PromptMode::GenericQuestion
    };

    let backend = backend.unwrap_or_else(|| cfg.default_backend.clone());

    Ok(Some(CliQuery {
        prompt,
        backend,
        mode,
        output_json,
        include_request,
    }))
}

fn read_prompt_from_stdin() -> anyhow::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    Ok(input)
}

fn run_cli_query(
    cfg: config::Config,
    prompt_profiles: prompt_profiles::PromptProfiles,
    cli: CliQuery,
) -> anyhow::Result<()> {
    let backend = cli.backend;
    let mode = cli.mode;
    let prompt = cli.prompt;
    let output_json = cli.output_json;
    let include_request = cli.include_request;
    let runtime = Runtime::new()?;
    let query_input = QueryInput {
        prompt,
        images: Vec::new(),
        conversation: Vec::new(),
        active_window_context: None,
    };
    let total_started = Instant::now();
    let prepare_started = Instant::now();
    let prepared_prompt = runtime.block_on(backends::prepare_request(
        &backend,
        &query_input,
        &cfg,
        &prompt_profiles,
        mode,
    ));
    let prepare_ms = prepare_started.elapsed().as_millis();

    let query_started = Instant::now();
    let response = runtime.block_on(backends::query_with_prepared_request(
        &backend,
        &query_input,
        &cfg,
        None,
        prepared_prompt.clone(),
        None,
    ));
    let query_ms = query_started.elapsed().as_millis();
    let total_ms = total_started.elapsed().as_millis();

    if output_json {
        let is_error = response.starts_with("❌");
        let mode = match mode {
            PromptMode::TextAssist => "text_assist",
            PromptMode::GenericQuestion => "generic_question",
        };
        let payload = serde_json::json!({
            "ok": !is_error,
            "backend": backend,
            "mode": mode,
            "request": if include_request { Some(prepared_prompt.clone()) } else { None },
            "prepare_ms": prepare_ms,
            "query_ms": query_ms,
            "total_ms": total_ms,
            "response": response,
        });
        println!("{}", serde_json::to_string(&payload)?);
    } else {
        if include_request {
            println!("Request:\n{prepared_prompt}\n");
        }
        println!("{response}");
    }
    Ok(())
}

fn print_help() {
    println!(
        "armando\n\
         \n\
         Usage:\n\
           armando                       Start desktop UI\n\
           armando --rag-index           Index RAG documents\n\
           armando --ask \"...\"           Ask from CLI (default mode: generic question)\n\
           armando --stdin               Read prompt from stdin and answer in CLI\n\
         \n\
         Options for CLI query mode:\n\
           --backend <name>              Backend override (chatgpt|claude|gemini|ollama)\n\
           --generic                     Force generic question mode (default in CLI)\n\
           --text-assist                 Force text assist mode\n\
           --json                        Print structured JSON output\n\
           --request                     Print prepared request instructions (and include in JSON)\n\
           --help, -h                    Show this help\n\
         \n\
         Tip:\n\
           Use `GENERIC: ...` to force generic-question behavior even when text-assist is selected."
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_backend(default_backend: &str) -> config::Config {
        let mut cfg = config::Config::default();
        cfg.default_backend = default_backend.to_string();
        cfg
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parse_cli_returns_none_without_cli_prompt_flags() {
        let parsed =
            parse_cli_query_args_with_reader(&args(&[]), &cfg_with_backend("gemini"), || {
                Ok(String::new())
            })
            .unwrap();

        assert!(parsed.is_none());
    }

    #[test]
    fn parse_cli_ask_uses_default_backend_and_generic_mode() {
        let parsed = parse_cli_query_args_with_reader(
            &args(&["--ask", "hello world"]),
            &cfg_with_backend("claude"),
            || Ok(String::new()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(parsed.prompt, "hello world");
        assert_eq!(parsed.backend, "claude");
        assert_eq!(parsed.mode, PromptMode::GenericQuestion);
        assert!(!parsed.output_json);
        assert!(!parsed.include_request);
    }

    #[test]
    fn parse_cli_accepts_mode_and_output_flags() {
        let parsed = parse_cli_query_args_with_reader(
            &args(&[
                "--ask",
                "rewrite me",
                "--backend",
                "ollama",
                "--text-assist",
                "--json",
                "--request",
            ]),
            &cfg_with_backend("gemini"),
            || Ok(String::new()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(parsed.backend, "ollama");
        assert_eq!(parsed.mode, PromptMode::TextAssist);
        assert!(parsed.output_json);
        assert!(parsed.include_request);
    }

    #[test]
    fn parse_cli_stdin_reads_prompt_from_reader() {
        let parsed = parse_cli_query_args_with_reader(
            &args(&["--stdin", "--generic"]),
            &cfg_with_backend("gemini"),
            || Ok("from stdin".to_string()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(parsed.prompt, "from stdin");
        assert_eq!(parsed.mode, PromptMode::GenericQuestion);
    }

    #[test]
    fn parse_cli_rejects_ask_and_stdin_together() {
        let err = parse_cli_query_args_with_reader(
            &args(&["--ask", "hello", "--stdin"]),
            &cfg_with_backend("gemini"),
            || Ok("ignored".to_string()),
        )
        .unwrap_err();

        assert!(err.to_string().contains("Use either --ask or --stdin"));
    }

    #[test]
    fn parse_cli_rejects_conflicting_mode_flags() {
        let err = parse_cli_query_args_with_reader(
            &args(&["--ask", "hello", "--generic", "--text-assist"]),
            &cfg_with_backend("gemini"),
            || Ok(String::new()),
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("Use either --generic or --text-assist"));
    }
}

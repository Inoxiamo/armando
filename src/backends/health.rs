use crate::backends::{HealthCheck, HealthLevel};
use crate::config::Config;
use std::process::Command;

pub fn health_checks(config: &Config) -> Vec<HealthCheck> {
    vec![
        health_check_openai(config),
        health_check_claude(config),
        health_check_gemini(config),
        health_check_ollama(config),
    ]
}

pub fn startup_health_checks(config: &Config, selected_backend: &str) -> Vec<HealthCheck> {
    let provider_health_checks = health_checks(config);
    vec![
        startup_config_health_check(config),
        startup_selected_backend_health_check(selected_backend, &provider_health_checks),
        startup_dictation_tools_health_check_for(
            command_exists("ffmpeg"),
            command_exists("arecord"),
        ),
        startup_clipboard_tools_health_check(),
    ]
}

pub fn startup_dictation_tools_health_check_for(
    ffmpeg_available: bool,
    arecord_available: bool,
) -> HealthCheck {
    if ffmpeg_available {
        ok(
            "dictation-tools",
            "Ready",
            "Voice dictation will use `ffmpeg` for microphone capture.".to_string(),
        )
    } else if arecord_available {
        ok(
            "dictation-tools",
            "Ready",
            "Voice dictation will use `arecord` for microphone capture.".to_string(),
        )
    } else {
        warning(
            "dictation-tools",
            "Tools missing",
            "Voice dictation needs `ffmpeg` or `arecord` on the system. Install one of them, then reopen dictation.",
        )
    }
}

pub fn startup_clipboard_tools_health_check_for(
    wl_paste_available: bool,
    xclip_available: bool,
) -> HealthCheck {
    if wl_paste_available {
        ok(
            "clipboard-tools",
            "Ready",
            "Clipboard image paste can use `wl-paste` on Wayland.".to_string(),
        )
    } else if xclip_available {
        ok(
            "clipboard-tools",
            "Ready",
            "Clipboard image paste can use `xclip` on X11.".to_string(),
        )
    } else {
        warning(
            "clipboard-tools",
            "Limited",
            "Clipboard image paste falls back to native clipboard handling on this platform. Install `wl-paste` or `xclip` on Linux if image paste stays unavailable.",
        )
    }
}

fn health_check_openai(config: &Config) -> HealthCheck {
    match &config.chatgpt {
        Some(chatgpt)
            if !chatgpt.api_key.is_empty() && chatgpt.api_key != "YOUR_OPENAI_API_KEY" =>
        {
            if chatgpt.model.trim().is_empty() {
                warning(
                    "chatgpt",
                    "Model missing",
                    "Open Settings, fill `chatgpt.model`, then click Refresh on the model field.",
                )
            } else {
                ok(
                    "chatgpt",
                    "Ready",
                    format!("Configured with model `{model}`.", model = chatgpt.model),
                )
            }
        }
        Some(_) => error(
            "chatgpt",
            "API key missing",
            "Open Settings, add `chatgpt.api_key`, then click Refresh or retry the request.",
        ),
        None => warning(
            "chatgpt",
            "Not configured",
            "Add a `chatgpt` section to the config or switch to another backend in the top bar.",
        ),
    }
}

fn health_check_claude(config: &Config) -> HealthCheck {
    match &config.claude {
        Some(claude)
            if !claude.api_key.is_empty() && claude.api_key != "YOUR_ANTHROPIC_API_KEY" =>
        {
            if claude.model.trim().is_empty() {
                warning(
                    "claude",
                    "Model missing",
                    "Open Settings, fill `claude.model`, then click Refresh on the model field.",
                )
            } else {
                ok(
                    "claude",
                    "Ready",
                    format!("Configured with model `{model}`.", model = claude.model),
                )
            }
        }
        Some(_) => error(
            "claude",
            "API key missing",
            "Open Settings, add `claude.api_key`, then click Refresh or retry the request.",
        ),
        None => warning(
            "claude",
            "Not configured",
            "Add a `claude` section to the config or switch to another backend in the top bar.",
        ),
    }
}

fn health_check_gemini(config: &Config) -> HealthCheck {
    match &config.gemini {
        Some(gemini) if !gemini.api_key.is_empty() && gemini.api_key != "YOUR_GEMINI_API_KEY" => {
            if gemini.model.trim().is_empty() {
                warning(
                    "gemini",
                    "Model missing",
                    "Open Settings, fill `gemini.model`, then click Refresh on the model field.",
                )
            } else {
                ok(
                    "gemini",
                    "Ready",
                    format!("Configured with model `{model}`.", model = gemini.model),
                )
            }
        }
        Some(_) => error(
            "gemini",
            "API key missing",
            "Open Settings, add `gemini.api_key`, then click Refresh or retry the request.",
        ),
        None => warning(
            "gemini",
            "Not configured",
            "Add a `gemini` section to the config or switch to another backend in the top bar.",
        ),
    }
}

fn health_check_ollama(config: &Config) -> HealthCheck {
    match &config.ollama {
        Some(ollama) => {
            if ollama.base_url.trim().is_empty() {
                error(
                    "ollama",
                    "Base URL missing",
                    "Open Settings, fill `ollama.base_url`, then click Refresh to reach the server.",
                )
            } else if ollama.model.trim().is_empty() {
                warning(
                    "ollama",
                    "Model missing",
                    "Open Settings, fill `ollama.model`, then click Refresh to verify the model list.",
                )
            } else {
                ok(
                    "ollama",
                    "Ready",
                    format!(
                        "Configured with `{}` on `{}`.",
                        ollama.model, ollama.base_url
                    ),
                )
            }
        }
        None => warning(
            "ollama",
            "Not configured",
            "Add an `ollama` section to the config or switch to another backend in the top bar.",
        ),
    }
}

fn ok(backend: &str, summary: &str, detail: String) -> HealthCheck {
    HealthCheck {
        backend: backend.to_string(),
        level: HealthLevel::Ok,
        summary: summary.to_string(),
        detail,
    }
}

fn warning(backend: &str, summary: &str, detail: &str) -> HealthCheck {
    HealthCheck {
        backend: backend.to_string(),
        level: HealthLevel::Warning,
        summary: summary.to_string(),
        detail: detail.to_string(),
    }
}

fn error(backend: &str, summary: &str, detail: &str) -> HealthCheck {
    HealthCheck {
        backend: backend.to_string(),
        level: HealthLevel::Error,
        summary: summary.to_string(),
        detail: detail.to_string(),
    }
}

fn startup_config_health_check(config: &Config) -> HealthCheck {
    match &config.loaded_from {
        Some(path) => ok(
            "config",
            "Loaded",
            format!(
                "Loaded from `{}`. If this is not the config you expected, open the right profile and save once.",
                path.display()
            ),
        ),
        None => warning(
            "config",
            "Source missing",
            "No config source was recorded. Save a setting once so the app can remember the active file.",
        ),
    }
}

fn startup_selected_backend_health_check(
    selected_backend: &str,
    provider_health_checks: &[HealthCheck],
) -> HealthCheck {
    provider_health_checks
        .iter()
        .find(|check| check.backend == selected_backend)
        .map(|check| HealthCheck {
            backend: "selected-backend".to_string(),
            level: check.level.clone(),
            summary: check.summary.clone(),
            detail: format!(
                "Selected backend `{selected_backend}`: {}. If this is not what you want, switch it in the top toolbar before sending.",
                check.detail
            ),
        })
        .unwrap_or_else(|| {
            error(
                "selected-backend",
                "Unknown backend",
                &format!(
                    "The selected backend `{selected_backend}` is not supported. Use the top toolbar to choose chatgpt, claude, gemini, or ollama."
                ),
            )
        })
}

fn startup_clipboard_tools_health_check() -> HealthCheck {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        startup_clipboard_tools_health_check_for(
            command_exists("wl-paste"),
            command_exists("xclip"),
        )
    }

    #[cfg(not(all(unix, not(target_os = "macos"))))]
    {
        ok(
            "clipboard-tools",
            "Ready",
            "Clipboard image paste uses the platform clipboard integration.".to_string(),
        )
    }
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

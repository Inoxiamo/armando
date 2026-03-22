#[cfg(all(unix, not(target_os = "macos")))]
use serde_json::Value;
use std::process::Command;

const ACTIVE_WINDOW_CONTEXT_ENV: &str = "ARMANDO_ACTIVE_WINDOW_CONTEXT";

pub fn current_active_window_context() -> Option<String> {
    if let Ok(value) = std::env::var(ACTIVE_WINDOW_CONTEXT_ENV) {
        return normalize_context(&value);
    }

    platform_active_window_context().and_then(|value| normalize_context(&value))
}

fn platform_active_window_context() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        return macos_active_window_context();
    }

    #[cfg(target_os = "windows")]
    {
        return windows_active_window_context();
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if command_exists("hyprctl") {
            if let Some(context) = hyprctl_active_window_context() {
                return Some(context);
            }
        }

        if command_exists("swaymsg") {
            if let Some(context) = sway_active_window_context() {
                return Some(context);
            }
        }

        if command_exists("xdotool") {
            if let Some(context) = xdotool_active_window_context() {
                return Some(context);
            }
        }

        return None;
    }

    #[allow(unreachable_code)]
    None
}

fn normalize_context(value: &str) -> Option<String> {
    let normalized = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    if normalized.is_empty() {
        None
    } else if normalized.chars().count() > 180 {
        let truncated = normalized.chars().take(180).collect::<String>();
        Some(format!("{truncated}..."))
    } else {
        Some(normalized)
    }
}

#[cfg(target_os = "macos")]
fn macos_active_window_context() -> Option<String> {
    let script = r#"
        tell application "System Events"
            if not (exists (first application process whose frontmost is true)) then return ""
            set frontApp to first application process whose frontmost is true
            set appName to name of frontApp
            try
                set windowTitle to name of front window of frontApp
                return appName & " - " & windowTitle
            on error
                return appName
            end try
        end tell
    "#;

    run_command_text(
        "osascript",
        &["-e", script],
        Some(|text| text.trim().to_string()),
    )
    .and_then(|text| normalize_context(&text))
}

#[cfg(target_os = "windows")]
fn windows_active_window_context() -> Option<String> {
    let script = r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Win32 {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
}
"@;
$hWnd = [Win32]::GetForegroundWindow();
if ($hWnd -eq [IntPtr]::Zero) { return "" }
$buffer = New-Object System.Text.StringBuilder 512
[void][Win32]::GetWindowText($hWnd, $buffer, $buffer.Capacity)
$buffer.ToString()
    "#;

    run_command_text(
        powershell_command(),
        &["-NoProfile", "-Command", script],
        Some(|text| text.trim().to_string()),
    )
    .and_then(|text| normalize_context(&text))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn hyprctl_active_window_context() -> Option<String> {
    let output = run_command_output("hyprctl", &["activewindow", "-j"])?;
    let parsed: Value = serde_json::from_str(&output).ok()?;
    let title = parsed
        .get("title")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let class = parsed
        .get("class")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let initial_title = parsed
        .get("initialTitle")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let context = [class, title, initial_title]
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" - ");
    normalize_context(&context)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn sway_active_window_context() -> Option<String> {
    let output = run_command_output("swaymsg", &["-t", "get_tree"])?;
    let parsed: Value = serde_json::from_str(&output).ok()?;
    let focused = find_focused_tree_node(&parsed)?;
    let name = focused
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let app_id = focused
        .get("app_id")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let window_properties = focused
        .get("window_properties")
        .and_then(|value| value.get("class"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let context = [app_id, window_properties, name]
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" - ");
    normalize_context(&context)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn xdotool_active_window_context() -> Option<String> {
    let title = run_command_text(
        "xdotool",
        &["getactivewindow", "getwindowname"],
        Some(|text| text.trim().to_string()),
    )?;
    normalize_context(&title)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn find_focused_tree_node(value: &Value) -> Option<&Value> {
    if value.get("focused").and_then(|value| value.as_bool()) == Some(true) {
        return Some(value);
    }

    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = value.get(key).and_then(|value| value.as_array()) {
            for child in children {
                if let Some(found) = find_focused_tree_node(child) {
                    return Some(found);
                }
            }
        }
    }

    None
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_command_text(
    command: &str,
    args: &[&str],
    transform: Option<fn(String) -> String>,
) -> Option<String> {
    let output = run_command_output(command, args)?;
    Some(match transform {
        Some(transform) => transform(output),
        None => output,
    })
}

#[cfg(target_os = "windows")]
fn powershell_command() -> &'static str {
    if command_exists("pwsh") {
        "pwsh"
    } else {
        "powershell"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_context_collapses_whitespace_and_trims() {
        assert_eq!(
            normalize_context("   App   -   Window   Title   "),
            Some("App - Window Title".to_string())
        );
    }

    #[test]
    fn normalize_context_discards_empty_values() {
        assert_eq!(normalize_context("   \n\t  "), None);
    }

    #[test]
    fn normalize_context_truncates_long_values() {
        let long_value = "a".repeat(181);
        let normalized = normalize_context(&long_value).unwrap();
        assert!(normalized.len() <= 183);
        assert!(normalized.ends_with("..."));
    }

    #[test]
    fn env_override_wins_for_active_window_context() {
        let previous = std::env::var(ACTIVE_WINDOW_CONTEXT_ENV).ok();
        std::env::set_var(ACTIVE_WINDOW_CONTEXT_ENV, "  Firefox   -   docs  ");

        assert_eq!(
            current_active_window_context(),
            Some("Firefox - docs".to_string())
        );

        match previous {
            Some(value) => std::env::set_var(ACTIVE_WINDOW_CONTEXT_ENV, value),
            None => std::env::remove_var(ACTIVE_WINDOW_CONTEXT_ENV),
        }
    }
}

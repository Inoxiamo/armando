use base64::Engine as _;
use eframe::egui;
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::backends::ImageAttachment;

pub(super) struct VoiceRecording {
    child: Child,
    path: PathBuf,
}

#[derive(Debug, Default)]
pub(super) struct PromptPasteAction {
    pub(super) saw_text_paste_event: bool,
    pub(super) image_path_from_paste: Option<PathBuf>,
}

pub(super) fn load_image_attachment_from_path(path: &Path) -> Result<ImageAttachment, String> {
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

pub(super) fn load_image_attachment_from_clipboard() -> Result<ImageAttachment, String> {
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

pub(super) fn classify_prompt_paste_events(events: &[egui::Event]) -> PromptPasteAction {
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

pub(super) fn should_attach_clipboard_image_from_shortcut(
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

pub(super) fn format_size(bytes: usize) -> String {
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

pub(super) fn copy_text_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text.to_string());
    }
}

pub(super) fn begin_voice_recording() -> Result<VoiceRecording, String> {
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

pub(super) fn finish_voice_recording(mut recording: VoiceRecording) -> Result<Vec<u8>, String> {
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

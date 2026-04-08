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

pub(super) fn copy_markdown_rendered_text_to_clipboard(text: &str) {
    copy_text_to_clipboard(&markdown_to_rendered_text(text));
}

fn markdown_to_rendered_text(text: &str) -> String {
    let mut rendered_lines = Vec::new();
    let mut in_code_block = false;

    for line in text.trim_end().lines() {
        let trimmed_start = line.trim_start();

        if trimmed_start.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            rendered_lines.push(line.to_string());
            continue;
        }

        rendered_lines.push(render_markdown_line_for_copy(line));
    }

    rendered_lines.join("\n")
}

fn render_markdown_line_for_copy(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Some((_, content)) = parse_heading_line(trimmed) {
        return strip_inline_markdown(content);
    }

    if let Some(content) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        return format!("• {}", strip_inline_markdown(content.trim()));
    }

    if let Some((index, content)) = parse_numbered_list_line(trimmed) {
        return format!("{index}. {}", strip_inline_markdown(content));
    }

    if let Some(content) = trimmed.strip_prefix("> ") {
        return strip_inline_markdown(content);
    }

    if let Some((text, url)) = parse_single_link_line(trimmed) {
        return format!("{text} ({url})");
    }

    strip_inline_markdown(trimmed)
}

fn parse_heading_line(line: &str) -> Option<(usize, &str)> {
    let hash_count = line.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hash_count) {
        return None;
    }

    let content = line.get(hash_count..)?.trim_start();
    if content.is_empty() {
        return None;
    }

    Some((hash_count, content))
}

fn parse_numbered_list_line(line: &str) -> Option<(usize, &str)> {
    let dot_index = line.find('.')?;
    if dot_index == 0 {
        return None;
    }

    let (prefix, tail) = line.split_at(dot_index);
    if !prefix.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    let content = tail.strip_prefix('.')?.trim_start();
    if content.is_empty() {
        return None;
    }

    Some((prefix.parse().ok()?, content))
}

fn parse_single_link_line(line: &str) -> Option<(&str, &str)> {
    if !(line.starts_with('[') && line.ends_with(')')) {
        return None;
    }

    let text_end = line.find("](")?;
    let text = line.get(1..text_end)?;
    let url = line.get(text_end + 2..line.len() - 1)?;
    if text.is_empty() || url.is_empty() {
        return None;
    }
    Some((text, url))
}

fn strip_inline_markdown(text: &str) -> String {
    let without_bold = strip_paired_delimiter(text, "**");
    let without_code = strip_paired_delimiter(&without_bold, "`");
    strip_paired_delimiter(&without_code, "*")
}

fn strip_paired_delimiter(input: &str, delimiter: &str) -> String {
    let mut output = String::new();
    let mut rest = input;

    while let Some(start) = rest.find(delimiter) {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + delimiter.len()..];

        if let Some(end) = after_start.find(delimiter) {
            output.push_str(&after_start[..end]);
            rest = &after_start[end + delimiter.len()..];
        } else {
            output.push_str(&rest[start..]);
            return output;
        }
    }

    output.push_str(rest);
    output
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

#[cfg(test)]
mod tests {
    use super::markdown_to_rendered_text;

    #[test]
    fn markdown_copy_converts_headers_lists_and_inline_styles() {
        let input = "# Title\n- **Bold** item\n2. `code` item\n> *note*\n";
        let output = markdown_to_rendered_text(input);
        assert_eq!(output, "Title\n• Bold item\n2. code item\nnote");
    }

    #[test]
    fn markdown_copy_keeps_code_block_content_without_fences() {
        let input = "before\n```rust\nlet x = 1;\n```\nafter";
        let output = markdown_to_rendered_text(input);
        assert_eq!(output, "before\nlet x = 1;\nafter");
    }

    #[test]
    fn markdown_copy_expands_single_link_lines() {
        let input = "[Armando](https://github.com/Inoxiamo/armando)";
        let output = markdown_to_rendered_text(input);
        assert_eq!(output, "Armando (https://github.com/Inoxiamo/armando)");
    }
}

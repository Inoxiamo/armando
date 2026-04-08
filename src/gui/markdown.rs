pub(super) enum MarkdownLine<'a> {
    Empty,
    Heading { level: usize, content: &'a str },
    Bullet { content: &'a str },
    Numbered { index: usize, content: &'a str },
    Quote { content: &'a str },
    SingleLink { text: &'a str, url: &'a str },
    Plain { content: &'a str },
}

pub(super) fn is_code_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

pub(super) fn classify_line(line: &str) -> MarkdownLine<'_> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return MarkdownLine::Empty;
    }

    if let Some((level, content)) = parse_heading_line(trimmed) {
        return MarkdownLine::Heading { level, content };
    }

    if let Some(content) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        return MarkdownLine::Bullet {
            content: content.trim(),
        };
    }

    if let Some((index, content)) = parse_numbered_list_line(trimmed) {
        return MarkdownLine::Numbered { index, content };
    }

    if let Some(content) = trimmed.strip_prefix("> ") {
        return MarkdownLine::Quote { content };
    }

    if let Some((text, url)) = parse_single_link_line(trimmed) {
        return MarkdownLine::SingleLink { text, url };
    }

    MarkdownLine::Plain { content: trimmed }
}

pub(super) fn markdown_to_rendered_text(text: &str) -> String {
    let mut rendered_lines = Vec::new();
    let mut in_code_block = false;

    for line in text.trim_end().lines() {
        if is_code_fence(line) {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            rendered_lines.push(line.to_string());
            continue;
        }

        let rendered_line = match classify_line(line) {
            MarkdownLine::Empty => String::new(),
            MarkdownLine::Heading { content, .. } => strip_inline_markdown(content),
            MarkdownLine::Bullet { content } => format!("• {}", strip_inline_markdown(content)),
            MarkdownLine::Numbered { index, content } => {
                format!("{index}. {}", strip_inline_markdown(content))
            }
            MarkdownLine::Quote { content } => strip_inline_markdown(content),
            MarkdownLine::SingleLink { text, url } => format!("{text} ({url})"),
            MarkdownLine::Plain { content } => strip_inline_markdown(content),
        };

        rendered_lines.push(rendered_line);
    }

    rendered_lines.join("\n")
}

pub(super) fn strip_inline_markdown(text: &str) -> String {
    let without_bold = strip_paired_delimiter(text, "**");
    let without_code = strip_paired_delimiter(&without_bold, "`");
    strip_paired_delimiter(&without_code, "*")
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

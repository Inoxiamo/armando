use anyhow::{Context, Result};
use calamine::{open_workbook_auto, Reader};
use std::fs;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub(super) fn build_keyword_match_query(query: &str) -> Option<String> {
    let mut terms = Vec::new();
    for term in query.split(|ch: char| !ch.is_alphanumeric()) {
        let term = term.trim().to_ascii_lowercase();
        if term.is_empty() {
            continue;
        }
        if !terms.contains(&term) {
            terms.push(term);
        }
        if terms.len() >= 12 {
            break;
        }
    }

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" AND "))
    }
}

pub(super) fn is_supported(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "md" | "py" | "java" | "xml" | "txt" | "pdf" | "doc" | "docx" | "xls" | "xlsx"
    )
}

pub(super) fn extract_text(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match ext.as_str() {
        "pdf" => Ok(pdf_extract::extract_text(path)?),
        "docx" => extract_docx_text(path),
        "xls" | "xlsx" => extract_spreadsheet_text(path),
        "doc" => extract_doc_legacy_text(path),
        _ => Ok(fs::read_to_string(path)?),
    }
}

pub(super) fn extract_docx_text(path: &Path) -> Result<String> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut xml_file = archive
        .by_name("word/document.xml")
        .context("DOCX missing word/document.xml")?;
    let mut xml = String::new();
    xml_file.read_to_string(&mut xml)?;
    Ok(strip_xml_tags(&xml))
}

pub(super) fn extract_spreadsheet_text(path: &Path) -> Result<String> {
    let mut workbook = open_workbook_auto(path)?;
    let mut out = String::new();
    for name in workbook.sheet_names().to_owned() {
        if let Ok(range) = workbook.worksheet_range(&name) {
            out.push_str(&format!("Sheet: {name}\n"));
            for row in range.rows() {
                let line = row
                    .iter()
                    .map(|cell| cell.to_string())
                    .collect::<Vec<_>>()
                    .join(" | ");
                if !line.trim().is_empty() {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
            out.push('\n');
        }
    }
    Ok(out)
}

pub(super) fn extract_doc_legacy_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let ascii = bytes
        .iter()
        .map(|byte| {
            if (32..=126).contains(byte) || *byte == b'\n' || *byte == b'\t' || *byte == b' ' {
                *byte as char
            } else {
                ' '
            }
        })
        .collect::<String>();
    let normalized = ascii
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    Ok(normalized)
}

pub(super) fn strip_xml_tags(xml: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in xml.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}

pub(super) fn chunk_text(text: &str, chunk_size: usize) -> Vec<String> {
    let chunk_size = chunk_size.max(128);
    let normalized = text.replace("\r\n", "\n");
    let mut chunks = Vec::new();
    let mut current = String::new();

    for paragraph in normalized.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        if current.len() + trimmed.len() + 2 <= chunk_size {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(trimmed);
            continue;
        }

        if !current.is_empty() {
            chunks.push(current.clone());
            current.clear();
        }

        if trimmed.len() <= chunk_size {
            current.push_str(trimmed);
            continue;
        }

        let chars = trimmed.chars().collect::<Vec<_>>();
        let mut start = 0usize;
        while start < chars.len() {
            let end = (start + chunk_size).min(chars.len());
            let part = chars[start..end]
                .iter()
                .collect::<String>()
                .trim()
                .to_string();
            if !part.is_empty() {
                chunks.push(part);
            }
            start = end;
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current);
    }

    chunks
}

pub(super) fn vector_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

pub(super) fn cosine_similarity_precomputed(a: &[f32], a_norm: f32, b: &[f32], b_norm: f32) -> f32 {
    if a_norm <= f32::EPSILON || b_norm <= f32::EPSILON {
        return 0.0;
    }

    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    dot / (a_norm * b_norm)
}

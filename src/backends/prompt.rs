use crate::backends::{ConversationTurn, PromptMode};
use crate::prompt_profiles::{GenericPromptTag, PromptProfiles};
use crate::rag::RetrievedDocument;
use std::collections::HashMap;

pub(crate) fn prepare_prompt(
    prompt: &str,
    conversation: &[ConversationTurn],
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    has_images: bool,
    active_window_context: Option<&str>,
) -> String {
    prepare_prompt_with_retrieval(
        prompt,
        conversation,
        prompt_profiles,
        mode,
        has_images,
        active_window_context,
        &[],
    )
}

pub(crate) fn prepare_prompt_with_retrieval(
    prompt: &str,
    conversation: &[ConversationTurn],
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    has_images: bool,
    active_window_context: Option<&str>,
    retrieved_docs: &[RetrievedDocument],
) -> String {
    let (expanded_prompt, detected_tags) = match mode {
        PromptMode::TextAssist => expand_tags(
            prompt,
            &prompt_profiles.text_assist_tags,
            &prompt_profiles.language_tags,
        ),
        PromptMode::GenericQuestion => expand_generic_question_prompt(
            prompt,
            &prompt_profiles.generic_question_tags,
            &prompt_profiles.language_tags,
        ),
    };
    let explicit_language = detected_tags
        .iter()
        .find_map(|tag| prompt_profiles.language_tags.get(tag))
        .cloned();

    let mut instructions = match mode {
        PromptMode::TextAssist => vec![
            "Act as a text transformation assistant focused on rewriting, improving, correcting, translating, and adapting text.".to_string(),
            "Treat the user's input as text to transform unless the user explicitly asks for something else.".to_string(),
            "Produce a final version that is ready to copy and use directly in the target context.".to_string(),
            "Preserve the original meaning while improving clarity, tone, grammar, syntax, and readability.".to_string(),
            "Apply style, tone, and formatting instructions directly in the final text without explaining what you changed.".to_string(),
            "Return only the final requested content.".to_string(),
            "Do not add introductions, commentary, explanations, or closing remarks.".to_string(),
            "Do not add quotation marks or special formatting unless explicitly requested.".to_string(),
        ],
        PromptMode::GenericQuestion => vec![
            "Treat the user's text as a general question or request, not as a text-cleanup task."
                .to_string(),
            "Answer the user's request directly and accurately.".to_string(),
            "Keep the response useful, concise, and free of unnecessary preambles.".to_string(),
        ],
    };

    match mode {
        PromptMode::TextAssist => {
            if let Some(language) = explicit_language.as_deref() {
                instructions.push(format!("Write the final output in {language}."));
            } else {
                instructions.push(
                    "Unless an explicit language tag is provided, keep the final output in the same language as the source text that follows the tags.".to_string(),
                );
            }
        }
        PromptMode::GenericQuestion => {
            if let Some(language) = explicit_language.as_deref() {
                instructions.push(format!("Answer in {language}."));
            } else {
                instructions.push(
                    "Unless an explicit language tag is provided, answer in the same language as the user's request.".to_string(),
                );
            }
        }
    }

    if has_images {
        instructions.push(
            "If images or screenshots are attached, use them as visual context to read text, understand interfaces, extract details, and improve the answer."
                .to_string(),
        );
    }

    if let Some(active_window_context) = active_window_context {
        instructions.push(format!(
            "If relevant, use this active window context only as a hint: {active_window_context}."
        ));
    }

    if mode == PromptMode::GenericQuestion {
        let generic_tag_instructions = detected_tags
            .iter()
            .filter_map(|tag| prompt_profiles.generic_question_tags.get(tag))
            .map(|tag| tag.instruction.trim().to_string())
            .filter(|instruction| !instruction.is_empty())
            .collect::<Vec<_>>();

        if generic_tag_instructions.is_empty() {
            instructions
                .push("Use clear Markdown formatting when it helps readability.".to_string());
        } else {
            instructions.extend(generic_tag_instructions);
        }
    }

    if !detected_tags.is_empty() {
        instructions.push(format!(
            "Automatically apply these context instructions: {}.",
            detected_tags.join(", ")
        ));
    }

    if !retrieved_docs.is_empty() {
        instructions.push("Use the retrieved context below as additional grounding instructions when relevant. If retrieved context conflicts with the direct user request, prioritize the user request.".to_string());
        let context = retrieved_docs
            .iter()
            .enumerate()
            .map(|(idx, doc)| {
                format!(
                    "[{}] source={} score={:.4}\n{}",
                    idx + 1,
                    doc.file_path,
                    doc.score,
                    doc.chunk_text.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        instructions.push(format!("Retrieved context:\n{context}"));
    }

    let effective_prompt = if expanded_prompt.trim().is_empty() && has_images {
        "Analyze the attached images or screenshots and respond in a useful, direct, and concrete way."
            .to_string()
    } else {
        expanded_prompt.trim().to_string()
    };

    let conversation_block = if conversation.is_empty() {
        String::new()
    } else {
        let turns = conversation
            .iter()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|turn| {
                format!(
                    "User:\n{}\n\nAssistant:\n{}",
                    turn.user_prompt.trim(),
                    turn.assistant_response.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        format!(
            "\n\nCurrent conversation context:\nUse these previous turns only as context for the ongoing conversation. Do not automatically reinterpret the new request as a text cleanup or transformation task unless the user explicitly asks for that.\n\n{turns}"
        )
    };

    format!(
        "{}{}\n\nUser request:\n{effective_prompt}",
        instructions.join("\n"),
        conversation_block,
    )
}

pub(crate) fn expand_generic_question_prompt(
    prompt: &str,
    generic_tags: &HashMap<String, GenericPromptTag>,
    language_tags: &HashMap<String, String>,
) -> (String, Vec<String>) {
    let Some(colon_idx) = prompt.find(':') else {
        return (prompt.trim().to_string(), Vec::new());
    };

    let header = prompt[..colon_idx].trim();
    let body = prompt[colon_idx + 1..].trim_start();
    let tags = parse_known_tags(header, |tag| {
        generic_tags.contains_key(tag) || language_tags.contains_key(tag)
    });
    if tags.is_empty() {
        return (prompt.trim().to_string(), Vec::new());
    }

    let should_strip_header = !body.is_empty()
        && tags.iter().any(|tag| {
            generic_tags
                .get(tag)
                .is_some_and(|tag_config| tag_config.strip_header)
                || language_tags.contains_key(tag)
        });
    let effective_prompt = if should_strip_header {
        body.trim().to_string()
    } else {
        prompt.trim().to_string()
    };

    (effective_prompt, tags)
}

pub(crate) fn expand_tags(
    prompt: &str,
    text_assist_tags: &HashMap<String, String>,
    language_tags: &HashMap<String, String>,
) -> (String, Vec<String>) {
    let Some(colon_idx) = prompt.find(':') else {
        return (prompt.to_string(), Vec::new());
    };

    let header = prompt[..colon_idx].trim();
    let body = prompt[colon_idx + 1..].trim_start();
    if header.is_empty() || body.is_empty() {
        return (prompt.to_string(), Vec::new());
    }

    let tags = parse_header_tags(header, text_assist_tags, language_tags);
    if tags.is_empty() {
        return (prompt.to_string(), Vec::new());
    }

    let mut instructions = Vec::new();

    for tag in &tags {
        if let Some(custom) = text_assist_tags.get(tag) {
            instructions.push(custom.trim().to_string());
        }
    }

    let expanded = if instructions.is_empty() {
        body.to_string()
    } else {
        format!("{}\n\n{body}", instructions.join("\n"))
    };

    (expanded, tags)
}

pub(crate) fn parse_header_tags(
    header: &str,
    text_assist_tags: &HashMap<String, String>,
    language_tags: &HashMap<String, String>,
) -> Vec<String> {
    parse_known_tags(header, |tag| {
        text_assist_tags.contains_key(tag) || language_tags.contains_key(tag)
    })
}

fn parse_known_tags<F>(header: &str, is_known: F) -> Vec<String>
where
    F: Fn(&str) -> bool,
{
    let tags: Vec<String> = header
        .split(|c: char| c.is_whitespace() || matches!(c, '-' | '+' | ',' | '/' | '|'))
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }

            let normalized = normalize_tag(part);
            let valid = !normalized.is_empty()
                && normalized
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());

            valid.then_some(normalized)
        })
        .collect();

    let all_known = tags.iter().all(|tag| is_known(tag));

    if all_known {
        tags
    } else {
        Vec::new()
    }
}

fn normalize_tag(tag: &str) -> String {
    tag.trim().to_uppercase()
}

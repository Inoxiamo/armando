use crate::config::{Config, RagEngine};
use crate::prompt_profiles::PromptProfiles;
use crate::rag::{RagSystem, RetrievedDocument};
use anyhow::{anyhow, Result};

use super::{
    chatgpt, claude, gemini, langchain, ollama, prepare_prompt, prepare_prompt_with_retrieval,
    PromptMode, QueryInput, ResponseProgressSink,
};

pub(super) async fn build_prepared_prompt(
    backend: &str,
    input: &QueryInput,
    effective_prompt: &str,
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    config: &Config,
) -> String {
    if matches!(config.rag.engine, RagEngine::Langchain) && config.rag.enabled {
        if let Some(prepared_prompt) =
            prepare_prompt_with_langchain_fallback(backend, input, mode, config).await
        {
            return prepared_prompt;
        }
    }

    let retrieved_docs = retrieve_docs(backend, effective_prompt, config).await;
    build_simple_prepared_prompt(
        input,
        effective_prompt,
        prompt_profiles,
        mode,
        &retrieved_docs,
    )
}

pub(super) async fn retrieve_docs(
    backend: &str,
    effective_prompt: &str,
    config: &Config,
) -> Vec<RetrievedDocument> {
    if !config.rag.enabled {
        return Vec::new();
    }

    let rag = RagSystem::new(config.rag.clone());
    match rag.retrieve(backend, effective_prompt, config).await {
        Ok(docs) => docs,
        Err(err) => {
            log::warn!("RAG retrieval failed: {err:#}");
            Vec::new()
        }
    }
}

fn build_simple_prepared_prompt(
    input: &QueryInput,
    effective_prompt: &str,
    prompt_profiles: &PromptProfiles,
    mode: PromptMode,
    retrieved_docs: &[RetrievedDocument],
) -> String {
    if retrieved_docs.is_empty() {
        prepare_prompt(
            effective_prompt,
            &input.conversation,
            prompt_profiles,
            mode,
            !input.images.is_empty(),
            input.active_window_context.as_deref(),
        )
    } else {
        prepare_prompt_with_retrieval(
            effective_prompt,
            &input.conversation,
            prompt_profiles,
            mode,
            !input.images.is_empty(),
            input.active_window_context.as_deref(),
            retrieved_docs,
        )
    }
}

async fn prepare_prompt_with_langchain_fallback(
    backend: &str,
    input: &QueryInput,
    mode: PromptMode,
    config: &Config,
) -> Option<String> {
    let client_config = langchain::LangChainClientConfig {
        base_url: config.rag.langchain_base_url.clone(),
        timeout_ms: config.rag.langchain_timeout_ms,
        retry_count: config.rag.langchain_retry_count,
    };
    let request = langchain::LangChainPrepareRequest {
        prompt: input.prompt.clone(),
        conversation: input
            .conversation
            .iter()
            .map(|turn| langchain::LangChainConversationTurn {
                user_prompt: turn.user_prompt.clone(),
                assistant_response: turn.assistant_response.clone(),
            })
            .collect(),
        prompt_mode: prompt_mode_to_wire(mode).to_string(),
        active_window_context: input.active_window_context.clone(),
        documents_folder: config
            .rag
            .documents_folder
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        query_backend: backend.to_string(),
        query_model: selected_model_for_backend(backend, config),
    };

    match langchain::prepare_prompt_with_retry(&client_config, &request).await {
        Ok(prepared_prompt) => Some(prepared_prompt),
        Err(err) => {
            log::warn!("LangChain prepare failed, falling back to simple RAG: {err:#}");
            None
        }
    }
}

fn prompt_mode_to_wire(mode: PromptMode) -> &'static str {
    match mode {
        PromptMode::TextAssist => "text_assist",
        PromptMode::GenericQuestion => "generic_question",
    }
}

fn selected_model_for_backend(backend: &str, config: &Config) -> Option<String> {
    match backend {
        "chatgpt" => config.chatgpt.as_ref().map(|section| section.model.clone()),
        "claude" => config.claude.as_ref().map(|section| section.model.clone()),
        "gemini" => config.gemini.as_ref().map(|section| section.model.clone()),
        "ollama" => config.ollama.as_ref().map(|section| section.model.clone()),
        _ => None,
    }
    .map(|model| model.trim().to_string())
    .filter(|model| !model.is_empty())
}

pub(super) async fn dispatch_backend_query(
    backend: &str,
    prepared_prompt: &str,
    input: &QueryInput,
    config: &Config,
    progress: Option<ResponseProgressSink>,
) -> Result<String> {
    match backend {
        "chatgpt" => chatgpt::query(prepared_prompt, &input.images, config).await,
        "claude" => claude::query(prepared_prompt, &input.images, config).await,
        "gemini" => gemini::query(prepared_prompt, &input.images, config).await,
        "ollama" => ollama::query(prepared_prompt, &input.images, config, progress).await,
        _ => Err(anyhow!("Unknown backend: {backend}")),
    }
}

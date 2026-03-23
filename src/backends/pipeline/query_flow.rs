use crate::config::Config;
use crate::prompt_profiles::PromptProfiles;
use crate::rag::{RagSystem, RetrievedDocument};
use anyhow::{anyhow, Result};

use super::{
    chatgpt, claude, gemini, ollama, prepare_prompt, prepare_prompt_with_retrieval, PromptMode,
    QueryInput, ResponseProgressSink,
};

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

pub(super) fn build_prepared_prompt(
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

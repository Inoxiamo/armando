# Changelog

## v0.0.2-rc5 - 2026-03-26

- added direct CLI query mode with `--ask`, `--stdin`, backend override, and mode selection flags
- added optional `--json` CLI output for shell scripts and automation
- added automatic switch to generic-question mode via `GENERIC:` prompt tag
- optimized prompt instruction payload to reduce token usage while preserving language behavior
- improved language handling: keep original/request language by default, change only on explicit translation intent
- optimized GitHub Actions with workflow concurrency cancellation and Rust cache steps
- reduced `tokio` dependency footprint by replacing `full` feature set with minimal required features

## v0.0.2-rc4 - 2026-03-23

- added dynamic Ollama model selection with search-as-you-type autocomplete and automatic pull support
- unified Ollama model selection UI by merging text input and available models dropdown
- optimized RAG context size with 8000-character truncation limit to improve prompt performance
- refined "Generic Question" mode by minimizing system instructions for a "pure" prompt experience
- improved Ollama suggestions with an expanded list of popular library models
- temporarily disabled LangChain selection in the RAG UI and added an in-app availability notice

## v0.0.2-rc3 - 2026-03-22

- added startup diagnostics, first-run profile setup, and reusable bundled config templates
- added guided bootstrap install/update support for Linux and macOS, with platform-aware update actions in the app
- expanded prompt preset handling, active-window context hints, and configuration/profile coverage
- improved documentation, media previews, release notes alignment, and validation guidance
- strengthened integration, packaging, and regression coverage for the current release flow

## v0.0.2-rc2 - 2026-03-18

- added in-app GitHub update checking with version comparison and a direct latest-release shortcut from settings
- refined settings panel layout, dropdown styling, footer metadata, and configurable initial window height
- externalized prompt presets into `prompt-tags.yaml` and `generic-prompts.yaml` with built-in defaults and legacy alias fallback
- centralized explicit language-tag handling for prompt presets and generic mode, with broad alias coverage for common languages
- preserved prompt preset files during local and release installs unless overwrite is explicitly requested
- improved toolbar icon rendering, prompt shortcut hints, and removed empty status-card artifacts from the main UI
- hardened GitHub Actions release workflows by decoupling release packaging from optional Sonar configuration

## v0.0.2-rc1 - 2026-03-16

- added containerized CI validation before release packaging
- added integration and functional tests for config loading, resource discovery, packaging, and installers
- aligned Linux, macOS, and Windows install layouts and bundled asset installation
- improved release packaging and repository documentation for public GitHub distribution

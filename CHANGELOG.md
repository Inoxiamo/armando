# Changelog

## Unreleased

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

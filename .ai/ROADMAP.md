# Roadmap

## Milestone 0.1 - Working Core

- [x] Minimal desktop UI in Rust/egui
- [x] Multiple backends: Ollama, ChatGPT, Gemini, Claude
- [x] YAML configuration
- [x] Response rendering in the popup
- [x] Automatic selected-text capture
- [x] Configurable theming system

## Milestone 0.2 - Workflow Reliability

- [ ] Early diagnostics for runtime environment and configured backends
- [ ] Clearer status messages for backend state, config persistence, and failure recovery
- [ ] Guided first-run setup and central directory verification
- [x] Documented manual release packaging and install flow for downloadable bundles

## Milestone 0.3 - UX and Identity

- [x] First visual pass on the `NERV HUD` theme with cards, lighter depth, and clearer buttons
- [x] Persistent history accessible from the popup with filters and quick actions
- [x] Themes externalized into dedicated files and selectable from configuration
- [x] Settings panel with real-time theme and language switching
- [x] UI localization through external files
- [x] History multi-select and deletion
- [x] Coherent dropdown styling across popup, settings, and history
- [x] Local desktop icon and launcher integration for user installs
- [ ] Final refinement pass on the `NERV HUD` theme
- [ ] Additional presets and more complete overrides
- [ ] Response streaming
- [ ] More expressive UI components for state, backend visibility, and quick actions

## Milestone 0.4 - Productivity Features

- [ ] Session-only history distinct from persistent history
- [x] External prompt preset files with startup loading and legacy alias fallback
- [ ] Prompt/snippet templates
- [x] Optional conversational memory
- [ ] Active-window context as a hint
- [ ] Multiple config profiles such as `work`, `personal`, and `beta`

## Milestone 0.5 - Beta Integrations

- [ ] Beta terminal mode with proposed command plus explicit user confirmation
- [ ] Beta MCP client integration for tools and external context
- [ ] Beta tools panel covering `terminal`, `CLI`, `MCP`, and AI backend status
- [ ] Beta tool and command output with logs and execution state
- [ ] Beta sandbox UX with explicit confirmation before sensitive actions

## Milestone 1.0 - Distribution

- [x] Simplified local installation for Linux, macOS, and Windows
- [x] Release packaging
- [x] Downloadable release bundles with packaged assets and install scripts
- [x] Release checksums for published artifacts
- [x] Graphical configuration editor
- [x] Bundled desktop assets included in local install and release packaging
- [ ] Cross-platform stabilization
- [ ] Release notes and distribution changelog

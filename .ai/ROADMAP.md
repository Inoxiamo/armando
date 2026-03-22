# Roadmap

## Current Execution Cadence

Work should be proposed and executed two items at a time.

### Recommended Pair 1

- [x] Startup diagnostics and visible health states for config, tools, and selected backend readiness
- [x] UI regression coverage for editor sizing, viewport constraints, and maximized-window layout

### Recommended Pair 2

- [x] Clearer runtime status and recovery messaging for backend state, persistence, and failure handling
- [x] Backend HTTP fault-injection coverage for queries, model lookup, and malformed/error responses

## Milestone 0.1 - Working Core

- [x] Minimal desktop UI in Rust/egui
- [x] Multiple backends: Ollama, ChatGPT, Gemini, Claude
- [x] YAML configuration
- [x] Response rendering in the popup
- [x] Automatic selected-text capture
- [x] Configurable theming system

## Milestone 0.2 - Workflow Reliability

- [x] Early diagnostics for runtime environment and configured backends
- [x] Clearer status messages for backend state, config persistence, and failure recovery
- [x] Guided first-run setup and central directory verification
- [x] Backend reliability tests for HTTP failures, malformed payloads, and provider model lookup errors
- [x] Documented manual release packaging and install flow for downloadable bundles
- [x] In-app GitHub release check with version comparison and latest-release shortcut
- [x] Guided update footer that distinguishes direct release downloads from Linux/macOS bootstrap updates

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
- [x] Response streaming
- [ ] More expressive UI components for state, backend visibility, and quick actions
- [x] UI regression coverage for editor sizing, viewport constraints, and maximized-window layout

## Milestone 0.4 - Productivity Features

- [x] Session-only history distinct from persistent history
- [x] External prompt preset files with startup loading and legacy alias fallback
- [x] Bundled first-run config templates and reusable config profiles
- [ ] Prompt/snippet templates
- [x] Optional conversational memory
- [x] Active-window context as a hint
- [x] Expanded profile set beyond the bundled `default`, `local`, and `work` presets

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
- [x] Bootstrap install/update flow for Linux and macOS via remote shell script
- [x] Release checksums for published artifacts
- [x] Graphical configuration editor
- [x] Bundled desktop assets included in local install and release packaging
- [x] Cross-platform stabilization
- [ ] Release notes and distribution changelog

## Future Exploration Queue

- [ ] RAG support for retrieving product docs, roadmap notes, and release context before making larger changes
- [ ] MCP integration for safe external tools with explicit user confirmation and clear execution logs
- [ ] Agent workflow for delegated work, parent-agent recaps, and push gating before release-ready changes
- [ ] Review how RAG, MCP, and Agent support fit alongside the existing vibe-coding workflow and manual double-check process

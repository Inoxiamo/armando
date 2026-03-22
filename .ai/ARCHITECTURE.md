# Architecture

## Stack

- Language: Rust
- Desktop UI: `eframe` / `egui`
- Async runtime: `tokio`
- HTTP client: `reqwest`
- Configuration: `serde_yaml`
- Clipboard: `arboard`
- File picker: `rfd`

## Main Components

- `src/main.rs`
  Application entry point. Loads config and theme, configures the native viewport, and opens the popup UI directly.

- `src/gui.rs`
  Owns the main interaction flow: prompt entry, image attachments, clipboard screenshot paste, voice dictation, optional in-memory chat session state, backend selection, response rendering, startup diagnostics, first-run setup, settings, provider model discovery/picking, history, localization, and theme-aware widget styling. The file is still large, but the rendering path is now split into helper functions for the settings side panel, main panel, prompt/response sections, session history, saved history, startup health, first-run setup, and provider subsections. Startup health rows now include short recovery hints, the first-run setup can seed the writable config from a bundled profile template or reveal the writable config destination, settings save failures report the target config path to make recovery clearer, Ollama chunks can stream progressively into the response area, and session history is intentionally kept separate from saved history.

- `src/config.rs`
  Defines the YAML configuration schema, persistence behavior, config path resolution, and bundled template loading, including opt-in local history and debug logging. Config loading now falls back to bundled defaults or built-in defaults on a fresh install so the UI can guide first-run setup instead of aborting, and named template profiles can be loaded into a writable config seed.

- `src/prompt_profiles.rs`
  Loads prompt preset registries from `prompt-tags.yaml` and `generic-prompts.yaml`, merges them with built-in defaults and legacy config aliases, and centralizes explicit language-tag handling.

- `src/history.rs`
  Manages opt-in local history persistence, 7-day retention, selective deletion, and `history.jsonl` lookup.

- `src/theme.rs`
  Loads external theme files, discovers available presets, and resolves the color palette used by the UI.

- `src/i18n.rs`
  Loads external locale files, falls back to English, and discovers available UI languages.

- `src/app_paths.rs`
  Centralizes application paths for config, prompt preset files, themes, locales, history, and bundled config-template discovery.

- `src/backends/*`
  Backend integrations for Ollama, ChatGPT/OpenAI, Gemini, and Claude, including multimodal image payload mapping and provider-specific available-model discovery. The query layer now also accepts an optional progress sink so Ollama can emit incremental response chunks while the other providers keep the single-shot path for now.

- `src/window_context.rs`
  Best-effort active-window context discovery. It supports an env override plus platform-specific probes such as `hyprctl`, `swaymsg`, `xdotool`, `osascript`, and PowerShell, then normalizes the result before it is used only as a backend hint.

- `src/update.rs`
  Fetches the latest GitHub release metadata, normalizes version tags, compares semantic and prerelease version components, and exposes the release URL used by the settings footer update shortcut plus the bootstrap script URL used by the guided Linux/macOS update path.

- `src/logging.rs`
  Handles opt-in local debug logging for request, success, and error events.

- `scripts/install-local.sh`
  Builds the release binary and installs the user-local binary, config, themes, locales, icon, and desktop entry.

- `scripts/package-release.sh`
  Produces distributable archives that bundle the binary together with shipped config, theme, locale, asset, and install-script files.

- `scripts/release-install.sh` and `scripts/release-install.ps1`
  Install a downloaded release bundle into the user profile without requiring the full repository checkout.

- `scripts/bootstrap-release.sh`
  Thin remote-friendly wrapper for Linux and macOS that resolves the release version and platform artifact, downloads the bundle plus checksum from GitHub releases, verifies integrity when checksum tools are available, extracts into a temporary directory, and delegates installation to the bundled `scripts/install.sh`.
  The bootstrap path now has dry-run regression coverage for Linux and macOS target resolution, which helps catch accidental platform drift without requiring network access.

- `assets/`
  Holds desktop-facing assets, currently the project icon and the Linux desktop entry.

## Runtime Flow

1. The main process loads the application config
2. Prompt preset registries are loaded and resolved from built-ins plus optional external YAML files
3. The selected external theme is resolved
4. The selected UI locale is loaded
5. The desktop popup UI opens directly
6. The UI may preload selected text from the OS
7. The user can enrich the prompt with attached images, clipboard screenshots, or voice dictation
8. Startup diagnostics remain available from a collapsed settings section so config source, selected backend readiness, and optional tool availability stay one click away without cluttering the main popup
9. When no config source is recorded yet, the settings section also exposes guided first-run actions to seed the config from a bundled profile template or open the writable config folder
10. The user can optionally keep the current popup session conversational through an in-memory turn buffer
11. Voice dictation records to a temporary audio file and sends it to OpenAI transcription before appending text to the prompt
12. The user submits a prompt
13. The app captures a best-effort active-window context hint before dispatch when the platform can provide one
14. An async task queries the selected backend
15. If debug logging is enabled, the app appends request and outcome events to a local JSONL log file
16. Ollama can stream response chunks into the UI progressively, while the cloud providers still resolve through the final-response path
17. Successful responses are always appended to in-memory session history
18. If local history is enabled, successful responses are also appended to saved history on disk
19. The response is rendered in the popup
20. The user can change theme, language, backend, models, credentials, history, and debug settings from the settings panel with immediate persistence
21. Provider sections stay collapsed by default and can asynchronously load model lists from the configured backend, including the currently editable Ollama base URL
22. The settings panel also surfaces a lightweight credit-availability indicator: `∞` for local Ollama usage and `n/d` for cloud providers whose remaining balance is not exposed through this integration
23. Opening the settings side panel can widen the viewport when needed so the main content area keeps a usable preview width
24. Local installation can register a desktop icon and launcher entry that match the app viewport identity on Linux
25. The settings footer can asynchronously query GitHub for the latest published release and compare it with the local app version
26. When a newer release exists, the user can open the latest downloadable release directly from the settings footer or use the bootstrap shortcut on Linux and macOS
27. Downloaded release bundles can be installed directly through the bundled platform-specific install scripts
28. Linux and macOS users can alternatively bootstrap install or update through a remote wrapper that downloads the correct release bundle and then invokes the bundled installer
29. Toolbar SVG textures are regenerated when `egui` reports a different `pixels_per_point` scale so icon rendering stays sharp after DPI changes
30. Prompt preparation resolves style tags from external preset files, applies language fallback once at the app level instead of repeating that rule inside each preset, and can inject active-window context only as a hint

## UI Structure

- Main popup area:
  backend selector, settings access, generic mode toggle, prompt editor, compact icon-based toolbars for prompt/response actions including a prompt-snippet menu, mouse-resizable prompt/response areas, and response area
- Settings side panel:
  language/theme/backend selectors, history/debug toggles, model/key sections, provider model dropdowns, credit indicators, and persistence feedback
- Session history section:
  read-only in-memory cards with copy/reuse actions and an explicit "kept in memory" note
- Saved history panel:
  backend filter, text filter, batch actions, and reusable history cards backed by `history.jsonl`

The UI styling is theme-driven, but high-friction controls such as dropdowns, buttons, cards, and text areas are normalized in code so they remain visually coherent across themes.

## Theming

The active theme is selected through configuration or the settings panel, while the actual palette lives in external YAML files under `themes/` or an explicit custom path.
This keeps the approach aligned with `egui` while allowing shipped presets and user overrides.

Theme selector fields:

- `name`
- `path`

Main theme file fields:

- `window_fill`
- `panel_fill`
- `panel_fill_soft`
- `panel_fill_raised`
- `accent_color`
- `accent_hover_color`
- `accent_text_color`
- `text_color`
- `weak_text_color`
- `border_color`
- `danger_color`

Current presets:

- `default-dark`
- `nerv-hud`
- `nerv-magi-system`
- `magi`

The UI uses lightweight cards, dedicated scroll areas, and a scrollable side settings panel.
The startup health and first-run setup surfaces now live in a collapsed settings section so the default popup stays minimal while diagnostics and recovery actions remain available on demand.
Prompt and response editors now use explicit allocated heights so drag-resize constraints remain effective even when `egui` would otherwise expand a multiline text edit beyond its desired row count.
Their manual growth is capped to roughly one third of the current viewport height, while startup defaults stay more compact to avoid large empty space under the main content.
Session history stays in memory for the current run only, while saved history is clearly labeled, optionally persisted, and still limited by the 7-day retention policy.
When history is opened, the window updates `MinInnerSize` and `InnerSize` through native viewport commands so the layout expands downward instead of hiding the panel.
Bundled config profiles currently provide a compact onboarding surface, with `default`, `local`, `work`, `personal`, and `beta` templates available from the first-run card and the config loader.
The response area can now accept progressive chunks, but real streaming is currently implemented only on the Ollama path.

## Platform Notes

- Local history is stored in the user data directory, not in the repository
- Themes and locales are resolved both from the repository and from central user directories
- Prompt preset files are resolved both from the repository/bundle root and from central user directories
- On Linux/Wayland, the viewport `app_id` should match the installed `.desktop` entry so launchers and taskbars can resolve the correct application identity and icon
- Active-window context is intentionally best-effort and non-blocking: unsupported platforms or missing tools simply produce no hint
- Voice dictation relies on `ffmpeg` or `arecord` being available on the system path
- The bootstrap release wrapper is intentionally orchestration-only: artifact selection, download, optional checksum verification, extraction, and handoff to the bundled installer

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
  Owns the main interaction flow: prompt entry, image attachments, clipboard screenshot paste, voice dictation, optional in-memory chat session state, backend selection, response rendering, settings, provider model discovery/picking, history, localization, and theme-aware widget styling. The file is still large, but the rendering path is now split into helper functions for the settings side panel, main panel, prompt/response sections, history, and provider subsections.

- `src/config.rs`
  Defines the YAML configuration schema, persistence behavior, and config path resolution, including opt-in local history and debug logging.

- `src/prompt_profiles.rs`
  Loads prompt preset registries from `prompt-tags.yaml` and `generic-prompts.yaml`, merges them with built-in defaults and legacy config aliases, and centralizes explicit language-tag handling.

- `src/history.rs`
  Manages opt-in local history persistence, 7-day retention, selective deletion, and `history.jsonl` lookup.

- `src/theme.rs`
  Loads external theme files, discovers available presets, and resolves the color palette used by the UI.

- `src/i18n.rs`
  Loads external locale files, falls back to English, and discovers available UI languages.

- `src/app_paths.rs`
  Centralizes application paths for config, prompt preset files, themes, locales, and history.

- `src/backends/*`
  Backend integrations for Ollama, ChatGPT/OpenAI, Gemini, and Claude, including multimodal image payload mapping and provider-specific available-model discovery.

- `src/logging.rs`
  Handles opt-in local debug logging for request, success, and error events.

- `scripts/install-local.sh`
  Builds the release binary and installs the user-local binary, config, themes, locales, icon, and desktop entry.

- `scripts/package-release.sh`
  Produces distributable archives that bundle the binary together with shipped config, theme, locale, asset, and install-script files.

- `scripts/release-install.sh` and `scripts/release-install.ps1`
  Install a downloaded release bundle into the user profile without requiring the full repository checkout.

- `assets/`
  Holds desktop-facing assets, currently the project icon and the Linux desktop entry.

## Runtime Flow

1. The main process loads the application config
2. Prompt preset registries are loaded and resolved from built-ins plus optional external YAML files
3. The selected external theme is resolved
4. The selected UI locale is loaded
5. The desktop popup UI opens directly
6. The UI may preload selected text from the OS
6. The user can enrich the prompt with attached images, clipboard screenshots, or voice dictation
7. The user can optionally keep the current popup session conversational through an in-memory turn buffer
8. Voice dictation records to a temporary audio file and sends it to OpenAI transcription before appending text to the prompt
9. The user submits a prompt
10. An async task queries the selected backend
11. If debug logging is enabled, the app appends request and outcome events to a local JSONL log file
12. If local history is enabled, successful responses are appended to local history
13. The response is rendered in the popup
14. The user can change theme, language, backend, models, credentials, history, and debug settings from the settings panel with immediate persistence
15. Provider sections stay collapsed by default and can asynchronously load model lists from the configured backend, including the currently editable Ollama base URL
16. The settings panel also surfaces a lightweight credit-availability indicator: `∞` for local Ollama usage and `n/d` for cloud providers whose remaining balance is not exposed through this integration
17. Opening the settings side panel can widen the viewport when needed so the main content area keeps a usable preview width
18. Local installation can register a desktop icon and launcher entry that match the app viewport identity on Linux
19. Downloaded release bundles can be installed directly through the bundled platform-specific install scripts
20. Toolbar SVG textures are regenerated when `egui` reports a different `pixels_per_point` scale so icon rendering stays sharp after DPI changes
21. Prompt preparation resolves style tags from external preset files and applies language fallback once at the app level instead of repeating that rule inside each preset

## UI Structure

- Main popup area:
  backend selector, settings access, generic mode toggle, prompt editor, compact icon-based toolbars for prompt/response actions, mouse-resizable prompt/response areas, and response area
- Settings side panel:
  language/theme/backend selectors, history/debug toggles, model/key sections, provider model dropdowns, credit indicators, and persistence feedback
- History panel:
  backend filter, text filter, batch actions, and reusable history cards

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
When history is opened, the window updates `MinInnerSize` and `InnerSize` through native viewport commands so the layout expands downward instead of hiding the panel.

## Platform Notes

- Local history is stored in the user data directory, not in the repository
- Themes and locales are resolved both from the repository and from central user directories
- Prompt preset files are resolved both from the repository/bundle root and from central user directories
- On Linux/Wayland, the viewport `app_id` should match the installed `.desktop` entry so launchers and taskbars can resolve the correct application identity and icon
- Voice dictation relies on `ffmpeg` or `arecord` being available on the system path

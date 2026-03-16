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
  Owns the main interaction flow: prompt entry, image attachments, clipboard screenshot paste, voice dictation, optional in-memory chat session state, backend selection, response rendering, settings, history, localization, and theme-aware widget styling.

- `src/config.rs`
  Defines the YAML configuration schema, persistence behavior, and config path resolution, including opt-in debug logging.

- `src/history.rs`
  Manages local history persistence, 7-day retention, selective deletion, and `history.jsonl` lookup.

- `src/theme.rs`
  Loads external theme files, discovers available presets, and resolves the color palette used by the UI.

- `src/i18n.rs`
  Loads external locale files, falls back to English, and discovers available UI languages.

- `src/app_paths.rs`
  Centralizes application paths for config, themes, locales, and history.

- `src/backends/*`
  Backend integrations for Ollama, ChatGPT/OpenAI, Gemini, and Claude, including multimodal image payload mapping.

- `src/logging.rs`
  Handles opt-in local debug logging for request, success, and error events.

- `scripts/install-local.sh`
  Builds the release binary and installs the user-local binary, config, themes, locales, icon, and desktop entry.

- `scripts/package-release.sh`
  Produces distributable archives that bundle the binary together with shipped config, theme, locale, and asset files.

- `assets/`
  Holds desktop-facing assets, currently the project icon and the Linux desktop entry.

## Runtime Flow

1. The main process loads the application config
2. The selected external theme is resolved
3. The selected UI locale is loaded
4. The desktop popup UI opens directly
5. The UI may preload selected text from the OS
6. The user can enrich the prompt with attached images, clipboard screenshots, or voice dictation
7. The user can optionally keep the current popup session conversational through an in-memory turn buffer
8. Voice dictation records to a temporary audio file and sends it to OpenAI transcription before appending text to the prompt
9. The user submits a prompt
10. An async task queries the selected backend
11. If debug logging is enabled, the app appends request and outcome events to a local JSONL log file
12. Successful responses are appended to local history
13. The response is rendered in the popup
14. The user can change theme, language, backend, models, and credentials from the settings panel with immediate persistence
15. Local installation can register a desktop icon and launcher entry that match the app viewport identity on Linux

## UI Structure

- Main popup area:
  backend selector, settings access, generic mode toggle, prompt editor, multimodal input actions, primary actions, and response area
- Settings side panel:
  language/theme/backend selectors, model and key sections, and persistence feedback
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
- On Linux/Wayland, the viewport `app_id` should match the installed `.desktop` entry so launchers and taskbars can resolve the correct application identity and icon
- Voice dictation relies on `ffmpeg` or `arecord` being available on the system path

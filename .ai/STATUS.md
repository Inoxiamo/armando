# Current Status

## Implemented

- Desktop AI popup that can be launched directly from the operating system
- Support for `ollama`, `chatgpt`, `gemini`, and `claude` backends
- Centralized YAML configuration
- Automatic selected-text capture
- Prompt aliases for contextual workflows
- Copy-to-clipboard for responses
- Configurable theming system
- Themes loaded from external YAML files
- UI localization loaded from external YAML files
- Settings panel with real-time persistence for theme, language, backend, models, and credentials
- Build version displayed in the settings panel
- Active `NERV HUD` theme with revised palette
- Compatibility preserved for legacy theme names `nerv-magi-system` and `magi`
- Unified dropdown styling across the main popup, settings panel, and history filter
- Settings gear aligned to the right side of the top toolbar
- Project icon integrated both in the native viewport and in the local desktop installation assets
- Prompt preparation optimized for cleanup, rewriting, translation, and text adaptation
- UI toggle between text-assist mode and generic-question mode
- Optional checkbox to keep an in-memory chat session inside the current popup run
- Image attachments from file picker
- Screenshot/image paste from clipboard
- Voice dictation flow with microphone recording and OpenAI transcription
- Multimodal request forwarding for ChatGPT, Claude, Gemini, and Ollama image-capable models
- Regression-oriented unit tests for prompt preparation, tag parsing, history retention, config loading, and theme loading
- Persistent local history with 7-day retention
- History filters by backend and text
- Fast reuse and copy actions from history entries
- Multi-select and deletion for history entries
- Resizable window with native decorations
- Direct UI startup without a background daemon mode

## Current Behavior Notes

- Theme and language are applied from config at startup and can be changed live
- Backend dropdowns use plain backend names without extra per-provider symbols
- The prompt area can now carry text plus zero or more attached images
- Clipboard screenshots are converted to PNG and attached directly from the UI
- Voice dictation records microphone audio through `ffmpeg` or `arecord` when available, then appends the transcription to the prompt
- When chat session mode is enabled, previous popup turns are reused as conversational context for the next request
- History reloads when the panel is opened and after every successful response
- The history list uses its own scroll region, separate from the rest of the UI
- Opening history expands the window downward through native viewport sizing so the panel remains visible
- History expansion is capped to the usable monitor space to avoid excessive stretching
- Primary actions use explicit labels to avoid ambiguous buttons
- Text on accent buttons is configurable through `accent_text_color` in the theme file
- Local installation places the binary, shipped themes, shipped locales, desktop icon, and `.desktop` launcher entry in the user profile
- Default assistant behavior prefers output that can be reapplied immediately
- In `Generic question` mode, the prompt is treated as a direct question rather than text to rewrite
- In `Generic question` mode, the `CMD` tag requests only the final command; without `CMD`, the answer is formatted as Markdown
- In `Generic question` mode, text-assist aliases and rewrite-oriented prompt expansions are bypassed, even if in-memory chat session mode is enabled

## Known Gaps

- No token-by-token streaming yet
- No startup-time visual diagnostics for backend/config health
- No distinction yet between session-only history and persistent history
- No automated UI tests for layout, scrolling, and popup interactions
- No safe terminal or MCP tool integration yet
- Window icon visibility may still vary by desktop environment even when the app id and desktop entry are aligned
- Voice dictation currently depends on system audio capture tools and an OpenAI API key for transcription
- Image support depends on the selected backend model actually being vision-capable

## Immediate Priorities

- Consolidate the new settings/history UI with richer feedback and better metadata
- Continue refining the `NERV HUD` visual identity
- Add diagnostics and health checks for backend/config issues
- Finish packaging, release notes, and cross-platform stabilization work
- Evaluate a beta tools mode for terminal/CLI/MCP behind explicit confirmation

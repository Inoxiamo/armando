# armando

`armando` is a lightweight cross-platform desktop AI popup built in Rust with `egui`.
It stays close to your workflow so you can ask questions, rewrite text, attach images, and send prompts to Gemini, ChatGPT, Claude, or a local Ollama model.

## Highlights

- Native desktop app in Rust with a compact popup UI
- Multiple backends: `ollama`, `chatgpt`, `gemini`, `claude`
- Text-assist mode and generic-question mode
- External YAML configuration, themes, locales, and prompt preset files
- In-app GitHub release check with version comparison and download shortcut when an update is available
- Optional local history and optional debug logging
- Image attachments, clipboard image paste, and voice dictation
- Release bundles for Linux, macOS, and Windows

## Development Approach

This repository is developed in a vibe-coding style, with fast iterative changes, but every change still goes through automated validation plus a double human check before any push: one review by the person or agent implementing the change, and a second human review at the parent/final check stage.

## Get armando

- Latest release: <https://github.com/Inoxiamo/armando/releases/latest>
- All releases: <https://github.com/Inoxiamo/armando/releases>

Start with [`INSTALL.md`](INSTALL.md) for the release download, OS-specific install steps, config paths, and first-run setup. The first-run card can also seed a new config from one of the bundled templates in `configs/`.

## Configure It

The repository ships defaults under [`configs/`](configs), [`themes/`](themes), [`locales/`](locales), plus [`prompt-tags.yaml`](prompt-tags.yaml) and [`generic-prompts.yaml`](generic-prompts.yaml).
`configs/` now doubles as a small set of reusable config templates for first-run setup, so the initial profile can start from a known-good base instead of an empty file. The bundled set currently includes `default`, `local`, `work`, `personal`, and `beta`.
After installation, `armando` reads configuration from the platform-standard config directory for `armando`, with this structure:

```text
armando/
  prompt-tags.yaml
  generic-prompts.yaml
  configs/
    default.yaml
    local.yaml
    work.yaml
    personal.yaml
    beta.yaml
  themes/
    my-theme.yaml
  locales/
    custom-language.yaml
```

The ChatGPT backend uses OpenAI's Responses API.
For exact install paths and first configuration on each OS, see [`INSTALL.md`](INSTALL.md).

The `ui` section supports visual preferences such as language and initial window height. Example:

```yaml
ui:
  language: "it"
  window_height: 640
```

When the settings panel is open, the footer shows the current app version and, only if a newer GitHub release exists, a small update button that opens the latest downloadable release.

## What The App Can Do

- Rewrite, clean up, summarize, translate, and reformat selected text in a popup without leaving the current workflow
- Switch between `Text assist` for rewriting existing text and `Generic question` for direct prompting
- Use `ollama`, `chatgpt`, `gemini`, or `claude` as the active backend
- Attach images from the file picker, paste screenshots from the clipboard, and use voice dictation
- Keep an in-memory chat session for follow-up turns without forcing persistent history on disk
- Save optional local history, then filter, copy, reuse, multi-select, and delete saved entries
- Change theme, language, backend, models, and credentials from the settings panel with live persistence
- Load provider model lists from backend APIs or a local Ollama instance and pick them from dropdowns
- See startup diagnostics, backend readiness, and recovery hints directly in settings
- Check for newer GitHub releases from the app and open the right update path for the current platform
- Start from bundled config profiles such as `default`, `local`, `work`, `personal`, and `beta`

## Planned

- MCP integration for safe external tools and richer runtime context
- RAG support for retrieving project docs, notes, and release context before larger changes
- Agent-oriented workflow improvements with clearer delegation, recap, and push-gating rules
- Beta tools panel for terminal, CLI, MCP, and backend-status visibility
- Safer command execution flow with explicit confirmation for sensitive actions

## Prompt Presets

Text-assist tags such as `WORK`, `EMAIL`, `FORMAL`, `SHORT`, and `CMD` are loaded from `prompt-tags.yaml`.
Generic-question presets such as `CMD:` are loaded from `generic-prompts.yaml`.
Language selection is handled centrally by the app, with support for explicit tags such as `EN`, `ENG`, `ITALIAN`, `ESP`, `FRA`, `DEU`, `JPN`, and many other common aliases.

Both files are read only at startup. The merge order is:

- built-in defaults
- legacy `aliases` from `configs/default.yaml`
- dedicated prompt files, which win on conflicts

Example `prompt-tags.yaml`:

```yaml
tags:
  WORK: "Keep the output professional and work-oriented."
  EMAIL: "Write or rewrite the text as a professional, clear, and natural email."
  SHORT: "Keep the final result short and concise."
```

Example `generic-prompts.yaml`:

```yaml
tags:
  CMD:
    instruction: "If the requested answer is a shell command or terminal one-liner, return only the final command, with no Markdown, no backticks, and no extra text."
    strip_header: true
```

If no explicit language tag is provided, `armando` keeps the language of the source text in text-assist mode and answers in the user's request language in generic-question mode.

The old `aliases` section in the main config is still supported as a legacy fallback, but new presets should go into the dedicated files.

### Customize Preprompts

The fastest way to adapt `armando` to your workflow is to customize the prompt preset files.

Use `prompt-tags.yaml` for rewrite-oriented tags that modify how an existing text should be transformed:

```yaml
tags:
  FORMAL: "Rewrite the result in a formal and polished tone."
  BUGFIX: "Focus on actionable technical corrections and keep the output precise."
  SHORT: "Keep the final result short and concise."
```

Use `generic-prompts.yaml` for direct-question presets that should inject a reusable instruction block:

```yaml
tags:
  ARCH:
    instruction: "Answer like a pragmatic software architect. Prefer tradeoffs, constraints, and implementation steps."
    strip_header: false
```

This lets you build your own preprompt library for recurring work such as email cleanup, technical rewriting, architecture questions, translation, documentation polish, or shell-command generation.

### Automatic Formatting And Cleanup

In `Text assist` mode, `armando` is designed for quick text transformation rather than raw chat. A typical flow is:

1. paste or auto-capture the source text
2. add one or more tags such as `EMAIL`, `FORMAL`, `SHORT`, or a custom preset
3. send the request and copy the cleaned result back where you need it

This is the best place to show automatic formatting examples in the documentation, because the app can turn rough notes into a cleaner email, normalize bullet points, tighten wording, or adapt tone and language in one step.

![Text Assist Auto Format](docs/media/text-assist-auto-format.png)

### How To Use `CMD`

`CMD` is available in the generic prompt presets and is meant for command-style answers.

Use it in `Generic question` mode when you want only the final shell command or one-liner, without Markdown or explanation. Example prompt:

```text
CMD: find all .log files larger than 100MB under /var/log and sort them by size
```

Expected behavior: the assistant returns just the command, ready to copy into a terminal.

Without `CMD`, the same request can return a normal explanatory answer in Markdown. With `CMD`, the preset pushes the backend toward a direct command-only output.

![Generic Question CMD](docs/media/generic-question-cmd.png)

## Settings And Diagnostics

The settings panel centralizes the most important runtime controls and checks:

- active backend selection
- model selection and provider-specific settings
- language and theme preferences
- startup diagnostics and recovery hints
- update status and runtime configuration feedback

![Settings Diagnostics](docs/media/settings-diagnostics.png)

## Keyboard Shortcuts

System-level shortcuts are supported on Linux, macOS, and Windows.
The release bundle does not yet provide one built-in universal global hotkey that registers itself identically on every OS, so shortcut setup is still delegated to the operating system.

Use [`SHORTCUTS.md`](SHORTCUTS.md) for the practical setup steps.

## Public Docs

- Install and first setup: [`INSTALL.md`](INSTALL.md)
- Keyboard shortcuts: [`SHORTCUTS.md`](SHORTCUTS.md)
- Releases, versions, and artifacts: [`RELEASES.md`](RELEASES.md)
- Visual regression checklist: [`VISUAL_REGRESSION.md`](VISUAL_REGRESSION.md)
- Repository map: [`STRUCTURE.md`](STRUCTURE.md)

## Local Validation

The repository includes containerized validation in [`.github/workflows/ci.yml`](.github/workflows/ci.yml) and tagged release automation in [`.github/workflows/release.yml`](.github/workflows/release.yml).

To run the same Linux container flow locally:

```bash
docker build -f docker/test-runner.Dockerfile -t armando-test-runner .
docker run --rm -v "$(pwd):/workspace" -w /workspace armando-test-runner bash scripts/run-container-tests.sh
```

This produces logs under `target/test-artifacts/` and a Linux bundle under `target/dist/`.

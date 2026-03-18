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

## Get armando

- Latest release: <https://github.com/Inoxiamo/armando/releases/latest>
- All releases: <https://github.com/Inoxiamo/armando/releases>

Start with [`INSTALL.md`](INSTALL.md) for the release download, OS-specific install steps, config paths, and first-run setup.

## Configure It

The repository ships defaults under [`configs/`](configs), [`themes/`](themes), [`locales/`](locales), plus [`prompt-tags.yaml`](prompt-tags.yaml) and [`generic-prompts.yaml`](generic-prompts.yaml).
After installation, `armando` reads configuration from the platform-standard config directory for `armando`, with this structure:

```text
armando/
  prompt-tags.yaml
  generic-prompts.yaml
  configs/
    default.yaml
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

## Prompt Presets

Text-assist tags such as `ITA`, `WORK`, `EMAIL`, `FORMAL`, `SHORT`, and `CMD` are now loaded from `prompt-tags.yaml`.
Generic-question presets such as `CMD:` are loaded from `generic-prompts.yaml`.

Both files are read only at startup. The merge order is:

- built-in defaults
- legacy `aliases` from `configs/default.yaml`
- dedicated prompt files, which win on conflicts

Example `prompt-tags.yaml`:

```yaml
tags:
  ITA: "Traduci o riscrivi il risultato finale in italiano."
  WORK: "Mantieni un contesto professionale e orientato al lavoro."
  EMAIL: "Scrivi o riformula il testo come email professionale, chiara e naturale."
```

Example `generic-prompts.yaml`:

```yaml
tags:
  CMD:
    instruction: "Se la risposta richiesta e un comando o una one-liner da terminale, restituisci solo il comando finale, senza markdown, senza backtick e senza testo aggiuntivo."
    strip_header: true
```

The old `aliases` section in the main config is still supported as a legacy fallback, but new presets should go into the dedicated files.

## Keyboard Shortcuts

System-level shortcuts are supported on Linux, macOS, and Windows.
The release bundle does not yet provide one built-in universal global hotkey that registers itself identically on every OS, so shortcut setup is still delegated to the operating system.

Use [`SHORTCUTS.md`](SHORTCUTS.md) for the practical setup steps.

## Public Docs

- Install and first setup: [`INSTALL.md`](INSTALL.md)
- Keyboard shortcuts: [`SHORTCUTS.md`](SHORTCUTS.md)
- Releases, versions, and artifacts: [`RELEASES.md`](RELEASES.md)
- Repository map: [`STRUCTURE.md`](STRUCTURE.md)

## Local Validation

The repository includes containerized validation in [`.github/workflows/ci.yml`](.github/workflows/ci.yml) and tagged release automation in [`.github/workflows/release.yml`](.github/workflows/release.yml).

To run the same Linux container flow locally:

```bash
docker build -f docker/test-runner.Dockerfile -t armando-test-runner .
docker run --rm -v "$(pwd):/workspace" -w /workspace armando-test-runner bash scripts/run-container-tests.sh
```

This produces logs under `target/test-artifacts/` and a Linux bundle under `target/dist/`.

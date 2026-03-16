# armando

`armando` is a lightweight cross-platform desktop AI popup built in Rust with `egui`.
It opens a minimal assistant window on top of your workflow so you can ask questions, rewrite text, attach images, and send prompts to Gemini, ChatGPT, Claude, or a local Ollama model.

## Highlights

- Native desktop app in Rust with a compact popup UI
- Multiple backends: `ollama`, `chatgpt`, `gemini`, `claude`
- Text-assist mode and generic-question mode
- External YAML configuration, themes, and locales
- Optional local history and optional debug logging
- Image attachments and clipboard image paste
- Voice dictation flow with OpenAI transcription
- Release bundles for Linux, macOS, and Windows

## Quick Start

Build the project:

```bash
cargo build --release
```

Run it locally:

```bash
./target/release/armando
```

Install the local build plus bundled config, themes, locales, and assets:

```bash
./scripts/install-local.sh
```

## Releases And Installation

Every GitHub release bundle includes:

- the compiled binary
- default config
- bundled themes
- bundled locales
- bundled assets
- an install script
- a `.sha256` checksum file

After extracting a release bundle:

On Linux and macOS:

```bash
./scripts/install.sh
```

On Windows:

```powershell
.\scripts\install.ps1
```

Installer targets by platform:

- Linux: config in `~/.config/armando`, data/assets in `~/.local/share/armando`, desktop integration in `~/.local/share/applications` and icon directories
- macOS: config and assets in `~/Library/Application Support/armando`
- Windows: config in `%APPDATA%\armando`, data/assets in `%LOCALAPPDATA%\armando`

## Configuration

The repository ships defaults under [`configs/`](configs), [`themes/`](themes), and [`locales/`](locales).

The app looks for configuration in the platform-standard config directory for `armando`, with this structure:

```text
armando/
  configs/
    default.yaml
  themes/
    my-theme.yaml
  locales/
    custom-language.yaml
```

The ChatGPT backend uses OpenAI's Responses API.

## Development

Project notes are available in the [`.ai/`](.ai) folder:

- [Product](.ai/PRODUCT.md)
- [Architecture](.ai/ARCHITECTURE.md)
- [Status](.ai/STATUS.md)
- [Roadmap](.ai/ROADMAP.md)

## CI And Local Docker Testing

The repository includes:

- [`.github/workflows/ci.yml`](.github/workflows/ci.yml) for containerized validation on pushes and pull requests
- [`.github/workflows/release.yml`](.github/workflows/release.yml) for tagged release builds and artifacts

Run the same Linux container test flow locally:

```bash
docker build -f docker/test-runner.Dockerfile -t armando-test-runner .
docker run --rm -v "$(pwd):/workspace" -w /workspace armando-test-runner bash scripts/run-container-tests.sh
```

The containerized run produces:

- logs under `target/test-artifacts/`
- a Linux release bundle under `target/dist/`
- checksum files for the generated bundle

# armando

`armando` is a lightweight cross-platform desktop AI popup built in Rust with `egui`.
It lets you open a minimal assistant window from your operating system, send a prompt, and get a response from Gemini, ChatGPT, Claude, or a local Ollama model without leaving your current workflow.

## Documentation

Project documentation is centralized in [.ai/README.md](/home/simone/work/test-popup-ai/.ai/README.md).

- Product overview: [.ai/PRODUCT.md](/home/simone/work/test-popup-ai/.ai/PRODUCT.md)
- Architecture: [.ai/ARCHITECTURE.md](/home/simone/work/test-popup-ai/.ai/ARCHITECTURE.md)
- Current status: [.ai/STATUS.md](/home/simone/work/test-popup-ai/.ai/STATUS.md)
- Roadmap: [.ai/ROADMAP.md](/home/simone/work/test-popup-ai/.ai/ROADMAP.md)

## Quick Start

Build the app:

```bash
cargo build --release
```

Run the binary:

```bash
./target/release/armando
```

Install the release binary, default config, themes, and locales locally:

```bash
./scripts/install-local.sh
```

## Downloaded Release Bundles

GitHub Releases bundles now ship with:

- the compiled binary
- default config
- bundled themes
- bundled locales
- desktop assets
- a ready-to-run install script

On Linux and macOS, extract the archive and run:

```bash
./scripts/install.sh
```

On Windows, extract the archive and run:

```powershell
.\scripts\install.ps1
```

Each published archive also includes a `.sha256` checksum file.

## Configuration Layout

Recommended user configuration layout:

```text
~/.config/armando/
  configs/
    default.yaml
  themes/
    my-theme.yaml
  locales/
    custom-language.yaml
```

The repository ships default config, theme presets, and locale files under [configs/](/home/simone/work/test-popup-ai/configs), [themes/](/home/simone/work/test-popup-ai/themes), and [locales/](/home/simone/work/test-popup-ai/locales).

## Backends

- `gemini`
- `chatgpt`
- `claude`
- `ollama`

The ChatGPT backend uses OpenAI's Responses API.

## Release Pipeline

The repository includes [release.yml](/home/simone/work/test-popup-ai/.github/workflows/release.yml), which builds release artifacts for Linux, macOS, and Windows, bundles the shipped assets, generates checksums, and publishes release assets when a tag such as `v1.0.0` is pushed.

## Test Automation

The repository also includes [ci.yml](/home/simone/work/test-popup-ai/.github/workflows/ci.yml), which runs formatting checks plus unit, integration, and functional packaging tests inside a Linux Docker container.

Run the same containerized flow locally with:

```bash
docker build -f docker/test-runner.Dockerfile -t armando-test-runner .
docker run --rm -v "$(pwd):/workspace" -w /workspace armando-test-runner bash scripts/run-container-tests.sh
```

Container test runs publish:

- logs under `target/test-artifacts/`
- a Linux release bundle under `target/dist/`
- checksum files for the generated bundle

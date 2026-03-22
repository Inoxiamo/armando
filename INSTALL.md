# Installation

This page explains how to install `armando` from a GitHub release on Linux, macOS, and Windows.

## Before You Start

`armando` needs at least one configured backend before it can answer prompts:

- `ollama`: local server, no API key required
- `chatgpt`: OpenAI API key
- `gemini`: Google Gemini API key
- `claude`: Anthropic API key

If you want the simplest local setup, start with `ollama`.

## Download A Release

- Latest release: <https://github.com/Inoxiamo/armando/releases/latest>
- All releases: <https://github.com/Inoxiamo/armando/releases>

If you prefer a guided install or update on Linux or macOS, you can use the bootstrap wrapper instead of downloading the archive manually:

```bash
curl -fsSL https://raw.githubusercontent.com/Inoxiamo/armando/master/scripts/bootstrap-release.sh | bash
```

```bash
wget -qO- https://raw.githubusercontent.com/Inoxiamo/armando/master/scripts/bootstrap-release.sh | bash
```

Safer alternative without pipe-to-shell:

```bash
curl -fsSLO https://raw.githubusercontent.com/Inoxiamo/armando/master/scripts/bootstrap-release.sh
bash bootstrap-release.sh
```

The bootstrap wrapper currently supports Linux x86_64 and macOS Apple Silicon. It resolves the matching artifact, downloads the release bundle and checksum, verifies the checksum when supported by the system, extracts the archive into a temporary directory, and then runs the bundled `scripts/install.sh`.

If `armando` is already running, the settings footer also shows the latest release state and, on Linux or macOS, a copyable bootstrap command for the guided update path.
When the desktop app detects a newer GitHub release, the settings footer now also shows a platform-aware suggested update path. On Linux and macOS it points to the bootstrap command; on Windows it points to the release page and packaged installer flow.
On first run, the startup card can also seed the writable config from one of the bundled profiles, so you can begin from a known-good template instead of a blank file.

Pick the archive that matches your OS:

- Linux x86_64: `armando-<version>-x86_64-unknown-linux-gnu.tar.gz`
- macOS Apple Silicon: `armando-<version>-aarch64-apple-darwin.tar.gz`
- Windows x86_64: `armando-<version>-x86_64-pc-windows-msvc.zip`

Every release bundle includes:

- the compiled binary
- default config
- bundled prompt preset files
- bundled themes
- bundled locales
- bundled assets
- an install script
- a `.sha256` checksum file

## Optional: Verify The Download

Linux or macOS:

```bash
shasum -a 256 armando-<version>-<target>.tar.gz
cat armando-<version>-<target>.tar.gz.sha256
```

Windows PowerShell:

```powershell
Get-FileHash .\armando-<version>-x86_64-pc-windows-msvc.zip -Algorithm SHA256
Get-Content .\armando-<version>-x86_64-pc-windows-msvc.zip.sha256
```

The hashes should match.

## Linux

1. Download the Linux `.tar.gz` archive.
2. Extract it.
3. Open a terminal in the extracted folder.
4. Run:

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

Installed paths (default XDG layout):

- binary: `$HOME/.local/bin/armando`
- config: `$HOME/.config/armando/configs/default.yaml`
- prompt tags: `$HOME/.config/armando/prompt-tags.yaml`
- generic prompts: `$HOME/.config/armando/generic-prompts.yaml`
- themes: `$HOME/.config/armando/themes/`
- locales: `$HOME/.config/armando/locales/`
- assets: `$HOME/.local/share/armando/assets/`
- desktop entry: `$HOME/.local/share/applications/armando.desktop`

Example with a full explicit path:

- binary: `/home/<your-user>/.local/bin/armando`
- config: `/home/<your-user>/.config/armando/configs/default.yaml`

Launch it:

```bash
"$HOME/.local/bin/armando"
```

If you set `XDG_CONFIG_HOME` or `XDG_DATA_HOME`, the installer and the app honor those directories instead of the defaults above.
Existing config, prompt preset, theme, and locale files are preserved during install by default. Set `FORCE_CONFIG_INSTALL=1` only if you explicitly want bundled defaults to overwrite them.
The bootstrap wrapper forwards `FORCE_CONFIG_INSTALL=1` if you export it before running the command.

If you prefer the manual path, the in-app update footer still opens the release page so you can download the archive or installer directly.

## macOS

1. Download the macOS `.tar.gz` archive.
2. Extract it.
3. Open `Terminal` in the extracted folder.
4. Run:

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

Installed paths (defaults for a typical user):

- binary: `/Users/<your-user>/.local/bin/armando`
- config: `~/Library/Application Support/armando/configs/default.yaml`
- prompt tags: `~/Library/Application Support/armando/prompt-tags.yaml`
- generic prompts: `~/Library/Application Support/armando/generic-prompts.yaml`
- themes: `~/Library/Application Support/armando/themes/`
- locales: `~/Library/Application Support/armando/locales/`
- assets: `~/Library/Application Support/armando/assets/`

Launch it:

```bash
"/Users/<your-user>/.local/bin/armando"
```

Existing config, prompt preset, theme, and locale files are preserved during install by default. Set `FORCE_CONFIG_INSTALL=1` only if you explicitly want bundled defaults to overwrite them.
The bootstrap wrapper forwards `FORCE_CONFIG_INSTALL=1` if you export it before running the command.

## Windows

1. Download the Windows `.zip` archive.
2. Extract it.
3. Open `PowerShell` in the extracted folder.
4. If PowerShell blocks the script, allow it for the current session:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
```

5. Run:

```powershell
.\scripts\install.ps1
```

Installed paths (defaults for a typical user):

- binary: `C:\Users\<your-user>\AppData\Local\armando\bin\armando.exe`
- config: `C:\Users\<your-user>\AppData\Roaming\armando\configs\default.yaml`
- prompt tags: `C:\Users\<your-user>\AppData\Roaming\armando\prompt-tags.yaml`
- generic prompts: `C:\Users\<your-user>\AppData\Roaming\armando\generic-prompts.yaml`
- themes: `C:\Users\<your-user>\AppData\Roaming\armando\themes\`
- locales: `C:\Users\<your-user>\AppData\Roaming\armando\locales\`
- assets: `C:\Users\<your-user>\AppData\Local\armando\assets\`

Launch it:

```powershell
$env:LOCALAPPDATA\armando\bin\armando.exe
```

## First Setup

Open the default config file for your OS and configure at least one backend:

- `ollama`
- `chatgpt`
- `gemini`
- `claude`

Config file locations:

- Linux: `$HOME/.config/armando/configs/default.yaml`
- macOS: `~/Library/Application Support/armando/configs/default.yaml`
- Windows: `%APPDATA%\armando\configs\default.yaml`

Prompt preset file locations:

- Linux: `$HOME/.config/armando/prompt-tags.yaml` and `$HOME/.config/armando/generic-prompts.yaml`
- macOS: `~/Library/Application Support/armando/prompt-tags.yaml` and `~/Library/Application Support/armando/generic-prompts.yaml`
- Windows: `%APPDATA%\armando\prompt-tags.yaml` and `%APPDATA%\armando\generic-prompts.yaml`

On Linux you can open the installed config with:

```bash
xdg-open "$HOME/.config/armando/configs/default.yaml"
```

or edit it directly:

```bash
nano "$HOME/.config/armando/configs/default.yaml"
```

### What To Edit In The Config

The shipped config already contains the provider blocks. You usually only need to replace the placeholder value for the backend you want:

```yaml
gemini:
  api_key: "YOUR_GEMINI_API_KEY"

chatgpt:
  api_key: "YOUR_OPENAI_API_KEY"

claude:
  api_key: "YOUR_ANTHROPIC_API_KEY"

ollama:
  base_url: "http://localhost:11434"
```

Also set `default_backend` to the backend you actually configured.

### Prompt Presets

`armando` now reads prompt presets from two dedicated files loaded at startup:

- `prompt-tags.yaml`: text-assist tags such as `ITA`, `WORK`, `EMAIL`, `SHORT`
- `generic-prompts.yaml`: generic-question presets such as `CMD:`

Example:

```yaml
tags:
  ITA: "Traduci o riscrivi il risultato finale in italiano."
  WORK: "Mantieni un contesto professionale e orientato al lavoro."
```

```yaml
tags:
  CMD:
    instruction: "Se la risposta richiesta e un comando o una one-liner da terminale, restituisci solo il comando finale, senza markdown, senza backtick e senza testo aggiuntivo."
    strip_header: true
```

The legacy `aliases` section in the main config is still supported, but the dedicated files are the recommended path for new custom presets.

### How To Generate And Use API Keys

#### OpenAI (`chatgpt`)

1. Sign in to your OpenAI developer account.
2. Open the API keys page: <https://platform.openai.com/api-keys>.
3. Generate a new secret API key.
4. Copy it immediately and paste it into `chatgpt.api_key`.

Example:

```yaml
default_backend: "chatgpt"

chatgpt:
  api_key: "sk-..."
  model: "gpt-4o-mini"
```

#### Google Gemini (`gemini`)

1. Open Google AI Studio: <https://aistudio.google.com/apikey>.
2. Create an API key for the project you want to use with Gemini.
3. Paste it into `gemini.api_key`.

Example:

```yaml
default_backend: "gemini"

gemini:
  api_key: "AIza..."
  model: "gemini-1.5-flash"
```

#### Anthropic Claude (`claude`)

1. Sign in to the Anthropic Console: <https://console.anthropic.com/settings/keys>.
2. Generate a new API key for your workspace.
3. Paste it into `claude.api_key`.

Example:

```yaml
default_backend: "claude"

claude:
  api_key: "sk-ant-..."
  model: "claude-3-5-sonnet-latest"
```

#### Ollama (`ollama`)

Ollama does not need an API key when it runs locally.

1. Install Ollama on the same machine.
2. Start the local Ollama server.
3. Pull the model you want to use.
4. Keep `ollama.base_url` pointed at that local server.

Typical local setup:

```bash
ollama serve
ollama pull gemma3:1b
```

Example config:

```yaml
default_backend: "ollama"

ollama:
  base_url: "http://localhost:11434"
  model: "gemma3:1b"
```

If you use Ollama, make sure the local server is already running before launching `armando`.

### Common First-Run Mistakes

- The placeholder string such as `YOUR_OPENAI_API_KEY` was not replaced.
- `default_backend` still points to a different provider than the one you configured.
- The key was copied incompletely.
- The selected model is not available for that provider account.
- Ollama is installed, but the server is not running on `http://localhost:11434`.

### First Launch Checklist

1. Launch the installed binary (`/home/<your-user>/.local/bin/armando` on Linux).
2. Open Settings and pick the backend you configured.
3. Confirm the API key or Ollama URL is populated.
4. Send a simple prompt like `Hello`.
5. If the request fails, re-open Settings and verify the key, model, or Ollama URL.

## Next Steps

- For keyboard shortcut setup, see [`SHORTCUTS.md`](SHORTCUTS.md).
- For release naming and versioning, see [`RELEASES.md`](RELEASES.md).
- For the repository layout, see [`STRUCTURE.md`](STRUCTURE.md).

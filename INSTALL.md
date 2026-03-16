# Installation

This page explains how to install `armando` from a GitHub release on Linux, macOS, and Windows.

## Download A Release

- Latest release: <https://github.com/Inoxiamo/armando/releases/latest>
- All releases: <https://github.com/Inoxiamo/armando/releases>

Pick the archive that matches your OS:

- Linux x86_64: `armando-<version>-x86_64-unknown-linux-gnu.tar.gz`
- macOS Apple Silicon: `armando-<version>-aarch64-apple-darwin.tar.gz`
- Windows x86_64: `armando-<version>-x86_64-pc-windows-msvc.zip`

Every release bundle includes:

- the compiled binary
- default config
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

Installed paths:

- binary: `~/.local/bin/armando`
- config: `~/.config/armando/configs/default.yaml`
- themes: `~/.config/armando/themes/`
- locales: `~/.config/armando/locales/`
- assets: `~/.local/share/armando/assets/`
- desktop entry: `~/.local/share/applications/armando.desktop`

Launch it:

```bash
~/.local/bin/armando
```

## macOS

1. Download the macOS `.tar.gz` archive.
2. Extract it.
3. Open `Terminal` in the extracted folder.
4. Run:

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

Installed paths:

- binary: `~/.local/bin/armando`
- config: `~/Library/Application Support/armando/configs/default.yaml`
- themes: `~/Library/Application Support/armando/themes/`
- locales: `~/Library/Application Support/armando/locales/`
- assets: `~/Library/Application Support/armando/assets/`

Launch it:

```bash
~/.local/bin/armando
```

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

Installed paths:

- binary: `%LOCALAPPDATA%\armando\bin\armando.exe`
- config: `%APPDATA%\armando\configs\default.yaml`
- themes: `%APPDATA%\armando\themes\`
- locales: `%APPDATA%\armando\locales\`
- assets: `%LOCALAPPDATA%\armando\assets\`

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

- Linux: `~/.config/armando/configs/default.yaml`
- macOS: `~/Library/Application Support/armando/configs/default.yaml`
- Windows: `%APPDATA%\armando\configs\default.yaml`

If you use Ollama, make sure the local Ollama server is already running before launching `armando`.

## Next Steps

- For keyboard shortcut setup, see [`SHORTCUTS.md`](SHORTCUTS.md).
- For release naming and versioning, see [`RELEASES.md`](RELEASES.md).
- For the repository layout, see [`STRUCTURE.md`](STRUCTURE.md).

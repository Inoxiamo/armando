# AI Popup Assistant (Rust Edition)

A **lightweight cross-platform utility** that opens a minimal AI chat popup anywhere on your screen with a global keyboard shortcut.

Press **`Ctrl+Space`** → type your prompt → get a response from **Gemini**, **ChatGPT**, or a local **Ollama** model — without leaving your current window.

This project has been rewritten in Rust using `egui` and `global-hotkey` to create a single, efficient, cross-platform executable without the need for Python or system dependencies.

---

## Quick Start

### 1. Download or Build

**To build from source:**
Ensure you have Rust installed.
```bash
cargo build --release
```
The compiled executable will be located at `target/release/test-popup-ai` (or `.exe` on Windows).

### 2. Configure
Create a `config.yaml` file in the same directory as the executable (or in your user config directory, e.g. `~/.config/test-popup-ai/config.yaml`), and fill in your API keys:

```yaml
hotkey: "<ctrl>+<space>"     # Global hotkey to trigger the popup
default_backend: "ollama"    # gemini | chatgpt | ollama

# Advanced Features
auto_read_selection: true               # Auto-fills the text field with highlighted text
paste_response_shortcut: "<ctrl>+<enter>" # Pastes the response automatically into your active window and closes the popup

theme:
  name: "nerv-hud"            # loads ./themes/nerv-hud.yaml
  # path: "/absolute/path/to/custom-theme.yaml"

gemini:
  api_key: "YOUR_GEMINI_API_KEY"

chatgpt:
  api_key: "YOUR_OPENAI_API_KEY"

ollama:
  base_url: "http://localhost:11434"
  model: "gemma3:1b"
```

### API Keys

For Gemini:
1. Open https://aistudio.google.com/app/apikey
2. Create an API key
3. Paste it into `gemini.api_key` in `config.yaml`
4. Select `default_backend: "gemini"` if you want Gemini by default

For ChatGPT / OpenAI:
1. Open https://platform.openai.com/api-keys
2. Create a new API key
3. Paste it into `chatgpt.api_key` in `config.yaml`
4. Select `default_backend: "chatgpt"` if you want ChatGPT by default

### 3. Setup the OS Shortcut (Linux/GNOME)
Wayland security prevents background apps from intercepting keystrokes globally. To bypass this, we map the GUI strictly to your OS shortcut settings.

**Option A (Automatic via Script):**
```bash
./setup_hotkey.sh
```
This automatically configures GNOME to open your application when you press `Ctrl+Space`. 
*To change this combination*, you can either edit the `HOTKEY="<Primary>space"` variable inside the `setup_hotkey.sh` file before running it (for example, `<Super>p` for Win+P, or `<Primary><Alt>a` for Ctrl+Alt+A), or manage it manually.

**Option B (Manual via Settings):**
1. Open your OS **Settings > Keyboard > Keyboard Shortcuts > View and Customize Shortcuts > Custom Shortcuts**.
2. Add a new shortcut named "AI Popup".
3. Set the Command to your absolute executable path with the UI flag: `/absolute/path/to/test-popup-ai --ui`
4. Set the Shortcut to your preferred key combination.

If you are on Windows, macOS, or Linux X11, you can simply run the app daemon in the background (`./test-popup-ai`) and change the `hotkey` parameter directly in `config.yaml`.

### 4. Run
If you rely on the daemon:
```bash
./target/release/test-popup-ai
```

If you are on Linux Wayland and want `Ctrl+Enter` to apply the response into the previously focused window, install:

```bash
sudo apt install wl-clipboard wtype
```

If you are on Linux X11, install:

```bash
sudo apt install xclip xdotool
```

Then rebuild if needed:

```bash
cargo build --release
```

---

## Usage Mode

| Action | Shortcut / Click |
|---|---|
| Open popup | `Ctrl+Space` (or your configured hotkey) |
| Send prompt | `Enter` |
| Insert newline | `Shift+Enter` |
| Close popup | `Escape` |
| Copy response | `Copy` button |
| Apply response | `Ctrl+Enter` |

Switch between AI backends using the dropdown menu in the popup header.

The assistant is optimized to return only the final requested text, without intros, notes, or extra framing.

Every successful response is also saved to a local history log that is kept for 7 days. You can open it from the popup with the `History` button, filter by backend or text, reuse or copy previous results, and open the raw history file with `Open History File`.

You can also prepend routing/tone tags before `:` and combine them freely:

```text
GMAIL ENG: scrivi una mail per spostare la riunione a domani
SLACK ITA SHORT: avvisa il team che il deploy e finito
WHATSAPP ENG CASUAL: chiedi se ci vediamo alle 18
```

Supported built-in tags include:
- `GMAIL`, `EMAIL`, `MAIL`
- `SLACK`
- `WHATSAPP`
- `ITA`
- `ENG`
- `FORMAL`
- `CASUAL`
- `WORK`
- `SHORT`
- `LONG`

Custom aliases defined in `config.yaml` are still supported and can be combined with the built-in tags.

The popup theme is selected from `config.yaml`, but the full color palette now lives in external YAML files under `themes/`. The app currently ships with `default-dark`, `nerv-hud`, `nerv-magi-system` and `magi`, and you can also point `theme.path` to a custom theme file.

`nerv-hud` is the default visual preset.

---

## Cross-Platform Support

Because this application does not rely on GTK or Python anymore, it compiles natively for:
- **Windows**: `cargo build --release` (produces `test-popup-ai.exe`)
- **macOS**: `cargo build --release` (produces `test-popup-ai`)
- **Linux**: `cargo build --release` (produces `test-popup-ai` and uses X11/Wayland natively via egui)

No external system dependencies are required outside of standard compilation toolchains.

---

## Backends

| Backend | Requires |
|---|---|
| **Gemini** | `GEMINI_API_KEY` in config.yaml — [get one here](https://aistudio.google.com/app/apikey) |
| **ChatGPT** | `OPENAI_API_KEY` in config.yaml — [get one here](https://platform.openai.com/api-keys) |
| **Ollama** | Ollama running locally (`ollama serve`) — [install](https://ollama.com) |

The ChatGPT backend uses OpenAI's Responses API. If the key, model, or account permissions are wrong, the popup now shows the API error message directly instead of a truncated `{`.

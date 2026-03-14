# Architecture

## Stack

- Linguaggio: Rust
- UI desktop: `eframe` / `egui`
- Async runtime: `tokio`
- HTTP client: `reqwest`
- Configurazione: `serde_yaml`
- Clipboard: `arboard`
- Hotkey globali: `global-hotkey`

## Main Components

- `src/main.rs`
  Avvio applicazione. Sceglie tra modalita daemon e modalita UI.

- `src/daemon.rs`
  Registra l'hotkey globale e apre la UI su trigger.

- `src/gui.rs`
  Gestisce stato della finestra, invio prompt, rendering risposta, pannello history, shortcut locali, tema e auto-apply.

- `src/config.rs`
  Definisce il formato della configurazione YAML e il lookup dei path di caricamento.

- `src/history.rs`
  Gestisce persistenza locale della history, retention a 7 giorni e lookup del file `history.jsonl`.

- `src/theme.rs`
  Carica temi esterni da file YAML e risolve la palette colore usata dalla UI.

- `src/backends/*`
  Implementazioni dei backend AI supportati: Ollama, ChatGPT/OpenAI, Gemini.

## Runtime Flow

1. Il processo principale carica `config.yaml`
2. In modalita daemon registra l'hotkey globale
3. Alla pressione dell'hotkey apre una nuova UI (`--ui`)
4. La UI opzionalmente legge il testo selezionato
5. L'utente invia il prompt
6. Una task async interroga il backend selezionato
7. In caso di successo la risposta viene salvata nella history locale
8. La risposta viene mostrata nel popup
9. Con `Ctrl+Enter` la risposta viene applicata oppure copiata negli appunti con feedback esplicito

## Theming

Il tema e selezionato via configurazione YAML, ma la palette vera e propria vive in file esterni sotto `themes/` oppure in un path custom specificato in config.
L'approccio resta coerente con `egui`, che espone uno style system nativo.

Campi attuali:

- `name`
- `path`

Preset attuali:

- `default-dark`
- `nerv-hud`

La UI usa card con bordo, profondita leggera e aree scroll dedicate per separare risposta e history senza conflitti di input.
Quando la history viene aperta, la finestra aggiorna `MinInnerSize` e `InnerSize` tramite viewport command nativi per espandersi verso il basso.

## Platform Notes

- Wayland: per auto-apply affidabile servono `wl-copy` e `wtype`
- X11: per auto-apply affidabile servono `xclip` e `xdotool`
- In assenza dell'iniezione tastiera, il sistema effettua almeno la copia negli appunti
- La history locale viene salvata nella directory dati utente dell'applicazione e non nel repository

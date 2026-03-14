# Roadmap

## Milestone 0.1 - Working Core

- [x] UI desktop minimale in Rust/egui
- [x] Backend multipli: Ollama, ChatGPT, Gemini
- [x] Hotkey globale e apertura popup
- [x] Configurazione YAML
- [x] Visualizzazione risposta nel popup
- [x] Modalita `--ui` per ambienti con limiti Wayland
- [x] Lettura automatica del testo selezionato
- [x] Shortcut di apply/copy della risposta
- [x] Sistema di theming configurabile

## Milestone 0.2 - Workflow Reliability

- [ ] Verifica preventiva delle dipendenze OS per auto-apply
- [ ] Messaggi di stato piu chiari su apply, copy e fallback
- [ ] Setup guidato per Wayland / X11
- [ ] Script di bootstrap per installare o verificare le utility richieste
- [ ] Test manuali documentati per i flussi principali

## Milestone 0.3 - UX and Identity

- [x] Prima revisione visuale del tema `NERV HUD` con card, ombre leggere e bottoni piu leggibili
- [x] History persistente consultabile dal popup con filtri e azioni rapide
- [ ] Refinement visuale finale del tema `NERV HUD`
- [ ] Preset aggiuntivi e override piu completi
- [ ] Streaming della risposta
- [ ] Componenti UI piu espressivi per stato, backend e azioni rapide

## Milestone 0.4 - Productivity Features

- [ ] Cronologia di sessione distinta dalla history persistente
- [ ] Template prompt/snippet
- [ ] Memoria conversazionale opzionale
- [ ] Contesto finestra attiva come hint

## Milestone 1.0 - Distribution

- [ ] Installazione semplificata per Linux/macOS/Windows
- [ ] Packaging di release
- [ ] Editor grafico della configurazione
- [ ] Stabilizzazione cross-platform

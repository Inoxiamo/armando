# Current Status

## Implemented

- Popup AI desktop richiamabile da hotkey
- Supporto a backend `ollama`, `chatgpt`, `gemini`
- Configurazione YAML centralizzata
- Lettura automatica del testo selezionato
- Alias per prompt contestuali
- Copia risposta negli appunti
- Shortcut `Ctrl+Enter` per apply/copy-and-close
- Sistema di temi configurabile
- Temi caricati da file YAML esterni
- Tema di default `NERV HUD`
- Compatibilita mantenuta con i nomi tema storici `nerv-magi-system` e `magi`
- Preprompt orientato principalmente a pulizia, ottimizzazione, traduzione e adattamento del testo
- Toggle UI per passare da modalita text-assist a domanda generica
- Test unitari di non regressione su prompt preparation, tag parsing e retention della history
- History locale persistente con retention a 7 giorni
- Filtri history per backend e testo
- Riutilizzo e copia rapida delle entry dalla history
- Finestra ridimensionabile con decorazioni native

## Current Behavior Notes

- `Ctrl+Enter` esegue auto-apply se il sistema dispone delle utility richieste
- Se l'iniezione tastiera non e disponibile, la risposta viene comunque copiata e la UI informa l'utente
- Il tema viene applicato da `config.yaml` all'apertura della UI
- La history viene ricaricata all'apertura del pannello e dopo ogni risposta completata con successo
- La lista history usa un'area scroll dedicata separata dallo scroll del resto della finestra
- L'apertura della history espande nativamente la finestra verso il basso per evitare che il pannello resti nascosto
- L'espansione della history e limitata alla dimensione utile del monitor per evitare stretch eccessivi
- Le azioni principali sono etichettate in modo esplicito per evitare pulsanti ambigui
- Il testo sui pulsanti accentati e configurabile via `accent_text_color` nel file tema
- Il comportamento di default dell'assistente privilegia output testuali pronti da riapplicare subito
- In modalita `Generic question`, il prompt viene trattato come domanda diretta e non come testo da formattare
- In modalita `Generic question`, il tag `CMD` richiede solo il comando finale; senza `CMD`, la risposta viene formattata in Markdown

## Known Gaps

- Nessuna GUI per modificare la configurazione
- Nessun supporto a streaming token-by-token
- Nessuna diagnostica visuale per dipendenze OS mancanti prima del tentativo di apply
- Nessuna distinzione tra history di sessione singola e history persistente
- Mancano test automatici UI per layout, scroll e interazioni del popup

## Immediate Priorities

- Rendere l'auto-apply ancora piu osservabile e affidabile
- Consolidare la nuova UI history con feedback piu ricchi e metadati migliori
- Migliorare ulteriormente l'identita visiva `NERV HUD`
- Definire un setup UX piu chiaro per Wayland/X11
- Consolidare la documentazione per installazione e troubleshooting

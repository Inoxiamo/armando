# Rust Code Reading Guide

Questa guida e pensata per leggere questo progetto Rust se arrivi da Java o Python.
Scritta in modo volutamente diretto e semplice.
Puoi cancellarla quando vuoi.

## 1. Mappa mentale veloce

Pensa al progetto cosi:

- `main.rs`
  E il tuo `main()` Java o il file `app.py` che decide come parte il programma.

- `config.rs`
  E l'equivalente di una classe/config loader che deserializza YAML in oggetti.

- `daemon.rs`
  E un processo in loop che ascolta l'hotkey globale e lancia la UI.

- `gui.rs`
  E il file piu vicino a un controller + view state. Dentro c'e lo stato della finestra e il rendering `egui`.

- `backends/`
  E il layer servizi / client HTTP verso OpenAI, Gemini, Ollama.

- `history.rs`
  E un piccolo repository locale che salva/rilegge cronologia da file.

- `theme.rs`
  E il loader della palette esterna dai file YAML dei temi.

## 2. Come leggere Rust senza bloccarti

Regola pratica:

- Leggi prima le `struct`
- Poi cerca gli `impl`
- Poi guarda le funzioni pubbliche `pub fn`
- Solo dopo entra nei dettagli delle helper function private

Traduzione mentale:

- `struct`
  Simile a una classe dati Java o a una dataclass Python

- `impl`
  Blocco metodi di una struct

- `enum`
  Simile a un enum Java, ma piu potente

- `Option<T>`
  O c'e un valore (`Some`) o non c'e (`None`)

- `Result<T, E>`
  O hai successo (`Ok`) o errore (`Err`)

- `?`
  Propaga l'errore al chiamante, come un return anticipato elegante

- `clone()`
  Duplica un valore quando Rust non permette di muoverlo/consumarlo

## 3. Come parte l'app

Apri [main.rs](/home/simone/work/test-popup-ai/src/main.rs).

Flusso:

1. carica config
2. carica il tema esterno
3. se trova `--ui`, apre la finestra
4. altrimenti parte in modalita daemon

Da leggere in ordine:

- `main()`
- `run_ui()`

## 4. Come leggere la config

Apri [config.rs](/home/simone/work/test-popup-ai/src/config.rs).

Cose importanti:

- `Config` e la struct principale
- `serde` fa il parsing YAML -> struct Rust
- `#[serde(default = ...)]` significa: se il campo manca nel file, usa questo default
- `Config::load()` prova piu path finche trova `config.yaml`

Se vieni da Java:

- pensa a `Config` come a un POJO popolato automaticamente da Jackson

Se vieni da Python:

- pensa a `yaml.safe_load(...)` + validazione + mapping su oggetto

## 5. Come leggere il tema

Apri [theme.rs](/home/simone/work/test-popup-ai/src/theme.rs).

Concetti:

- `ThemeDefinition`
  E il contenuto raw del file YAML

- `ResolvedTheme`
  E la versione pronta all'uso per `egui`, con colori gia convertiti

- `load_theme()`
  Cerca il file tema e lo carica

- `parse_hex_color()`
  Converte stringhe tipo `#99FF33` in `Color32`

Leggilo come:

file YAML -> oggetto Rust -> colori pronti per UI

## 6. Come leggere il flusso prompt -> backend

Apri [src/backends/mod.rs](/home/simone/work/test-popup-ai/src/backends/mod.rs).

Ordine consigliato:

1. `PromptMode`
2. `query(...)`
3. `prepare_prompt(...)`
4. `expand_tags(...)`
5. `parse_header_tags(...)`
6. `built_in_tag_instruction(...)`

Questo file fa tre cose:

- decide il backend da chiamare
- costruisce il preprompt
- espande i tag tipo `GMAIL`, `ITA`, `CMD`

Idea chiave:

- `TextAssist`
  Modalita editor del testo

- `GenericQuestion`
  Modalita domanda generica

- `CMD`
  In modalita generica prova a far tornare solo il comando finale

## 7. Come leggere i backend HTTP

Apri:

- [src/backends/chatgpt.rs](/home/simone/work/test-popup-ai/src/backends/chatgpt.rs)
- [src/backends/gemini.rs](/home/simone/work/test-popup-ai/src/backends/gemini.rs)
- [src/backends/ollama.rs](/home/simone/work/test-popup-ai/src/backends/ollama.rs)

Pattern comune:

1. legge config del backend
2. valida chiavi/modello
3. costruisce payload JSON
4. fa chiamata HTTP con `reqwest`
5. valida status code
6. estrae il testo dalla risposta JSON

Se vieni da Python:

- `reqwest` qui e il corrispettivo tipico di `requests`/`httpx`

Se vieni da Java:

- pensa a un client HTTP + mapper JSON, ma piu compatto

## 8. Come leggere la UI senza paura

Apri [gui.rs](/home/simone/work/test-popup-ai/src/gui.rs).

Questo e il file piu grosso.
Non leggerlo dall'alto al basso tutto di fila.

Ordine consigliato:

1. `struct AiPopupApp`
2. `AiPopupApp::new(...)`
3. `submit_prompt(...)`
4. `reload_history(...)`
5. `set_history_visibility(...)`
6. `impl eframe::App for AiPopupApp`
7. helper UI in basso (`card_frame`, `history_entry_card`, ecc.)

Mappa mentale:

- i campi della struct = stato della schermata
- `update(...)` = ciclo di rendering / evento UI
- ogni frame `egui` ricalcola e ridisegna l'interfaccia

Se vieni da Java:

- non pensarla come Swing con componenti persistenti
- pensala piu come "ridisegno la view a partire dallo stato corrente"

Se vieni da Python:

- somiglia piu a Streamlit/React-style stateful rendering che a Tkinter classico

## 9. Come funziona la history

Apri [history.rs](/home/simone/work/test-popup-ai/src/history.rs).

Fa poche cose:

- `new_entry()`
  crea una entry nuova

- `append_entry()`
  aggiunge una entry al file

- `recent_entries()`
  legge il file, pulisce le entry vecchie, restituisce la lista in ordine inverso

- `prune_old_entries()`
  elimina la history oltre retention

Pensalo come un repository molto piccolo basato su file JSONL.

## 10. Come leggere il daemon

Apri [daemon.rs](/home/simone/work/test-popup-ai/src/daemon.rs).

Fa questo:

1. parse della hotkey
2. registra la hotkey globale
3. loop infinito
4. quando la hotkey arriva, lancia `--ui`

Da leggere:

- `parse_hotkey()`
- `run()`
- `spawn_ui()`

## 11. Borrow, ownership, clone: la versione utile

Non serve studiare tutta la teoria per leggere questo progetto.

Ti basta questo:

- se una funzione prende `&str` o `&Config`
  sta solo leggendo

- se prende `String` o `Config`
  probabilmente sta consumando/spostando il valore

- se vedi `.clone()`
  il codice vuole una copia indipendente

- se il compilatore si lamenta di move/borrow
  di solito devi:
  - passare un riferimento `&x`
  - clonare `x.clone()`
  - cambiare l'ordine di uso delle variabili

## 12. Come seguire un bug

Metodo semplice:

1. identifica il file giusto
2. cerca la struct o funzione principale
3. segui i dati in ingresso
4. segui dove vengono trasformati
5. guarda i test esistenti
6. aggiungi un test che riproduce il bug
7. correggi il codice
8. rilancia `cargo test`

Comandi utili:

```bash
cargo build
cargo test
rg "nome_funzione" src
```

## 13. Come orientarti nei test

Test attuali:

- prompt/preprompt e parsing tag in `src/backends/mod.rs`
- history/retention in `src/history.rs`
- config/default in `src/config.rs`
- theme loading/parsing in `src/theme.rs`
- hotkey parsing in `src/daemon.rs`

Regola pratica:

- logica pura => test unitario
- rete vera => meglio mock o test separati
- UI `egui` => testare con cautela, preferendo la logica attorno alla UI

## 14. Se vuoi capire Rust piu in fretta

Quando leggi, traduci mentalmente cosi:

- `Vec<T>` = lista
- `HashMap<K, V>` = dict/map
- iterator chain (`iter().filter().map().collect()`) = pipeline funzionale
- `match` = switch molto piu serio
- `pub` = public
- `mod` = modulo/file namespace

## 15. Ordine di lettura consigliato del progetto

Se vuoi capirlo davvero in una sessione:

1. [src/main.rs](/home/simone/work/test-popup-ai/src/main.rs)
2. [src/config.rs](/home/simone/work/test-popup-ai/src/config.rs)
3. [src/theme.rs](/home/simone/work/test-popup-ai/src/theme.rs)
4. [src/backends/mod.rs](/home/simone/work/test-popup-ai/src/backends/mod.rs)
5. [src/backends/chatgpt.rs](/home/simone/work/test-popup-ai/src/backends/chatgpt.rs)
6. [src/backends/gemini.rs](/home/simone/work/test-popup-ai/src/backends/gemini.rs)
7. [src/backends/ollama.rs](/home/simone/work/test-popup-ai/src/backends/ollama.rs)
8. [src/history.rs](/home/simone/work/test-popup-ai/src/history.rs)
9. [src/daemon.rs](/home/simone/work/test-popup-ai/src/daemon.rs)
10. [src/gui.rs](/home/simone/work/test-popup-ai/src/gui.rs)

## 16. Ultimo consiglio importante

In Rust non provare a capire tutto insieme.
Capisci il flusso dei dati, poi i tipi, poi i dettagli del compilatore.

Se vuoi, nel prossimo passo posso anche generarti una seconda guida ancora piu pratica:

- "Rust per Java developer"
- oppure "Rust per Python developer"

con esempi 1:1 di sintassi equivalente.

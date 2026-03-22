# Refactor Coordination Plan (Multi-Agent)

## Goal
Rendere il codice più semplice, leggibile e gestibile, mantenendo piena compatibilità funzionale e test verde durante tutta la migrazione.

## Inputs (Specialized Agent Analysis)

### Agent A - Architecture (Lagrange)
- Proposta architettura target a strati:
  - `app` orchestrazione
  - `domain` use-case
  - `ports` contratti
  - `adapters/inbound` (UI/CLI)
  - `adapters/outbound` (provider, storage, OS)
  - `shared` cross-cutting
- Priorità:
  - split `src/gui.rs`
  - split `src/backends/mod.rs`
  - preservare contratti pubblici (`backends::query`, config schema, history format, runtime `!rag` override)

### Agent B - RAG/Backends (Lorentz)
- Piano incrementale 10 step (no big-bang), punti chiave:
  - ridurre accoppiamento `rag <-> backends`
  - trait `EmbeddingProvider`
  - indexing atomico (transaction + swap)
  - cache embeddings + invalidazione robusta
  - split `src/rag.rs` in moduli (`indexer`, `retriever`, `storage`, `extractors`)

### Agent C - UI/Quality (Mill)
- Strategia GUI:
  - decomporre `gui.rs` in `state`, `render`, `settings`, `async_tasks`, `layout`
  - settings in `Basic / Advanced / Privacy`
  - eliminare stato duplicato (`rag_ui` vs `config.rag`) con migrazione compatibile
- Quality gates:
  - fmt/clippy/test all targets
  - subset test layout GUI + visual regression checklist
  - cross-platform `cargo check`

## Coordinated Workstreams

### Workstream 1 - Core Architecture
Owner: Agent A  
Scope:
- estrazione contratti/tipi condivisi
- split backend orchestrator in moduli interni

### Workstream 2 - RAG/Backend Hardening
Owner: Agent B  
Scope:
- disaccoppiamento RAG embedding path
- indexing atomico
- caching robusto + performance

### Workstream 3 - UI/Settings Simplification
Owner: Agent C  
Scope:
- decomposizione `gui.rs`
- settings progressive disclosure
- consolidamento stato RAG in config canonical

### Workstream 4 - Test/CI Guardrails
Owner: Parent + Agent C support  
Scope:
- contract tests su comportamenti invarianti
- visual/layout checks
- phase gates per merge sicuro

## Phase Plan (Execution Order)

### Phase 0 - Baseline & Safety Net
- Freeze baseline behavior (test snapshot + smoke checklist).
- Confermare contratti pubblici non negoziabili.
- Go criteria:
  - `cargo test` verde
  - checklist smoke RAG/UI/backend passata

### Phase 1 - Correctness First (No UX change)
- Fix drift tra stato UI e config canonical (`rag.mode`, embedding overrides live).
- Eliminare side effect doppi non necessari (es. history writer unico).
- Go criteria:
  - test di persistenza/sync aggiunti e verdi
  - nessuna regressione funzionale

### Phase 2 - Structural Split (Low-Risk Extraction)
- Split `backends/mod.rs` per responsabilità.
- Split `gui.rs` in moduli senza cambiare comportamento.
- Go criteria:
  - parità output in test esistenti
  - nessuna variazione comportamento UI chiave

### Phase 3 - RAG Hardening
- Indexing atomico con swap.
- Cache con limiti/invalidation.
- Preparazione split `rag.rs` in sottocomponenti.
- Go criteria:
  - failure-injection test indexing
  - benchmark smoke migliore o invariato

### Phase 4 - UX Simplification
- Settings `Basic / Advanced / Privacy`.
- Path di config/history/log più espliciti in UI.
- Go criteria:
  - setup base completabile rapidamente
  - controlli avanzati nascosti by default

### Phase 5 - Cleanup & Legacy Removal
- Rimuovere compat code solo dopo 2 cicli CI verdi.
- Consolidare doc + changelog tecnico refactor.

## Risk Register

1. Regressioni su prompt composition/RAG interposition  
Mitigation: contract tests su prompt finale e runtime override.

2. Regressioni UI durante split `gui.rs`  
Mitigation: split meccanico per blocchi + test layout + visual checklist.

3. Inconsistenza DB durante reindex  
Mitigation: transaction + staging scope + rollback test.

4. Drift configurazione (`rag_ui` legacy)  
Mitigation: finestra di compatibilità breve + migrazione canonical + test roundtrip.

## Done Criteria (Global)

- Codice più semplice da leggere (moduli più piccoli, responsabilità chiare).
- Nessuna regressione funzionale sui flussi principali.
- Test e CI verdi durante tutte le fasi.
- RAG stabile in modalità `keyword/vector/hybrid`.

## RAG Practical Guidance (Recommended Default)

Per uso pragmatico e locale-first:
- backend query: `ollama`
- `rag.mode: hybrid` (fallback `keyword` se macchina limitata)
- `rag.embedding_backend: ollama`
- `rag.embedding_model: nomic-embed-text`

Questo riduce dipendenze cloud e semplifica troubleshooting.

## Execution Readiness

Stato: **READY**  
Tutti gli agenti hanno consegnato piano specialistico coerente e coordinabile in fasi incrementali.

## Execution Log

### 2026-03-22 - Docs Sync (Ownership Update)
- Marked `history_entry` module extraction as completed (`src/gui/history_entry.rs`).
- Noted active parallel cleanup tracks: GUI layout extraction and backend prompt extraction.

### 2026-03-22 - Parallel Optimization Batch
- Recorded the extracted backend modules `health.rs` and `models.rs`, plus the GUI panel modules under `src/gui/`.

### 2026-03-22 - GUI Modularization Update
- Recorded the latest `src/gui.rs` split across `settings_panel`, `history_panel`, `provider_settings`, and `rag_settings`.
- Recorded additional extraction of `history_entry` and layout helpers into dedicated GUI modules.

### 2026-03-22 - RAG Internal Modularization
- Split `src/rag.rs` internals into `src/rag/scoring.rs` and `src/rag/text.rs`.
- Kept retrieval/indexing behavior stable and validated with `cargo test` (including `tests/rag_retrieval.rs`).

### 2026-03-22 - Backend Query Flow Split
- Extracted backend query orchestration helpers into `src/backends/query_flow.rs` (`rag enable/disable resolution`, `retrieval dispatch`, `prompt assembly`, `backend dispatch`).
- Kept public `backends::query` behavior and output contracts stable; validated with full `cargo test`.

### 2026-03-22 - GUI Startup Health Split
- Moved startup diagnostics + first-run setup rendering from `src/gui.rs` to `src/gui/startup_health.rs`.
- Updated `provider_settings` imports to use backend health types directly.
- Behavior unchanged and validated via full `cargo test`.

### 2026-03-22 - Multi-Agent Optimization Pass (Docs Tracking)
- Recorded the latest optimization batch for coordination tracking only.
- Confirmed completed structural splits now reflected in status: collapsed settings ordering, extracted backend/startup health helpers, and the embedding dispatch split.
- Added Phase 2 follow-up bullets for the remaining backend and GUI extraction work.

### 2026-03-22 - Optimization Pass (Agent-Assisted Cleanup, Docs Tracking)
- Eseguita valutazione tecnica focalizzata su UI/settings e operatività (config discovery, update flow, logging/history paths, UX friction).
- Aggiornata la pianificazione di pulizia per milestone, criteri di done e rollback strategy.
- Nessuna modifica ai file di codice in questo pass; aggiornamento solo documentale di coordinamento/stato.

### 2026-03-22 - Optimization Pass (Agent-Assisted Code Cleanup)
- Estratto layer embedding backend in `src/backends/embedding.rs`, mantenendo API pubbliche stabili tramite pass-through in `src/backends/mod.rs`.
- Hardening RAG indexing: reindex reso atomico con transaction SQLite (`clear + insert` commit unico per scope).
- Validazione completa eseguita con `cargo fmt` e `cargo test` verde.

### 2026-03-22 - Phase 0 Completed
- Baseline verificata con `cargo test` interamente verde.
- Confermati i contratti pubblici chiave (`backends::query`, `!rag on/off`, schema config compatibile).

### 2026-03-22 - Phase 1 In Progress (Core Fix Applied)
- Consolidato stato RAG su `config.rag`:
  - aggiunto `rag.runtime_override` nel modello di config;
  - UI RAG ora legge/scrive direttamente `rag.mode`, `rag.runtime_override`, `rag.embedding_*`.
- Rimossa persistenza YAML duplicata `rag_ui` in `src/gui.rs` (fonte di drift).
- Ripuliti template (`configs/default.yaml`, `configs/beta.yaml`) dalla chiave legacy `retrieval_mode`.
- Test aggiornati per nuovo campo config e suite completa nuovamente verde.

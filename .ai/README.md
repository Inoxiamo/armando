# Project Context

Questa cartella contiene il contesto operativo del progetto in formato sintetico e professionale.
Va usata come base per onboarding, manutenzione e pianificazione.

## Documenti

- `PRODUCT.md`: obiettivi prodotto, use case, vincoli e valore del progetto
- `ARCHITECTURE.md`: panoramica tecnica, componenti principali e flussi runtime
- `STATUS.md`: stato corrente, funzionalita presenti, gap noti e priorita immediate
- `ROADMAP.md`: roadmap per milestone con focus su rilascio e qualita

## Regole di aggiornamento

- Aggiornare `STATUS.md` quando cambia il comportamento reale del prodotto
- Aggiornare `ARCHITECTURE.md` quando vengono introdotti nuovi componenti o flussi
- Aggiornare `ROADMAP.md` quando una priorita cambia o viene completata
- Mantenere `PRODUCT.md` stabile, salvo cambiamenti di visione o target

## Regole operative

- Prima di modificare il codice, leggere lo stato corrente in `.ai/STATUS.md`
- Ogni cambiamento UX o funzionale deve lasciare allineati codice e documentazione in `.ai`
- Non versionare segreti o configurazioni locali: `config.yaml` resta locale e non va committato
- Usare commit message in formato Conventional Commits, ad esempio `feat: improve history panel`
- Preferire commit piccoli e coerenti, uno per modifica significativa completata e verificata
- Eseguire almeno `cargo build` prima di chiudere una modifica che tocca codice Rust
- Aggiornare `STATUS.md` per bug fix percepibili dall'utente e `ROADMAP.md` quando una priorita viene assorbita o ridefinita

## Stato sintetico

Il progetto e un popup AI desktop in Rust/egui con backend multipli, hotkey globali e supporto a configurazione YAML.
L'obiettivo corrente e consolidare il flusso "ask -> receive -> apply", rendere la history realmente utile e migliorare l'interfaccia con un'identita visiva forte (`NERV HUD` come default).

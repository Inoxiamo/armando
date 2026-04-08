# Codex Skill Evaluation - Push Non-Regression Guard

## Objective
Definire una skill Codex leggera che renda sistematico il gate prima del push.

## Recommendation
Creare una skill dedicata (working name: `git:prepush-regression`) che faccia sempre:

1. verifica profilo (`quick/default/full`) in base al contesto della change;
2. esecuzione `scripts/ci/pre-push-regression.sh`;
3. report sintetico pass/fail con comando usato;
4. blocco esplicito del push quando i check falliscono.

## Why this is useful
- Riduce regressioni da push veloci in branch condivisi.
- Uniforma comportamento tra collaboratori umani e agenti.
- Mantiene la policy vicino al repo (`.githooks` + `scripts/ci`) invece che in sola memoria operativa.

## Suggested defaults
- `default` per sviluppo normale.
- `full` per release candidate o branch hardening.
- `quick` solo per iterazioni locali ad alta frequenza.

## Guardrails
- Bypass consentito solo con variabile esplicita: `SKIP_PRE_PUSH_REGRESSION=1`.
- In caso di bypass, richiedere una nota nel messaggio di commit/PR.

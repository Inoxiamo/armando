# Project Context

This folder is the canonical documentation hub for the project.
Use it as the default entry point for onboarding, maintenance, planning, and release prep.

## Documents

- `PRODUCT.md`: product goals, use cases, constraints, and value proposition
- `ARCHITECTURE.md`: technical overview, main components, and runtime flows
- `STATUS.md`: current behavior, implemented capabilities, known gaps, and immediate priorities
- `ROADMAP.md`: milestone-based roadmap focused on product maturity and distribution
- `RUST_GUIDE_FOR_JAVA_PYTHON.md`: quick orientation guide for reading this Rust codebase if you come from Java or Python

## Repository Structure

- `src/`: application code, UI, config, history, i18n, theming, and backend integrations
- `themes/`: shipped theme presets loaded at runtime from YAML
- `locales/`: shipped UI translations loaded at runtime from YAML
- `configs/`: default user-facing configuration templates
- `scripts/`: local install and release packaging helpers
- `assets/`: desktop integration assets such as the app icon and `.desktop` entry
- `.ai/`: living project documentation for product, architecture, status, and roadmap

## Update Rules

- Update `STATUS.md` whenever user-visible behavior changes
- Update `ARCHITECTURE.md` when new components, paths, or runtime flows are introduced
- Update `ROADMAP.md` when priorities change or milestones are completed
- Keep `PRODUCT.md` relatively stable unless product direction or audience changes

## Working Rules

- Read `.ai/STATUS.md` before making code changes
- Keep code and `.ai` documentation aligned after every UX or functional change
- Do not commit secrets or local-only configuration files
- Use Conventional Commits such as `feat: improve history panel`
- Prefer small, coherent commits with verified scope
- Run at least `cargo build` before closing changes that touch Rust code
- Reflect user-visible fixes in `STATUS.md` and roadmap shifts in `ROADMAP.md`

## Snapshot

The project is a Rust/egui desktop AI popup with multiple backends, YAML-based configuration, external themes, external locales, and persistent local history.
The current focus is to strengthen the "ask -> receive -> apply" workflow, make history genuinely useful, tighten visual consistency across the popup, and finish desktop-ready installation details such as icon and launcher integration.

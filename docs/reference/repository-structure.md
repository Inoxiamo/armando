# Repository Structure

This page is a quick map of the repository.

## Top Level

- `src/`: Rust application code
- `assets/`: icons and desktop integration assets
- `configs/`: default configuration templates and reusable first-run profiles
- `configs/prompts/`: bundled prompt preset templates
- `themes/`: shipped theme presets
- `locales/`: shipped UI translations
- `scripts/`: local install, packaging, validation, and release helpers
- `tests/`: integration and functional tests
- `docs/`: public documentation (getting started, guides, QA, reference, media)
- `docker/`: Docker image used for CI and local container validation
- `.github/workflows/`: CI, release, and static analysis automation
- `.ai/`: internal product, architecture, roadmap, and contributor notes
- `/docs/qa/visual-regression.md`: repeatable layout and visual verification checklist

## Application Modules

- `src/bin/app.rs`: native app entry point binary
- `src/lib.rs`: shared library exports for tests and app modules
- `src/gui/mod.rs`: UI flow, settings, history, prompt handling, clipboard behavior, and version display
- `src/rag/mod.rs`: RAG indexing, retrieval, and scoring orchestration
- `src/core/`: app paths, config, history, i18n, logging, prompt profiles, theme, updates, window context
- `src/backends/providers/`: provider integrations for Ollama, OpenAI, Gemini, and Claude
- `src/backends/pipeline/`: prompt preparation, retrieval flow, and embedding dispatch
- `src/backends/catalog/`: model discovery/listing
- `src/backends/ops/`: startup and runtime health checks

## Release And Validation Scripts

- `scripts/release/package-release.sh`: creates Unix release bundles
- `scripts/release/release-install.sh`: installs downloaded Unix release bundles
- `scripts/release/release-install.ps1`: installs downloaded Windows release bundles
- `scripts/dev/install-local.sh`: installs a locally built release into the user profile
- `scripts/ci/run-container-tests.sh`: CI-style Docker validation flow
- `scripts/release/pre-release-check.sh`: local pre-release gate for fmt, test, clippy, and tag validation
- `scripts/dev/start-sonar-local.sh`: starts or reuses a local SonarQube Docker container and waits for readiness
- `scripts/dev/run-sonar-local.sh`: runs the local Sonar scan, waits for processing, and reports the quality gate
- `scripts/quality/export-clippy-sonar-report.sh`: exports `clippy` diagnostics as a Sonar external issues report
- `scripts/quality/export-rust-coverage.sh`: generates Rust coverage artifacts and a Sonar generic coverage report
- `scripts/release/verify-changelog-version.sh`: validates that the tagged version exists in `CHANGELOG.md`
- `scripts/release/verify-release-version.sh`: validates Git tag vs Cargo version

## Documentation Map

- Official project overview: [`/README.md`](/README.md)
- Documentation index: [`/docs/README.md`](/docs/README.md)
- Installation guide: [`/docs/getting-started/install.md`](/docs/getting-started/install.md)
- Shortcut guide: [`/docs/guides/shortcuts.md`](/docs/guides/shortcuts.md)
- Release guide: [`/docs/guides/releases.md`](/docs/guides/releases.md)
- Visual regression checklist: [`/docs/qa/visual-regression.md`](/docs/qa/visual-regression.md)
- Documentation media index: [`/docs/media/README.md`](/docs/media/README.md)

Internal docs are intentionally kept under `.ai/` and are not linked from the public `README`.

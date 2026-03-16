# Repository Structure

This page is a quick map of the repository.

## Top Level

- `src/`: Rust application code
- `assets/`: icons and desktop integration assets
- `configs/`: default configuration templates
- `themes/`: shipped theme presets
- `locales/`: shipped UI translations
- `scripts/`: local install, packaging, validation, and release helpers
- `tests/`: integration and functional tests
- `docker/`: Docker image used for CI and local container validation
- `.github/workflows/`: CI, release, and static analysis automation
- `.ai/`: internal product, architecture, roadmap, and contributor notes

## Application Modules

- `src/main.rs`: native app entry point
- `src/lib.rs`: shared library exports for tests and app modules
- `src/gui.rs`: UI flow, settings, history, prompt handling, clipboard behavior, and version display
- `src/config.rs`: YAML config schema and persistence
- `src/app_paths.rs`: config, data, history, and log path resolution
- `src/history.rs`: local history persistence and retention
- `src/theme.rs`: theme loading and discovery
- `src/i18n.rs`: locale loading and discovery
- `src/logging.rs`: opt-in debug logging
- `src/backends/`: provider integrations for Ollama, OpenAI, Gemini, and Claude

## Release And Validation Scripts

- `scripts/package-release.sh`: creates Unix release bundles
- `scripts/release-install.sh`: installs downloaded Unix release bundles
- `scripts/release-install.ps1`: installs downloaded Windows release bundles
- `scripts/install-local.sh`: installs a locally built release into the user profile
- `scripts/run-container-tests.sh`: CI-style Docker validation flow
- `scripts/pre-release-check.sh`: local pre-release gate for fmt, test, clippy, and tag validation
- `scripts/start-sonar-local.sh`: starts or reuses a local SonarQube Docker container and waits for readiness
- `scripts/run-sonar-local.sh`: runs the local Sonar scan, waits for processing, and reports the quality gate
- `scripts/export-clippy-sonar-report.sh`: exports `clippy` diagnostics as a Sonar external issues report
- `scripts/export-rust-coverage.sh`: generates Rust coverage artifacts and a Sonar generic coverage report
- `scripts/verify-changelog-version.sh`: validates that the tagged version exists in `CHANGELOG.md`
- `scripts/verify-release-version.sh`: validates Git tag vs Cargo version

## Documentation Map

- Official project overview: [`README.md`](README.md)
- Installation guide: [`INSTALL.md`](INSTALL.md)
- Shortcut guide: [`SHORTCUTS.md`](SHORTCUTS.md)
- Release guide: [`RELEASES.md`](RELEASES.md)

Internal docs are intentionally kept under `.ai/` and are not linked from the public `README`.

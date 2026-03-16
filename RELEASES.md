# Releases

This page explains how `armando` versions, tags, and release artifacts are organized.

## Versioning

- the application version lives in [`Cargo.toml`](Cargo.toml)
- the Git tag format is `v<version>`
- example: crate version `0.0.2-rc1` maps to tag `v0.0.2-rc1`

## Downloading A Release

- Latest release: <https://github.com/Inoxiamo/armando/releases/latest>
- All releases: <https://github.com/Inoxiamo/armando/releases>

Use `latest` if you just want the newest published version.
Use the full releases page if you need a specific stable build, prerelease, or older artifact.

## Release Flow

Typical flow:

```bash
git commit -am "release: prepare v0.0.2-rc1"
git tag v0.0.2-rc1
git push origin master --tags
```

The release workflow validates that:

- the pushed tag matches `Cargo.toml`
- the pushed tag exists in `CHANGELOG.md`
- local formatting, tests, and clippy checks pass
- clippy external issues and Rust coverage reports are generated successfully
- Linux container validation passes before packaging
- Sonar passes too when Sonar secrets are configured

## Produced Artifacts

Current release outputs:

- Linux x86_64: `armando-<version>-x86_64-unknown-linux-gnu.tar.gz`
- macOS Apple Silicon: `armando-<version>-aarch64-apple-darwin.tar.gz`
- Windows x86_64: `armando-<version>-x86_64-pc-windows-msvc.zip`

Each release bundle includes:

- the compiled binary
- default config
- themes
- locales
- assets
- install script
- checksum file

## Pre-Release Validation

Before publishing a version, the repository can validate:

- `cargo fmt --all -- --check`
- `cargo test --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- tag and `Cargo.toml` version alignment
- changelog entry for the exact release tag
- clippy external issues export for Sonar
- Rust coverage export
- Linux container packaging flow

Local command:

```bash
bash scripts/pre-release-check.sh v0.0.2-rc1
```

Optional local Sonar smoke test:

```bash
bash scripts/run-sonar-local.sh
```

For the full containerized flow used in CI:

```bash
docker build -f docker/test-runner.Dockerfile -t armando-test-runner .
docker run --rm -v "$(pwd):/workspace" -w /workspace armando-test-runner bash scripts/run-container-tests.sh
```

## Next Steps

- For release installation, see [`INSTALL.md`](INSTALL.md).
- For keyboard shortcut setup after install, see [`SHORTCUTS.md`](SHORTCUTS.md).
- For the repository layout, see [`STRUCTURE.md`](STRUCTURE.md).

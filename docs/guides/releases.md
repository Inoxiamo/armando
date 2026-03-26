# Releases

This page explains how `armando` versions, tags, and release artifacts are organized.

## Versioning

- the application version lives in [`Cargo.toml`](Cargo.toml)
- the Git tag format is `v<version>`
- example: crate version `0.0.2-rc5` maps to tag `v0.0.2-rc5`

## Downloading A Release

- Latest release: <https://github.com/Inoxiamo/armando/releases/latest>
- All releases: <https://github.com/Inoxiamo/armando/releases>

Use `latest` if you just want the newest published version.
Use the full releases page if you need a specific stable build, prerelease, or older artifact.
The desktop app also checks the latest GitHub release in-app and compares it with the local version from `Cargo.toml`.
When an update is available, the settings footer offers both the direct release page and, on Linux or macOS, a guided bootstrap command.
When an update is available, the settings footer also surfaces a guided next step for the current platform: a copyable bootstrap command on Linux/macOS or a release-page shortcut on Windows.

Linux and macOS users can also use the guided bootstrap wrapper from the repository:

```bash
curl -fsSL https://raw.githubusercontent.com/Inoxiamo/armando/master/scripts/release/bootstrap-release.sh | bash
```

Pass a version as the first argument or set `ARMANDO_INSTALL_VERSION=<version>` before invoking it if you want a specific tag instead of `latest`.

## Release Flow

Typical flow:

```bash
git commit -am "release: prepare v0.0.2-rc5"
git tag v0.0.2-rc5
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

The bootstrap wrapper does not replace these installers. It only selects the correct artifact, downloads it, optionally verifies the checksum, extracts it, and delegates to the bundled installer.
For Windows, the release page remains the manual download path and the included `install.ps1` script remains the recommended installer entrypoint.

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
bash scripts/release/pre-release-check.sh v0.0.2-rc5
```

Optional local Sonar smoke test:

```bash
bash scripts/dev/run-sonar-local.sh
```

For the full containerized flow used in CI:

```bash
docker build -f docker/test-runner.Dockerfile -t armando-test-runner .
docker run --rm -v "$(pwd):/workspace" -w /workspace armando-test-runner bash scripts/ci/run-container-tests.sh
```

## Next Steps

- For release installation, see [`/docs/getting-started/install.md`](/docs/getting-started/install.md).
- For keyboard shortcut setup after install, see [`/docs/guides/shortcuts.md`](/docs/guides/shortcuts.md).
- For the repository layout, see [`/docs/reference/repository-structure.md`](/docs/reference/repository-structure.md).

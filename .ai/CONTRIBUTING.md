# Contributing

Thanks for contributing to `armando`.

## Read First

- Internal docs hub: [`README.md`](README.md)
- Status: [`STATUS.md`](STATUS.md)
- Architecture: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Roadmap: [`ROADMAP.md`](ROADMAP.md)
- Rust reading guide: [`RUST_GUIDE_FOR_JAVA_PYTHON.md`](RUST_GUIDE_FOR_JAVA_PYTHON.md)

## Local Setup

Build locally:

```bash
cargo build
```

Run the normal test suite:

```bash
cargo test --all-targets
```

Run the local pre-release gate:

```bash
bash scripts/pre-release-check.sh
```

Run the tagged release gate:

```bash
bash scripts/pre-release-check.sh v0.0.2-rc1
```

Run the same Linux container validation used by CI:

```bash
docker build -f docker/test-runner.Dockerfile -t armando-test-runner .
docker run --rm -v "$(pwd):/workspace" -w /workspace armando-test-runner bash scripts/run-container-tests.sh
```

Run a local SonarQube check:

```bash
bash scripts/run-sonar-local.sh
```

Useful overrides:

- `SONAR_ADMIN_USER` and `SONAR_ADMIN_PASSWORD` if your local SonarQube password is not the default
- `SONAR_TOKEN` if you prefer passing an existing token directly
- `SONAR_PORT` if port `9000` is already in use

## Working Rules

- Keep changes focused and coherent.
- Update public root docs when install, shortcut, release, or repository-structure behavior changes.
- Update [`STATUS.md`](STATUS.md) when user-visible behavior changes.
- Update [`ARCHITECTURE.md`](ARCHITECTURE.md) when components, runtime paths, or flows change.
- Prefer Conventional Commit messages.
- Do not commit secrets or local-only configuration.

## Release Hygiene

Before preparing a release:

- run `bash scripts/pre-release-check.sh <tag>` if the version tag already exists locally
- run the Docker validation flow
- run `bash scripts/run-sonar-local.sh` if you want to verify the local Sonar integration too
- make sure the tag already appears in `CHANGELOG.md`
- confirm docs and changelog are aligned with the shipped behavior

Public release behavior and artifact naming are documented in [`../RELEASES.md`](../RELEASES.md).

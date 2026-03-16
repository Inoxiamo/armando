#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TAG_NAME="${1:-}"

cd "${ROOT_DIR}"

if [[ -n "${TAG_NAME}" ]]; then
  chmod +x scripts/verify-release-version.sh
  chmod +x scripts/verify-changelog-version.sh
  scripts/verify-release-version.sh "${TAG_NAME}"
  scripts/verify-changelog-version.sh "${TAG_NAME}"
fi

cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
bash scripts/export-clippy-sonar-report.sh
bash scripts/export-rust-coverage.sh

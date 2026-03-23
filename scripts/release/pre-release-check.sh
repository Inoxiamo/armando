#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TAG_NAME="${1:-}"

cd "${ROOT_DIR}"

if [[ -n "${TAG_NAME}" ]]; then
  chmod +x scripts/release/verify-release-version.sh
  chmod +x scripts/release/verify-changelog-version.sh
  scripts/release/verify-release-version.sh "${TAG_NAME}"
  scripts/release/verify-changelog-version.sh "${TAG_NAME}"
fi

cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
bash scripts/quality/export-clippy-sonar-report.sh
bash scripts/quality/export-rust-coverage.sh

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${ROOT_DIR}/target/test-artifacts"
DIST_DIR="${ROOT_DIR}/target/dist"
VERSION_LABEL="${1:-ci-${GITHUB_SHA:-local}}"
PACKAGE_ARCHIVE="${DIST_DIR}/armando-${VERSION_LABEL}-x86_64-unknown-linux-gnu.tar.gz"

mkdir -p "${ARTIFACT_DIR}" "${DIST_DIR}"

{
  echo "date=$(date --iso-8601=seconds)"
  rustc --version
  cargo --version
} > "${ARTIFACT_DIR}/toolchain.txt"

cargo fmt --all -- --check 2>&1 | tee "${ARTIFACT_DIR}/cargo-fmt.log"
cargo test --all-targets -- --nocapture 2>&1 | tee "${ARTIFACT_DIR}/cargo-test.log"
cargo clippy --all-targets -- -D warnings 2>&1 | tee "${ARTIFACT_DIR}/cargo-clippy.log"
bash "${ROOT_DIR}/scripts/export-clippy-sonar-report.sh" 2>&1 | tee "${ARTIFACT_DIR}/export-clippy-sonar.log"
bash "${ROOT_DIR}/scripts/export-rust-coverage.sh" 2>&1 | tee "${ARTIFACT_DIR}/export-rust-coverage.log"
cargo build --release 2>&1 | tee "${ARTIFACT_DIR}/cargo-build-release.log"

"${ROOT_DIR}/scripts/package-release.sh" \
  "${ROOT_DIR}/target/release/armando" \
  "x86_64-unknown-linux-gnu" \
  "${VERSION_LABEL}" \
  2>&1 | tee "${ARTIFACT_DIR}/package-release.log"

sha256sum "${PACKAGE_ARCHIVE}" > "${PACKAGE_ARCHIVE}.sha256"
tar -tzf "${PACKAGE_ARCHIVE}" > "${ARTIFACT_DIR}/release-bundle-contents.txt"

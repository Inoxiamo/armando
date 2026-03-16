#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${ROOT_DIR}/target/test-artifacts/sonar"
RAW_REPORT="${ARTIFACT_DIR}/clippy.jsonl"
LOG_REPORT="${ARTIFACT_DIR}/cargo-clippy-sonar.log"
ISSUES_REPORT="${ARTIFACT_DIR}/clippy-external-issues.json"

cd "${ROOT_DIR}"
mkdir -p "${ARTIFACT_DIR}"

cargo clippy --all-targets --message-format=json \
  >"${RAW_REPORT}" \
  2>"${LOG_REPORT}"

python3 "${ROOT_DIR}/scripts/clippy_json_to_sonar_issues.py" \
  "${RAW_REPORT}" \
  "${ISSUES_REPORT}"

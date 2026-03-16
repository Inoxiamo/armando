#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXPECTED_TAG="${1:-}"

if [[ -z "${EXPECTED_TAG}" ]]; then
  echo "usage: $0 <git-tag>" >&2
  exit 1
fi

if ! grep -Eq "^## ${EXPECTED_TAG}( |$|-)" "${ROOT_DIR}/CHANGELOG.md"; then
  echo "Missing changelog entry for ${EXPECTED_TAG} in CHANGELOG.md" >&2
  exit 1
fi

echo "Changelog entry OK: ${EXPECTED_TAG}"

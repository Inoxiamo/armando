#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EXPECTED_TAG="${1:-}"

if [[ -z "${EXPECTED_TAG}" ]]; then
  echo "usage: $0 <git-tag>" >&2
  exit 1
fi

CARGO_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "${ROOT_DIR}/Cargo.toml" | head -n 1)"

if [[ -z "${CARGO_VERSION}" ]]; then
  echo "Could not determine version from Cargo.toml" >&2
  exit 1
fi

EXPECTED_VERSION="v${CARGO_VERSION}"

if [[ "${EXPECTED_TAG}" != "${EXPECTED_VERSION}" ]]; then
  echo "Release tag/version mismatch: tag=${EXPECTED_TAG} Cargo.toml=${CARGO_VERSION}" >&2
  exit 1
fi

echo "Release metadata OK: ${EXPECTED_TAG}"

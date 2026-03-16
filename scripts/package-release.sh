#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <binary-path> <target-triple> <version>" >&2
  exit 1
fi

BINARY_PATH="$1"
TARGET_TRIPLE="$2"
VERSION="$3"
APP_NAME="armando"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/target/dist/${APP_NAME}-${VERSION}-${TARGET_TRIPLE}"

rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}/configs" "${DIST_DIR}/themes"
mkdir -p "${DIST_DIR}/locales"
mkdir -p "${DIST_DIR}/assets"
mkdir -p "${DIST_DIR}/scripts"

cp "${BINARY_PATH}" "${DIST_DIR}/"
cp "${ROOT_DIR}/README.md" "${DIST_DIR}/"
cp "${ROOT_DIR}/configs/default.yaml" "${DIST_DIR}/configs/default.yaml"
cp "${ROOT_DIR}"/themes/*.yaml "${DIST_DIR}/themes/"
cp "${ROOT_DIR}"/locales/*.yaml "${DIST_DIR}/locales/"
cp "${ROOT_DIR}"/assets/* "${DIST_DIR}/assets/"
cp "${ROOT_DIR}/scripts/release-install.sh" "${DIST_DIR}/scripts/install.sh"
chmod 0755 "${DIST_DIR}/scripts/install.sh"

tar -C "$(dirname "${DIST_DIR}")" -czf "${DIST_DIR}.tar.gz" "$(basename "${DIST_DIR}")"

echo "${DIST_DIR}.tar.gz"

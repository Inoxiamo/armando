#!/usr/bin/env bash
set -euo pipefail

APP_NAME="armando"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
CONFIG_ROOT="${HOME}/.config/${APP_NAME}"
CONFIG_DIR="${CONFIG_ROOT}/configs"
THEMES_DIR="${CONFIG_ROOT}/themes"
LOCALES_DIR="${CONFIG_ROOT}/locales"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
DESKTOP_DIR="${HOME}/.local/share/applications"
RELEASE_BIN="${ROOT_DIR}/target/release/${APP_NAME}"
CONFIG_SOURCE="${ROOT_DIR}/configs/default.yaml"
ICON_SOURCE="${ROOT_DIR}/assets/${APP_NAME}.svg"
DESKTOP_SOURCE="${ROOT_DIR}/assets/${APP_NAME}.desktop"

mkdir -p \
  "${BIN_DIR}" \
  "${CONFIG_DIR}" \
  "${THEMES_DIR}" \
  "${LOCALES_DIR}" \
  "${ICON_DIR}" \
  "${DESKTOP_DIR}"

cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"

install -m 0755 "${RELEASE_BIN}" "${BIN_DIR}/${APP_NAME}"

if [[ ! -f "${CONFIG_DIR}/default.yaml" || "${FORCE_CONFIG_INSTALL:-0}" == "1" ]]; then
  install -m 0644 "${CONFIG_SOURCE}" "${CONFIG_DIR}/default.yaml"
fi

for theme_file in "${ROOT_DIR}"/themes/*.yaml; do
  install -m 0644 "${theme_file}" "${THEMES_DIR}/$(basename "${theme_file}")"
done

for locale_file in "${ROOT_DIR}"/locales/*.yaml; do
  install -m 0644 "${locale_file}" "${LOCALES_DIR}/$(basename "${locale_file}")"
done

install -m 0644 "${ICON_SOURCE}" "${ICON_DIR}/${APP_NAME}.svg"
sed "s|\${HOME}|${HOME}|g" "${DESKTOP_SOURCE}" > "${DESKTOP_DIR}/${APP_NAME}.desktop"
chmod 0644 "${DESKTOP_DIR}/${APP_NAME}.desktop"

cat <<EOF
Installed ${APP_NAME}

Binary:
  ${BIN_DIR}/${APP_NAME}

Config:
  ${CONFIG_DIR}/default.yaml

Themes:
  ${THEMES_DIR}

Locales:
  ${LOCALES_DIR}

Desktop icon:
  ${ICON_DIR}/${APP_NAME}.svg

Desktop entry:
  ${DESKTOP_DIR}/${APP_NAME}.desktop

Add ${BIN_DIR} to your PATH if needed, then bind your OS shortcut to:
  ${BIN_DIR}/${APP_NAME}
EOF

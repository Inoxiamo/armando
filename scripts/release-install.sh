#!/usr/bin/env bash
set -euo pipefail

APP_NAME="armando"
BUNDLE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SOURCE="${BUNDLE_DIR}/${APP_NAME}"
BIN_DIR="${HOME}/.local/bin"
PLATFORM="${ARMANDO_INSTALL_OS:-$(uname -s)}"

case "${PLATFORM}" in
  Linux)
    CONFIG_ROOT="${XDG_CONFIG_HOME:-${HOME}/.config}/${APP_NAME}"
    DATA_ROOT="${XDG_DATA_HOME:-${HOME}/.local/share}/${APP_NAME}"
    ;;
  Darwin)
    CONFIG_ROOT="${HOME}/Library/Application Support/${APP_NAME}"
    DATA_ROOT="${HOME}/Library/Application Support/${APP_NAME}"
    ;;
  *)
    echo "Unsupported platform for install.sh: ${PLATFORM}" >&2
    exit 1
    ;;
esac

CONFIG_DIR="${CONFIG_ROOT}/configs"
THEMES_DIR="${CONFIG_ROOT}/themes"
LOCALES_DIR="${CONFIG_ROOT}/locales"
ASSETS_DIR="${DATA_ROOT}/assets"

if [[ ! -f "${BIN_SOURCE}" ]]; then
  echo "Could not find ${APP_NAME} binary in bundle root: ${BIN_SOURCE}" >&2
  exit 1
fi

mkdir -p "${BIN_DIR}" "${CONFIG_DIR}" "${THEMES_DIR}" "${LOCALES_DIR}" "${ASSETS_DIR}"
install -m 0755 "${BIN_SOURCE}" "${BIN_DIR}/${APP_NAME}"

if [[ -f "${BUNDLE_DIR}/configs/default.yaml" ]]; then
  install -m 0644 "${BUNDLE_DIR}/configs/default.yaml" "${CONFIG_DIR}/default.yaml"
fi

if compgen -G "${BUNDLE_DIR}/themes/*.yaml" > /dev/null; then
  for theme_file in "${BUNDLE_DIR}"/themes/*.yaml; do
    install -m 0644 "${theme_file}" "${THEMES_DIR}/$(basename "${theme_file}")"
  done
fi

if compgen -G "${BUNDLE_DIR}/locales/*.yaml" > /dev/null; then
  for locale_file in "${BUNDLE_DIR}"/locales/*.yaml; do
    install -m 0644 "${locale_file}" "${LOCALES_DIR}/$(basename "${locale_file}")"
  done
fi

if [[ -d "${BUNDLE_DIR}/assets" ]]; then
  cp -R "${BUNDLE_DIR}/assets/." "${ASSETS_DIR}/"
fi

if [[ "${PLATFORM}" == "Linux" ]]; then
  ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
  DESKTOP_DIR="${HOME}/.local/share/applications"
  mkdir -p "${ICON_DIR}" "${DESKTOP_DIR}"

  if [[ -f "${BUNDLE_DIR}/assets/${APP_NAME}.svg" ]]; then
    install -m 0644 "${BUNDLE_DIR}/assets/${APP_NAME}.svg" "${ICON_DIR}/${APP_NAME}.svg"
  fi

  if [[ -f "${BUNDLE_DIR}/assets/${APP_NAME}.desktop" ]]; then
    sed "s|\${HOME}|${HOME}|g" "${BUNDLE_DIR}/assets/${APP_NAME}.desktop" > "${DESKTOP_DIR}/${APP_NAME}.desktop"
    chmod 0644 "${DESKTOP_DIR}/${APP_NAME}.desktop"
  fi
fi

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

Assets:
  ${ASSETS_DIR}
EOF

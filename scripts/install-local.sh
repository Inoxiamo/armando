#!/usr/bin/env bash
set -euo pipefail

APP_NAME="armando"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
PLATFORM="${ARMANDO_INSTALL_OS:-$(uname -s)}"
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"

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
    echo "Unsupported platform for install-local.sh: ${PLATFORM}" >&2
    exit 1
    ;;
esac

CONFIG_DIR="${CONFIG_ROOT}/configs"
THEMES_DIR="${CONFIG_ROOT}/themes"
LOCALES_DIR="${CONFIG_ROOT}/locales"
ASSETS_DIR="${DATA_ROOT}/assets"
RELEASE_BIN="${TARGET_DIR}/release/${APP_NAME}"
CONFIG_SOURCE="${ROOT_DIR}/configs/default.yaml"
PROMPT_TAGS_SOURCE="${ROOT_DIR}/prompt-tags.yaml"
GENERIC_PROMPTS_SOURCE="${ROOT_DIR}/generic-prompts.yaml"
ICON_SOURCE="${ROOT_DIR}/assets/${APP_NAME}.svg"
DESKTOP_SOURCE="${ROOT_DIR}/assets/${APP_NAME}.desktop"

install_config_file() {
  local source_path="$1"
  local destination_path="$2"

  if [[ ! -f "${source_path}" ]]; then
    return
  fi

  if [[ ! -f "${destination_path}" || "${FORCE_CONFIG_INSTALL:-0}" == "1" ]]; then
    install -m 0644 "${source_path}" "${destination_path}"
  fi
}

mkdir -p \
  "${BIN_DIR}" \
  "${CONFIG_DIR}" \
  "${THEMES_DIR}" \
  "${LOCALES_DIR}" \
  "${ASSETS_DIR}"

cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"

install -m 0755 "${RELEASE_BIN}" "${BIN_DIR}/${APP_NAME}"

if [[ ! -f "${CONFIG_DIR}/default.yaml" || "${FORCE_CONFIG_INSTALL:-0}" == "1" ]]; then
fi

install_config_file "${CONFIG_SOURCE}" "${CONFIG_DIR}/default.yaml"
install_config_file "${PROMPT_TAGS_SOURCE}" "${CONFIG_ROOT}/prompt-tags.yaml"
install_config_file "${GENERIC_PROMPTS_SOURCE}" "${CONFIG_ROOT}/generic-prompts.yaml"

for theme_file in "${ROOT_DIR}"/themes/*.yaml; do
  install -m 0644 "${theme_file}" "${THEMES_DIR}/$(basename "${theme_file}")"
done

for locale_file in "${ROOT_DIR}"/locales/*.yaml; do
  install -m 0644 "${locale_file}" "${LOCALES_DIR}/$(basename "${locale_file}")"
done

cp -R "${ROOT_DIR}/assets/." "${ASSETS_DIR}/"

if [[ "${PLATFORM}" == "Linux" ]]; then
  ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
  DESKTOP_DIR="${HOME}/.local/share/applications"
  mkdir -p "${ICON_DIR}" "${DESKTOP_DIR}"
  install -m 0644 "${ICON_SOURCE}" "${ICON_DIR}/${APP_NAME}.svg"
  sed "s|\${HOME}|${HOME}|g" "${DESKTOP_SOURCE}" > "${DESKTOP_DIR}/${APP_NAME}.desktop"
  chmod 0644 "${DESKTOP_DIR}/${APP_NAME}.desktop"
fi

cat <<EOF
Installed ${APP_NAME}

Binary:
  ${BIN_DIR}/${APP_NAME}

Config:
  ${CONFIG_DIR}/default.yaml

Prompt presets:
  ${CONFIG_ROOT}/prompt-tags.yaml
  ${CONFIG_ROOT}/generic-prompts.yaml

Themes:
  ${THEMES_DIR}

Locales:
  ${LOCALES_DIR}

Assets:
  ${ASSETS_DIR}

EOF

if [[ "${PLATFORM}" == "Linux" ]]; then
cat <<EOF

Desktop icon:
  ${ICON_DIR}/${APP_NAME}.svg

Desktop entry:
  ${DESKTOP_DIR}/${APP_NAME}.desktop

Add ${BIN_DIR} to your PATH if needed, then bind your OS shortcut to:
  ${BIN_DIR}/${APP_NAME}
EOF
else
cat <<EOF

Add ${BIN_DIR} to your PATH if needed, then launch:
  ${BIN_DIR}/${APP_NAME}
EOF
fi

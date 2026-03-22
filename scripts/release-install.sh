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

extract_top_level_block() {
  local source_path="$1"
  local key="$2"
  awk -v key="${key}" '
    $0 ~ ("^" key ":[[:space:]]*($|#)") { print; in_block=1; next }
    in_block && $0 ~ "^[^[:space:]#]" { exit }
    in_block { print }
  ' "${source_path}"
}

has_top_level_key() {
  local file_path="$1"
  local key="$2"
  grep -Eq "^${key}:[[:space:]]*($|#)" "${file_path}"
}

ensure_top_level_block() {
  local file_path="$1"
  local source_path="$2"
  local key="$3"
  if has_top_level_key "${file_path}" "${key}"; then
    return
  fi

  local block
  block="$(extract_top_level_block "${source_path}" "${key}")"
  if [[ -z "${block}" ]]; then
    return
  fi

  printf '\n%s\n' "${block}" >> "${file_path}"
}

extract_nested_line() {
  local source_path="$1"
  local top_key="$2"
  local nested_key="$3"
  awk -v top_key="${top_key}" -v nested_key="${nested_key}" '
    $0 ~ ("^" top_key ":[[:space:]]*($|#)") { in_block=1; next }
    in_block && $0 ~ "^[^[:space:]#]" { in_block=0 }
    in_block && $0 ~ ("^[[:space:]]{2}" nested_key ":[[:space:]]*") { print; exit }
  ' "${source_path}"
}

has_nested_key() {
  local file_path="$1"
  local top_key="$2"
  local nested_key="$3"
  awk -v top_key="${top_key}" -v nested_key="${nested_key}" '
    $0 ~ ("^" top_key ":[[:space:]]*($|#)") { in_block=1; next }
    in_block && $0 ~ "^[^[:space:]#]" { in_block=0 }
    in_block && $0 ~ ("^[[:space:]]{2}" nested_key ":[[:space:]]*") { found=1; exit }
    END { exit(found ? 0 : 1) }
  ' "${file_path}"
}

ensure_nested_key() {
  local file_path="$1"
  local source_path="$2"
  local top_key="$3"
  local nested_key="$4"

  ensure_top_level_block "${file_path}" "${source_path}" "${top_key}"
  if has_nested_key "${file_path}" "${top_key}" "${nested_key}"; then
    return
  fi

  local insert_line
  insert_line="$(extract_nested_line "${source_path}" "${top_key}" "${nested_key}")"
  if [[ -z "${insert_line}" ]]; then
    return
  fi

  local tmp_file
  tmp_file="$(mktemp)"
  awk -v top_key="${top_key}" -v insert_line="${insert_line}" '
    BEGIN { in_block=0; inserted=0 }
    $0 ~ ("^" top_key ":[[:space:]]*($|#)") { in_block=1; print; next }
    in_block && $0 ~ "^[^[:space:]#]" && !inserted {
      print insert_line
      inserted=1
      in_block=0
    }
    { print }
    END {
      if (in_block && !inserted) {
        print insert_line
      }
    }
  ' "${file_path}" > "${tmp_file}"
  mv "${tmp_file}" "${file_path}"
}

merge_essential_default_config_keys() {
  local source_path="$1"
  local destination_path="$2"
  local tmp_file
  tmp_file="$(mktemp)"
  cp "${destination_path}" "${tmp_file}"

  ensure_nested_key "${tmp_file}" "${source_path}" "update" "beta"
  for key in enabled mode runtime_override documents_folder vector_db_path max_retrieved_docs chunk_size embedding_backend embedding_model; do
    ensure_nested_key "${tmp_file}" "${source_path}" "rag" "${key}"
  done

  if ! cmp -s "${tmp_file}" "${destination_path}"; then
    install -m 0644 "${tmp_file}" "${destination_path}"
    echo "Updated essential missing keys in ${destination_path} (without overriding existing values)."
  fi
  rm -f "${tmp_file}"
}

install_or_merge_default_config() {
  local source_path="$1"
  local destination_path="$2"
  if [[ ! -f "${source_path}" ]]; then
    return
  fi
  if [[ ! -f "${destination_path}" || "${FORCE_CONFIG_INSTALL:-0}" == "1" ]]; then
    install -m 0644 "${source_path}" "${destination_path}"
    return
  fi

  merge_essential_default_config_keys "${source_path}" "${destination_path}"
}

if [[ ! -f "${BIN_SOURCE}" ]]; then
  echo "Could not find ${APP_NAME} binary in bundle root: ${BIN_SOURCE}" >&2
  exit 1
fi

mkdir -p "${BIN_DIR}" "${CONFIG_DIR}" "${THEMES_DIR}" "${LOCALES_DIR}" "${ASSETS_DIR}"
install -m 0755 "${BIN_SOURCE}" "${BIN_DIR}/${APP_NAME}"

install_or_merge_default_config "${BUNDLE_DIR}/configs/default.yaml" "${CONFIG_DIR}/default.yaml"
install_config_file "${BUNDLE_DIR}/prompt-tags.yaml" "${CONFIG_ROOT}/prompt-tags.yaml"
install_config_file "${BUNDLE_DIR}/generic-prompts.yaml" "${CONFIG_ROOT}/generic-prompts.yaml"

if compgen -G "${BUNDLE_DIR}/themes/*.yaml" > /dev/null; then
  for theme_file in "${BUNDLE_DIR}"/themes/*.yaml; do
    install_config_file "${theme_file}" "${THEMES_DIR}/$(basename "${theme_file}")"
  done
fi

if compgen -G "${BUNDLE_DIR}/locales/*.yaml" > /dev/null; then
  for locale_file in "${BUNDLE_DIR}"/locales/*.yaml; do
    install_config_file "${locale_file}" "${LOCALES_DIR}/$(basename "${locale_file}")"
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

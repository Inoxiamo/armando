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
  for key in enabled mode documents_folder vector_db_path max_retrieved_docs chunk_size embedding_backend embedding_model; do
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

mkdir -p \
  "${BIN_DIR}" \
  "${CONFIG_DIR}" \
  "${THEMES_DIR}" \
  "${LOCALES_DIR}" \
  "${ASSETS_DIR}"

cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"

install -m 0755 "${RELEASE_BIN}" "${BIN_DIR}/${APP_NAME}"

install_or_merge_default_config "${CONFIG_SOURCE}" "${CONFIG_DIR}/default.yaml"
install_config_file "${PROMPT_TAGS_SOURCE}" "${CONFIG_ROOT}/prompt-tags.yaml"
install_config_file "${GENERIC_PROMPTS_SOURCE}" "${CONFIG_ROOT}/generic-prompts.yaml"

if git -C "${ROOT_DIR}" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  chmod +x "${ROOT_DIR}/.githooks/pre-commit" 2>/dev/null || true
  git -C "${ROOT_DIR}" config core.hooksPath .githooks
fi

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

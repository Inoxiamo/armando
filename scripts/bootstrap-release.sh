#!/usr/bin/env bash
set -euo pipefail

APP_NAME="armando"
REPO_OWNER="Inoxiamo"
REPO_NAME="armando"
BASE_RELEASE_URL="${ARMANDO_RELEASE_BASE_URL:-https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download}"
LATEST_API_URL="${ARMANDO_RELEASE_LATEST_API_URL:-https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest}"
INSTALL_VERSION="${1:-${ARMANDO_INSTALL_VERSION:-latest}}"
DRY_RUN="${ARMANDO_INSTALL_DRY_RUN:-0}"
DOWNLOAD_TOOL=""
TEMP_DIR=""

fail() {
  echo "$*" >&2
  exit 1
}

log() {
  echo "[${APP_NAME}-bootstrap] $*"
}

warn() {
  echo "[${APP_NAME}-bootstrap] Warning: $*" >&2
}

need_command() {
  command -v "$1" >/dev/null 2>&1
}

choose_download_tool() {
  if need_command curl; then
    DOWNLOAD_TOOL="curl"
    return
  fi

  if need_command wget; then
    DOWNLOAD_TOOL="wget"
    return
  fi

  fail "Missing downloader. Install curl or wget and try again."
}

download_to_file() {
  local url="$1"
  local destination="$2"

  case "${DOWNLOAD_TOOL}" in
    curl)
      curl -fsSL "${url}" -o "${destination}"
      ;;
    wget)
      wget -qO "${destination}" "${url}"
      ;;
    *)
      fail "No supported download tool selected."
      ;;
  esac
}

download_to_stdout() {
  local url="$1"

  case "${DOWNLOAD_TOOL}" in
    curl)
      curl -fsSL "${url}"
      ;;
    wget)
      wget -qO- "${url}"
      ;;
    *)
      fail "No supported download tool selected."
      ;;
  esac
}

resolve_version() {
  local requested="$1"

  if [[ "${requested}" != "latest" ]]; then
    printf '%s\n' "${requested#v}"
    return
  fi

  local tag
  tag="$(
    download_to_stdout "${LATEST_API_URL}" \
      | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
      | head -n 1
  )"

  [[ -n "${tag}" ]] || fail "Could not resolve the latest release version from GitHub."
  printf '%s\n' "${tag#v}"
}

resolve_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}:${arch}" in
    Linux:x86_64|Linux:amd64)
      printf '%s\n' "x86_64-unknown-linux-gnu"
      ;;
    Darwin:arm64|Darwin:aarch64)
      printf '%s\n' "aarch64-apple-darwin"
      ;;
    *)
      fail "Unsupported platform ${os}/${arch}. This bootstrap currently supports Linux x86_64 and macOS arm64."
      ;;
  esac
}

checksum_supported() {
  need_command sha256sum || need_command shasum
}

verify_checksum() {
  local checksum_file="$1"
  local archive_file="$2"

  if need_command sha256sum; then
    (
      cd "$(dirname "${archive_file}")"
      sha256sum -c "$(basename "${checksum_file}")"
    )
    return
  fi

  if need_command shasum; then
    local expected actual
    expected="$(awk '{print $1}' "${checksum_file}")"
    actual="$(shasum -a 256 "${archive_file}" | awk '{print $1}')"
    [[ "${expected}" == "${actual}" ]] || fail "Checksum verification failed for $(basename "${archive_file}")."
    return
  fi

  warn "No checksum tool available; skipping integrity verification."
}

extract_archive() {
  local archive_file="$1"
  local destination="$2"
  tar -xzf "${archive_file}" -C "${destination}"
}

main() {
  choose_download_tool

  local version target archive_name archive_url checksum_url
  version="$(resolve_version "${INSTALL_VERSION}")"
  target="$(resolve_target)"
  archive_name="${APP_NAME}-${version}-${target}.tar.gz"
  archive_url="${BASE_RELEASE_URL}/v${version}/${archive_name}"
  checksum_url="${archive_url}.sha256"

  local archive_file checksum_file bundle_dir
  TEMP_DIR="$(mktemp -d)"
  archive_file="${TEMP_DIR}/${archive_name}"
  checksum_file="${archive_file}.sha256"
  bundle_dir="${TEMP_DIR}/bundle"

  trap 'rm -rf "${TEMP_DIR}"' EXIT

  log "Preparing guided install/update for ${APP_NAME} ${version} (${target})."

  if [[ "${DRY_RUN}" == "1" ]]; then
    cat <<EOF
Dry run for ${APP_NAME} bootstrap

Version:
  ${version}

Target:
  ${target}

Archive URL:
  ${archive_url}

Checksum URL:
  ${checksum_url}
EOF
    return
  fi

  log "Downloading ${archive_name} from GitHub releases."
  download_to_file "${archive_url}" "${archive_file}"

  log "Downloading checksum."
  download_to_file "${checksum_url}" "${checksum_file}"

  if checksum_supported; then
    log "Verifying checksum."
    verify_checksum "${checksum_file}" "${archive_file}"
  else
    warn "Checksum tools not found; continuing without verification."
  fi

  mkdir -p "${bundle_dir}"
  log "Extracting release bundle."
  extract_archive "${archive_file}" "${bundle_dir}"

  [[ -f "${bundle_dir}/scripts/install.sh" ]] || fail "The downloaded bundle does not contain scripts/install.sh."
  chmod +x "${bundle_dir}/scripts/install.sh"

  log "Running bundled installer."
  (
    cd "${bundle_dir}"
    FORCE_CONFIG_INSTALL="${FORCE_CONFIG_INSTALL:-0}" ./scripts/install.sh
  )

  cat <<EOF
Guided install/update completed for ${APP_NAME} ${version}.

Binary:
  ${HOME}/.local/bin/${APP_NAME}

This bootstrap is a guided installer/updater. It does not perform silent background auto-updates.
EOF
}

main "$@"

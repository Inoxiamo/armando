#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

PROFILE="${ARMANDO_PRE_PUSH_PROFILE:-default}"

if [[ "${SKIP_PRE_PUSH_REGRESSION:-0}" == "1" ]]; then
  echo "pre-push regression checks skipped (SKIP_PRE_PUSH_REGRESSION=1)."
  exit 0
fi

run_quick() {
  cargo fmt --all -- --check
  cargo test --locked --all-targets
}

run_default() {
  run_quick
  bash scripts/ci/verify-ui-smoke-checklist.sh
}

run_full() {
  run_default
  cargo clippy --locked --all-targets -- -D warnings
}

case "${PROFILE}" in
  quick)
    echo "Running pre-push regression profile: quick"
    run_quick
    ;;
  default)
    echo "Running pre-push regression profile: default"
    run_default
    ;;
  full)
    echo "Running pre-push regression profile: full"
    run_full
    ;;
  *)
    echo "Unknown ARMANDO_PRE_PUSH_PROFILE: ${PROFILE}" >&2
    echo "Allowed values: quick | default | full" >&2
    exit 1
    ;;
esac


#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SONAR_PORT="${SONAR_PORT:-9000}"
SONAR_URL="${SONAR_URL:-http://127.0.0.1:${SONAR_PORT}}"
SONAR_ADMIN_USER="${SONAR_ADMIN_USER:-admin}"
SONAR_ADMIN_PASSWORD="${SONAR_ADMIN_PASSWORD:-admin}"
SONAR_WAIT_SECONDS="${SONAR_WAIT_SECONDS:-60}"
PROJECT_KEY="$(
  sed -n 's/^sonar.projectKey=\(.*\)$/\1/p' "${ROOT_DIR}/sonar-project.properties" | head -n 1
)"

cd "${ROOT_DIR}"

if ! command -v sonar-scanner >/dev/null 2>&1; then
  echo "sonar-scanner is required to run the local Sonar analysis" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to query SonarQube locally" >&2
  exit 1
fi

if [[ -z "${PROJECT_KEY}" ]]; then
  echo "Could not determine sonar.projectKey from sonar-project.properties" >&2
  exit 1
fi

bash "${ROOT_DIR}/scripts/dev/start-sonar-local.sh"
bash "${ROOT_DIR}/scripts/quality/export-clippy-sonar-report.sh"
bash "${ROOT_DIR}/scripts/quality/export-rust-coverage.sh"

GENERATED_TOKEN=""
TOKEN_NAME=""

revoke_generated_token() {
  if [[ -n "${GENERATED_TOKEN}" && -n "${TOKEN_NAME}" ]]; then
    curl -fsS -u "${SONAR_ADMIN_USER}:${SONAR_ADMIN_PASSWORD}" \
      -X POST \
      "${SONAR_URL}/api/user_tokens/revoke" \
      --data-urlencode "name=${TOKEN_NAME}" \
      >/dev/null || true
  fi
}

trap revoke_generated_token EXIT

if [[ -n "${SONAR_TOKEN:-}" ]]; then
  GENERATED_TOKEN="${SONAR_TOKEN}"
else
  TOKEN_NAME="armando-local-$(date +%s)"
  GENERATED_TOKEN="$(
    curl -fsS -u "${SONAR_ADMIN_USER}:${SONAR_ADMIN_PASSWORD}" \
      -X POST \
      "${SONAR_URL}/api/user_tokens/generate" \
      --data-urlencode "name=${TOKEN_NAME}" \
      | sed -n 's/.*"token":"\([^"]*\)".*/\1/p'
  )"

  if [[ -z "${GENERATED_TOKEN}" ]]; then
    echo "Failed to generate a temporary Sonar token" >&2
    exit 1
  fi
fi

echo "Running Sonar analysis for project ${PROJECT_KEY}"
rm -rf "${ROOT_DIR}/.scannerwork"
sonar-scanner \
  -Dsonar.host.url="${SONAR_URL}" \
  -Dsonar.login="${GENERATED_TOKEN}"

REPORT_FILE="${ROOT_DIR}/.scannerwork/report-task.txt"

if [[ ! -f "${REPORT_FILE}" ]]; then
  echo "Missing ${REPORT_FILE}; SonarScanner did not write task metadata" >&2
  exit 1
fi

CE_TASK_ID="$(sed -n 's/^ceTaskId=\(.*\)$/\1/p' "${REPORT_FILE}" | head -n 1)"
DASHBOARD_URL="$(sed -n 's/^dashboardUrl=\(.*\)$/\1/p' "${REPORT_FILE}" | head -n 1)"

if [[ -z "${CE_TASK_ID}" ]]; then
  echo "Could not determine Sonar compute task id" >&2
  exit 1
fi

echo "Waiting for Sonar compute task ${CE_TASK_ID}"

TASK_STATUS=""
for _ in $(seq 1 "${SONAR_WAIT_SECONDS}"); do
  TASK_STATUS="$(
    curl -fsS -u "${GENERATED_TOKEN}:" \
      "${SONAR_URL}/api/ce/task?id=${CE_TASK_ID}" \
      | sed -n 's/.*"status":"\([^"]*\)".*/\1/p'
  )"

  case "${TASK_STATUS}" in
    SUCCESS)
      break
      ;;
    PENDING|IN_PROGRESS)
      sleep 1
      ;;
    FAILED|CANCELED)
      echo "Sonar compute task finished with status ${TASK_STATUS}" >&2
      exit 1
      ;;
    *)
      sleep 1
      ;;
  esac
done

if [[ "${TASK_STATUS}" != "SUCCESS" ]]; then
  echo "Sonar compute task did not finish successfully within ${SONAR_WAIT_SECONDS}s" >&2
  exit 1
fi

QUALITY_GATE_STATUS="$(
  curl -fsS -u "${GENERATED_TOKEN}:" \
    "${SONAR_URL}/api/qualitygates/project_status?projectKey=${PROJECT_KEY}" \
    | sed -n 's/.*"status":"\([^"]*\)".*/\1/p'
)"

echo "Dashboard: ${DASHBOARD_URL}"
echo "Quality gate: ${QUALITY_GATE_STATUS}"

if [[ "${QUALITY_GATE_STATUS}" != "OK" ]]; then
  echo "Sonar quality gate failed: ${QUALITY_GATE_STATUS}" >&2
  exit 1
fi

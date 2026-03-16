#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTAINER_NAME="${SONAR_CONTAINER_NAME:-armando-sonarqube-local}"
SONAR_IMAGE="${SONAR_IMAGE:-sonarqube:9.9-community}"
SONAR_PORT="${SONAR_PORT:-9000}"
SONAR_URL="${SONAR_URL:-http://127.0.0.1:${SONAR_PORT}}"
WAIT_SECONDS="${SONAR_WAIT_SECONDS:-180}"

cd "${ROOT_DIR}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required to start SonarQube locally" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to probe the local SonarQube instance" >&2
  exit 1
fi

if docker ps --filter "name=^/${CONTAINER_NAME}$" --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
  echo "SonarQube container already running: ${CONTAINER_NAME}"
elif docker ps -a --filter "name=^/${CONTAINER_NAME}$" --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
  echo "Starting existing SonarQube container: ${CONTAINER_NAME}"
  docker start "${CONTAINER_NAME}" >/dev/null
else
  echo "Starting new SonarQube container: ${CONTAINER_NAME}"
  docker run -d \
    --name "${CONTAINER_NAME}" \
    -p "${SONAR_PORT}:9000" \
    -e SONAR_ES_BOOTSTRAP_CHECKS_DISABLE=true \
    "${SONAR_IMAGE}" \
    >/dev/null
fi

echo "Waiting for SonarQube at ${SONAR_URL}"

for _ in $(seq 1 "${WAIT_SECONDS}"); do
  status="$(
    (
      curl -fsS "${SONAR_URL}/api/system/status" 2>/dev/null \
        | sed -n 's/.*"status":"\([^"]*\)".*/\1/p'
    ) || true
  )"

  if [[ "${status}" == "UP" ]]; then
    echo "SonarQube is UP"
    exit 0
  fi

  sleep 1
done

echo "SonarQube did not become ready within ${WAIT_SECONDS}s" >&2
docker logs --tail 120 "${CONTAINER_NAME}" >&2 || true
exit 1

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
SYSROOT="$(rustc --print sysroot)"
LLVM_BIN_DIR="${SYSROOT}/lib/rustlib/${HOST_TRIPLE}/bin"
LLVM_PROFDATA="${LLVM_BIN_DIR}/llvm-profdata"
LLVM_COV="${LLVM_BIN_DIR}/llvm-cov"
ARTIFACT_DIR="${ROOT_DIR}/target/test-artifacts/coverage"
RAW_METADATA="${ARTIFACT_DIR}/cargo-test-binaries.jsonl"
EXECUTABLE_LIST="${ARTIFACT_DIR}/test-executables.txt"
PROFILE_DATA="${ARTIFACT_DIR}/merged.profdata"
LCOV_REPORT="${ARTIFACT_DIR}/rust.lcov"
SONAR_REPORT="${ARTIFACT_DIR}/sonar-generic-coverage.xml"
SUMMARY_REPORT="${ARTIFACT_DIR}/summary.txt"

cd "${ROOT_DIR}"
mkdir -p "${ARTIFACT_DIR}"
rm -f "${ARTIFACT_DIR}"/*.profraw "${RAW_METADATA}" "${EXECUTABLE_LIST}" "${PROFILE_DATA}" \
  "${LCOV_REPORT}" "${SONAR_REPORT}" "${SUMMARY_REPORT}"

if [[ ! -x "${LLVM_PROFDATA}" || ! -x "${LLVM_COV}" ]]; then
  echo "llvm-tools-preview is required; expected ${LLVM_PROFDATA} and ${LLVM_COV}" >&2
  exit 1
fi

export RUSTFLAGS="${RUSTFLAGS:-} -C instrument-coverage"
export LLVM_PROFILE_FILE="${ARTIFACT_DIR}/armando-%p-%m.profraw"

cargo test --all-targets
cargo test --all-targets --no-run --message-format=json >"${RAW_METADATA}"

python3 "${ROOT_DIR}/scripts/quality/extract_cargo_test_executables.py" \
  "${RAW_METADATA}" \
  >"${EXECUTABLE_LIST}"

mapfile -t TEST_EXECUTABLES <"${EXECUTABLE_LIST}"

if [[ "${#TEST_EXECUTABLES[@]}" -eq 0 ]]; then
  echo "Could not determine test executables for coverage export" >&2
  exit 1
fi

"${LLVM_PROFDATA}" merge -sparse "${ARTIFACT_DIR}"/*.profraw -o "${PROFILE_DATA}"

"${LLVM_COV}" export \
  --format=lcov \
  --instr-profile="${PROFILE_DATA}" \
  --ignore-filename-regex='(/\.cargo/registry|/rustc/|/target/)' \
  "${TEST_EXECUTABLES[@]}" \
  >"${LCOV_REPORT}"

"${LLVM_COV}" report \
  --instr-profile="${PROFILE_DATA}" \
  --ignore-filename-regex='(/\.cargo/registry|/rustc/|/target/)' \
  "${TEST_EXECUTABLES[@]}" \
  >"${SUMMARY_REPORT}"

python3 "${ROOT_DIR}/scripts/quality/lcov_to_sonar_coverage.py" \
  "${LCOV_REPORT}" \
  "${SONAR_REPORT}"

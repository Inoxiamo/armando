#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

declare -a TESTS=(
  "gui::tests::requested_viewport_inner_size_expands_only_smaller_axes"
  "gui::tests::main_viewport_min_size_respects_base_history_and_settings_modes"
  "gui::tests::main_viewport_min_size_clamps_extreme_heights"
  "gui::tests::editor_max_height_stays_within_a_third_of_the_viewport"
  "gui::tests::visual_layout_snapshot_matrix_matches_expected_summary"
  "gui::tests::status_section_visibility_reacts_to_any_message_or_error_state"
)

for test_name in "${TESTS[@]}"; do
  cargo test --locked "${test_name}" -- --nocapture
done

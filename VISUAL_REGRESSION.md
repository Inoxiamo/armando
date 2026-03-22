# Visual Regression

This page is the repeatable check for the popup layout and startup flow.

## Automated Checks

Run the layout-focused tests that guard the current resize and visibility rules:

```bash
cargo test gui::tests::requested_viewport_inner_size_expands_only_smaller_axes
cargo test gui::tests::main_viewport_min_size_respects_base_history_and_settings_modes
cargo test gui::tests::main_viewport_min_size_clamps_extreme_heights
cargo test gui::tests::editor_max_height_stays_within_a_third_of_the_viewport
cargo test gui::tests::visual_layout_snapshot_matrix_matches_expected_summary
cargo test gui::tests::status_section_visibility_reacts_to_any_message_or_error_state
```

You can also run the full suite if you want the broader safety net:

```bash
cargo test
```

## Manual Pass

Use the same window and content states every time:

1. Launch the app with `cargo run`.
2. Check the default popup at a medium height, around `820x540`.
3. Check a wider popup, around `1320x600`, with the settings panel open.
4. Confirm the prompt and response toolbars stay aligned to the same right inset.
5. Confirm the prompt and response editors never grow beyond roughly one third of the window height.
6. Confirm the status card only appears when there is actually attachment, dictation, or settings feedback to show.
7. Confirm the startup health card and first-run setup card appear above the prompt when appropriate.
8. Open the history panel and make sure the saved history section remains distinct from the in-memory session section.
9. Compare the `visual_layout_snapshot_matrix_matches_expected_summary` output against the same window states if you want a golden-like textual regression check.

## What To Look For

- The top-right buttons should feel aligned to the same gutter as the editors below.
- The prompt and response areas should be compact on startup, but still manually resizable downward.
- The update footer should surface a platform-specific next step when a newer release exists.
- The session history should never expose destructive select/delete affordances.
- The textual snapshot matrix should stay stable unless a deliberate layout change lands.

## Residual Risk

This harness checks the logic and the repeatable manual flow, not pixel-perfect rendering across every window manager or DPI scale.
If a desktop environment behaves differently, capture that screenshot and compare it against the same window sizes listed above.

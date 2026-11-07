# Tasks

## 1. Mix

- [x] 1.1 Add unit tests in `src/lib.rs` for the spec scenarios: halfway crossfade at 0.5, no dry component at 1/2.5/5, sign preserved across the sweep, and harmonic share rising from Drive 1 to 2 to 5. Verify the crossfade and no-dry tests fail on the current code. (Done. The sign and harmonic-share tests already pass on the old mix: its dry weight goes negative above drive 2, but the shaped term always outweighs it, so the output's sign never flips. Both stay as guards.)
- [x] 1.2 Change `apply_drive`'s mix to `shaped·m + sample·(1 − m)`, `m = drive.min(1.0)`. Verify the new tests and `tests/drive_dynamics.rs` pass, re-basing any `drive_dynamics` assertion that encoded the old mix (with the reason in its comment).
- [x] 1.3 Update the `apply_drive` doc comment, the `DRIVE_OUTPUT_CLAMP` note and the README master-drive description. Verify `cargo doc --no-deps --no-default-features` adds no warnings.

## 2. Presets

- [x] 2.1 Run `preset_levels_table`. If any preset hits the limiter or falls more than 12 dB under the median, apply the table to `PRESET_LEVELS` and regenerate with `cargo run --bin seed_presets`. Write `levels.md` in this change with before and after per drive preset. Verify `preset_levels_are_sane` and `tests/preset_roundtrip.rs` pass.

## 3. Close-out

- [x] 3.1 Mark H2 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run `cargo fmt --check`, clippy with `-D warnings`, and the full test suite (debug), and verify all are green.

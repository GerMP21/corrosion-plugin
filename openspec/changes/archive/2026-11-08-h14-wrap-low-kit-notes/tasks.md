# Tasks

## 1. Wrap below the block

- [x] 1.1 Extend the Kit-mode tests in `src/lib.rs` with the spec's three scenarios (inside, above, below the block). Verify the below-the-block scenario fails on the current code and the others pass.
- [x] 1.2 Change `note_to_kit_object` to `(note − 24).div_euclid(10).rem_euclid(9)`. Verify all Kit-mode tests pass, including `kit_mode_overrides_object_param`.
- [x] 1.3 Update the function's doc comment and the README's Kit-mode line to describe the wrap in both directions. Verify `cargo doc` adds no warnings.

## 2. Close-out

- [x] 2.1 Mark H14 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy and the full suite, and verify all are green.

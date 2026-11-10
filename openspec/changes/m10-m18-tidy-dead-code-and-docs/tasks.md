# Tasks

## 1. Tests first

- [x] 1.1 Add the `preset-loading` tests (every parameter's name, range and display against the live parameter; UI Scale and Filter Cutoff; exciter 0 and 17; exciters 1–16). Verify the display tests fail on the current code and the exciter tests pass (they pin today's fallback). Record a reference render hash of a damaged stereo scenario (friction and hit notes) from a clean worktree in `design.md` under Measurements.

## 2. Parameters and presets (M15, M18)

- [x] 2.1 Move the constructors `into_params` needs into shared `params.rs` functions and use them on both sides. Verify the 1.1 display tests and `tests/preset_roundtrip.rs` pass. *Changed (maintainer, 2026-11-10):* preset levels aren't a gate; `preset_levels_are_sane` still reports the same three placeholder presets under its floor, unchanged by this change.
- [x] 2.2 Correct the exciter range in docs and comments (`lib.rs`, `params.rs`) and document the fallback on `ExciterType::from_int`. Verify `cargo doc` adds no warnings.

## 3. Dead code and naming (M10, M11, M17)

- [x] 3.1 Remove `exciter_force` and `set_exciter_force`, update the interaction bus docs to say feed-forward; remove the redundant `active = false`; rename `highpass_noise` and document the shared rattle. Verify the render hash matches the 1.1 reference and the full suite passes.

## 4. Docs and tooling (M14, re-export, archive)

- [x] 4.1 Document quality mode's note-on versus immediate timing in the README, `docs/ARCHITECTURE.md` and the parameter's description. Verify the docs build without warnings.
- [x] 4.2 Drop the `dsp` re-export of offline specs and import them from `offline` in `src/dsp/tests.rs`; move `docs/CODE-REVIEW.md` to `docs/archive/CODE-REVIEW-2026-05-27.md` with a superseded note and fix links. Verify clippy `-D warnings` and the suite pass and no link points at the old path.

## 5. Close-out

- [x] 5.1 Mark M2, M10, M11, M14, M15, M17 (naming part; decorrelation noted as not done), M18 and M19 in `FABLE-CODE-REVIEW-2026-07-07.md`, plus the two tooling notes. Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

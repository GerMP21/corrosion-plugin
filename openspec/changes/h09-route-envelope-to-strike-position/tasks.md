# Tasks

## 1. Wander Rate parameter and migration

- [x] 1.1 Add `position_wander_rate` (0.1–5 Hz, default 0.1, skewed) to `CorrosionParams` and the preset (an optional field, migrated as `0.1 + position_envelope·4.9` with Position Envelope reset to 0 when absent). Add a migration test for the spec's 0.5 → 2.55 Hz case, and verify it passes. Verify `tests/preset_roundtrip.rs` passes.
- [x] 1.2 Pass Wander Rate through `VoiceControls`, `update_live_controls` and `set_interaction_params` in place of the derived rate. Add a test that the wander period follows Wander Rate at Position Envelope 0 and 1, and verify it passes.

## 2. Envelope routing

- [x] 2.1 Add `envelope_offset` to `InteractionState`, included in the effective position before the clamp, and set it from the voice each sample (`position_envelope · envelope · 0.5`). Add tests for the spec scenarios (0.2 → 0.7 on a Bow swell; outputs differ at Wander 0; and bit-identity at Position Envelope 0 against a render captured before the change). Verify they pass.
- [x] 2.2 Verify that `tests/no_alloc.rs` (debug), the interaction tests and the resonator reuse test pass, and that the 16-voice benchmark at Position Envelope 0 is within run-to-run noise of before. Record the numbers in the change.

## 3. Editor, docs and close-out

- [x] 3.1 Add a Wander Rate knob beside Wander, and fix the Position Envelope tooltip and docs in `src/params.rs`, `docs/ARCHITECTURE.md` (interaction bus) and the README. Verify the editor builds and `cargo doc` adds no warnings.
- [x] 3.2 Regenerate the factory presets. Verify the only diff is the new field at its default, and that `preset_levels_are_sane` passes. Mark H9 FIXED, along with "Missing features" item 6, in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy and the full suite, and verify all are green.

## 4. Host state (follow-up)

- [x] 4.1 Implement `Plugin::filter_state` so host state saved before Wander Rate existed (no `position_wander_rate`) gets `0.1 + position_envelope·4.9` Hz and Position Envelope 0, the preset migration's rule; state that has the field is left alone. Add tests for both cases and verify they pass, along with the full suite.

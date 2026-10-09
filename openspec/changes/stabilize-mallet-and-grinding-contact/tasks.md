# Tasks

## 1. Failing tests

- [x] 1.1 In `src/voice/mod.rs`, add:
  - the spec scenarios: the median-level sweep (4 and 10 against 5 and 6), the 16× drive stability sweep, and the held hard-mallet rest check;
  - a test-only drive override.

  Verify the level and stability tests fail on the current code, and record the current medians, worst cases and divergence in `design.md` under "Implementation findings".

## 2. Resonator response and contact solver

- [x] 2.1 Add `ModalResonator::contact_prediction()` (design Decision 1), with a unit test: the displacement predicted for a given force matches the next `process_sample` within 1e-6 relative, for a coupled and an uncoupled strike position. Verify the test passes and `tests/no_alloc.rs` passes in debug.
- [x] 2.2 Add `dsp::utils::contact` with the safeguarded monotone solver (design Decision 2), with unit tests against a brute-force solve, for a Hertzian and a tanh friction law, including zero force and saturated cases. Verify the tests pass.

## 3. Exciters

- [x] 3.1 HardMallet (design Decision 3): Hunt–Crossley contact solved with the prediction, gravity only before the first contact, leaving after the strike. The voice passes the scaled compliance to exciter 4. Verify:
  - the stability sweep passes for exciter 4;
  - the rest check passes;
  - `struck_attack_timing_does_not_depend_on_sample_rate` passes, and record whether HardMallet still needs its 20 ms window.
- [x] 3.2 HeavyGrinding (design Decision 4): tanh friction solved with the prediction, and bounded tearing. The voice passes the scaled compliance to exciter 10. Verify the stability sweep passes for exciter 10.

## 4. Levels

- [x] 4.1 Re-tune `EXCITER_DRIVE_TRIM[4]` and `[10]` with `print_exciter_overdrive` (design Decision 5). Verify:
  - the median-level test passes;
  - `default_notes_do_not_overdrive_the_voice`, `every_exciter_has_the_same_level_at_any_sample_rate` and `struck_attack_timing_does_not_depend_on_sample_rate` pass;
  - `tests/no_alloc.rs` passes in debug.

  Record the new trims, medians and worst cases. Record any tolerance change, with the design Risks rule and its numbers.
- [x] 4.2 Update `docs/ARCHITECTURE.md` (the drive-trim section's HardMallet/HeavyGrinding note, and the contact prediction and solver). Update `openspec/changes/resolve-exciter-tradeoffs/tasks.md` so its contact tasks build on this change. Verify `cargo doc --no-deps --no-default-features` gives no new warnings.

## 5. Integration

- [x] 5.1 Run the full gate:
  - debug and release tests, and the gnu lib tests;
  - fmt;
  - clippy on 1.99, and on 1.97 with `-A unknown_lints`;
  - the render smoke test with `scripts/check_wav.py`;
  - `preset_levels_are_sane`. If the louder exciters push any preset into the limiter, re-run `preset_levels_table` and apply it.

  Verify all green and record it in the findings.

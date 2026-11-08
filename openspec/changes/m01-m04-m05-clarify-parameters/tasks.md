# Tasks

## 1. Tests first

- [ ] 1.1 Add tests for the spec scenarios: separate onset time and level, default friction note bit-identical (against a recorded reference from the current code), old preset and old project onset migration, unique names, Output display and typed value, taper positions, stored Output gain. Verify they fail on the current code where they should (all but the bit-identity reference).

## 2. Onset Level (M1)

- [ ] 2.1 Add `mseg_onset_level` (default 0.01) to `CorrosionParams`, `VoiceControls` and the friction MSEG wiring; rename Onset's display to "Onset Time". Verify the onset tests and the default bit-identity test pass.
- [ ] 2.2 Add the field to presets with the `migrate_preset_json` rule, and the `filter_state` rule beside `migrate_host_wander_rate`. Verify both migration tests and `tests/preset_roundtrip.rs` pass.
- [ ] 2.3 Add the Onset Level knob to the editor and its randomizer range. Verify the editor-controls coverage test and the randomizer tests pass, and regenerate the factory bank with `cargo run --bin seed_presets` (`preset_levels_are_sane` still passes).

## 3. Names and Output (M4, M5)

- [ ] 3.1 Rename the duplicated display names in `params.rs` and `into_params`. Verify the unique-names test passes.
- [ ] 3.2 Give Output the skewed gain range, dB unit and formatters in `params.rs` and `into_params`. Verify the display, typed-value, taper and stored-gain tests pass, and that `no_alloc` and the preset level tests pass.

## 4. Docs and close-out

- [ ] 4.1 Document Onset Level, the new names and the Output taper (with the BREAKING automation note) in the README and `docs/ARCHITECTURE.md`; mark M1, M4 and M5 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

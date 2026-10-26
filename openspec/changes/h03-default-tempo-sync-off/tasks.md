# Tasks

## 1. Default and display

- [ ] 1.1 Factor the Off threshold and the band index out of `sync_rate_to_delay_time` into one shared function. Verify the existing sync tests pass unchanged.
- [ ] 1.2 Add tests for the spec scenarios: fresh-instance default (sync off, Delay Time effective at 120 BPM); display "Off"/"1/8"/"2/1" at 0.0/0.3/1.0 with the matching delays; and the 0.01-step sweep where display matches behaviour. Verify the default and display tests fail on the current code.
- [ ] 1.3 Set the `sync_rate` default to 0.0 and give it a `value_to_string` formatter from the shared function (with a matching `string_to_value`). Verify all the tests from 1.2 pass.

## 2. Presets and close-out

- [ ] 2.1 Regenerate the factory presets with `cargo run --bin seed_presets`. Verify only `sync_rate` changed (0.25 → 0.0, with Hanging Chain Echo still at 0.5), and that `tests/preset_roundtrip.rs` and `preset_levels_are_sane` pass.
- [ ] 2.2 Mark H3 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`, noting that its GUI part is in H5. Run fmt and clippy, and verify both are clean.

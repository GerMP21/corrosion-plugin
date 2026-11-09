# Tasks

## 1. Tests first

- [x] 1.1 Add the `diode-stage` curve tests (hard knee at 0, wide knee at 1, fewer high harmonics as Softness rises, peak-to-peak per Asymmetry, even harmonics, DC under 0.001 of the ceiling, symmetric curve unfiltered, peaks under the ceiling in every mode, Eco sounds like Render, no click when Asymmetry jumps) and the updated `output-clipper` transparency and Eco scenarios. Verify which fail on the current code, and record today's values in `design.md` under Measurements.
- [x] 1.2 Add the migration tests (old preset file, old host state, current state untouched, new-instance defaults) and the editor dot tests (idle, full at 6 dB, release within a second), using a test-only hook for the dot's drawn colour. Verify they fail on the current code.

## 2. The curve

- [x] 2.1 Replace `diode_clip` with the per-polarity tanh knee (decisions 1–3), with smoothed Softness and Asymmetry and constants recomputed only while they move. Use the same limit in every quality mode. Verify the curve tests that don't depend on DC removal or headroom pass.
- [x] 2.2 Add the 2 Hz DC blocker (5 Hz in the first draft, see decision 5) with its asymmetry-weighted fade (decision 5). Verify the DC, unfiltered-symmetric and no-click tests pass.
- [x] 2.3 Pick the tanh evaluation (table or rational approximation, decision 7) by comparing `plugin_output_path_sixteen_voices_render` within one run; check the error bound with a test.

## 3. Headroom and cost

- [x] 3.1 Sweep the headroom `H` (decision 4) and set it. Verify the peak test in every mode at Asymmetry 0, 0.25 and 1, and the `output-clipper` alias bounds (alone and through the hard clamp) at Softness 0 and at the defaults. Record the sweep. Stop and ask if `H` must exceed 1.3 or one constant can't satisfy both.
- [ ] 3.2 Measure `plugin_output_path_sixteen_voices` and its Render twin against a clean worktree (median of 5 within-run ratios). Record them; stop and ask if Render is more than 20 % slower.

## 4. Parameter, migration and editor

- [ ] 4.1 Add `diode_asymmetry` (0–1, default 0.25, display "Diode Asymmetry") to the parameters, the preset `extra` block and `into_params`, and wire it to the chain. Add the `migrate_preset_json` and `filter_state` rules. Verify the migration tests and `tests/preset_roundtrip.rs` pass.
- [ ] 4.2 Add the shared reduction meter (decision 8): peaks per block in the clipper, published by the plugin, taken and decayed by the editor. Verify `tests/no_alloc.rs` passes in debug.
- [ ] 4.3 Add the Asymmetry fader to the brickwall panel and drive the dot from the meter. Verify the dot tests and the editor-controls coverage test pass; update the editor content-height expectations if the panel grows.
- [ ] 4.4 Regenerate the factory bank with `cargo run --bin seed_presets`. Verify `preset_levels_are_sane` and the preset tests pass.

## 5. Docs and close-out

- [ ] 5.1 Document the diode stage as a feature in the README (Softness, Asymmetry, the dot, the same in every quality mode, the BREAKING Softness automation note) and in `docs/ARCHITECTURE.md` (output protection, quality modes). Run fmt, clippy `-D warnings`, the full suite on gnu and musl, and `cargo doc`, and verify all are green.

## Workflow follow-up

- ~~Archive `m06-m09-smooth-starvation-and-oversample-clipper` before archiving this change~~: done 2026-11-09; `openspec/specs/output-clipper/` exists for this change's MODIFIED delta.
- Judge the knee by ear once built (design Open Questions) and adjust the defaults if needed before archive.

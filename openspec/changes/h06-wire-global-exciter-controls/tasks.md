# Tasks

## 1. Inventory and tests

- [x] 1.1 Read each exciter and list its motion-speed and grit/randomness parameters, with ranges. Record the final lists in this change's design (Decisions 2 and 3). Verify every exciter is either listed or noted as inert for that control.
- [x] 1.2 Add voice tests for the spec scenarios:
  - default bit-identity, for all 16 exciters;
  - Pressure levels at 0.25/0.5/0.75;
  - Pressure automated on a held Bow;
  - Speed on Bow, and Roughness on HeavyGrinding;
  - scaled parameters within range at the extremes;
  - the no-overdrive sweep at 1/1/1.

  Verify the Pressure, Speed and Roughness tests fail on the current code, and that the bit-identity test passes.

## 2. Pressure

- [x] 2.1 Add the Pressure factor to `force_scale`, with a smoothed live value pushed through `update_live_controls`. Verify the Pressure tests, the default bit-identity test, and the existing level, overdrive, governor and sample-rate tests all pass.

## 3. Speed and Roughness

- [x] 3.1 Add the helper that scales the dedicated controls at note-on (Decisions 2–4, clamped to ranges). Verify the Speed, Roughness, in-range and no-overdrive tests pass, along with the default bit-identity test.

## 4. Editor, docs and close-out

- [x] 4.1 Add Pressure, Speed and Roughness knobs beside the exciter selector (and take them off H5's host-only list if that exists). Verify the editor builds with the GUI feature.
- [x] 4.2 Update the parameter docs in `src/params.rs` (ranges, neutral points, which exciters each affects) and the exciter section in `docs/ARCHITECTURE.md`. Verify `cargo doc` adds no warnings.
- [ ] 4.3 Mark H6 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy and the full test suite, and verify all are green.

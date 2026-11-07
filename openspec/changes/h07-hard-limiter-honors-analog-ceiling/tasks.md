# Tasks

## 1. Ceiling in Hard mode

- [ ] 1.1 Add tests for the spec scenarios: Hard mode at ceiling 0.5 with output gain 4; Hard and Lookahead at ceiling 0.7 (peak ≤ 0.7, reaching ≥ 0.65); and a factory preset bit-identical at 0.9661. Verify the low-ceiling and both-modes tests fail on the current code.
- [ ] 1.2 Change `apply_output_limiter` to take the ceiling, pass the per-buffer threshold in `process`, and update the test renderer and `src/bin/render_presets.rs`. Verify the new tests, the doc test, and the preset level tests pass.
- [ ] 1.3 Update the `analog_ceiling` parameter doc and the output-stage comment to say it is the output ceiling in both modes. Verify `cargo doc` adds no warnings.

## 2. Close-out

- [ ] 2.1 Mark H7 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy and the full test suite, and verify all are green.

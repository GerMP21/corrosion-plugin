# Tasks

## 1. Bias Starvation

- [ ] 1.1 Add the starvation tests (no-clicks bound at 48 kHz, starvation-off bit-identity, the bound at 44.1 and 96 kHz). Verify the no-clicks tests fail on the current code and the bit-identity test passes.
- [ ] 1.2 Smooth the gate with a 5 ms one-pole follower (skipped at starvation 0) and remove `tube_state`. Verify the 1.1 tests and the existing `post-chain-drive` tests pass.

## 2. Clipper tests and filters

- [ ] 2.1 Add a clipper-only measurement and the `output-clipper` tests (transparency per quality, Eco plain clipping, hard-driven sine bounds, quality ordering, delay at 1 kHz). Record today's values for each in `design.md` under Measurements, and verify which fail.
- [ ] 2.2 Implement the half-band IIR designer and a 2× up/down stage, with a test that checks each stage's stop-band rejection (about 90 dB) and pass-band flatness directly. Verify it passes.

## 3. Clipper

- [ ] 3.1 Rebuild `OversampledClipper` as up → diode curve → down over 2, 3 or 4 stages per quality mode (Eco unchanged), with per-channel fixed state; drop the unused `sample_rate`. Verify the 2.1 tests pass, the existing aliasing tests pass (tighten Render's budget to the new value with margin), and `tests/no_alloc.rs` passes in debug.
- [ ] 3.2 Measure `plugin_output_path_sixteen_voices` against a clean worktree (median of 5 within-run ratios) in Normal and Render. Record the numbers; if Render is more than 20 % slower, stop and ask before continuing.

## 4. Docs and close-out

- [ ] 4.1 Update `docs/ARCHITECTURE.md` (post chain and output protection) and the README quality-modes line; mark M6 and M9 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

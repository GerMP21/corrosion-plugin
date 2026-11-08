# Tasks

## 1. Tests first

- [ ] 1.1 Add the echo tests: glide (sine step bound), no-modulation impulse, highs-fade-lows-stay, darker-each-repeat and same-tone-at-every-rate. Verify the glide and HF Damping tests fail on the current code, and the no-modulation one and the existing `space-effects` tests pass.

## 2. Echo

- [ ] 2.1 Replace the rounded read with a 4-point Hermite read. Verify the glide and no-modulation tests pass, and the existing timing tests (Echo delay at every rate, Delay Time sweep, synced divisions) still pass.
- [ ] 2.2 Replace the broadband damping with the in-loop one-pole low-pass (20 kHz → 1.5 kHz, bypassed at 0) and the fixed 0.7 wet level. Verify the three HF Damping tests pass and `tests/no_alloc.rs` (debug) passes. Record the measured repeat levels in `design.md` under Measurements.

## 3. Spread

- [ ] 3.1 Remove the redundant spread buffer and rewrite the comments to describe the mid delay. Verify bit-identity with a one-off render hash of a Width 1 stereo scenario against a clean worktree, and record the hashes in Measurements.

## 4. Docs and close-out

- [ ] 4.1 Update the README's Echo controls and `docs/ARCHITECTURE.md`'s post-chain section; mark M7 and M8 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

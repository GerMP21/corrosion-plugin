# Tasks

## 1. Reproduce

- [x] 1.1 Add a unit test in `src/dsp/post_processing/wdf_filter.rs` that runs the filter at 48 kHz, cutoff 20 kHz, 0.1-amplitude 1 kHz sine for 1 s then 1 s silence, and asserts finite output within ±1.0 and |mean of last 100 ms| < 1e-3; verify it FAILS on the current code (`cargo test --no-default-features wdf`)
- [x] 1.2 Add a unit test sweeping cutoff through 0.24·fs, 0.25·fs, 0.26·fs, 0.375·fs and 20 kHz at 44.1 kHz and 48 kHz, asserting no NaN/inf and decay to within ±1e-3 within 1 s of silence; verify it FAILS on the current code

## 2. Fix coefficient computation

- [x] 2.1 In `update_coefficients()`, compute `fc_eff = cutoff_hz.min(0.49 * sr)` with `sr = sample_rate.max(1.0)` and set `g = (PI * fc_eff / sr).tan()`; verify tests 1.1 and 1.2 now pass
- [x] 2.2 Add a unit test for the Nyquist clamp (cutoff 20 kHz at 32 kHz sample rate, and cutoff set before `set_sample_rate` lowers fs) asserting stability; verify it passes
- [x] 2.3 Add a unit test measuring the per-stage −3 dB point at 1 kHz / 48 kHz (expect 900–1100 Hz) and a test that 8 kHz cutoff passes more >4 kHz energy than 2 kHz; verify both pass
- [x] 2.4 Add a unit test: full-scale input at 20 kHz, `reset()`, then silence → every output sample is exactly 0.0; verify it passes
- [x] 2.5 Run the existing filter and post-chain tests; re-base any assertion that encoded the old (doubled) cutoff mapping, documenting the reason in the test comment; verify `cargo test --no-default-features --release` is fully green

- [x] 2.6 Replace each ladder stage with a unity-DC-gain TPT one-pole (linear integrator state, saturator on the stage output, resonance fed from the previous saturated output) after the default note measured ≈ −42 dBFS peak; verify `hosted_normalized_note_velocity_reaches_output_chain` passes (peak ≈ 0.118)
- [x] 2.7 Add unit tests for unity passband gain (100 Hz at 20 kHz cutoff within 0.97–1.01) and bounded output at resonance 1.0 across cutoffs and 44.1/48/96 kHz; verify both pass

## 3. Integration guard

- [x] 3.1 Add an integration test in `tests/` that drives `PostProcessingChain` with default parameters at 48 kHz (note-like burst then silence) and asserts no DC offset and finite, bounded output; verify it passes
- [x] 3.2 Update the module doc comment in `wdf_filter.rs` to state the prewarp formula, the 0.49·fs clamp, and the unity-gain TPT stage that replaced the old 0.5-DC-gain stage; verify `cargo doc --no-deps --no-default-features` builds without new warnings

## 4. Close out

- [x] 4.1 Run `cargo fmt --check` and `cargo clippy --no-default-features --all-targets`; verify no new warnings
- [x] 4.2 Update `FABLE-CODE-REVIEW-2026-07-07.md` C1 status from IN PROGRESS to FIXED with the commit reference, and note the corrected "octave above, not below" observation; verify the heading and priority list reflect the new status

# Tasks

## 1. Failing tests

- [ ] 1.1 Extract the Space stage of `PostProcessingChain::process` into `process_space(left, right) -> (wet_l, wet_r)` with no behaviour change (existing post-chain tests stay green), then add spring tests against it in `src/dsp/post_processing/post_chain.rs`: left-only impulse leaves the right wet output exactly 0.0; with a known `delay_length` N, each channel's first wet sample lands N samples after a stereo impulse; verify both FAIL on the current code
- [ ] 1.2 Add a mono-input test (identical L/R → identical wet tails); record whether it passes on the current code (it may, by symmetry), so the result is documented

## 2. Fix

- [ ] 2.1 Replace the single `spring_reverb` in `PostProcessingChain` with `spring_left` / `spring_right`, fanning out `set_spring_params`, `set_sample_rate` and `reset`, and process each channel through its own instance in Spring mode; verify 1.1 and 1.2 pass
- [ ] 2.2 Make `SpringReverb::reset()` also zero `delay_index`, and add the "reset clears both channels" test (ringing tail → reset → silence gives exactly 0.0); verify it passes
- [ ] 2.3 Run the debug `tests/no_alloc.rs` suite (reset with sounding voices, process callback); verify it still passes

## 3. Close out

- [ ] 3.1 Render the two factory presets that use Space = Spring through the post chain before and after, and note the audible change (echo spacing, cross-talk) in the change summary; verify the presets are named
- [ ] 3.2 Run `cargo test --workspace --no-default-features` (debug), `cargo test --no-default-features --release`, `cargo fmt --check`, and `cargo clippy --workspace --all-targets --no-default-features -- -D warnings`; verify all green
- [ ] 3.3 Update `FABLE-CODE-REVIEW-2026-07-07.md` C6 status from IN PROGRESS to FIXED with a short summary; verify the heading and priority list reflect it

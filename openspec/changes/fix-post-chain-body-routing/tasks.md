# Tasks

## 1. Baselines

- [ ] 1.1 Record, on the current code, the default-note peak from `hosted_normalized_note_velocity_reaches_output_chain` and each factory preset's post-chain output RMS plus its body-to-dry energy ratio (a small offline harness or test under `#[ignore]`); save the numbers in `openspec/changes/fix-post-chain-body-routing/baseline.md`; verify the file lists all 50 presets
- [ ] 1.2 Add failing routing tests in `src/dsp/post_processing/post_chain.rs`: hard-left input stays ≥ 20 dB louder on the left (BODY 0, spread width 0, space off); BODY 0 output is unchanged as Chassis Volume sweeps 0 → 1; verify both FAIL on the current code

## 2. Routing

- [ ] 2.1 Split filter and drive into left/right instances in `PostProcessingChain`, fanning out setters, `set_sample_rate` and `reset`; feed each channel through its own pair; verify a mono input still gives L = R and the hard-left test passes
- [ ] 2.2 Make the body parallel: body runs on the mono sum of the driven channels, and its output is added to both channels at a new smoothed `body_amount` (`set_body_amount`, ≈20 ms one-pole, first-call snap); verify the "BODY 0 equals no-body chain" test passes
- [ ] 2.3 Remove the `(0.5 + 0.5·volume)` output gain from `FemBodyResonator::process` (volume keeps tuning the modes), and add the "larger chassis resonates lower" and "no volume-dependent gain" tests from the spec; verify they pass
- [ ] 2.4 In `src/lib.rs`, route `chassis_volume` to `set_body_params` alone and `body / 5` to `set_body_amount` (dropping the `max()`); update the `body` and `chassis_volume` doc comments in `src/params.rs`; verify the "Chassis Volume does not override BODY" and "body level tracks BODY ×4 (±5 %)" tests pass (measured at low amplitude with space off so the clipper stays linear)
- [ ] 2.5 Add the spec's cutoff test (noise, 20 kHz → 1 kHz cutoff drops > 4 kHz energy by ≥ 20 dB, BODY at factory value); verify it passes

## 3. Eco

- [ ] 3.1 Delete the Eco early return so every quality mode runs body, spread and space; replace the existing "Eco differs" test with the spec's "Eco keeps the reverb" (tail energy within 3 dB of Normal) and "Eco keeps the body" tests; verify they pass and `offline::tests::higher_quality_reduces_alias_ratio` still passes
- [ ] 3.2 Run `cargo bench --no-default-features --bench performance` for Eco before/after and record the delta in `baseline.md`; verify the numbers are recorded

## 4. Re-voice presets

- [ ] 4.1 Using the 1.1 body-to-dry ratios, choose the single `body` value whose new output best matches the old ratio across presets; record it and the per-preset residual in `baseline.md`; verify the chosen value is documented
- [ ] 4.2 Set that value for every preset in `src/bin/seed_presets.rs` and as the `body` parameter default in `src/params.rs`; regenerate with `cargo run --bin seed_presets`; verify 50 presets are rewritten and `tests/preset_roundtrip.rs` passes
- [ ] 4.3 Re-measure the default-note peak and preset RMS; verify the default peak is within ±6 dB of 0.118 and no preset exceeds its old limiter-hit count; record the results in `baseline.md`

## 5. Docs and close out

- [ ] 5.1 Update README (Quality line: Eco no longer bypasses; Post chain line: dual-mono filter/drive, parallel body, BODY / Chassis Volume meaning) and `docs/ARCHITECTURE.md` (signal-flow diagram, Eco row); verify no remaining "Eco … bypasses" text with `grep -rn "bypass" README.md docs/ARCHITECTURE.md`
- [ ] 5.2 Run `cargo test --workspace --no-default-features` (debug), `cargo test --no-default-features --release`, the `tests/no_alloc.rs` suite in debug, `cargo fmt --check`, and `cargo clippy --workspace --all-targets --no-default-features -- -D warnings`; verify all green
- [ ] 5.3 Update `FABLE-CODE-REVIEW-2026-07-07.md` C5 status from IN PROGRESS to FIXED with a short summary (noting HRTF spread's mono remix remains open); verify the heading and priority list reflect it

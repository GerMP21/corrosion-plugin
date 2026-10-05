# Tasks

## 1. Failing tests

- [x] 1.1 Add envelope unit tests in `src/dsp/envelopes/mod.rs`: sustain level 0 at velocity 0 outputs exactly 0.0; `velocity_to_level = 0` with sustain 0.5 outputs 0.5 at any velocity; level 1.0 at velocity 0 with `vtl = 1` outputs 0.5; verify they FAIL on the current code (`cargo test --no-default-features --lib envelopes`)
- [x] 1.2 Add a release-continuity test at velocity 0.3: the step from the last Release sample to the first Idle sample is ≤ the largest sample-to-sample change within Release; verify it FAILS on the current code

## 2. Fix

- [x] 2.1 Replace the velocity formula in `process_sample()` with `level · ((1 − vtl) + vtl · velocity_scale)` (clamped to [0, 1]) · `env_amount`, and apply the same gain on the Release → Idle return; verify the 1.1 and 1.2 tests pass
- [x] 2.2 Before touching the golden hashes, run the C2 tests other than the golden one (Sustain-start deadline, loop-end semantics, empty region, release from loop, exhaustive sweep, transition cap); verify they all pass unchanged
- [x] 2.3 Confirm the full-velocity golden case (`scrape_envelope()` at velocity 1.0) keeps its exact pre-fix hash *(it doesn't: f32 rounding moves it by ≤ 1.5e-8; replaced by the 1e-7 `full_velocity_matches_previous_formula` test — see design findings)*, then re-capture the golden hashes in `default_and_drone_loops_are_bit_identical_to_pre_fix_output` (renamed to reflect the C4 baseline), and say why in its comment; verify it passes
- [x] 2.4 Update the existing `velocity_affects_level` test so it asserts the multiplicative scale (peak ratio between velocities 0.3 and 1.0 ≈ (0.5 + 0.15) / 1.0), not just that the peaks differ; verify it passes

## 3. Close out

- [x] 3.1 Run `cargo test --workspace --no-default-features` (debug, as CI does), `cargo test --no-default-features --release`, `cargo fmt --check`, and `cargo clippy --workspace --all-targets --no-default-features -- -D warnings`; verify all green
- [x] 3.2 Update `FABLE-CODE-REVIEW-2026-07-07.md` C4 status from IN PROGRESS to FIXED with a short summary; verify the heading and priority list reflect it

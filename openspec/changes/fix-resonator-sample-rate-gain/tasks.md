# Tasks

## 1. Sample-rate-independent mode gain

- [ ] 1.1 Before changing any code, add a 48 kHz golden hash test of a resonator render: Tank, Pipe, Plate and Cog at C4, a 1 ms pulse plus a 1 s sine drive, default controls. Also add the spec's two scenario tests (pulse peak and sustained RMS at 44.1/48/96/192 kHz within 0.5 dB of 48 kHz) in `src/dsp/resonators/core.rs`. Verify the golden test passes and both scenario tests FAIL (96 kHz +12 dB).
- [ ] 1.2 Scale `b0` by `(REFERENCE_SAMPLE_RATE / fs)²` in `ResonatorCoefficients::for_mode`, with a doc comment explaining why. Verify that the scenario tests pass, and that the golden hash, `mode_banks_match_golden_fixture` and the whole 48 kHz suite pass unchanged.

## 2. Voice-level check and docs

- [ ] 2.1 Measure one struck (HandStrike) and one bowed note through the full voice for Tank, Pipe and Plate at 44.1, 48 and 96 kHz, before and after the change (an ignored measurement test). Record the per-rate level differences that remain in `design.md` under "Implementation findings", naming which per-sample constants cause them, as input for `fix-sample-rate-time-constants`. Verify the table is recorded.
- [ ] 2.2 Document the sample-rate normalization in `docs/ARCHITECTURE.md` (resonator section) and the module docs. Verify that `cargo doc --no-deps --no-default-features` gives no new warnings.

## 3. Integration

- [ ] 3.1 Run `cargo test --workspace --no-default-features` (debug), `cargo test --no-default-features --release`, `cargo fmt --check`, clippy `-D warnings` on Rust 1.97 and 1.99, the render CLI smoke test with `scripts/check_wav.py`, and `c5_verify_revoiced_presets`. Verify all green.

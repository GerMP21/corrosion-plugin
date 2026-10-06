# Tasks

## 1. Measurements and failing tests

- [x] 1.1 Confirm `fix-resonator-sample-rate-gain` has landed. Then add the spec's resonator-level tests (pitch C2/C4/C6 within 3 dB; all objects within 6 dB; Eco vs Render and damage 0 vs 1 within 2 dB, each with a 1 ms pulse at 48 kHz) in `src/dsp/resonators/core.rs`, plus the voice overdrive sweep in `src/voice/mod.rs`. Verify they FAIL on the current code and record the measured spreads in `design.md` under "Implementation findings".

## 2. Bank normalization

- [x] 2.1 Measure the default patch's bank `L = Σ gain_i / f_i` (Pipe, C4, default controls) and define `L_REF`. Scale each bank by `L_REF / L` at note-on in the shared rebuild path and the constructors, without allocation. Verify that the pitch, object, quality and damage tests pass, that `tests/no_alloc.rs` passes in debug, and that `rebuilding_a_reused_resonator_matches_a_fresh_one` still holds. Re-capture `tests/fixtures/mode_bank_hashes.txt`, saying why in the test comment.
- [x] 2.2 Run the voice overdrive sweep. If any bowed or specialty case exceeds ±1.0, apply the fallback from design Risks (decay-weighted bound or a friction makeup cap). Record which, with numbers, under Implementation findings. Verify the sweep passes.
- [x] 2.3 Verify `hosted_normalized_note_velocity_reaches_output_chain` (±6 dB of 0.118) still passes, and re-base the other level-sensitive tests: compare each failure against the new levels instead of loosening it blindly, and list the re-based tests in the findings.
- [x] 2.4 Precision fix handed over by `fix-resonator-sample-rate-gain`: run the modes in a better-conditioned form, so low, long-ringing modes keep their level at 192 kHz. Verify the sustained-level test passes at 192 kHz.

## 3. Placeholder presets

- [ ] 3.1 Render every factory preset through the normalized engine. Apply design Decision 4: trim `output` only for limiter hits or for presets more than 12 dB below the median, through `C5_REVOICE` in `src/bin/seed_presets.rs` (renamed to a neutral `PRESET_LEVELS`). Regenerate with `cargo run --bin seed_presets`, and record before/after levels in `openspec/changes/normalize-object-loudness/levels.md`. Verify `tests/preset_roundtrip.rs` passes.
- [ ] 3.2 Replace `c5_verify_revoiced_presets` and `c5_revoice_table` with an ignored `preset_levels_are_sane` check (no limiter hits; nothing more than 12 dB below the median), and remove the now-unused `tests/fixtures/c5_preset_baseline.md` and its loader. Verify the new check passes and nothing else references the fixture.

## 4. Docs and integration

- [ ] 4.1 Update the README (loudness and placeholder presets) and `docs/ARCHITECTURE.md` (bank normalization; `decay_seconds` is a 1/e time constant, so T60 ≈ 6.9·τ). Note in `FABLE-CODE-REVIEW-2026-07-07.md` that the Tank overdrive and clipping found during the voice-stealing work is fixed. Verify with `grep -rn "0.5 dB of" README.md docs` that no stale level guarantee remains.
- [ ] 4.2 Run the full gate: debug and release tests, `tests/no_alloc.rs` in debug, fmt, clippy on 1.97 and 1.99, the render smoke test with `scripts/check_wav.py`, and `preset_levels_are_sane`. Verify all green.

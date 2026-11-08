# Tasks

## 1. Tests first

- [x] 1.1 Add the latency tests with a test `InitContext` that records `set_latency_samples` calls: Lookahead at initialization, Hard at initialization, and a Hard→Lookahead change between blocks. Verify the initialization scenario fails on the current code and the mode-change one passes.
- [x] 1.2 Add the fresh-versus-primed render test (96 kHz, Pipe C4, all 16 exciters) and the bend-before-first-sample test (44.1 kHz). Record which of them fail on the current code, and for which exciters, in `design.md` under Measurements.

## 2. Implement

- [x] 2.1 Report latency in `initialize` from the restored Limiter Mode and seed `reported_latency_samples`. Verify the 1.1 tests pass.
- [x] 2.2 Add `VoiceManager::set_sample_rate`, call it from `initialize` and from `begin_block` on a rate change, and verify `tests/no_alloc.rs` passes in debug with a rate change inside the checked zone.
- [x] 2.3 Pass the voice's rate into `rebuild_in_place` instead of 48 000. Verify the 1.2 tests pass, the golden mode-bank fixture matches unchanged, and the sample-rate suites pass.
- [x] 2.4 Compare a 48 kHz render hash of `plugin_output_path_sixteen_voices`'s scenario against a clean worktree, and count coefficient builds per note-on at 96 kHz before and after (temporary counter). Record both in Measurements.
- [x] 2.5 *(Added: the maintainer chose to fix it here.)* Restart HeavyGrinding, PneumaticJet, ElectromagneticHum and ParticleRain from rest on every trigger (noise filter states, hum phase, particle accumulator and in-flight particles), so the fresh-versus-primed test passes for all 16 exciters. Verify `tests/no_alloc.rs` covers a retrigger on used voices.

## 3. Docs and close-out

- [x] 3.1 Update `docs/ARCHITECTURE.md` (sample-rate rule: voices are rated at initialization; latency reporting) and mark M3 and M20 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

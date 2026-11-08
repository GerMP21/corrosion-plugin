# Tasks

## 1. Tests first

- [ ] 1.1 Add the latency tests with a test `InitContext` that records `set_latency_samples` calls: Lookahead at initialization, Hard at initialization, and a Hard→Lookahead change between blocks. Verify the initialization scenario fails on the current code and the mode-change one passes.
- [ ] 1.2 Add the fresh-versus-primed render test (96 kHz, Pipe C4, all 16 exciters) and the bend-before-first-sample test (44.1 kHz). Record which of them fail on the current code, and for which exciters, in `design.md` under Measurements.

## 2. Implement

- [ ] 2.1 Report latency in `initialize` from the restored Limiter Mode and seed `reported_latency_samples`. Verify the 1.1 tests pass.
- [ ] 2.2 Add `VoiceManager::set_sample_rate`, call it from `initialize` and from `begin_block` on a rate change, and verify `tests/no_alloc.rs` passes in debug with a rate change inside the checked zone.
- [ ] 2.3 Pass the voice's rate into `rebuild_in_place` instead of 48 000. Verify the 1.2 tests pass, the golden mode-bank fixture matches unchanged, and the sample-rate suites pass.
- [ ] 2.4 Compare a 48 kHz render hash of `plugin_output_path_sixteen_voices`'s scenario against a clean worktree, and count coefficient builds per note-on at 96 kHz before and after (temporary counter). Record both in Measurements.

## 3. Docs and close-out

- [ ] 3.1 Update `docs/ARCHITECTURE.md` (sample-rate rule: voices are rated at initialization; latency reporting) and mark M3 and M20 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

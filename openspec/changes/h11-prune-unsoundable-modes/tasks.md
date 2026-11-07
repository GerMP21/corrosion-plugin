# Tasks

## 1. Tests

- [x] 1.1 Add the spec tests: the small high Pipe at Render; the sweep over every object, quality and damage for the 80 kHz and 0.1 ms limits; the 24-semitone retune keeping the bank. Add a level test comparing the pruned and full banks (behind a test-only switch) with HandStrike at velocity 100. Verify the limit tests fail on the current code, and record how many modes each object drops.

## 2. Prune

- [x] 2.1 Add the prune step (`retain` on `transformed_specs` with the two limits) between the transforms and the bank fill, with the test-only switch. Verify the limit and retune tests pass, the level test is within 0.5 dB, and the loudness tests (`objects_are_comparably_loud`, `loudness_does_not_depend_on_pitch`, `quality_mode_and_damage_do_not_change_loudness`, sustained level) pass.
- [x] 2.2 Re-capture `tests/fixtures/mode_bank_hashes.txt` with the reason in its header comment. Verify it matches on musl and gnu, and that `tests/no_alloc.rs` passes in debug.
- [ ] 2.3 Run the 16-voice benchmarks (Normal, and Render on the damaged Plate), and record them against the pre-change numbers measured in the same session. Verify Render is no slower.

## 3. Docs and close-out

- [ ] 3.1 Document the prune rule in `docs/ARCHITECTURE.md`'s resonator model section. Mark H11 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`, noting that the band fade from `resolve-exciter-tradeoffs` covered the aliasing half. Run fmt, clippy and the full suite, and verify all are green.

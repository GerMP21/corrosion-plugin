# Tasks

## 1. Benchmarks and tests first

- [x] 1.1 Add `voice_manager_sixteen_voices_bent` (Render, held +2 semitones) and `voice_manager_sixteen_voices_cable_dynamics` (TautCable with tension drop) to `benches/performance.rs`, each next to its plain twin. Run them and record the current ratios in the change. Verify they exceed the spec bounds today.
- [x] 1.2 Add tests: held-bend output bit-identical to an every-sample rebuild (behind a test-only switch), and the moving TautCable factor staying within 0.05 cents at every sample. Verify the bit-identity test passes today (it's the reference), and that the in-tune test has a reference to compare against.

## 2. Skip and retune

- [x] 2.1 Add `applied_factor` and the 0.05-cent skip in `apply_dynamics`, reset by `set_pitch_bend_factor` and note-on. Verify both 1.2 tests pass, along with the dynamics, sheet-sweep and pitch-bend tests.
- [x] 2.2 Add `SecondOrderMode::retune` (cached radius, angle-dependent terms only) and use it in `apply_dynamics` and `set_pitch_bend_factor`. Verify the golden fixture still matches (note-on is unaffected) and that `tests/no_alloc.rs` passes in debug.
- [ ] 2.3 Re-run the benchmarks from 1.1. Verify bent ≤ 1.1× plain and dynamics ≤ 1.5× static, and record the numbers.

## 3. Close-out

- [ ] 3.1 Mark H12 FIXED, and the benchmark tooling note, in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy and the full suite, and verify all are green.

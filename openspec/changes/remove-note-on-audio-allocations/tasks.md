# Tasks

## 1. Baselines and failing tests

- [x] 1.1 Add a golden mode-bank fixture test: for each of the 9 objects × 4 quality modes × damage {0, 0.5, 1}, record mode frequency/decay/gain from the current `ModalResonator::with_algorithm_controls_and_note` into a fixture under `tests/fixtures/`; verify the test passes on the unmodified code
- [x] 1.2 Extend `tests/no_alloc.rs` with a test that performs note-on for every object × quality mode × damage {0, 1} inside `assert_no_alloc`; verify it FAILS on the current code
- [x] 1.3 Add no-alloc tests for note-off-to-idle, ninth-note voice stealing across objects, TautCable pitch-bend sweep, and reset with active voices; record which fail on the current code

## 2. Allocation-free mode generation

- [x] 2.1 Change `ModalModeSpec::damaged()` in `src/dsp/profile.rs` to return a fixed-size (≤ 2) result instead of `Vec`, updating its callers and existing tests; verify `cargo test --no-default-features profile` and the 1.1 fixture pass
- [x] 2.2 Add `generate_modes_into(&mut Vec<ModalModeSpec>)` to the generator trait in `src/dsp/resonators/basic.rs` and implement it for all 9 objects, keeping `generate_modes()` as an allocating wrapper; verify the 1.1 fixture passes
- [x] 2.3 Define `MAX_MODES` from the profile tables (max base count × 4 × 2) and add a test that every object × quality × extreme character setting produces ≤ `MAX_MODES` modes; verify it passes

## 3. In-place rebuild on the voice

- [x] 3.1 Preallocate `modes`, scratch buffers, and interaction-bus coefficients with capacity `MAX_MODES` when a `Voice` / `ModalResonator` is constructed, and add `ModalResonator::rebuild_in_place(...)` that clears and refills them (truncating at capacity); verify the 1.1 fixture passes against the in-place path
- [x] 3.2 Switch `Voice::note_on_with_controls` in `src/voice/mod.rs` to `rebuild_in_place` instead of assigning a new resonator; verify tests 1.2 and the stealing test from 1.3 pass
- [x] 3.3 Fix any remaining allocation reported by the 1.3 note-off and pitch-bend tests; verify they pass

## 4. Allocation-free reset

- [x] 4.1 Add `VoiceManager::reset()` and `PostProcessingChain::reset()` that clear state in place, and call them from `Plugin::reset()` in `src/lib.rs`; verify the reset no-alloc test from 1.3 passes and output after reset is silent
- [x] 4.2 Update the "No heap allocation" notes in `README.md` and `docs/ARCHITECTURE.md` to state that note-on, stealing, expression, and reset are covered by `tests/no_alloc.rs`; verify the referenced test names exist

## 5. Close out

- [x] 5.1 Run `cargo test --no-default-features --release`, `cargo fmt --check`, `cargo clippy --no-default-features --all-targets`, and `cargo bench --no-run`; verify all green with no new warnings
- [x] 5.2 Update `FABLE-CODE-REVIEW-2026-07-07.md` C3 status from IN PROGRESS to FIXED with the commit reference; verify the heading, the tests section bullet, and the priority list reflect the new status

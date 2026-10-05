# Proposal

## Why

Every MIDI note-on runs inside `process()` and rebuilds the voice's modal resonator from scratch, allocating several `Vec`s per note (mode generation, transform pass, one `vec!` per mode in the damage step, the second-order mode bank, the interaction bus) and freeing the old ones. This breaks the plugin's documented "no heap allocation in the audio callback" guarantee and, because `Cargo.toml` enables nih-plug's `assert_process_allocs` on non-Windows targets, a debug build aborts the DAW on the first note. `tests/no_alloc.rs` misses it because it triggers notes outside the no-alloc zone. `Plugin::reset()` also reconstructs the voice manager and post chain (code review 2026-07-07, finding **C3**).

## What Changes

- Note-on, note-off, voice stealing, and all live MIDI expression perform zero heap allocations and deallocations on the audio thread, for every object, quality mode, damage amount, and character setting.
- Each voice owns preallocated, worst-case-sized storage for its modal bank and interaction coefficients; note-on rebuilds the bank in place.
- `reset()` clears state in place instead of reconstructing large structures.
- `tests/no_alloc.rs` is extended so note-on, stealing, and `reset()` happen inside the no-alloc zone, across all objects and quality modes.
- No audible change: generated modes, their order, and their parameters stay identical.

## Capabilities

### New Capabilities
- `realtime-safety`: Guarantees about what the audio-thread callback may do (no heap allocation or deallocation), covering note events, voice management, expression, and reset.

### Modified Capabilities
<!-- none: no existing specs in openspec/specs/ -->

## Impact

- Code: `src/dsp/resonators/core.rs` and the per-object generators in `src/dsp/resonators/*.rs` (generate into a caller-provided buffer); `src/dsp/profile.rs` (`ModalModeSpec::damaged()` returns a fixed-size result); `src/dsp/interaction.rs` (preallocated coefficients); `src/voice/mod.rs` (in-place rebuild); `src/lib.rs` (`reset()`).
- Memory: each voice reserves its worst-case mode capacity up front (a few KB per voice).
- Tests: `tests/no_alloc.rs` rewritten to cover note-on paths; a mode-equivalence test guards against sound changes.
- Docs: README / `docs/ARCHITECTURE.md` "No heap allocation" claims become accurate; `FABLE-CODE-REVIEW-2026-07-07.md` C3 status.

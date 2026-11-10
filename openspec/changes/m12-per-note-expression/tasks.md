# Tasks

## 1. Tests first

- [x] 1.1 Add tests for every `note-expression` scenario (overlapping IDs, plain MIDI fallback, one-note tuning, tuning plus bend, pressure by ID, choke fade and idle) and extend `tests/no_alloc.rs` with the new events. Verify the scenario tests fail on the current code and the plain-MIDI and no-alloc tests pass.

## 2. Addressing

- [x] 2.1 Store voice ID and channel on `Voice`; add the shared addressing rule to `VoiceManager`; route note-off and `PolyPressure` through it. Verify the overlapping-ID, plain-MIDI and pressure tests pass, with every existing voice-allocation test.

## 3. Tuning and choke

- [x] 3.1 Add per-voice tuning combined with the channel bend and route `PolyTuning`. Verify both tuning tests pass and the H12 held-bend tests still pass.
- [x] 3.2 Add the 2 ms choke fade and route `Choke`. Verify the choke test and `tests/no_alloc.rs` (debug) pass.

## 4. Docs and close-out

- [x] 4.1 Document note expressions in the README and `docs/ARCHITECTURE.md`; mark M12 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md` (noting channel MPE as out of scope). Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

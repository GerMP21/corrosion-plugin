# Tasks

## 1. Tests first

- [x] 1.1 Add tests for each `midi-control` scenario and the stealing scenario: pedal chord, key held at pedal-up, empty pedal, repeated note, All Notes Off with and without pedal, All Sound Off fade and replay, Reset All Controllers, 17th note under the pedal. Verify they fail on the current code.
- [x] 1.2 Extend `tests/no_alloc.rs` with the pedal, All Notes Off, All Sound Off and Reset All Controllers paths. Verify it passes today (the CCs are ignored) so it can guard the implementation.

## 2. Pedal and All Notes Off

- [x] 2.1 Add `sustain_down` to `VoiceManager` and `pedal_held` to `Voice`; route CC64 with the ≥ 64 threshold; defer note-offs while down and release on pedal up; release the older instance on a re-strike. Verify the pedal and repeated-note tests pass.
- [x] 2.2 Add the pedal-held stealing tier. Verify the 17th-note test and every existing voice-allocation test pass.
- [x] 2.3 Route CC123 through the note-off path. Verify both All Notes Off tests pass.

## 3. All Sound Off and Reset All Controllers

- [x] 3.1 Add the 5 ms panic fade to the output path (out of line, skipped when idle), resetting voices and the post chain at its end. Verify the fade and replay tests pass, and that `plugin_output_path_sixteen_voices` renders bit-identically to before (no panic in it).
- [x] 3.2 Route CC121 to lift the pedal and reset mod wheel, bend and channel pressure; clear pedal state in `Plugin::reset`. Verify the Reset All Controllers test and `tests/no_alloc.rs` (debug) pass.

- [x] 3.3 *(Added after verification.)* Check the repeated-note limit after every strike, add `plugin_reset_lifts_the_pedal`, and measure the panic branch's cost against the base. Verify the tests pass and record the numbers.

## 4. Docs and close-out

- [x] 4.1 Document the MIDI controllers in the README and the pedal in `docs/ARCHITECTURE.md`'s voice lifecycle; mark M13 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md` (M12's MPE part stays open). Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

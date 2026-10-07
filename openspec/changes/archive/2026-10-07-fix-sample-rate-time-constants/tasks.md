# Tasks

## 1. Inventory, golden renders and failing tests

- [x] 1.1 Confirm `fix-resonator-sample-rate-gain` has landed. Complete the design's inventory table by reading every exciter, `voice/mod.rs`, `interaction.rs` and `fem_body.rs`, and update the table. Verify each listed constant has a file:line in the table.
- [x] 1.2 Add an ignored test that prints 48 kHz render hashes per exciter (Pipe C4, velocity 100, 1 s held + 1 s released) and for the FEM body impulse. Run it before any edit and record the hashes (design Decision 5: hashes aren't portable across libm). Add the spec's voice-level tests (RMS at 44.1/96 kHz within 1.5 dB for every exciter; struck time-to-peak within 10 %; struck tail end time within 10 %) and the body test. Verify the hashes are recorded and the sample-rate tests FAIL. Record the measured per-exciter differences under "Implementation findings".

## 2. Shared helper and voice

- [x] 2.1 Add the `rate_ratio` conversion helper (decay `c^ratio`, step `× ratio`, smoother `1 − (1−k)^ratio`) in `src/dsp/utils`, with a test that a ratio of 1.0 returns each input exactly. Verify it passes.
- [x] 2.2 Convert the voice high-pass, `PEAK_DECAY` and `TAIL_DEACTIVATE_FRAMES`, with coefficients cached per sample rate. Verify the tail-timing test passes, the 48 kHz golden hashes are unchanged, and the voice-stealing tests pass.

## 3. Exciters, interaction and body

- [x] 3.1 Give exciters a cached rate ratio (set by the voice at note-on and on rate change) and convert the bow, then the struck exciters (hand strike, felt mallet, hard mallet, drumstick, metal pipe, metal chain). After each one, verify its 48 kHz golden hash is unchanged and its sample-rate test passes.
- [x] 3.2 Convert the friction and specialty exciters (stiff point, tension rise, corrugated drag, heavy grinding, plus any others in the inventory) and the interaction position wander. Verify the same way.
- [x] 3.3 Convert the FEM body's decay and force. Verify the body test passes and its 48 kHz hash is unchanged.
- [x] 3.4 Run `tests/no_alloc.rs` in debug and verify it passes, since exciters gained cached state.

## 4. Docs and integration

- [x] 4.1 Document the sample-rate rule in `docs/ARCHITECTURE.md` (per-sample constants are 48 kHz references, converted through the helper), and list any exciter that needed a widened tolerance. Verify `cargo doc --no-deps --no-default-features` gives no new warnings.
- [x] 4.2 Run the full gate: debug and release tests, fmt, clippy on 1.97 and 1.99, the render smoke test with `scripts/check_wav.py`, and the preset level check. Verify all green.

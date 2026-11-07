# Tasks

## 1. Tests first

- [x] 1.1 Add tests for the spec scenarios: Factory −30 dB time at 44.1/48/96/192 kHz; Echo at 0.8 s across rates; Spring at tank 1 at 48 and 192 kHz; the 96 kHz Delay Time sweep; and distinct comb delays at five sizes. Verify each fails on the current code, and record the measured numbers in the change.

## 2. Time-defined delays and buffers

- [x] 2.1 Convert the FactoryReverb comb and allpass delays to seconds, move the buffers to `Box<[f32]>` sized for 192 kHz, and remove `diffusion_y`. Verify the Factory and comb tests pass, and that `tests/no_alloc.rs` (debug) passes.
- [x] 2.2 Size the FactoryEcho buffers for 0.8 s plus modulation at 192 kHz. Verify the Echo and sweep tests pass.
- [x] 2.3 Size SpringReverb for the largest tank at 192 kHz, and drop the 8 000 clamp. Verify the Spring test and the existing `space-effects` tests (channel independence, echo timing, mono, reset) pass.
- [x] 2.4 Size the HrtfSpread buffer for 1 ms at 192 kHz, and replace `.min(63)` with the capacity guard. Verify the spread tests pass.

## 3. Presets and docs

- [x] 3.1 Run `preset_levels_table`. Apply any trims to `PRESET_LEVELS`, regenerate with `seed_presets`, and write `levels.md` with before and after for the five Factory presets. Verify `preset_levels_are_sane` and `tests/preset_roundtrip.rs` pass.
- [x] 3.2 Update `docs/detailed-specs/post-processing.md` (or ARCHITECTURE's post-chain section) with the time-defined delays and the buffer sizing. Mark H10 and M16 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy and the full suite, and verify all are green.

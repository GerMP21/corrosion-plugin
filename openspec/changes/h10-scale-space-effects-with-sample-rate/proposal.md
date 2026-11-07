# Proposal

## Why

The Space section's delays are written in raw samples, and its buffers are sized for 48 kHz (fable review 2026-07-07, finding **H10**, with medium finding **M16**):
- **Factory reverb comb delays** (1553–1913 samples, scaled by size) aren't converted for the sample rate, so the room is half as big at 96 kHz.
- **The combs clamp at 2000 samples** for a 2048-sample buffer. Already at size 0.5 at 48 kHz (all five Factory presets), three of the four combs collapse onto the same 2000-sample delay: a periodic, metallic tail.
- **The reverb's allpass delays** (200 and 300 samples) aren't converted either.
- **Echo's 16 384-sample buffer** holds only 0.34 s at 48 kHz and 0.17 s at 96 kHz, against a Delay Time range that goes to 0.8 s. Most of the knob silently clamps.
- **Spring's 8 192-sample buffer** clamps the tank at 192 kHz.
- **HRTF's 64-sample delay buffer** can't hold 1 ms above 64 kHz.

## What Changes

- **Delays defined in time, converted per sample rate:**
  - the Factory reverb's comb and allpass delays;
  - Echo's delay range, 0.02–0.8 s plus modulation;
  - Spring's tank delay;
  - HRTF's 1 ms delay.
- **Buffers sized at construction for the largest supported rate (192 kHz),** so no delay clamps at any knob position and the audio thread never allocates.
- **The four Factory combs stay distinct at every size,** keeping the mutually-prime spacing of their 48 kHz base delays.
- **Audible change:**
  - The Factory reverb is no longer metallic at most sizes. The five Factory presets change and get a level pass.
  - Echo reaches its full Delay Time range. Hanging Chain Echo, at 0.215 s, is unchanged at 48 kHz.
  - Every Space mode now sounds the same at 44.1, 48, 96 and 192 kHz.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `space-effects`: adds requirements that every Space mode keeps its timing at every supported sample rate, and that no control position silently clamps.

## Impact

- **Code:** `src/dsp/post_processing/space.rs` (FactoryReverb, FactoryEcho, SpringReverb); `src/dsp/post_processing/hrtf_spread.rs`.
- **Memory:** buffers become heap slices allocated in `new()`, off the audio thread. That is about 1.5 MB per post chain, mostly Echo's two 0.8 s buffers at 192 kHz.
- **Presets:** level pass for the 5 Factory presets, recorded in `levels.md`.
- **Docs:** the post-processing spec notes, and the review's H10 and M16 status.

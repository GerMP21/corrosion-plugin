# Proposal

## Why

Two post-chain findings from the fable review (2026-07-07):

- **M7.** The Echo reads its modulated delay at a whole-sample position, so as Machinery Movement sweeps the delay the read point jumps by one sample at a time: repeated and skipped samples that sound as zipper noise. Its High Frequency Damping knob is a broadband wet gain (`1 − 0.6·knob`), not a filter: the name promises darker repeats and it only turns the echo down.
- **M8.** The HRTF spread stage keeps two delay buffers that always hold the same samples and are read at the same position. One is redundant, and the comment promises a "1 ms interaural time difference" that the code doesn't do (it delays the mid signal on both channels equally). The maintainer chose to keep the sound and remove only the redundancy.

## What Changes

- **The Echo reads between samples** (fractional, interpolated delay), so modulation glides instead of stepping. With no modulation the delays stay exact, as the existing timing requirements demand.
- **High Frequency Damping becomes a low-pass in the echo's feedback path:** 0 leaves the repeats full-range, 1 darkens them strongly, and each repeat is darker than the last. The echo's low-frequency level no longer depends on the knob; it stays at today's default level (the old gain at 0.5).
- **The spread stage keeps one delay buffer,** and its comments describe what it does (a common delay of the mid signal of up to 1 ms with Width, no ITD). Output is bit-identical.
- Sound changes for sessions using Echo: smoother modulation, and HF Damping now changes tone rather than level (louder repeats below 0.5, quieter above). No parameter range or ID changes.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `space-effects`: adds that echo modulation is continuous and that High Frequency Damping filters the repeats instead of changing their level.

## Impact

- **Code:** `src/dsp/post_processing/space.rs` (`FactoryEcho`), `src/dsp/post_processing/hrtf_spread.rs`.
- **Docs:** README (Echo controls), `docs/ARCHITECTURE.md` (post chain), the review's M7 and M8 status.

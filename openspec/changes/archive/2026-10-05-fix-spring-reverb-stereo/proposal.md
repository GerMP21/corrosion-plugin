# Proposal

## Why

In `SpaceMode::Spring` the post chain calls `spring_reverb.process(left)` and then `spring_reverb.process(right)` on the **same** `SpringReverb`. Its single delay line, write index and dispersion state therefore take left and right samples interleaved. Each channel's echo comes back roughly half the intended delay later, on the opposite channel's call, and the feedback path cross-mixes the channels sample by sample (code review 2026-07-07, finding **C6**). Two factory presets use the spring reverb.

## What Changes

- The spring reverb keeps independent state per channel, so each channel's tail comes from its own input, at the configured delay length.
- Parameter, sample-rate and reset calls apply to both channels.
- **Audible change:** spring presets get the intended (twice as long) spring delay and no channel cross-talk in the tail. A mono input still gives an identical tail on both sides.

## Capabilities

### New Capabilities
- `space-effects`: Behaviour of the post-chain Space section (Factory reverb, Spring reverb, Echo), starting with per-channel independence and delay timing of the spring reverb.

### Modified Capabilities
<!-- none in openspec/specs/ yet -->

## Impact

- Code: `src/dsp/post_processing/post_chain.rs` (two spring instances or a stereo wrapper), possibly `src/dsp/post_processing/space.rs` (reset also clears the write index).
- Memory: one more 8192-sample delay line (32 KB) per plugin instance.
- Tests: new spring tests for channel isolation, echo timing, and mono-in/mono-out.
- Docs: `FABLE-CODE-REVIEW-2026-07-07.md` C6 status.

# Proposal

## Why

Two post-chain findings from the fable review (2026-07-07):

- **M6.** The Lorenz drive's Bias Starvation gate switches the drive's input gain between 1.0 and 0.3 in a single sample, keyed off the chaotic attractor's state. Every switch is a step in the signal: clicks and crackle, undocumented and unsmoothed. Its `tube_state` field is dead.
- **M9.** The "oversampled" clipper doesn't oversample in the DSP sense. It interpolates linearly between two consecutive input samples, clips the interpolated points and averages them (a box filter). That adds no bandwidth, so it can't stop the clip's harmonics from folding back, and it low-passes and delays every signal by about half a sample even when nothing is clipped. The docs describe it as an anti-aliasing stage. The maintainer chose real IIR oversampling.

## What Changes

- **Bias Starvation ramps the gain** between full and starved over a few milliseconds instead of stepping, so starvation sounds like sag rather than clicks. The dead `tube_state` goes.
- **The clipper oversamples for real:** cascaded half-band IIR filters up to the high rate, the diode curve there, and the same filters back down. Normal, High and Render keep their 4×, 8× and 16× factors; Eco stays at 1× (plain clipping).
- **Below the ceiling the clipper is transparent** (flat to near the band edge, no box-filter droop); above it, far less aliasing.
- No latency to report: the IIR filters are minimum-phase, with a delay well under a sample at low frequencies.
- Sound changes for patches that use Bias Starvation or drive the clipper; parameters and stored values don't change.

## Capabilities

### New Capabilities
- `output-clipper`: what the post chain's final clipper does to the signal: transparency below the ceiling, band-limited clipping above it, how quality modes set its oversampling, and its latency.

### Modified Capabilities
- `post-chain-drive`: adds that Bias Starvation changes the drive's gain smoothly.

## Impact

- **Code:** `src/dsp/post_processing/lorenz_drive.rs`, `src/dsp/post_processing/oversampled_clipper.rs` (rewritten around a half-band IIR stage), `src/dsp/post_processing/post_chain.rs` (factor per quality mode, unchanged values).
- **CPU:** Render's 16× clipper becomes four cascaded 2× stages per channel; measured with `plugin_output_path_sixteen_voices`.
- **Tests:** the aliasing regression tests in `src/offline/mod.rs` keep passing and tighten.
- **Docs:** `docs/ARCHITECTURE.md` (post chain, output protection), README quality modes line, the review's M6 and M9 status.

# Proposal

## Why

Two post-chain findings from the fable review (2026-07-07):

- **M6.** The Lorenz drive's Bias Starvation gate switches the drive's input gain between 1.0 and 0.3 in a single sample, keyed off the chaotic attractor's state. Every switch is a step in the signal: clicks and crackle, undocumented and unsmoothed. Its `tube_state` field is dead. Measuring it (2026-11-09) also showed that the gate only switches for Bias Starvation between about 0.2 and 0.7: below, the attractor never reaches the threshold; above 0.71 the threshold falls below the attractor's whole range, and the control becomes a fixed −10.5 dB cut.
- **M9.** The "oversampled" clipper doesn't oversample in the DSP sense. It interpolates linearly between two consecutive input samples, clips the interpolated points and averages them (a box filter). That adds no bandwidth, so it can't stop the clip's harmonics from folding back, and it low-passes and delays every signal by about half a sample even when nothing is clipped. The docs describe it as an anti-aliasing stage. The maintainer chose real IIR oversampling.

## What Changes

- **Bias Starvation ramps the gain** between full and starved over a few milliseconds instead of stepping, so starvation sounds like sag rather than clicks. The dead `tube_state` goes.
- **Bias Starvation works across its whole range:** the threshold is rescaled to the attractor's actual `z` range, so the starved share of time rises from about 5 % at 0.1 to about 87 % at 1, and the gain keeps switching at every value above 0 (maintainer's choice, 2026-11-09).
- **The clipper oversamples for real:** cascaded half-band IIR filters up to the high rate, the diode curve there, and the same filters back down. Normal, High and Render keep their 4×, 8× and 16× factors; Eco stays at 1× (plain clipping).
- **Below its threshold the clipper is transparent** (flat to near the band edge, no box-filter droop); above it, far less aliasing. When oversampling, the threshold is the ceiling ÷ 1.3: band-limiting lets peaks overshoot the ceiling, and the default hard output clamp would clip that overshoot at the host rate (maintainer's choice, 2026-11-09).
- No latency to report: the IIR filters delay low frequencies by about 5 samples in Normal and 6 in Render at 48 kHz (0.1–0.125 ms), which the plugin doesn't report (maintainer's choice, 2026-11-09; the proposal first said under a sample).
- Sound changes for patches that use Bias Starvation (smoothing, and a different starved share at each value) or drive the clipper; in Normal, High and Render the clipper starts 2.3 dB below the Analog Ceiling and clipped material is about 1.4 dB quieter. Parameters and stored values don't change.

## Capabilities

### New Capabilities
- `output-clipper`: what the post chain's final clipper does to the signal: transparency below the ceiling, band-limited clipping above it, how quality modes set its oversampling, and its latency.

### Modified Capabilities
- `post-chain-drive`: adds that Bias Starvation changes the drive's gain smoothly and acts across its whole range.

## Impact

- **Code:** `src/dsp/post_processing/lorenz_drive.rs`, `src/dsp/post_processing/oversampled_clipper.rs` (rewritten around a half-band IIR stage), `src/dsp/post_processing/post_chain.rs` (factor per quality mode, unchanged values).
- **CPU:** Render's 16× clipper becomes four cascaded 2× stages per channel; measured with `plugin_output_path_sixteen_voices`.
- **Tests:** the aliasing regression tests in `src/offline/mod.rs` keep passing and tighten.
- **Docs:** `docs/ARCHITECTURE.md` (post chain, output protection), README quality modes line, the review's M6 and M9 status.

# Proposal

## Why

The post-chain ladder filter computes its prewarp coefficient as `tan(2π·fc/fs)` instead of `tan(π·fc/fs)`. For any cutoff at or above `fs/4` the per-stage recursion becomes unstable and the soft saturator latches the output to a ±1.0 DC rail until the plugin is reset. The default cutoff (20 kHz) and all 50 factory presets sit in that region, so the plugin is broken at default settings at 44.1/48/96 kHz (code review 2026-07-07, finding **C1**).

## What Changes

- Correct the cutoff prewarp so the filter maps the displayed cutoff to the intended frequency.
- Replace each ladder stage with a unity-DC-gain TPT one-pole. The old stage had a DC gain of 0.5 (−24 dB through four stages). That loss was hidden while the filter railed, but once stable it left a default note at ≈ −42 dBFS peak. *(Scope added during implementation; see design Decision 5.)*
- Clamp the effective cutoff strictly below Nyquist (relative to the current sample rate) so no host-reachable cutoff/sample-rate combination can make a stage unstable.
- Add regression tests that pin the filter's behavior at high cutoffs: no DC rail, bounded output, and stable decay to silence at 20 kHz and across a sweep through `fs/4` at several sample rates.
- **Audible change:** cutoffs below `fs/4` currently land roughly an octave *above* the displayed value (e.g. 1 kHz displayed → ≈2 kHz −3 dB point at 48 kHz). After the fix they track the displayed value, so presets with cutoffs below `fs/4` will sound darker than today. (The review text says "half"; numeric check of the per-stage transfer function shows it is double.)
- **Audible change:** passband level rises by ≈24 dB relative to the old stable-region behavior, and resonance feedback now acts through unity-gain stages (stronger peak). All factory presets use resonance 0.
- **Resonance feedback is zero-delay** *(added after verification; see design Decision 6)*. With unity-gain stages, the one-sample-delayed feedback made the filter ring forever from resonance ≈ 0.3 at the 20 kHz default cutoff. The feedback is now solved each sample, with `k = 4·resonance/0.9`. Tails always decay below 90 % resonance, and the filter self-oscillates from 90 % up at any cutoff and sample rate (the user's choice: keep self-oscillation above 90 %).

## Capabilities

### New Capabilities
- `post-chain-filter`: Stability and cutoff-mapping guarantees for the post-chain ladder filter across the full cutoff range and supported sample rates.

### Modified Capabilities
<!-- none: no existing specs in openspec/specs/ -->

## Impact

- Code: `src/dsp/post_processing/wdf_filter.rs` (coefficient computation, cutoff clamp, unit tests).
- Tests: new regression tests in the filter's test module and/or `tests/` exercising the full post chain at the default cutoff.
- Presets/renders: no preset file changes; preset WAV renders under `output/` may be re-rendered for audition once the fix lands (they currently bypass the post chain — review H13 — so they do not show the bug).
- Docs: `FABLE-CODE-REVIEW-2026-07-07.md` C1 status.

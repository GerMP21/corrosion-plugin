# Design

## Context

`WdfLadderFilter` (`src/dsp/post_processing/wdf_filter.rs`) is four cascaded one-pole stages with a soft saturator between stages and resonance feedback from stage 4. Each stage computes:

```
y[n] = (g·x[n] + (1 − g)·y[n−1]) / (1 + g),   g = tan(2π·fc/fs)   ← bug
```

The recursion pole is `(1 − g)/(1 + g)`. It is inside the unit circle only for `g > 0`. With the doubled argument, `g` becomes negative (or infinite) once `fc ≥ fs/4`, which is unstable, and the saturator turns the divergence into a permanent ±1 rail. Cutoff smoothing calls `update_coefficients()` at audio rate during ramps, so the fix must stay cheap (one `tan` per change, no allocation).

Numeric check of the stage transfer function at 48 kHz: with the bug, 1 kHz displayed gives a −3 dB point at ≈2.04 kHz; with `tan(π·fc/fs)` it is ≈1.0 kHz. Per-stage DC gain is 0.5 in both cases (see Non-Goals).

## Goals / Non-Goals

**Goals:**
- Stable at every host-reachable cutoff/sample-rate pair.
- Cutoff tracks the displayed value.
- Regression tests that would have caught C1.

**Non-Goals:**
- Rewriting the stage into a canonical zero-delay-feedback TPT ladder. The current stage has a per-stage DC gain of 0.5 (≈ −24 dB through four stages). Fixing that changes output loudness of every preset by ~24 dB and interacts with the body/remix topology in review finding C5. It is tracked as a follow-up, not part of this change.
- Changing resonance behavior, the saturator curve, or the `0.3/0.7` post-chain remix (C5).
- Renaming `WdfLadderFilter` (kept for ABI/preset stability).

## Decisions

1. **Use `g = tan(π · fc_eff / fs)`.** This is the standard bilinear prewarp for a one-pole stage and matches what the review prescribed. Alternative: approximate `g ≈ π·fc/fs` (no `tan`) — rejected because it detunes high cutoffs and `tan` is only evaluated when the smoothed cutoff moves.

2. **Clamp to `fc_eff = min(cutoff_hz, 0.49 · fs)` inside `update_coefficients()`.** Clamping where `g` is computed (not only in `set_parameters`) protects every path: sample-rate changes after the cutoff is set, the smoother, and `settle()`. 0.49 keeps `g` finite (≈ 32) with margin; the host range stays 20 Hz–20 kHz so no parameter or preset changes. Alternative: clamp in `set_parameters` only — rejected because `set_sample_rate` can lower fs after the cutoff was set.

3. **Guard non-positive sample rate.** Use `sample_rate.max(1.0)` in the coefficient path, mirroring `recompute_smoothing_coeff`, so a default-constructed filter used before `set_sample_rate` cannot divide by zero.

4. **Tests at two levels.** Unit tests in `wdf_filter.rs` cover the sweep through `fs/4`, the Nyquist clamp, the −3 dB mapping, and reset. One integration test runs `PostProcessingChain` with default parameters at 48 kHz and checks for no DC at the output, since that is the path users hear.

## Risks / Trade-offs

- [Presets voiced against the buggy mapping sound darker below `fs/4`] → Note it in the change summary; preset re-voicing is a separate decision. All factory presets use 20 kHz, so the factory bank is affected only through the clamp (which removes the DC rail).
- [Existing filter tests assume the old mapping] → Any test that breaks is re-based to the corrected mapping in the same task that changes the formula; failures are inspected, not loosened blindly.
- [Saturator can still mask instability in future edits] → The new tests assert decay to zero after input stops, which a latched rail fails.

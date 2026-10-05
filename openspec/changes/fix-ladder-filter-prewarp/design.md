# Design

## Context

`WdfLadderFilter` (`src/dsp/post_processing/wdf_filter.rs`) is four cascaded one-pole stages with a soft saturator between stages and resonance feedback from stage 4. Each stage computes:

```
y[n] = (g·x[n] + (1 − g)·y[n−1]) / (1 + g),   g = tan(2π·fc/fs)   ← bug
```

The recursion pole is `(1 − g)/(1 + g)`. It is inside the unit circle only for `g > 0`. With the doubled argument, `g` becomes negative (or infinite) once `fc ≥ fs/4`, which is unstable, and the saturator turns the divergence into a permanent ±1 rail. Cutoff smoothing calls `update_coefficients()` at audio rate during ramps, so the fix must stay cheap (one `tan` per change, no allocation).

Numeric check of the stage transfer function at 48 kHz: with the bug, 1 kHz displayed gives a −3 dB point at ≈2.04 kHz; with `tan(π·fc/fs)` it is ≈1.0 kHz. Per-stage DC gain was 0.5 in both cases; Decision 5 later made the stages unity-gain.

## Goals / Non-Goals

**Goals:**
- Stable at every host-reachable cutoff/sample-rate pair.
- Cutoff tracks the displayed value.
- Regression tests that would have caught C1.

**Non-Goals:**
- ~~A zero-delay-feedback (implicit) resonance solve.~~ Adopted after verification (Decision 6).
- Changing the `g_comp` compensation curve or the saturator curve. The `0.3/0.7` post-chain remix was later removed by C5 (`fix-post-chain-body-routing`). (Resonance *does* sound stronger as a side effect of Decision 5.)
- Renaming `WdfLadderFilter` (kept for ABI/preset stability).

## Decisions

1. **Use `g = tan(π · fc_eff / fs)`.** This is the standard bilinear prewarp for a one-pole stage and matches what the review prescribed. Alternative: approximate `g ≈ π·fc/fs` (no `tan`) — rejected because it detunes high cutoffs and `tan` is only evaluated when the smoothed cutoff moves.

2. **Clamp to `fc_eff = min(cutoff_hz, 0.49 · fs)` inside `update_coefficients()`.** Clamping where `g` is computed (not only in `set_parameters`) protects every path: sample-rate changes after the cutoff is set, the smoother, and `settle()`. 0.49 keeps `g` finite (≈ 32) with margin; the host range stays 20 Hz–20 kHz so no parameter or preset changes. Alternative: clamp in `set_parameters` only — rejected because `set_sample_rate` can lower fs after the cutoff was set.

3. **Guard non-positive sample rate.** Use `sample_rate.max(1.0)` in the coefficient path, mirroring `recompute_smoothing_coeff`, so a default-constructed filter used before `set_sample_rate` cannot divide by zero.

4. **Tests at two levels.** Unit tests in `wdf_filter.rs` cover the sweep through `fs/4`, the Nyquist clamp, the −3 dB mapping, and reset. One integration test runs `PostProcessingChain` with default parameters at 48 kHz and checks for no DC at the output, since that is the path users hear.

5. **Unity-gain TPT stages (added during implementation).** The design originally deferred the old stage's 0.5 DC gain. Once the filter stopped railing, a full-velocity default note through the real post chain peaked at 0.0075 (≈ −42 dBFS), versus 0.342 before the fix, and that 0.342 was almost all DC rail. Each stage is now the canonical TPT one-pole (`v = (x − s)·g/(1+g)`, `y = v + s`, `s ← y + v`) with the saturator applied to the stage output only, so the integrator state stays linear and cannot latch. Measured default-note peak: 0.118 (≈ −18.5 dBFS). Resonance feedback now taps the saturated last-stage output from the previous sample. Alternatives considered: keep the 0.5 gain and lower the failing test threshold (plugin nearly silent at defaults), or add a fixed ×16 makeup gain (changes how hard the saturators are driven relative to a true unity stage). Chosen by the user.

6. **Zero-delay resonance feedback, self-oscillation from 90 % (added after verification).** Verification found that Decision 5 plus the one-sample-delayed feedback (`u = x − 4·res·y₄[n−1]`) made the loop gain exceed 1 where its phase reaches 180°. Measured at the 20 kHz default cutoff, any resonance ≥ 0.35 at 48 kHz (≥ 0.3 at 44.1 kHz) left a tone at about −13 dBFS running after the input stopped; at 5 kHz, from 0.75. The tests missed it because they checked the tail's mean. The feedback is now solved implicitly against the linear cascade: each stage is `y = G·x + (1 − G)·s`, so `y₄ = G⁴·u + Σ`, and `u = (x − k·Σ)/(1 + k·G⁴)`. Because the bilinear transform preserves the analog ladder's stability boundary, the linear loop is stable for `k < 4` and self-oscillates for `k > 4` at every cutoff and sample rate. Mapping `k = 4·res/0.9` puts that boundary at 90 % resonance, which the user asked to keep as a self-oscillation zone. The saturators (identity below 0.7) leave the small-signal threshold unchanged and bound the oscillation; the stage-1 integrator, which sits outside the saturators, stays stable on its own (its self-coefficient `2(1−G)/(1+kG⁴) − 1` lies in (−1, 1]). At resonance 0, `k = 0` and the output is bit-identical to Decision 5. Tail checks now use the per-sample maximum. Alternatives: lower the resonance ceiling so the delayed loop never reaches unity gain (loses self-oscillation entirely and still tunes with cutoff), or a full Newton solve through the saturators (more CPU for no small-signal difference).
7. **Component tolerance is gated on input (added after verification).** The tolerance jitter was derived from `s[0]` and injected even with silent input, so a filter with tolerance > 0 produced a −60 dB·tol noise floor from silence, including right after a reset. It is now injected only while the input is non-zero.

## Risks / Trade-offs

- [Presets voiced against the buggy mapping sound darker below `fs/4`] → Note it in the change summary; preset re-voicing is a separate decision. All factory presets use 20 kHz and resonance 0, so the factory bank changes through the removed DC rail and the restored passband level (Decision 5), not the cutoff mapping.
- [Existing filter tests assume the old mapping] → Any test that breaks is re-based to the corrected mapping in the same task that changes the formula; failures are inspected, not loosened blindly.
- [Stronger resonance with unity-gain stages] → `g_comp = 1/(1 + 3.5·r)` still compensates level; tests check resonance 1.0 stays bounded and finite at 44.1/48/96 kHz across the cutoff range, including its self-oscillating tail (Decision 6).
- [Saturator can still mask instability in future edits] → The tests assert that every sample of the tail returns to within 1e-3 of zero after input stops (below 90 % resonance), which a latched rail or a ringing loop fails.

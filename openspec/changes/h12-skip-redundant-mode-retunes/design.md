# Design

## Context

- **Per sample:** `prepare_sample` calls `apply_dynamics` when `has_dynamics()` is true (a dynamic object, or `|pitch_bend_factor − 1| > 1e-6`).
- **`apply_dynamics`** computes `factor = dyn_factor · pitch_bend_factor` and calls `mode.set_frequency(base·factor, fs)` for every mode.
- **`set_frequency`** runs `CoupledCoefficients::for_mode`:
  - `exp` for the radius;
  - `sin`/`cos` for the pole;
  - the `b0/ω` gain and the band fade;
  - since `resolve-exciter-tradeoffs`, the voice high-pass response for the ring bound.

  All of it is in f64.
- **What depends on frequency:** the pole radius `r` depends only on the decay and `fs`. The pole angle, the gain scale, the fade and the high-pass response depend on frequency.
- **Benchmarks:** `benches/performance.rs` covers 1, 8 and 16 voices plus Render, with no bend and no dynamic objects.

## Goals / Non-Goals

**Goals:**
- A held bend is free.
- Moving dynamics cost a bounded amount.
- The cost is measured.

**Non-Goals:**
- Struct-of-arrays or SIMD layout.
- Changing what the dynamics compute.

## Decisions

1. **Cache the applied factor on the bank.**
   - `applied_factor` is set whenever coefficients are built.
   - `apply_dynamics` returns early if `|factor / applied_factor − 1| ≤ 2.9e-5` (0.05 cents). A constant bend is equal, so it's skipped.
   - `set_pitch_bend_factor` and note-on reset it.
   - *Alternative:* rebuild every N samples. That gives N-sample pitch steps whatever the rate of change, and still wastes work for a held bend.
2. **A cheaper retune.**
   - `SecondOrderMode::retune(frequency, fs)` keeps the cached `r` and recomputes only the angle-dependent terms (in f64, as now).
   - It halves the transcendental calls (no `exp`). A full `for_mode` still runs when the decay or `fs` changes.
3. **Benchmarks for the worst case:** a held-bend case and a TautCable dynamics case, at 16 voices, Normal and Render. The spec ratios compare each with its plain twin in the same run, so machine speed cancels.

## Risks / Trade-offs

- **[Stepped pitch on fast dynamics]** → Steps are at most 0.05 cents: below the just-noticeable difference by a factor of about 100. The sheet sweep (`a_mode_swept_through_the_top_of_the_band_does_not_click`) and the dynamics tests guard against clicks.
- **[Benchmark ratios are noisy]** → The ratio uses criterion's median of each pair in one session, and the spec bounds leave margin: 1.1× for a bend that should cost about 1.0×.

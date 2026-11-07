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

## Measurements

### Before (task 1.1)

`cargo bench --no-default-features --bench performance -- voice_manager_sixteen_voices_`, Criterion medians, two runs in one session, Linux x86_64 container (4 vCPU).

| Benchmark | Run 1 | Run 2 |
|---|---|---|
| `voice_manager_sixteen_voices_render` (plain twin) | 18.74 ms | 17.85 ms |
| `voice_manager_sixteen_voices_bent` (+2 semitones held) | 97.06 ms | 85.27 ms |
| `voice_manager_sixteen_voices_cable_dynamics` (tension drop 0.4) | 45.26 ms | 43.01 ms |
| `voice_manager_sixteen_voices_cable_static` (tension drop 0) | 43.05 ms | 44.01 ms |

- **Bent / plain: 5.18× and 4.78×**, far above the 1.1× bound.
- **Dynamics / static: 1.05× and 0.98×**, inside the 1.5× bound, but only because the static twin pays the same: TautCable always runs `apply_dynamics`, and with tension drop 0 its factor is exactly 1 and still rebuilds every mode every sample. Both cost about 2.4× the plain Render bank. The ratio becomes meaningful once the static twin stops rebuilding.

### After tasks 2.1–2.2 (in progress)

Same session. Two runs with the shipped variant (the high-pass response computed from `cos ω`, `apply_dynamics` out of line, the pole radii in a separate buffer):

| Benchmark | Run 1 | Run 2 |
|---|---|---|
| `voice_manager_sixteen_voices_render` (plain twin) | 23.95 ms | 24.15 ms |
| `voice_manager_sixteen_voices_bent` | 29.16 ms | 26.53 ms |
| `voice_manager_sixteen_voices_cable_dynamics` | 29.75 ms | 29.79 ms |
| `voice_manager_sixteen_voices_cable_static` | 15.56 ms | 15.64 ms |

- **Bent / plain: 1.22× and 1.10×** (later runs: 1.02–1.07×), down from about 5×.
- **Static TautCable: 43 ms → 15 ms**, now that a factor of exactly 1 skips the rebuild.
- **Dynamics / static: 1.9×**, above the 1.5× bound. The dynamic run got cheaper (45 → 30 ms), but its static twin got much cheaper still, so the ratio rose. Per voice-sample the dynamics cost about 280 ns over static: roughly one rebuild of every mode every few samples, since the tension drop moves the factor by more than 0.05 cents that often.

**Code layout dominates the plain benchmarks.** Plain notes don't run any of the new code, yet `voice_manager_sixteen_voices_render` measured 17.5–18.3 ms or 22.4–25.5 ms depending only on cold code elsewhere in `core.rs`. The bench scenario's output hash is bit-identical between a fast and a slow build (`0xa3012f403966c099`), so this is code placement, not work. Ratios within one build are still comparable; absolute numbers across builds are not.

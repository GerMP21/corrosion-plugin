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

### Code-layout noise fixed

The plain-bench swings came from `SecondOrderMode::ensure_rate`, which every mode calls every sample from `process`. Its rebuild branch (`CoupledCoefficients::for_mode`) was inlined into the per-sample loops, so any change to the coefficient code's size moved plain notes by up to 30 % with bit-identical output. The rebuild is now a separate `#[cold] #[inline(never)]` function. Two variants that measured 18 and 24 ms before (high-pass from `acos` vs from `cos`) now both measure 17–18.5 ms.

### Incremental retune (added after the first measurements)

The cache-and-skip alone left dynamics at about 1.9× static, because TautCable's tension drop moves the factor past 0.05 cents on almost every sample (4 040 of 4 096 in the benchmark), so every mode was rebuilt with `sin_cos` almost every sample. As built:

- **`RetuneCache`, one per mode, kept off the modes** (`retune_cache`): pole radius, input gain `b0`, `2π/fs`, the angle and its sine and cosine, `1/ω`, and the angle the ring-bound terms were last computed at. A new bank (note-on) or rate rebuilds it; live damping and brightness refresh only the radius and gain.
- **Held bend (`set_pitch_bend_factor`): exact.** `retune_exact` reuses the radius and calls `sin_cos`, bit-identical to `for_mode`.
- **Moving factor (`apply_dynamics`): incremental.**
  - `retune_incremental` rotates the cached sine and cosine by the angle step with a fifth-order Taylor rotation and updates `1/ω` to second order, with no transcendental call and no division. The angle itself comes exactly from the frequency.
  - An exact `sin_cos` refresh runs every 256 rotations, or for a step above 1e-3 rad.
  - Only the pole, input gain and band fade are rewritten. `|g|` and the high-pass response feed only the governor's ring bound, so they are recomputed once the angle has moved about 1 cent since they were last computed. A bound up to 0.06 % stale is far inside the governor's 10 % margin.
- `moving_dynamics_stay_in_tune` checks every mode's frequency and its actual pole angle (`atan2` of the f32 pole) against `base × factor` at every sample, for TautCable and SheetMetal.

### After (task 2.3)

Three runs in one session, after all of the above.

| Run | plain Render | bent | Bent / plain | cable static | cable dynamics | Dynamics / static |
|---|---|---|---|---|---|---|
| 1 | 21.00 ms | 22.52 ms | 1.07× | 11.67 ms | 19.71 ms | 1.69× |
| 2 | 20.28 ms | 18.94 ms | 0.93× | 11.31 ms | 17.67 ms | 1.56× |
| 3 | 19.04 ms | 21.12 ms | 1.11× | 12.34 ms | 18.16 ms | 1.47× |

Earlier runs with the same code gave dynamics/static 1.49×, 1.52× and 1.57×, and bent/plain 1.11×, 1.00× and 1.06×.

- **The floor for dynamics is about 1.1×, not 1.0×.** With every retune disabled (an experiment, not shipped), dynamics cost 1.10–1.15× static: the two notes sound different, so the exciter and governor do different work.
- **Bent / plain is about 1.0–1.1×** (from about 5×), and **dynamics / static is about 1.5–1.6×** (from 1.9× after the first pass; the dynamic run went from 45 ms to about 18 ms). Run-to-run noise on this container is ±10–15 % per benchmark, so single ratios land on both sides of the bounds.

### Median of 5 runs (`scripts/bench_ratios.py`)

The spec's ratios are now the median, over 5 runs, of the ratio within each run (each benchmark against its twin under the same machine conditions). First measurement with the code above:

| Run | Bent / plain | Dynamics / static |
|---|---|---|
| 1 | 0.936× (17.66 / 18.88 ms) | 1.459× (16.02 / 10.98 ms) |
| 2 | 1.057× (19.09 / 18.05 ms) | 1.558× (18.38 / 11.80 ms) |
| 3 | 1.180× (19.40 / 16.43 ms) | 1.720× (17.21 / 10.01 ms) |
| 4 | 1.063× (19.51 / 18.35 ms) | 1.481× (17.02 / 11.49 ms) |
| 5 | 1.094× (19.05 / 17.42 ms) | 1.795× (20.03 / 11.16 ms) |
| **Median** | **1.063× (bound 1.1×): met** | **1.558× (bound 1.5×): not met** |

### Bounds accepted (task 2.3)

The maintainer accepted the medians above against bounds of **1.1× (bent)** and **1.6× (dynamics)**: bent/plain 1.063×, dynamics/static 1.558×. The dynamics bound was raised from 1.5× because its floor with retuning free is about 1.1× (the dynamic note sounds different, so its exciter and governor do different work).

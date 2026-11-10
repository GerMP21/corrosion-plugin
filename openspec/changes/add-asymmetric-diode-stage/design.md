# Design

## Context

- After `m06-m09-smooth-starvation-and-oversample-clipper`, `OversampledClipper` up-samples through half-band IIR stages, applies `diode_clip`, and comes back down. When oversampling, the curve's threshold is the Analog Ceiling ÷ 1.3 (`CEILING_HEADROOM`); Eco clips at the ceiling itself.
- `diode_clip` is linear up to the threshold `T`, then `T + (1 − T)·0.1·e/(1 + k·e)` with `e` the excess and `k = 2 + 3·softness`. Just above `T` the slope drops from 1 to `(1 − T)·0.1` ≈ 0.026: a hard knee. Diode Softness moves the curve's top by under 0.01 (0.7560 at 0, 0.7483 at 1 with `T` = 0.743).
- Diode Softness and Analog Ceiling reach the chain once per block (`set_clipper_params`, `src/lib.rs`). They live in the preset's `extra` block (`diode_softness`, `analog_ceiling`). The randomizer doesn't touch them.
- The editor draws Ceiling and Softness faders in the "BRICKWALL LIMITER" panel. Its red dot is `DANGER_RED` at 30 %, always. Nothing passes audio-thread data to the editor today; only parameters.
- Migrations follow H9: `migrate_preset_json` rewrites raw preset JSON, and `filter_state` (`migrate_host_wander_rate`) rewrites restored host state. Both key on the absence of the new field.
- `apply_drive` (master drive, before the post chain) has its own asymmetric tube curve. This change doesn't touch it.

## Goals / Non-Goals

**Goals:** a curve that sounds good driven hot, with a real knee and controllable asymmetry; the same character in every quality mode; no DC; peaks still under the ceiling; a visible activity indicator; older presets and projects kept close to their sound.

**Non-Goals:**
- A circuit-level diode solve (Newton iterations per sample).
- An input "push" or auto-gain control for the stage.
- Showing the brickwall clamp's own activity (the dot shows only the diode).
- Changing the Analog Ceiling's meaning or the output limiters.

## Decisions

1. **A tanh knee per polarity.** For each polarity with limit `L` and knee width `w` (fraction of `L`): linear below `K = L·(1 − w)`, then `y = K + (L − K)·tanh((x − K)/(L − K))`. The slope is 1 at `K` and falls smoothly to 0, and the output never passes `L`. At `w = 0` it's a hard clip at `L`. *Alternative:* the Shockley diode equation (`asinh`-like). Its output keeps growing logarithmically, so it doesn't bound the peak, and the implicit form needs iterations. The tanh knee is the usual bounded stand-in for a diode's exponential conduction.
2. **Softness sets the width:** `w = 0.8·softness`, so the knee starts 14 dB under `L` at Softness 1 and 4.4 dB under at the default 0.5. *Changed before implementation (maintainer, 2026-11-09):* the first draft had `w = 0.5·softness` (6 dB at Softness 1), but a tanh knee bends so little where it starts that a sine at 0.6 of `L` lost only 0.006 dB at Softness 1, against the spec's 0.2 dB. Loss of a sine at 0.6 `L` at Softness 1 by factor: 0.5 → 0.006 dB, 0.7 → 0.141, 0.75 → 0.207, 0.8 → 0.287, 1.0 → 0.728. The maintainer chose a wider knee over a louder test tone; 0.8 leaves margin over the bound where 0.75 only just meets it.
3. **Asymmetry lowers the negative limit:** `L₊ = L`, `L₋ = L·2^(−asymmetry)` (one diode against two in series at 1, 6 dB), and the negative knee is firmer: `w₋ = w·(1 − asymmetry/2)`. Default 0.25 (`L₋` 1.5 dB under `L₊`).
4. **The same `L` in every mode:** `L = ceiling / H` in Eco too. `H` is re-measured for the new curve (task 3.2). It must keep the hard-driven tone's peaks at or under the ceiling after DC removal at Softness 0.5 and every Asymmetry, and keep `output-clipper`'s alias bounds through the hard clamp at Softness 0 (the migrated setting). Start from 1.3. **Stop and ask** if it needs more than 1.3 (louder loss than today) or if one constant can't satisfy both.
5. **DC blocker after the down-sampling:** a one-pole high-pass at 2 Hz per channel at the host rate (coefficient from the cutoff, so it's rate-independent). It's mixed in with weight `g = min(1, asymmetry_smoothed / 0.05)`: at exactly 0 it's bypassed and its state cleared, so a symmetric curve is unfiltered and bit-identical; turning asymmetry up fades the blocker in, so there's no step. *Alternative:* always on. It would filter every patch, and the "symmetric curve is unfiltered" property couldn't hold. *Changed during implementation (maintainer, 2026-11-09):* the first draft had 5 Hz, but a high-pass tilts the flat tops of a hard-clipped wave, and at 5 Hz a 200 Hz tone at four times the limit came out with a peak-to-peak span 3.3–3.4 % over the curve's limits (Asymmetry 0.25–1), against the spec's 2 %. At 2 Hz it's 1.3–1.4 %; the curve alone is exact. The blocker's time constant grows from 32 ms to 80 ms.
6. **Smoothed controls:** Softness and Asymmetry go through one-pole smoothers (τ 20 ms, like the drive's). The per-polarity constants (`L`, `K`, `1/(L − K)`) are recomputed only while a smoother is moving, so the settled path has no divisions.
7. **tanh evaluation:** a precomputed table (built in `new()`) or a rational approximation, whichever is cheaper in `plugin_output_path_sixteen_voices_render`, with error under 1e-4 and monotonic. It runs once per high-rate sample (up to 16× per channel).
8. **The meter:** the clipper tracks, per host block, the peak of its input and of its curve output (before DC removal). The reduction is `20·log10(peak_in / peak_out)`, or 0 when the input stays under the knee. The plugin publishes the larger of both channels' reductions to a shared `AtomicU32` (f32 bits) holding the maximum since the editor last read it. The editor takes and clears it every frame, shows `max(new, previous decayed)` with a ~300 ms release, maps 0.1–6 dB to the dot's brightness (resting 30 % to full `DANGER_RED`), and requests a repaint while it's above rest. Atomics only: no allocation, no locks.
9. **Migration:** preset JSON whose `extra` lacks `diode_asymmetry` gets `diode_softness = 0` and `diode_asymmetry = 0`; restored host state without `diode_asymmetry` gets the same, next to `migrate_host_wander_rate`. Factory presets are regenerated with the new defaults (Softness 0.5, Asymmetry 0.25).
10. **Editor:** a third fader, "Asymmetry", after Softness in the brickwall panel. The panel keeps its title. The randomizer leaves Asymmetry alone, as it does Softness.

## Risks / Trade-offs

- **[The DC removal raises one polarity's peaks]** → The headroom is measured after the blocker (decision 4), and a spec scenario checks peaks at Asymmetry 0, 0.25 and 1.
- **[Migrated projects don't match today's sound exactly]** → Softness 0, Asymmetry 0 is a hard clip at `L`. Today's curve tops out about 0.013 higher (0.756 vs 0.743), about 0.15 dB louder peaks. Recorded, not hidden.
- **[Softness automation changes meaning]** → BREAKING in the proposal; can't be migrated.
- **[Eco changes sound]** → Intended: Eco gets the same headroom and curve as the other modes. Its aliasing should improve, because the soft knee makes fewer high harmonics.
- **[CPU at 16×]** → Decision 7, and the output-path benchmarks. Stop and ask if Render is more than 20 % slower than after m06.
- **[The editor's repaint while the dot is lit]** → Only while above rest; idle editors stay idle.

## Migration Plan

Preset loader and `filter_state` as in decision 9. New instances and regenerated factory presets use the new defaults. Host automation of Diode Softness is BREAKING.

## Open Questions

- The final `H` (decision 4) and whether it can drop below 1.3 for the soft default.
- Whether the knee's tanh shape needs a gentler start for Softness near 0 to avoid a near-hard knee sounding harsh; to judge by ear once built.

## Measurements

### Before (task 1.1)

The current curve (m06's), with the new tests' settings. `limit` is the ceiling ÷ 1.3, the curve's limit in every mode after this change.

| Test | Current code |
|---|---|
| Wide knee: sine at 0.6 · limit, Softness 1 vs 0 (Eco) | +0.000 dB vs +0.000 dB (bound ≤ −0.2 dB) |
| 7th+ harmonics vs fundamental, 1 kHz at 2 · limit, Render, Softness 0 / 0.5 / 1 | −30.33 / −30.27 / −30.24 dB (rises: Softness barely acts) |
| Peak-to-peak, 200 Hz at 4 · limit, Eco, Asymmetry 0 | 1.935, want 1.486 (Eco clips at the ceiling, not the limit) |
| 2nd harmonic vs fundamental, Asymmetry 0 / 0.25 / 1 | −171 dB at all three (no asymmetry) |
| Peak, 5 kHz at 2 · ceiling, Softness 0.5, Eco / Normal / High / Render | 0.9668 / 0.9673 / 0.9697 / 0.9719, over the 0.9661 ceiling |
| Eco vs Render, 200 Hz at 2 · ceiling: fundamental | 69.02 vs 66.96 dB (2.1 dB apart, bound 0.1) |
| Eco is the bare curve at the limit | fails (Eco clips at the ceiling) |

Passing on the current code, as references: hard knee at 0, DC centring, symmetric curve unfiltered, no click on Asymmetry jumps (the last three can't fail while Asymmetry has no effect), and `output-clipper`'s transparency at 0.3, alias bounds and delay.

Task 1.2, on the current code: the dot tests for hard clipping and release fail (the dot is always drawn at rest), the idle test passes; an old preset or host state keeps its Diode Softness (0.7 in the tests) and gets Asymmetry 0.25 instead of 0 / 0; state that has Diode Asymmetry and the new-instance defaults (0.5 / 0.25) pass.

### The curve and DC removal (tasks 2.1–2.2)

All `diode-stage` curve tests pass with the tanh knee (`w = 0.8·softness`) and the 2 Hz blocker. `output-clipper`, clipper alone, ~5 kHz at twice the ceiling, defaults (Softness 0.5, Asymmetry 0.25):

| Mode | Alias before (m06) | Alias after | Lag at 1 kHz |
|---|---|---|---|
| Eco | — | −19.7 dB | −0.015 samples (the blocker's phase lead) |
| Normal | −51.0 dB | −67.0 dB | 4.2 samples |
| High | −64.1 dB | −94.2 dB | 4.7 samples |
| Render | −75.8 dB | −105.0 dB | 5.0 samples |

The same figures through the hard clamp (the clipper's peaks, 0.89 at most, stay under it). The whole chain is unchanged at −22.8 dB in Normal to Render (the drive's own aliasing dominates).

*As built (measurement):* `clipper_sine` now warms up for about a second (whole windows) instead of one window, so the blocker (time constant 80 ms) has settled: a decaying offset leaks into the low bins and read as fold-back (−46.8 dB in every mode). `clipper_delays_less_than_eight_samples` accepts down to −0.05 samples, the blocker's lead at 1 kHz; the spec bound (under eight samples) is unchanged.

*As built (code):* Softness and Asymmetry snap to their targets until the first sample is processed, and `reset()` snaps them too, so a preset loaded before playback doesn't glide in. The meter tracks the curve's input and output peaks on the high-rate samples.

### tanh evaluation (task 2.3)

Medians of five within-run ratios against libm's `tanhf`:

| Variant | Error | Clipper alone, 16×, stereo, every sample in the knee | Whole output path, Render |
|---|---|---|---|
| Rational (Lambert 7/6, clamped at 1) | 9.6e-5 | 0.654× | 1.020× (noise) |
| Table, 513 points on [0, 8], linear | 2.3e-5 | 0.629× | 1.035× (noise) |

Chosen: the table (decision 7). It is the cheaper, its error is a quarter of the bound, it is monotonic by construction, and it doesn't depend on the libm, so gnu and musl compute the same curve. It is built once, in `OversampledClipper::new`, off the audio thread. `table_tanh_is_accurate_and_monotonic` checks it over [0, 12] in 1e-5 steps.

### Headroom sweep (task 3.1)

`print_headroom_sweep` (release build). Peaks after the DC removal, Softness 0.5, Asymmetry 0 / 0.25 / 1, every mode; alias through the hard clamp:

| H | 5 kHz at 2 · ceiling, worst peak | Any frequency 50 Hz–19 kHz, worst peak | Alias, Softness 0 / Asym 0, Normal / Render | Alias, defaults, Normal / Render |
|---|---|---|---|---|
| 1.10 | 1.125× ceiling | 1.383× (7 kHz) | −28.9 / −28.9 dB | −36.8 / −36.7 dB |
| 1.15 | 1.088× | 1.338× | −31.5 / −31.4 | −46.1 / −45.9 |
| 1.20 | 1.052× | 1.295× | −35.5 / −35.4 | −70.3 / −105.8 |
| 1.25 | 1.019× | 1.254× | −43.2 / −43.5 | −68.3 / −105.4 |
| **1.30** | **0.987×** | 1.215× | **−50.7 / −75.5** | **−67.0 / −104.9** |

`H` stays at 1.3: it's the smallest swept value that keeps the 5 kHz test tone under the ceiling, and it holds the alias bounds (−40 / −60 dB) at Softness 0 as well as at the defaults. It can't drop for the soft default (Open Question 1): at 1.25 the test tone peaks 1.9 % over the ceiling. As with m06, no headroom covers every frequency: at 7 kHz the half-bands' overshoot still reaches 1.21× the ceiling, which the output limiter clamps; the spec only bounds the 5 kHz test tone. `clipper_alias_bounds_hold_through_the_hard_clamp` now checks Softness 0 / Asymmetry 0 too.

### CPU (task 3.2)

`plugin_output_path_sixteen_voices` and its Render twin, this change against a clean worktree of the previous commit (`d06272a`), five alternating runs of both frozen binaries, median of the per-run ratios: Normal **1.030×**, Render **1.003×** (ratios 0.93–1.06 and 0.92–1.04: noise). Far under the 20 % bound.

### Open issue: the output's own DC (paused, 2026-11-10)

With the factory presets regenerated (Softness 0.5, Asymmetry 0.25), `preset_levels_are_sane` fails: Cable Bow −13.2, Chain Bell Tower −12.7, Workshop Plate Scrape −14.7 dB under the median (bound −12). The cause isn't the curve: several presets already carry a large infrasonic offset at the output, before this change. Rusted Plate Strike's left channel spans −0.016 to +0.421 (mean +0.097 over 2 s, +0.37 over the first 100 ms). The DC blocker (decision 5) filters the whole signal whenever Asymmetry > 0, so it removes that offset even when the signal never reaches the knee: RMS −9.1 dB (Industrial Plate Bow), −8.6 (Rusted Plate Strike, Pipe Bell Cluster), −4 to −5 dB (others); Softness alone changes nothing (−0.0 to −0.3 dB). The waveform becomes bipolar, and some negative peaks grow (Industrial Plate Bow 0.312 → 0.627, at the negative limit).

The maintainer chose to pause this change and investigate where the output's offset comes from first.

**Findings (2026-11-10).** The offset comes from the voices, not from a post-chain stage: with BODY 0 it is still 21–35 % of the output's power in the affected presets, and the FEM body (fed the mono sum) scales it with the rest of the signal (about 5–8× here). It follows the force envelope: with a felt mallet it rises over ~60 ms and falls slowly (Rusted Plate Strike, BODY 0: +0.016, +0.043, +0.071, +0.076, then −0.003 per 20 ms); with a bow it is constant while bowing (Industrial Plate Bow: +0.33 with BODY 2.4); after note-off it is gone. Each mode is a coupled-form one-pole whose output, `Im(z)`, has a non-zero gain at 0 Hz, so a one-signed force (bow pressure, a mallet's long contact) leaves a static deflection on top of the ringing. Across the factory bank, 21 of 50 presets have at least 1 % of their output power at DC and 9 at least 20 %; by exciter (largest share): Bow 47 %, StiffPoint 33 %, FeltMallet 26 %, CorrugatedDrag 22 %, HardMallet 16 %, MetalPipe 2 %, the other ten 0 %. Besides the diode's blocker, the offset uses up headroom in the drive and the clipper (a +0.41 offset makes the positive half clip early) and reaches the host's output. Options recorded for when it resumes: remove only the DC the curve creates (`y − LP(y − x)`), or keep filtering the whole signal and re-level the factory presets.

*As built (test):* "Limits per polarity" is measured in Eco; in the oversampled modes the half-bands ring on the clipped edges and add several percent to the peak-to-peak span.

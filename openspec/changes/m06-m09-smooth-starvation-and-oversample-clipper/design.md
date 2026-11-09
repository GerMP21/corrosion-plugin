# Design

## Context

- `LorenzDrive::process` computes `starvation_gate` as 0.3 when `(z·0.01).fract() + 0.5 > 1 − 0.7·starvation`, else 1.0, and multiplies the drive input by it. On the attractor `z` stays between about 10 and 42 (5th–95th percentile 15–35), so `fract` is just `z·0.01` and the test is `z > 50 − 70·starvation`: never true below about 0.12, always true above 0.71.
- `OversampledClipper::process_channel` interpolates `prev → input` at `k/N`, clips each point with `diode_clip` and averages. `set_oversample_factor` takes 1, 4, 8 or 16 from the post chain's quality mode. `set_sample_rate` stores a value nobody reads.
- `analyze_post_chain_aliasing_at_quality` measures the whole chain (drive included) and two tests guard it: Render under −10 dB, and Render below Eco.

## Goals / Non-Goals

**Goals:** a click-free starvation effect that acts across the control's whole range; a clipper that is transparent below its ceiling and band-limited above it; a delay small enough not to report (under eight samples); bounded CPU.

**Non-Goals:**
- Changing the diode curve, the ceiling or the softness control.
- Oversampling the Lorenz drive itself (its aliasing is part of its character and outside these findings).

## Decisions

1. **Smooth the starvation gate with a one-pole follower, τ = 5 ms** (coefficient from the time constant, so it is sample-rate independent). The decision stays a two-state test on `z`; only its output is smoothed. At starvation 0 the target is 1; once the follower is within 1e-4 of 1 it snaps to exactly 1 and is skipped from then on, so drive output with starvation off is bit-identical and turning starvation off doesn't click. *Alternative:* a continuous gate function of `z`. It would change the effect's character, not just remove the steps.
1b. **Rescale the threshold to the attractor (added 2026-11-09, maintainer's choice):** starved when `z > 38 − 22·starvation`, i.e. the threshold runs from about the attractor's 99th percentile at 0 to its 5th at 1. See Measurements for the starved share at each value (5 % at 0.1 to 87 % at 1 over a minute). The starved gain stays 0.3. *Alternative:* keep the old test. Half the control's range is then a fixed cut. *Alternative:* a quantile table mapping starvation to an exact share. More code for little audible difference.
2. **Half-band polyphase IIR stages** (two parallel all-pass chains, the classic design used by HIIR). The coefficients come from the standard elliptic half-band designer, computed once in `new()` (allocation is allowed there), for about 90 dB of stop-band rejection with a transition band ending at 0.5 of the stage's rate. Four stages cover 16×; Normal uses the first two, High three. Each stage keeps its own state per channel in fixed arrays.
3. **Up, clip, down.** Up-sampling inserts zeros and filters stage by stage (each stage doubles the rate); the diode curve runs on every high-rate sample; down-sampling runs the stages in reverse. *Alternative:* FIR half-bands with linear phase. They add a fixed latency that would have to be reported and compensated, and cost more at 16×.
   *As built (2026-11-09):* the half-bands are not "well under a sample". A polyphase half-band is `0.5·(A0(z²) + z⁻¹·A1(z²))`, so it carries half a sample of structural delay, and the elliptic steepness adds more: about 3.5 base samples for the first stage, about 5 (Normal) to 6 (Render) for the whole up/down chain at 1 kHz. A filter soft enough to stay under a sample lets the 5 kHz test sine's 5th harmonic (25 kHz) fold back to 23 kHz above the −60 dB bound. The maintainer chose to keep the IIR and not report the delay (0.1–0.125 ms at 48 kHz, not note-dependent); the spec's bound is now eight samples in every mode.
3b. **Pass band to 22 kHz (relative to 48 kHz):** the first stage's transition band runs from 22 to 26 kHz (normalised width 0.0208 at its rate), so the 5th harmonic of a 5 kHz tone (25 kHz) is rejected by about 110 dB instead of 36 dB with a 20 kHz pass band, for 9 coefficients instead of 8. Later stages keep the same pass edge, which leaves them wide transitions: 5, 4 and 4 coefficients at about 90 dB. All widths are fractions of the stage rate, so the filters scale with the host rate.
4. **Eco stays 1×:** `diode_clip` on the input, bit-identical to today's Eco.
5. **Spec bounds checked on the clipper alone:** a dedicated measurement drives a sine past the ceiling straight into the clipper (no Lorenz drive), so the alias figure isolates M9. The existing whole-chain tests stay as regression lines.
6. **CPU budget:** if Render costs more than 20 % above today in `plugin_output_path_sixteen_voices` (median of 5 runs, within-run comparison against a clean worktree), stop and ask before going further. Shorter filters for the later stages (which see only the first stage's residue) are the first fallback.

## Risks / Trade-offs

- **[IIR phase shift colours the top octave slightly]** → Minimum-phase half-bands keep the magnitude flat; the transparency scenario bounds it.
- **[Coefficient design code is numerically delicate]** → It runs once, in f64, and a test checks every stage's stop-band rejection and pass-band flatness directly.
- **[Patches that relied on the box filter's high-cut sound brighter]** → Expected; the old behaviour was a side effect, not a control.

## Migration Plan

None: no parameter or stored-value change.

## Measurements

### Bias Starvation (tasks 1.1–1.2, 2026-11-09)

Largest sample-to-sample output step, as a multiple of the step with Bias Starvation 0 (100 Hz sine at 0.5, Drive Amount 2.5, chaos 0, two seconds; `print_starvation_steps`):

| Starvation | Before 44.1k | Before 48k | Before 96k | After 44.1k | After 48k | After 96k |
|---|---|---|---|---|---|---|
| 0.10 | 25.3× | 27.5× | 54.8× | 1.000× | 1.000× | 1.000× |
| 0.25 | 13.5× | 14.9× | 31.6× | 1.000× | 1.004× | 1.005× |
| 0.35 | 26.2× | 28.6× | 57.1× | 1.000× | 1.000× | 1.000× |
| 0.50 | 25.9× | 28.3× | 56.8× | 1.000× | 1.000× | 1.000× |
| 0.75 | 0.52× | 0.52× | 0.52× | 0.992× | 0.995× | 0.995× |
| 1.00 | 0.52× | 0.52× | 0.52× | 0.992× | 0.995× | 0.995× |

Before, the steps are up to 0.31 per sample; at 0.75 and above there are none because the gate never switches (a fixed 0.3 gain, hence 0.52×). The spec's original scenario used only Bias Starvation 1 and so passed on the old code; the maintainer chose a sweep over 0.1–1 (2026-11-09).

Attractor `z` after the first second, 20 s (`print_z_percentiles`): 48 kHz min 10.2, p5 15.3, p50 26.1, p95 34.3, p99 38.8, max 41.5; 44.1 and 96 kHz within 2 of these.

Starved share over 60 s at 48 kHz, after the first second (`print_starved_shares`):

| Starvation | 0.05 | 0.10 | 0.25 | 0.35 | 0.50 | 0.75 | 1.00 |
|---|---|---|---|---|---|---|---|
| Before (old test) | – | 0 % | 0 %* | ~50 % | ~95 % | 100 % | 100 % |
| After | 3.6 % | 5.2 % | 12.7 % | 21.3 % | 38.7 % | 69.0 % | 87.3 % |

\* 0 % in the first ten seconds, 8 % over a longer run.

*As built:* the range scenario first used a ten-second window. That window isn't representative of a chaotic signal: after `reset()` the attractor spirals around one lobe for several seconds with `z` in about 23–30, so values whose threshold sits outside that band stay all-starved or all-full for that stretch (0 % at 0.1 and 0.25, 100 % at 0.75 and 1 over seconds 1–10 in Rust). Over two minutes, at most 2 of 24 five-second windows per value are stuck (a float32 Python replica of the integrator). The scenario now uses sixty seconds. The stretch right after `reset()` is left as is: starting the attractor elsewhere would also change the chaos modulation, outside this finding.

### Clipper before the change (task 2.1, 2026-11-09)

The clipper alone (`analyze_clipper_aliasing`, `measure_clipper_response`, printed by `print_clipper_measurements`), 48 kHz, 4096-frame bin-centred tones, ceiling 0.9661, softness 0.5. The analyzers' DFT moved to f64 with exact phase reduction: the f32 one's phase error at N = 4096 (about 1e-3 rad) left a leakage floor near −60 dB.

Gain and lag for a sine at 0.5:

| Mode | 105 Hz | 996 Hz | 10.0 kHz | 18.0 kHz | 20.0 kHz | Lag at 1 kHz |
|---|---|---|---|---|---|---|
| Eco | 0.000 dB | 0.000 dB | 0.000 dB | 0.000 dB | 0.000 dB | 0.000 |
| Normal | −0.000 dB | −0.017 dB | −1.85 dB | −6.99 dB | −9.02 dB | 0.375 |
| High | −0.000 dB | −0.018 dB | −1.97 dB | −7.96 dB | −10.89 dB | 0.437 |
| Render | −0.000 dB | −0.018 dB | −2.00 dB | −8.25 dB | −11.52 dB | 0.469 |

Alias ratio for a 5004 Hz sine at twice the ceiling (strongest alias at 22 980 Hz, the 5th harmonic folded, in every mode):

| Mode | Eco | Normal | High | Render |
|---|---|---|---|---|
| Clipper alone | −25.42 dB | −32.39 dB | −32.87 dB | −32.99 dB |
| Whole chain (`analyze_post_chain_aliasing_at_quality`, drive 2.5, after task 1.2) | −19.55 dB | −27.52 dB | −29.58 dB | −30.31 dB |

So on the old clipper: transparency fails (−1.9 to −2.0 dB at 10 kHz, −7.0 to −8.2 dB at 18 kHz); the alias bounds fail (Normal −32.4, bound −40; Render −33.0, bound −60); Eco plain clipping, the quality ordering (by 0.5 and 0.1 dB) and the delay pass.

### Half-band stages (task 2.2)

Designed at 90 dB, pass edge 22/48 of the base rate (`print_stage_responses`, levels of a down-sampled tone):

| Stage | Transition | Coefs | Pass band (worst sampled) | Stop band (worst sampled) |
|---|---|---|---|---|
| 1 (2×) | 0.0208 | 9 | −0.000 dB | −93.1 dB |
| 2 (4×) | 0.1354 | 5 | −0.000 dB | −103.8 dB |
| 3 (8×) | 0.1927 | 4 | −0.0013 dB (at 0.2 of its rate, past the 22 kHz edge) | −107.2 dB |
| 4 (16×) | 0.2214 | 4 | −0.0025 dB (likewise) | −107.2 dB |


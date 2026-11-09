# Design

## Context

- `LorenzDrive::process` computes `starvation_gate` as 0.3 when `(z·0.01).fract() + 0.5 > 1 − 0.7·starvation`, else 1.0, and multiplies the drive input by it. On the attractor `z` stays between about 10 and 42 (5th–95th percentile 15–35), so `fract` is just `z·0.01` and the test is `z > 50 − 70·starvation`: never true below about 0.12, always true above 0.71.
- `OversampledClipper::process_channel` interpolates `prev → input` at `k/N`, clips each point with `diode_clip` and averages. `set_oversample_factor` takes 1, 4, 8 or 16 from the post chain's quality mode. `set_sample_rate` stores a value nobody reads.
- `analyze_post_chain_aliasing_at_quality` measures the whole chain (drive included) and two tests guard it: Render under −10 dB, and Render below Eco.

## Goals / Non-Goals

**Goals:** a click-free starvation effect that acts across the control's whole range; a clipper that is transparent below its ceiling and band-limited above it; no reported latency; bounded CPU.

**Non-Goals:**
- Changing the diode curve, the ceiling or the softness control.
- Oversampling the Lorenz drive itself (its aliasing is part of its character and outside these findings).

## Decisions

1. **Smooth the starvation gate with a one-pole follower, τ = 5 ms** (coefficient from the time constant, so it is sample-rate independent). The decision stays a two-state test on `z`; only its output is smoothed. At starvation 0 the target is 1; once the follower is within 1e-4 of 1 it snaps to exactly 1 and is skipped from then on, so drive output with starvation off is bit-identical and turning starvation off doesn't click. *Alternative:* a continuous gate function of `z`. It would change the effect's character, not just remove the steps.
1b. **Rescale the threshold to the attractor (added 2026-11-09, maintainer's choice):** starved when `z > 38 − 22·starvation`, i.e. the threshold runs from about the attractor's 99th percentile at 0 to its 5th at 1. See Measurements for the starved share at each value (5 % at 0.1 to 87 % at 1 over a minute). The starved gain stays 0.3. *Alternative:* keep the old test. Half the control's range is then a fixed cut. *Alternative:* a quantile table mapping starvation to an exact share. More code for little audible difference.
2. **Half-band polyphase IIR stages** (two parallel all-pass chains, the classic design used by HIIR). The coefficients come from the standard elliptic half-band designer, computed once in `new()` (allocation is allowed there), for about 90 dB of stop-band rejection with a transition band ending at 0.5 of the stage's rate. Four stages cover 16×; Normal uses the first two, High three. Each stage keeps its own state per channel in fixed arrays.
3. **Up, clip, down.** Up-sampling inserts zeros and filters stage by stage (each stage doubles the rate); the diode curve runs on every high-rate sample; down-sampling runs the stages in reverse. *Alternative:* FIR half-bands with linear phase. They add a fixed latency that would have to be reported and compensated, and cost more at 16×.
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


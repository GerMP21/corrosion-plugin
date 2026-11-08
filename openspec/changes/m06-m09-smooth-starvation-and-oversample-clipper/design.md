# Design

## Context

- `LorenzDrive::process` computes `starvation_gate` as 0.3 when `(z·0.01).fract() + 0.5 > 1 − 0.7·starvation`, else 1.0, and multiplies the drive input by it.
- `OversampledClipper::process_channel` interpolates `prev → input` at `k/N`, clips each point with `diode_clip` and averages. `set_oversample_factor` takes 1, 4, 8 or 16 from the post chain's quality mode. `set_sample_rate` stores a value nobody reads.
- `analyze_post_chain_aliasing_at_quality` measures the whole chain (drive included) and two tests guard it: Render under −10 dB, and Render below Eco.

## Goals / Non-Goals

**Goals:** a click-free starvation effect; a clipper that is transparent below its ceiling and band-limited above it; no reported latency; bounded CPU.

**Non-Goals:**
- Changing the diode curve, the ceiling or the softness control.
- Oversampling the Lorenz drive itself (its aliasing is part of its character and outside these findings).

## Decisions

1. **Smooth the starvation gate with a one-pole follower, τ = 5 ms** (coefficient from the time constant, so it is sample-rate independent). The decision logic stays; only its output is smoothed. At starvation 0 the gate is exactly 1 and the follower is skipped, so drive output with starvation off is bit-identical. *Alternative:* a continuous gate function of `z`. It would change the effect's character, not just remove the steps.
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

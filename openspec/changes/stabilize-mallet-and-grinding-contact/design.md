# Design

## Context

See proposal.md, "Why", for the two faults.

- **The loop today.** Each sample, the voice calls the exciter with the resonator's previous-sample contact displacement `D` and velocity `V`, with `V = ΔD·fs/48k`, in 48 kHz-sample units. The returned force is multiplied by the force envelope `e` and the drive trim `t`, then fed to the resonator. Every mode then gets force `F·e·t·b_i`, where `b_i = (1 − c) + w_i·c` is the strike-position blend.
- **The modes** are coupled-form: `z_i ← p_i·z_i + g_i·x_i`, with output `y_i = Im z_i` and `Im g_i = b0_i`. The contact displacement is `D = Σ w_i·y_i`.
- **Immediate response.** This sample's displacement is therefore affine in this sample's force: `D = D_free + C·F`, where:
  - `D_free = Σ w_i·Im(p_i·z_i)` is where the resonator goes on its own;
  - `C = e·t·Σ w_i·b_i·b0_i` is the instantaneous compliance;
  - the velocity follows as `V = (D − D_prev)·fs/48k`.

## Goals / Non-Goals

**Goals:**
- No divergence for these two exciters at any drive. Tested at 16× above the new default.
- A median default note near the other struck exciters, and a worst case of at most 0.9 (see specs).
- The four "done when" tests pass, and `tests/no_alloc.rs` passes in debug.

**Non-Goals:**
- **Other exciters.** `resolve-exciter-tradeoffs` adopts the compliance and helper for the rest.
- **The hash noise** in HeavyGrinding (`sin(phase)·43758`), which is replaced in `resolve-exciter-tradeoffs`.
- **A per-voice drive governor.** The trims stay fixed here.
- **Bit-identical output** for these two exciters at 48 kHz. Every other exciter's output stays bit-identical.

## Decisions

1. **The resonator reports its immediate response on request.**
   - `ModalResonator::contact_prediction()` returns `(D_free, C₀, D_prev)`, where `C₀ = Σ w_i·b_i·b0_i` and `D_free` uses the current strike-position weights.
   - It costs one extra pass over the modes: two multiplies and an add per mode. It runs only when the active exciter needs it, so voices on other exciters pay nothing.
   - The voice scales `C₀` by the force envelope and the drive trim before passing it on, so the exciter sees its own force's real effect.
   - *Alternative:* compute `D_free` every sample inside `process_sample`. That costs every voice.

2. **A shared monotone contact solver** (`dsp::utils::contact`).
   - Both contact laws are monotone in `F`: more force means less penetration (mallet) or less slip (grinder). So `F = h(F)` has exactly one root in `[0, h(0)]`, or `[h(0), 0]` for a negative force.
   - The solver runs safeguarded Newton, kept inside the bracket and falling back to bisection, with at most 4 iterations. It is allocation-free, and it has a unit test against a brute-force solve.
   - This is the zero-delay feedback pattern already used for the filter (C1).
   - *Alternatives:*
     - Bounding the explicit terms, which only moves the threshold.
     - Oversampling the contact, which costs more and also only moves the threshold.

3. **HardMallet: dissipative Hertz, solved implicitly, and the mallet leaves.**
   - The force is `F = m·k·d^1.5·(1 + λ·v_rel)` for penetration `d > 0`, else 0 (Hunt–Crossley), with:
     - `d = x_m − (D_free + C·F)`;
     - `v_rel = v_m − V(F)`;
     - `λ = impact_damping` mapped to the old range.
   - The mallet's velocity is updated with the solved force.
   - "Gravity" applies only before the first contact, as the throw. After contact the mallet rebounds and goes inactive once it moves away and is separated by more than a small margin, with no re-pull into the surface.
   - The old energy-transfer and impact-count damping are replaced by λ.
   - The parameters keep their names and ranges: mass, stiffness, damping.

4. **HeavyGrinding: smooth friction, solved implicitly, and bounded tearing.**
   - Friction: `F_f = P·μ·tanh(v_rel / v_s)`, with `v_rel = v_g − V(F)` and `v_s = 0.05·v_g` (minimum 1e-4). This replaces the sign function, which flipped every sample on a high mode.
   - Solved with the same solver: more friction lowers the slip.
   - Tearing becomes `noise·min(|v_rel|, 2·v_g)·grit·2`, using the solved `v_rel`. It is then bounded by the grinder's own speed and can't grow with the resonator's motion.
   - The output keeps its `·v_g·3` scale. The trim absorbs the level.

5. **Trims re-tuned by measurement.** Run `print_exciter_overdrive` for exciters 4 and 10. Set each trim so its median over the 27-note sweep is the mean of Drumstick's and WireBrush's medians (about 0.176), then lower it if the worst case exceeds 0.9. Record the median, the worst case and where it falls.

## Risks / Trade-offs

- **[The worst/median spread over objects is more than 5× (14 dB), so the 0.9 worst case pins the median under 6 dB of the target]** → Measure first. If so, report it and stop: the per-voice governor in `resolve-exciter-tradeoffs` is the fix for spread. Don't widen the spec here.
- **[HeavyGrinding's hash noise breaks `every_exciter_has_the_same_level_at_any_sample_rate` once the exciter is louder]** → Its tolerance is the default 1.5 dB today. If it now fails because of the noise, which fails for the jet the same way, record the numbers and give it a documented tolerance like the jet's (3 dB), pending `resolve-exciter-tradeoffs`. A failure from the contact itself must be fixed.
- **[The mallet's bounce-off shortens its sustained excitation]** → Intended. The struck tail and attack-timing tests guard the behaviour. Once the mallet leaves, its whole-note peak should be the strike, so try dropping HardMallet from the 20 ms timing window, and keep the window only if the whole-note peak still fails.
- **[`contact_prediction` uses the strike-position weights from the start of the sample]** → The bus updates them before the modes run. Call the prediction after that update, so they are this sample's.

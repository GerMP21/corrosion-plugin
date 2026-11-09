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
2. **Softness sets the width:** `w = 0.5·softness`, so the knee starts up to 6 dB under `L` at Softness 1.
3. **Asymmetry lowers the negative limit:** `L₊ = L`, `L₋ = L·2^(−asymmetry)` (one diode against two in series at 1, 6 dB), and the negative knee is firmer: `w₋ = w·(1 − asymmetry/2)`. Default 0.25 (`L₋` 1.5 dB under `L₊`).
4. **The same `L` in every mode:** `L = ceiling / H` in Eco too. `H` is re-measured for the new curve (task 3.2). It must keep the hard-driven tone's peaks at or under the ceiling after DC removal at Softness 0.5 and every Asymmetry, and keep `output-clipper`'s alias bounds through the hard clamp at Softness 0 (the migrated setting). Start from 1.3. **Stop and ask** if it needs more than 1.3 (louder loss than today) or if one constant can't satisfy both.
5. **DC blocker after the down-sampling:** a one-pole high-pass at 5 Hz per channel at the host rate (coefficient from the cutoff, so it's rate-independent). It's mixed in with weight `g = min(1, asymmetry_smoothed / 0.05)`: at exactly 0 it's bypassed and its state cleared, so a symmetric curve is unfiltered and bit-identical; turning asymmetry up fades the blocker in, so there's no step. *Alternative:* always on. It would filter every patch, and the "symmetric curve is unfiltered" property couldn't hold.
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

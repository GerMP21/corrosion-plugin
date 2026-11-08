# Design

## Context

- **Plumbing exists:** `VoiceControls` already carries `exciter_pressure`, `exciter_speed` and `exciter_roughness` from the host parameters.
- **Note-on:** `Voice::note_on_with_controls` calls each exciter's `set_parameters` from its dedicated controls, then `trigger`.
- **Force scaling:** `Voice::force_scale()` multiplies the force (drive level × test drive × governor gain). Contact exciters see the same scale through `contact_prediction`, so their solve stays consistent.
- **Live updates:** `update_live_controls` pushes a few live controls into held voices once per buffer.
- **Safety net:** the drive governor (`resolve-exciter-tradeoffs`) bounds the pre-clamp signal, so a larger force can't overdrive.

## Goals / Non-Goals

**Goals:**
- Each control does something on every exciter it can.
- The defaults are bit-identical.
- Nothing overdrives.

**Non-Goals:**
- Replacing the dedicated parameters.
- Speed or Roughness on exciters with no motion or grit parameter (they're inert there, and documented as such).
- Live Speed or Roughness on held notes: dedicated parameters are note-on only today.

## Decisions

1. **Pressure: `2^((p − 0.5)·4)`, applied in `force_scale`.**
   - The factor is 0.25× at 0 (−12 dB), 1× at 0.5 and 4× at 1 (+12 dB). It is exactly 1.0 at 0.5 in f32, so the default stays bit-identical.
   - Because it's part of `force_scale`, contact exciters see it in their prediction, and the governor follows.
   - It's pushed live through `update_live_controls`, smoothed with a one-pole of about 10 ms (through `rate::smoother`) to avoid zipper noise.
2. **Speed: multiplier `2^((s − 0.5)·2)`, ×0.5 to ×2, on motion-speed parameters at note-on.** Spreads and durations take the inverse, since faster motion means a shorter spread. Each scaled value is clamped to its host parameter's range (`src/params.rs`). Inventory (task 1.1, from each exciter's `set_parameters`):

   | Exciter | Parameter | Range | Scaling |
   |---|---|---|---|
   | Bow (1) | `bow_speed` | 0.1–2.0 | × |
   | WireBrush (6) | `spread_duration` (ms) | 10–250 | ÷ |
   | MetalChain (8) | `drop_envelope_spread` (ms) | 40–400 | ÷ |
   | StiffPoint (9) | `scrape_speed` | 0.1–2.5 | × |
   | HeavyGrinding (10) | `grind_speed` | 0.1–2.5 | × |
   | CorrugatedDrag (11) | `drag_speed` | 0.1–2.5 | × |
   | TensionRise (12) | `pull_speed` | 0.05–1.55 | × |
   | ParticleRain (16) | `flow_rate` | 0.1–3.1 | × |

   **Inert:** HandStrike, FeltMallet, HardMallet, Drumstick and MetalPipe (one strike, no motion parameter); PneumaticJet (`air_pressure` is a force); ElectromagneticHum (`mains_frequency` is a pitch); TensionSnap (`pull_distance` is a distance, with no rate).
3. **Roughness: multiplier `r / 0.3`, from 0 to ×3.33, on grit and randomness parameters at note-on.** A linear factor keeps 0 meaning "no grit", which is musically useful, and 1.0 at 0.3 exactly. Values are clamped to each host parameter's range. Inventory (task 1.1):

   | Exciter | Parameter | Range |
   |---|---|---|
   | Bow (1) | `rosin_grip` (the bow's `roughness`: friction curve and ripple) | 0.05–1.5 |
   | WireBrush (6) | `amplitude_randomization` | 0–1 |
   | MetalChain (8) | `internal_rattle` | 0–1 |
   | HeavyGrinding (10) | `surface_grit` | 0–1 |
   | TensionRise (12) | `slip_stochasticity` | 0–1 |
   | PneumaticJet (13) | `turbulence_chaos` | 0–2 |
   | ParticleRain (16) | `mass_variance` | 0–2 |

   **Inert:** HandStrike, FeltMallet, HardMallet, Drumstick and MetalPipe (no grit parameter); StiffPoint (its chatter is a deterministic stick-slip); CorrugatedDrag (its ridges are regular geometry, not grit); ElectromagneticHum; TensionSnap.

   Bow's `rosin_grip` has a floor of 0.05, so Roughness 0 leaves the bow at its minimum grip rather than at zero.
4. **Apply the multipliers in one place.** A helper scales the dedicated controls in a copy of `VoiceControls` before the per-exciter `set_parameters` dispatch, so the 16 call sites don't change. When a factor is exactly 1 the helper leaves the values untouched (no clamp), so the defaults are bit-identical even for a value outside its host range.
   - *Alternative:* scale inside each exciter. That's 16 edits, and each exciter would need to know about globals.

## Risks / Trade-offs

- **[Raising Pressure engages the governor more]** → That's its job. The no-overdrive scenario pins it at the maximum settings.
- **[Speed changes friction exciters' level, not just their character]** → Accepted. The governor bounds the peak, and the levels are tuned at the default.
- **[Bit-identity at the defaults relies on exact factors]** → `2^0 = 1` and `0.3/0.3 = 1` are exact in f32, and the default-unchanged scenario tests it.

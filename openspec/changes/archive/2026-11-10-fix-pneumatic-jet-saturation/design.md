# Design

## Context

`PneumaticJet::process_sample` computes `saturated = v² − chaos·v³`, clamped at 0, where `v = air_pressure − resonator_velocity`. For `v ≥ 0` the cubic peaks at `v* = 2/(3·chaos)` with value `4/(27·chaos²)` and is 0 at `1/chaos`. Air Pressure ranges 0.1–2.1, Turbulence Chaos 0–2, and velocity scales the pressure by 0.3–1.0.

## Decisions

1. **Hold the peak above the knee** (`v.min(v*)` before the cubic). The curve stays monotonic, identical below the knee (so default notes don't move), and bounded, which is what the saturation was for. Alternatives: a smooth soft-clip (tanh) would change every note; removing the cubic would let the burst grow as `v²`.
2. **The knee is computed in `set_parameters`**, with infinity at chaos 0 (the curve is then `v²`, which never turns over). No division per sample.
3. **Negative `v` is untouched.** The cubic is positive there and grows; the `min` doesn't affect it.

## Risks / Trade-offs

- Notes past the knee are louder than before (they were quiet or silent). The voice's drive governor bounds them as it does every exciter.

## Measurements

**Tests first.** `harder_playing_never_silences_the_jet` (every object, Air Pressure 0.1/1.1/2.1 × Turbulence Chaos 0/0.6/2, velocity 64 vs 127, 1 s held) failed on 27 of 81 cases before the fix, all silent (−240 dB):
- Air 1.1, Chaos 2 and Air 2.1, Chaos 2: silent at both velocities on all 9 objects;
- Air 2.1, Chaos 0.6: −10 to −23 dB at velocity 64, silent at 127.

After the fix all 81 pass. *As built:* the bound is "not below −120 dB and at most 6 dB under velocity 64". The first draft said 3 dB, but Pipe plays 3.3 dB quieter at 127 than at 64, from the voice's dynamics: the jet's force is now monotonic. Air Pressure 0.1 plays around −72 dB: it is the bottom of the range, and it gets louder with velocity.

**Default notes** (`print_jet_default_notes`, C4, 1 s, default controls): velocity 100 bit-identical on all 9 objects. At velocity 127 the hash changes on Pipe, Plate, Taut Cable, Coil Spring, Sheet Metal and Industrial Cog; RMS is unchanged to 0.01 dB on all of them.

**Factory bank:** reseeded. `pneumatic_jet/pipe`, `/chain` and `/taut_cable` go back to their first draw (try 0). Only `stiff_point_scrape/coil_spring` is still re-drawn: it is chaotic, not silenced (−9.6 dB at velocity 100, −29.6 at 127, −31 to −40 at 20–80).

**CPU:** not benchmarked. The change adds one `min` per sample, only while exciter 13 plays.

**Other parameters** (maintainer's question, 2026-11-10). `print_velocity_sweep` (ignored, in `presets::factory::tests`) sets every parameter to each end of its range on the default patch, for all 16 exciters on Pipe, Plate and Taut Cable, and compares velocity 127 with 64. After the fix, nothing goes silent at full velocity. It flagged:
- **Env Amount 0 silences the friction exciters** (Bow, Stiff Point Scrape, Heavy Grinding, Corrugated Drag, Tension Rise). That matches the `mseg-envelope` spec (the MSEG output is multiplied by `env_amount`), but the tooltip says "Lower flattens it". Left for the maintainer.
- **Coil Proximity 0 silences Electromagnetic Hum.** This matches its help ("higher couples more strongly").
- **Quieter at 127 by more than 6 dB**, at range extremes only:

  | Patch | Setting | v64 | v127 |
  |---|---|---|---|
  | Bow on Plate | Size 0.05 | −42.8 dB | −55.8 dB |
  | Wire Brush on Pipe | Size 0.05 | −30.9 dB | −38.8 dB |
  | Wire Brush on Taut Cable | Wire Density 10 | −10.5 dB | −19.8 dB |
  | Metal Chain on Pipe | Roughness or Rattle 0 | −51.1 dB | −62.6 dB |
  | Metal Chain on Taut Cable | Size 10 | −13.1 dB | −19.7 dB |

  These are nonlinear contact regimes at the edges of the ranges and none goes silent; left as they are.

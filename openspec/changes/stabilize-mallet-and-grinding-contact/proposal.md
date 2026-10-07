# Proposal

## Why

HardMallet (exciter 4) and HeavyGrinding (exciter 10) are effectively silent: their median default note peaks at about 0.004–0.005 before the voice clamp, or −47 dBFS (`normalize-object-loudness/design.md`, "Drive trims").
- Just above a small drive (trim ≈ 0.0037 for HardMallet, ≈ 0.0043 for HeavyGrinding), their exciter↔resonator loop diverges on some objects, IndustrialCog and CoilSpring among them.
- So their drive trims sit at half that threshold, 1.58e-3 and 1.80e-3.
- Before normalization they were running away into the clamp.

The causes are in the two contact models, not in their level:
- **HardMallet:**
  - Its damping term `impact_damping·(v_mallet − v_resonator)` is added even when the mallet isn't touching. It acts as a free-running force fed back from the resonator's velocity.
  - Its "gravity" keeps pulling the mallet back into the surface, so contact never ends.
  - Both forces react to the resonator's previous sample, and above a small loop gain that delayed feedback oscillates instead of damping.
- **HeavyGrinding:**
  - Its Coulomb friction `P·μ·sign(grind_speed − v_resonator)` also reads last sample's velocity. On a high mode, whose velocity flips sign each sample, the force lands in phase with the motion and pumps energy in.
  - Its tearing noise grows with `|v_rel|`, so a growing resonator velocity raises the drive further.

## What Changes

- **HardMallet:**
  - The contact becomes dissipative Hertz (Hunt–Crossley): force only while in contact, with damping proportional to penetration.
  - It is solved together with the resonator's immediate response to this sample's force, so there's no one-sample lag in the loop.
  - The mallet rebounds and leaves after its strike, no longer pulled back by gravity, and stops once it has separated for good.
- **HeavyGrinding:**
  - The friction curve is smooth (no sign flip at zero slip). It is solved together with the resonator's immediate velocity response.
  - Tearing noise is scaled by the grinder's own sliding speed, bounded, rather than by the resonator's velocity.
- **The resonator** reports its instantaneous contact response (compliance), and a shared contact helper solves implicit contact. Other exciters can adopt both later; `resolve-exciter-tradeoffs` does.
- **Drive trims:** the two exciters' trims are re-tuned so their median default note lands near the other struck exciters (Drumstick ≈ 0.15, WireBrush ≈ 0.2), with the worst case at most 0.9.
- **Audible change:** HardMallet and HeavyGrinding become audible, and both sound different at 48 kHz: a clean bounce-off strike, and a smoother grind. That is intended.

## Capabilities

### New Capabilities
- `exciter-drive`: how loud the HardMallet and HeavyGrinding exciters play relative to the other struck exciters, and that their contact with the resonator stays stable at any drive. `resolve-exciter-tradeoffs` extends it to every exciter.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:**
  - `src/dsp/exciters/hard_mallet.rs` and `heavy_grinding.rs`;
  - a new contact helper in `src/dsp/utils/`;
  - `src/dsp/resonators/core.rs` (instantaneous compliance);
  - `src/voice/mod.rs` (passing the compliance to these two exciters; their drive trims; tests).
- **Tests:** the existing `default_notes_do_not_overdrive_the_voice`, `every_exciter_has_the_same_level_at_any_sample_rate` and `struck_attack_timing_does_not_depend_on_sample_rate`, and `tests/no_alloc.rs`, plus new level and stability tests.
- **Related:** `resolve-exciter-tradeoffs` builds on this change's compliance and contact helper. Its HardMallet and HeavyGrinding work becomes "already done": apply this change first.

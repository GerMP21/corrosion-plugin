# Proposal

## Why

While levelling the factory bank for full velocity (`reorganize-factory-presets`, 2026-11-10), three of the nine experimental Pneumatic Jet presets went silent: Chain and Taut Cable above velocity ~100 and ~50, Pipe at every velocity. The seeder had to re-draw them.

The cause is the jet's saturation, `v² − chaos·v³` of the relative air velocity `v`. It is meant to keep the air burst from exploding in level, but the cubic turns over at `v = 2/(3·chaos)` and reaches zero at `1/chaos`; past that the exciter clamps it to 0, so the jet makes no sound at all. `v` grows with Air Pressure and with velocity (`trigger` scales the pressure by `0.3 + 0.7·velocity`), so playing harder, or turning Air Pressure or Turbulence Chaos up, silences the note. At Turbulence Chaos 2 (its top), Air Pressure 1.1 (its default) is already silent on every object.

The maintainer asked for it to be fixed (2026-11-10).

## What Changes

- The saturation holds its peak above the knee `2/(3·chaos)` instead of falling to zero: the force grows with air pressure and velocity up to the knee and stays there, still scaled by the air pressure as before.
- Below the knee the curve is unchanged, so default notes at velocity 100 render bit-identical; at velocity 127 a few samples pass the knee on 6 of the 9 objects (hashes change, RMS within 0.01 dB).
- **Sound change** only where the old curve turned over: those notes now sound instead of fading or going silent. No migration: there is no setting to restore.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `exciter-drive`: a new requirement, harder playing never silences the Pneumatic Jet.

## Impact

- **Code:** `src/dsp/exciters/other_specialty.rs` (`PneumaticJet`: a `saturation_knee` computed in `set_parameters`, one `min` per sample).
- **Tests:** `harder_playing_never_silences_the_jet` (voice), `print_jet_default_notes` (ignored, before/after hashes).
- **Factory presets:** reseeded; the three jets go back to their first draw.

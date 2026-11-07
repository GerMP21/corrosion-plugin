# Design

## Context

- **Master drive:** `apply_drive(sample, drive)` in `src/lib.rs`:
  - scales the input by `1 + 4·drive`;
  - shapes it with an asymmetric soft-knee curve (positive ceiling 1.2, negative 1.1);
  - mixes `shaped·drive + sample·(1 − drive/2)`;
  - clamps the result to ±1.5.
- **Call sites:** the plugin calls it per sample with the smoothed Drive. The test renderer and `tests/drive_dynamics.rs` call it directly.
- **Presets:** 11 factory presets use Drive 1.0–1.6. The other 39 use 0.

## Goals / Non-Goals

**Goals:**
- The weights sum to 1, with no negative dry.
- More drive means more saturation.

**Non-Goals:**
- Changing the waveshaper curve or its gain law.
- Matching each preset's old timbre. They get the light level pass, not a re-voice: the presets are placeholders.

## Decisions

1. **`m = min(drive, 1)`; output `shaped·m + dry·(1 − m)`.**
   - The knob's first unit fades the drive in. Above it, the input gain keeps rising (5× at 1, 21× at 5), so the shaper does the work.
   - *Alternatives:*
     - A linear crossfade over 0–5: presets at 1.0–1.6 would become 20–32 % wet, a far bigger change.
     - Keeping the old curve below 1: that keeps the loudness jump, the dry weight summing to 1.5. The user chose the crossfade.
2. **Keep the ±1.5 output clamp.**
   - The shaped signal peaks at 1.2 (positive) and 1.1 (negative), so the crossfade never exceeds 1.2 for inputs up to ±1. The clamp only guards larger inputs, as before.
   - The output limiter still bounds the master.
3. **The preset level pass uses `preset_levels_table`'s existing rules.**
   - The 11 presets lose about 0.5 of dry signal under a much louder shaped signal, so their RMS should move by well under 1 dB. The pass catches anything that doesn't.

## Risks / Trade-offs

- **[Drive below 1 sounds thinner than today]** → Intended: today's extra level came from the over-unity mix. Drive settings below 1 are only reachable by automation or a user patch; no factory preset uses one.
- **[A held note's level jumps if Drive is automated across 1]** → None. The mix is continuous at 1: both sides give the shaped signal alone.

## Implementation findings

- `apply_drive` is split into `drive_shape` (gain and waveshaper) and the mix, so the tests compare against the shaped sample exactly.
- **The old mix never actually flipped the output's sign.** Its dry weight goes negative above drive 2, but the shaped term (up to 1.2 × drive) always outweighs `|x|·(drive/2 − 1)`. So the sign and harmonic-share tests pass on both mixes and stay as guards. The crossfade and no-dry tests fail on the old mix.
- **The 11 Drive presets drop 0.3–1.6 dB RMS** (`levels.md`): the extra dry signal the old mix added is gone. That's more than the design's "well under 1 dB" for three of them: Chain Whip Snap −1.60, Sheet Lightning −1.35 and Snapped Cable −1.04. Still, none hits the limiter and none falls under the 12 dB floor, so the level rules call for no trims. `tests/drive_dynamics.rs` passes unchanged.

# Proposal

## Why

The MSEG that shapes friction-family exciters applies velocity with `0.5 + (level − 0.5)·velocity_scale·vtl + level·(1 − vtl)`. That formula pins a floor under the envelope: at the default `velocity_to_level = 1`, a stage level of 0 outputs `0.5·(1 − velocity_scale)` (0.25 at velocity 0). At `velocity_to_level = 0` every sample gets a +0.5 offset. Friction exciters are therefore still driven during the whole release, and the envelope drops straight from that floor to 0 when Release ends, which clicks on every friction note (code review 2026-07-07, finding **C4**). All 50 factory presets use `velocity_to_level = 1`.

## What Changes

- Velocity scales the envelope level multiplicatively: `level · ((1 − vtl) + vtl · velocity_scale)`, so a zero stage level is silence at any velocity and any `velocity_to_level`.
- The Release → Idle hand-off is continuous: the last Release samples converge on the end level (0), so the envelope reaches Idle without a step.
- **Audible change:** with `velocity_to_level = 1` (every factory preset), full-velocity output equals the old output within f32 rounding (≤ 1.5e-8; the old formula is mathematically `level` there). With `velocity_to_level < 1`, full velocity also loses the old +0.5·(1 − vtl) offset. Below full velocity, sustained friction notes get quieter, fully decay in release instead of sitting on a floor, and lose the release-end click. With `velocity_to_level < 1` the +0.5 offset disappears.
- The C2 golden-output test (`envelopes::tests`) is re-captured, because its expected values encode the floor. Its loop-timing protections stay.

## Capabilities

### New Capabilities
- `mseg-envelope`: Level and velocity behaviour of the multi-stage friction envelope (shares the capability introduced by `fix-mseg-loop-recursion`; whichever change archives second adds its requirements to the same spec).

### Modified Capabilities
<!-- none in openspec/specs/ yet -->

## Impact

- Code: `src/dsp/envelopes/mod.rs` (`process_sample` level/velocity math; Release → Idle).
- Tests: new envelope tests for silence at zero level, velocity scaling, `velocity_to_level = 0`, and release continuity; the C2 golden hashes are re-captured with the reason recorded.
- Presets: no file changes. Friction presets at sub-maximal velocity sound quieter in sustain and release.
- Docs: `FABLE-CODE-REVIEW-2026-07-07.md` C4 status.

### Added after verification

- Release ramps from the level the envelope had when the gate closed, instead of starting at the sustain level. Releasing during Attack or Decay (including a running Decay → Sustain loop) no longer jumps. Releases from Sustain are unchanged.

# Proposal

## Why

`exciter_pressure`, `exciter_speed` and `exciter_roughness` are host-automatable, documented ("Overall intensity of excitation", "Velocity for friction exciters", "Surface texture"), and stored in every preset. The voice copies them into `VoiceControls`, and then no exciter reads them. Every exciter uses its own dedicated parameters instead, so these three do nothing (fable review 2026-07-07, finding **H6**). The user chose to make them work rather than remove them.

## What Changes

- **Pressure** becomes a global trim on the exciter force, ±12 dB over its range.
  - It is neutral at its default 0.5.
  - It applies live, so it can be automated on held notes. The drive governor still keeps the voice under full scale.
- **Speed** becomes a global multiplier, ×0.5 to ×2, on each exciter's motion-speed parameters: bow speed, scrape, grind, drag and pull speeds, and so on. It is neutral at 0.5 and applies at note-on, like the dedicated exciter parameters.
- **Roughness** becomes a global multiplier, 0 to about 3.3×, on each exciter's grit and randomness parameters: bow roughness, surface grit, slip stochasticity, turbulence, brush randomization, chain rattle, rain mass variance. It is neutral at its default 0.3 and applies at note-on.
- **Defaults change nothing.** At 0.5 / 0.5 / 0.3 every note renders bit-identically to today. All factory presets hold exactly those values, so no preset changes.
- **Editor knobs** for the three controls are added beside the exciter selector.

## Capabilities

### New Capabilities
- `exciter-controls`: the global Pressure, Speed and Roughness controls and how they act on every exciter.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:**
  - `src/voice/mod.rs`: the force scale, and scaling the dedicated parameters at note-on;
  - `src/params.rs`: docs and value formatting;
  - `src/gui/editor.rs`: three knobs.
- **Tests:** new voice tests. The existing level, overdrive and sample-rate tests must still pass at the defaults.
- **Depends on:** nothing. If H5's coverage test exists when this lands, these three parameters move off its host-only list.
- **Docs:** `docs/ARCHITECTURE.md` (exciter system), the parameter docs, and the review's H6 status.

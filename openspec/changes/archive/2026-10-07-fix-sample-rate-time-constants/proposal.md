# Proposal

## Why

Much of the voice path is written in per-sample steps that silently assume 48 kHz:
- **Exciters:** they integrate position with fixed steps (`position += velocity · 0.0005`) and decay their state per sample (`*= 0.98 … 0.999`). The bow ramps its speed with a per-sample `0.001`, and the stiff-point, tension-rise and drag exciters accumulate per sample.
- **Voice:** the high-pass boost (`0.8/0.2` per sample), the peak-hold decay (`0.999`) and the tail cut-off (`4 800` frames).
- **Interaction bus:** the position-wander phase.
- **Post chain:** the FEM body's per-sample decay and force.

At 96 kHz all of these run twice as fast, which changes:
- each exciter's contact time and force, and so its level and brightness;
- the voice's high-pass corner;
- how long tails last before deactivating;
- the level voice stealing compares;
- the body's ring and level.

`fix-resonator-sample-rate-gain` fixes the resonator itself. This change fixes the rest, so the instrument sounds the same at every sample rate.

## What Changes

- Every per-sample time constant becomes sample-rate aware, while keeping its exact value at 48 kHz:
  - **decay factors:** `c^(48k/fs)`;
  - **integration steps and rates:** `× 48k/fs`;
  - **one-pole smoothers:** `1 − (1−k)^(48k/fs)`;
  - **frame counts:** scaled by `fs/48k`.
- Output at 48 kHz stays bit-identical, so presets, golden fixtures and every 48 kHz test are unchanged.
- Voice-level checks at 44.1 and 96 kHz are added for every exciter, plus a body check.
- **Audible change:** at sample rates other than 48 kHz, exciters, tails, stealing and the body behave as they do at 48 kHz. At 96 kHz this mainly means attacks, contact times and tail cut-offs stop being twice as fast.

## Capabilities

### New Capabilities
- `sample-rate-independence`: The instrument's notes, tails and body response sound the same at every supported host sample rate.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:**
  - `src/dsp/exciters/*.rs`: all exciters with per-sample steps or decays;
  - `src/voice/mod.rs`: high-pass, `PEAK_DECAY`, `TAIL_DEACTIVATE_FRAMES`;
  - `src/dsp/interaction.rs`: position wander;
  - `src/dsp/post_processing/fem_body.rs`;
  - a small shared helper in `src/dsp/utils`.
- **Depends on:** `fix-resonator-sample-rate-gain`. Without it, the voice-level checks can't pass at non-48k rates.
- **Not covered:** stochastic phase generators (the `GOLDEN_STEP` noise in the wire brush, metal chain and others) produce white-ish noise per sample. That is acceptable at any rate and stays per sample.

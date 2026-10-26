# Proposal

## Why

The master **Drive** (0–5) mixes its shaped signal in at weight `drive` and the dry signal at `1 − drive/2`. Above drive 2 the dry weight goes negative, so the dry signal is mixed in phase-inverted. The two weights also add to more than 1 from about drive 0.67 up, so turning Drive up adds level as much as saturation. The output is then clamped at ±1.5 (fable review 2026-07-07, finding **H2**).

## What Changes

- **A crossfade over the first unit of the knob.**
  - The mix becomes `shaped·m + dry·(1 − m)`, with `m = min(drive, 1)`.
  - The weights always sum to 1, and the dry weight is never negative.
- **Above 1, only the input gain rises.** The waveshaper's input gain (`1 + 4·drive`) keeps rising up to 5, so the upper range is more grit, not more phase-smeared level.
- **Factory presets get a level pass** with the existing rules: trim any preset that hits the limiter, and raise any preset more than 12 dB below the median.
- **Audible change:**
  - Drive below 1 is drier than today (dry weight `1 − d` instead of `1 − d/2`).
  - The 11 factory presets that use Drive 1.0–1.6 lose their remaining 0.5 dry share and become fully driven.

## Capabilities

### New Capabilities
- `master-drive`: how the master Drive control maps to its dry/shaped balance and saturation.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:** `src/lib.rs`, in `apply_drive` and its constants.
- **Tests:** `tests/drive_dynamics.rs`, plus new unit tests.
- **Presets:** `PRESET_LEVELS` in `src/bin/seed_presets.rs` and the regenerated `presets/factory/*` (output trims only), with per-preset numbers in this change's `levels.md`.
- **Docs:** the README master-drive description, and the review's H2 status.

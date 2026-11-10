# Proposal

## Why

The maintainer reported that raising Rust or Damage turns the sound down until it vanishes, very early in each control's range, and that Heat does almost nothing. Measured on Pipe and Plate C4 (HandStrike, velocity 100):

- **Rust** is silent from about 3.5 (range 0–5). `corroded()` multiplies each mode's decay by `1 − rust·(0.30 + 0.45·w)`, written for Rust 0–1. The factor reaches 0 at Rust 1.33 for the top mode and 3.33 for the fundamental, is clamped to ε, and the mode lasts under 0.1 ms.
- **Damage** loses the fundamental from about 4 (range 0–10). The note jumps up nearly three octaves (lowest mode 1 746 Hz at Damage 6), and the sound is silent at 10. The primary mode's decay factor `1 − d·(0.10 + 0.18·low)` dies first on the low modes, and the companion's decay formula turns negative from about 3.3. Loudness isn't monotonic either: −41 dB at 4, −32 dB at 6.
- **Heat** (range 0–1) only lowers the pitch by up to 5 %. Its brightness loss is a uniform gain, which the note-on bank normalization cancels (−0.04 dB). The specification's thermal wander is missing.

The intended models (`docs/detailed-specs/transformation-algorithms.md`): Rust is a frequency-dependent damping increase that leaves the low thud; Damage splits modes into beating pairs plus rattle; Heat drops the pitch, warbles, and darkens.

## What Changes

- **Rust:** decay is divided by `1 + rust·(a + b·w²)` instead of multiplied by a factor that crosses zero, and the high modes lose gain the same way. High modes die first, the fundamental always rings, and the darkening is audible across the whole 0–5 range.
- **Damage:**
  - Detune and companion level saturate with Damage, so the fundamental stays within a semitone of the note.
  - The decay changes only mildly and never reaches zero.
  - Damage changes character (beating, splitting), not level.
- **Heat:**
  - The pitch drop is kept.
  - The brightness loss tilts the spectrum (high modes lose more than low), so normalization can't cancel it.
  - A slow thermal warble modulates the bank's pitch (up to about ±8 cents at Heat 1), through the existing dynamic-retune path.
- **Bounds:** at every setting of each control, every object keeps its fundamental, and its 2 s RMS stays within 6 dB of the control at 0.
- Sound changes for any patch using Rust, Damage or Heat. There are no users yet, so no compatibility handling is needed, and parameter meanings and ranges are unchanged.

## Capabilities

### New Capabilities
- `material-transforms`: how Rust, Damage and Heat reshape a note across their whole ranges without silencing it.

### Modified Capabilities
<!-- none: resonator-level's "Quality mode and damage do not change loudness" (Damage 0 vs 1 within 2 dB) must keep passing as it is -->

## Impact

- **Code:**
  - `src/dsp/profile.rs` (`corroded`, `damaged`, `heated`);
  - `src/dsp/resonators/core.rs` (heat warble as a pitch dynamic, its LFO state, `has_dynamics`);
  - the voice wiring of the heat amount if needed.
- **Tests:** the golden mode-bank fixture is regenerated for the intended sound change, on gnu and musl.
- **Docs:** `docs/ARCHITECTURE.md` (transformations table) and the README if it describes these controls.

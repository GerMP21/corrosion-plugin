# Proposal

## Why

A note's loudness depends on things that shouldn't set it. Measured with a unit impulse at C4 and 48 kHz:
- **Object:** IndustrialCog 3.4, Tank 2.6, Pipe 0.63 and Plate 0.067, a 50× (34 dB) spread.
- **Pitch:** +6 dB per octave down (Tank C2 10.5, C4 2.6, C6 0.66), because the voice outputs modal displacement, which grows as 1/f.
- **Quality and damage:** both add modes, and so add level.

Low, long-ringing modes (the Tank's 87 Hz cavity, Cog and Pipe) push the raw resonator output far past full scale: about 280 for a bowed Tank. The voice's hard `clamp(±1)` hides this as clipping. A bowed Tank stays clipped for about 18 s after release. That is the finding from the voice-stealing work.

## What Changes

- **Bank normalization.** At note-on each mode bank is scaled so its strike response has one target level, whatever the object, pitch, quality mode, damage or character settings. Relative mode gains stay as they are, so each object's timbre is unchanged.
- **No overdrive.** With default controls, every object stays under full scale before the voice's safety clamp for a struck, a bowed and a specialty exciter, from C2 to C6 at full velocity. The clamp stays as a safety net.
- **Factory presets get a light level pass.** They are placeholders, to be replaced by real ones later. One global makeup level keeps the default patch near its current loudness, and individual `output` trims are applied only where a preset clips the limiter or sits far below the rest. The C5 "within 0.5 dB of the pre-C5 level" guarantee is retired, since it pinned levels that included the clipping.
- **Audible change:**
  - every object and register lands near the same loudness;
  - Tank, Cog and Pipe get much quieter and stop clipping;
  - Plate gets louder;
  - low notes get quieter relative to high ones;
  - bowed Tank and Cog lose the hard-clipped grit they had by accident.

## Capabilities

### New Capabilities
<!-- none: `resonator-level` is introduced by `fix-resonator-sample-rate-gain`, which this change depends on -->

### Modified Capabilities
- `resonator-level`: adds requirements that loudness doesn't depend on pitch, object, quality mode or damage, and that default-control notes don't overdrive the voice.

## Impact

- **Code:**
  - `src/dsp/resonators/core.rs`: bank normalization at note-on, in the allocation-free rebuild and in the constructors;
  - the level tests in `src/voice/mod.rs` (voice pre-clamp checks);
  - `src/lib.rs`: the default-note test band and the C5 measurement harness, whose strict re-voice check is replaced by a looser preset-level check;
  - `src/bin/seed_presets.rs` and `presets/factory/*`: the light level pass.
- **Depends on:** `fix-resonator-sample-rate-gain`, which must land first. The normalization is calibrated at 48 kHz and relies on the level being independent of sample rate.
- **Docs:** README (loudness note), `docs/ARCHITECTURE.md` (resonator output scaling; `decay_seconds` is a 1/e time constant).

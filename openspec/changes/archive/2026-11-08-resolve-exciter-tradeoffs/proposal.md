# Proposal

## Why

`fix-sample-rate-time-constants` and `normalize-object-loudness` fixed the engine's level and timing, but they left trade-offs the user wants gone:

- **Continuous exciters are 20–25 dB quiet.** Bow, scrapes, jet and hum are trimmed for the single worst object and note, so they sit far below full scale everywhere else. The median preset fell 15 dB.
- **Two exciters are effectively silent.** HardMallet and HeavyGrinding sit at about −47 dBFS. Above a small drive, their contact loop runs away.
- **Some exciters only meet the sample-rate spec with widened tolerances or test workarounds:**
  - widened tolerances: HandStrike 3 dB, Drumstick 3 dB, WireBrush 2.5 dB, MetalChain 3 dB, TensionRise 3 dB, ParticleRain 8 dB, Bow 2 dB and PneumaticJet 3 dB;
  - Drumstick and HardMallet are timed on their first 20 ms only, because of chatter after the strike.
- **MetalChain is nearly silent** (about −80 dBFS). Its rattle filter never keeps its state.
- **Modes at or above 20 kHz are switched off, with a hard cutoff.** A mode retuned across the cutoff (sheet warp, pitch bend) switches on or off abruptly. The cutoff also hides the real faults rather than fixing them:
  - a discrete mode's gain blows up as `1 / sin ω` near Nyquist;
  - the exciters' contact coupling is explicit, so a stiff contact on a bright bank runs away.
- **16-voice CPU rose** from 12.6 % to 15.1 % of one core at Normal quality.

## What Changes

- **Exciter levels.**
  - Each exciter's drive level aims its typical note at one common level, instead of its worst case. Quiet exciters are raised too: HandStrike, MetalChain.
  - A per-voice drive governor reduces the exciter's force when a voice nears full scale, so no note overdrives and none is turned down for everyone.
- **Stable contact.** Exciters whose force depends on the resonator's position (mallets, sticks, hand, pipe, chain, particles, grinding, snap) solve the contact together with the resonator's immediate response. Raising the drive can no longer make them diverge, so HardMallet and HeavyGrinding become usable.
- **Strikes settle.** After its last bounce, a drumstick or hard mallet comes to rest instead of being pinned to the surface and chattering for as long as the note is held.
- **Clean noise.** One shared integer noise generator replaces the `sin(phase)·43758` hash noise, whose spectrum drifted with note length and sample rate.
- **Sub-sample events.** Short events (particle spawns, brush impulses, the hand's contact, snap and break spikes, chain impacts) land between samples at their true time. The Hit envelope's attack is sampled at sample centres. Their level then no longer depends on how the sample grid falls.
- **MetalChain rattle** becomes a real high-pass with kept state.
- **Top of the band.**
  - A mode's gain is the true oscillator's `b0/ω` instead of the discrete `b0/sin ω`.
  - Modes fade out smoothly between 18 and 20 kHz instead of being cut at 20 kHz.
- **Back to the spec tolerances.**
  - Every exciter meets 1.5 dB across 44.1, 48 and 96 kHz, and struck attacks meet 10 % on their whole-note peak, with no per-exciter exceptions or windows.
  - The 16-voice "silent" threshold returns to 0.01.
- **CPU.** The strike-position weights are recomputed only when the position actually moves, not every sample. 16-voice cost returns to at most the pre-normalization figures.
- **Presets.** The factory presets get another light level pass (the same rules as before) on the new levels.
- **Audible change:**
  - continuous exciters and HandStrike get much louder;
  - HardMallet, HeavyGrinding and MetalChain become audible;
  - drumsticks stop buzzing on held notes;
  - noise-driven exciters sound slightly different, being white rather than drifting;
  - the very top of very bright banks (above 18 kHz) is gentler.

## Capabilities

### New Capabilities
<!-- none: `exciter-drive` is introduced by `stabilize-mallet-and-grinding-contact` -->

### Modified Capabilities
- `exciter-drive`: extends its HardMallet/HeavyGrinding requirements to every exciter (common level, no overdrive, stable contact, strikes come to rest).
- `sample-rate-independence`: every exciter meets the level and attack-timing tolerances with no per-exciter exceptions.
- `resonator-level`: the no-overdrive requirement covers all 16 exciters. High modes keep a sample-rate-independent level, and fade, rather than cut, at the top of the audible band.

These capabilities are introduced by `fix-sample-rate-time-constants`, `fix-resonator-sample-rate-gain` and `normalize-object-loudness`, which are implemented but not yet archived. **Archive them (and `fix-voice-stealing-16-voices`) before this change**, so its deltas apply to the main specs.

## Impact

- **Code:**
  - every exciter in `src/dsp/exciters/` (noise, sub-sample events, contact solve, settle);
  - `src/dsp/utils/` (noise generator, contact helper);
  - `src/dsp/resonators/core.rs` (mode gain correction, high-band fade, immediate-response query, cached strike weights);
  - `src/dsp/interaction.rs`;
  - `src/voice/mod.rs` (drive governor, drive levels, Hit envelope sampling, tests);
  - `src/voice/manager.rs` (16-voice test);
  - `src/bin/seed_presets.rs` and `presets/factory/*`.
- **Sound:** every exciter's sound changes at 48 kHz. The golden mode-bank fixture is re-captured.
- **Docs:** `docs/ARCHITECTURE.md` (exciter system, resonator model, sample-rate rule); `README.md` (loudness).
- **Builds on `stabilize-mallet-and-grinding-contact`, which must be applied and archived first.** That change provides the resonator's contact prediction and the shared contact solver, and it already fixes HardMallet and HeavyGrinding.
- **Supersedes** the suggested MetalChain rattle-filter task.

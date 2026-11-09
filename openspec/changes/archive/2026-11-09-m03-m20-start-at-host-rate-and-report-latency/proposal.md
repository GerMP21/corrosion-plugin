# Proposal

## Why

Two host-integration gaps from the fable review (2026-07-07):

- **M20.** The plugin tells the host its latency (the Lookahead limiter's delay) only from inside `process()`, and only when the limiter mode changes. A session loaded with Lookahead already on reports its latency on the first audio callback, after most hosts have finished delay compensation, so the track plays late against the others.
- **M3.** A note-on builds the resonator's coefficients for 48 kHz (`set_live_resonator_controls(…, 48_000)`), and an idle voice keeps the rate it last rendered at (48 kHz from construction). At any other rate the first sample rebuilds every mode again, so each note-on at 44.1 or 96 kHz does the coefficient work twice. A pitch bend that arrives before a voice's first sample is computed at the wrong rate too.

## What Changes

- **Latency is reported when the plugin is initialized**, from the restored Limiter Mode, as well as whenever the mode changes while processing.
- **Voices learn the host sample rate before they play:** the rate given at initialization, and every rate change seen while processing, reaches every voice, idle ones included. A note-on builds its bank once, at that rate, and a bend before the first sample uses it.
- **Each note starts its exciter from rest** (added during implementation, by the maintainer's choice): HeavyGrinding, PneumaticJet, ElectromagneticHum and ParticleRain no longer carry noise-filter state, hum phase or in-flight particles from the previous note on the same voice.
- No change to the sound at 48 kHz on fresh voices. At other rates, the first samples of a note on a voice that has never played become identical to those of a voice that has.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `output-limiting`: adds that the Lookahead limiter's latency is known to the host before audio starts.
- `sample-rate-independence`: adds that a note's first samples don't depend on whether its voice has played before.

## Impact

- **Code:** `src/lib.rs` (`initialize`, per-block rate tracking), `src/voice/manager.rs` and `src/voice/mod.rs` (a rate setter for every voice), `src/dsp/resonators/core.rs` (note-on builds at the voice's rate instead of a fixed 48 kHz).
- **Real-time safety:** the rate setter runs on the audio thread when the rate changes; it must not allocate (`tests/no_alloc.rs`).
- **Docs:** `docs/ARCHITECTURE.md` (sample-rate rule, latency), and the review's M3 and M20 status.

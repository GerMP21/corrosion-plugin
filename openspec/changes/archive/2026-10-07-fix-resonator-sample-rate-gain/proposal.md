# Proposal

## Why

Every resonator mode's input gain is a fixed `gain`, independent of the sample rate. The difference equation stands in for a physical oscillator, whose force term should scale with `dt² = 1/fs²`. Because it doesn't, a force signal that lasts a fixed time makes the resonator ring with a level proportional to **fs²**.

Measured at C4 for Tank and Plate, with a 1 ms pulse and with a sustained sine drive alike:
- ×4.00 (+12 dB) at 96 kHz relative to 48 kHz;
- ×0.84 (−1.5 dB) at 44.1 kHz;
- by the same law, ×16 (+24 dB) at 192 kHz.

The voice clamp and output limiter then work much harder at high sample rates, so a session sounds different just because the host's rate changed. Every level test so far ran at 48 kHz only, so nothing caught it. The ×2 per doubling seen in exploration came from a single-sample impulse, which isn't a fixed-time force.

## What Changes

- Each mode's input gain is scaled by `(48 000 / fs)²`. A given force over time then produces the same resonator output at any sample rate, for both short strikes and sustained (bowed) excitation.
- Output at 48 kHz is bit-identical, so presets, golden fixtures and every 48 kHz test are unchanged.
- Level tests at 44.1, 96 and 192 kHz are added.
- **Audible change:** at 88.2 / 96 / 192 kHz the instrument becomes about 11 / 12 / 24 dB quieter than today, matching 48 kHz, and it clips far less there. At 44.1 kHz it becomes about 1.5 dB louder. Anything voiced on a high-rate session will sound much quieter.

## Capabilities

### New Capabilities
- `resonator-level`: How loud the modal resonator responds to excitation. This change covers independence from the host sample rate.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:** `ResonatorCoefficients::for_mode` in `src/dsp/resonators/core.rs` (all modes build their coefficients there), plus tests in that file.
- **Follow-up:** `normalize-object-loudness` builds on this formula.
- **Not covered:** per-sample constants elsewhere in the voice (the high-pass boost, the peak-hold decay, exciter smoothers) also depend on the sample rate. This change measures their effect at the voice level and records it; `fix-sample-rate-time-constants` fixes them.

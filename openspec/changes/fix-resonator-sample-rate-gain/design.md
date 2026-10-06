# Design

## Context

`ResonatorCoefficients::for_mode` (`src/dsp/resonators/core.rs`) builds each mode as `b0 = gain`, `a1 = −2r·cos ω`, `a2 = r²`, where `ω = 2πf/fs` and `r = exp(−1/(τ·fs))`. `SecondOrderMode::process` computes `y = b0·x − a1·y1 − a2·y2`, and the voice outputs the sum of the modes' displacements.

The mode stands in for an oscillator `y'' + (2/τ)·y' + Ω²·y = K·F(t)`. A central-difference discretization puts the force term in as `K·F·dt²`, so the input gain should scale with `1/fs²`. It is a constant instead.

Two things follow:
- A force pulse lasting T covers `T·fs` samples, and each adds roughly `b0 / sin ω` (with `sin ω ≈ 2πf/fs`) to the peak, so the peak scales with fs².
- A sustained force at resonance reaches `b0 / (2(1−r)·sin ω)`, with `(1−r) ≈ 1/(τ·fs)`, which also scales with fs².

Measured at C4, with a 1 ms pulse and with a sine drive alike: ×4.00 at 96 kHz and ×0.84 at 44.1 kHz relative to 48 kHz, for Tank and Plate. A single-sample impulse only gives ×2 per doubling, because its duration halves; real exciters produce force over time.

Every coefficient rebuild goes through `for_mode`: at note-on, when the sample rate changes, for live damping and brightness, and for dynamic pitch.

## Goals / Non-Goals

**Goals:**
- Sample-rate-independent resonator level for strikes and sustained force.
- Bit-identical output at 48 kHz.

**Non-Goals:**
- **Loudness differences across pitch and objects** (`normalize-object-loudness`).
- **Other sample-rate-dependent per-sample constants** in the voice, exciters and post chain: the voice high-pass `0.8/0.2`, `PEAK_DECAY = 0.999`, the bow's `0.001` speed smoother, and others. Task 2.1 measures their combined effect; `fix-sample-rate-time-constants` fixes them.
- **Changing what `decay_seconds` means.** It stays a 1/e time constant.

## Decisions

1. **`b0 = gain · (48 000 / fs)²`**, using a named reference-rate constant.
   - A pulse of duration T then peaks at `T·fs · gain·(48k/fs)² / (2πf/fs)` = `gain · 48k² · T / (2πf)`, which is independent of fs.
   - A sustained force reaches `gain·(48k/fs)² / (2·(1/(τ·fs))·(2πf/fs))` = `gain · 48k² · τ / (4πf)`, also independent of fs.
   - At fs = 48 000 the factor is exactly 1.0, so `b0 = gain · 1.0` is bit-identical.

   *Alternatives:*
   - A linear `48 000 / fs`. That only fixes the single-sample impulse, which isn't how any exciter drives the resonator, and still leaves +6 dB at 96 kHz.
   - Normalizing by `sin ω`, which also removes the pitch tilt. That is a sound change, and belongs to `normalize-object-loudness`.

2. **The scale goes in `for_mode`, not in the voice.** It is the one place every coefficient path shares, and the exciters' bidirectional coupling (they read resonator displacement and velocity) then sees the same physical scale at any rate.

3. **The tests measure at the resonator level, using the spec's scenarios.** Golden 48 kHz behaviour is pinned by the existing mode-bank fixture plus a new hash of a 48 kHz resonator render captured before the change.

## Risks / Trade-offs

- [Sessions run at 96 kHz get about 12 dB quieter (192 kHz about 24 dB)] → Intended; that is the bug. Note it in the change summary.
- [The bow and other exciters couple to resonator velocity, so their equilibrium could shift slightly with fs at non-48k rates] → Measure bowed and struck voice output at 44.1/96 kHz (task 2.1) and record the remaining differences. Fixes to exciter and voice constants are a follow-up.
- [Floating point: the factor must be exactly 1.0 at 48 kHz] → Compute `ratio = REFERENCE / fs as f32` (exactly 1.0 for 48 000) and use `ratio * ratio`. The golden render hash proves it.

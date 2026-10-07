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

3. **The tests measure at the resonator level, using the spec's scenarios.** Golden 48 kHz behaviour is pinned by the existing mode-bank fixture plus an exact check that `b0` equals the mode gain at 48 kHz. *(A render hash was tried first. It passed on musl but not on glibc, because libm differs between the two CI targets. The hash was identical with and without the change on each target, but a hash isn't portable, so the exact `b0` check replaced it.)*

## Implementation findings

- **Strikes match at every rate.** After the change, the 1 ms pulse peak is within 0.03 dB of 48 kHz at 44.1, 96 and 192 kHz for Tank, Pipe, Plate and Cog. Sustained drive is within 0.1 dB at 44.1 and 96 kHz.
- **Voice-level levels before and after** (`voice::tests::sample_rate_voice_level_measurements`, ignored; C4, 1 s held plus 1 s released; raw resonator peak in dB relative to 48 kHz):

  | note | 44.1 kHz before → after | 96 kHz before → after |
  |---|---|---|
  | Bow, Tank / Pipe / Plate | −1.8 / −1.6 / −1.5 → −0.3 / −0.1 / −0.1 | +12.2 / +12.1 / +12.1 → +0.1 / +0.0 / +0.1 |
  | HandStrike, Tank / Pipe / Plate | −0.7 → +0.7 (all three) | +6.0 / +6.0 / +6.2 → −6.0 / −6.0 / −5.9 |

  - **Bowed notes are fixed.** The bow produces a continuous force signal, so the resonator scaling alone makes its level rate-independent.
  - **Struck notes flip from +6 dB to −6 dB at 96 kHz.** HandStrike's per-sample constants (`hand_velocity *= 0.99`, `hand_position += v · 0.01`, `voice/mod.rs` and `hand_strike.rs`) halve its contact time at 96 kHz, so the force-time it delivers halves; before, it grew only as fs, not fs².
  - `fix-sample-rate-time-constants` fixes exactly these constants. Until it lands, struck notes at 96 kHz are 6 dB too quiet instead of 6 dB too loud, so the two changes should ship together.
- **f32 precision limits very high rates (pre-existing, out of scope).** Under sustained drive at 192 kHz, Tank comes out −1.44 dB, Pipe −0.31 dB, Plate −0.01 dB and Cog −0.12 dB. Simulating Tank's 87 Hz cavity mode alone gives −1.47 dB with f32 arithmetic and −0.00 dB with f64. So the gain scaling is right, and the loss comes from f32 rounding in the direct-form recursion (`a1 ≈ −2`, `a2 ≈ 1`) for low, long-ringing modes. The rounded coefficients also detune that mode by about 0.2 Hz (4 cents) at 192 kHz.
  - Fixing it means f64 mode state or a better-conditioned structure (coupled or state-variable form), which would change 48 kHz output and break this change's bit-identity requirement.
  - The sustained scenario is therefore limited to 44.1 to 96 kHz, and the precision fix is left for a change that re-captures golden output anyway (suggested: `normalize-object-loudness`).

## Risks / Trade-offs

- [Sessions run at 96 kHz get about 12 dB quieter (192 kHz about 24 dB)] → Intended; that is the bug. Note it in the change summary.
- [The bow and other exciters couple to resonator velocity, so their equilibrium could shift slightly with fs at non-48k rates] → Measure bowed and struck voice output at 44.1/96 kHz (task 2.1) and record the remaining differences. Fixes to exciter and voice constants are a follow-up.
- [Floating point: the factor must be exactly 1.0 at 48 kHz] → Compute `ratio = REFERENCE / fs as f32` (exactly 1.0 for 48 000) and use `ratio * ratio`. The golden render hash proves it.

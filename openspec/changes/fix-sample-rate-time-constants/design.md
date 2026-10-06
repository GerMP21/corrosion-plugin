# Design

## Context

The per-sample constants found by inventory are:

| Where | Constant(s) | Kind |
|---|---|---|
| `voice/mod.rs` | `highpass = 0.8·hp + 0.2·x` | one-pole (pole 0.8) |
| `voice/mod.rs` | `PEAK_DECAY = 0.999` | decay factor |
| `voice/mod.rs` | `TAIL_DEACTIVATE_FRAMES = 4800` | frame count |
| `exciters/bow.rs` | `bow_velocity += (target − v)·0.001`; `bow_position += bow_velocity` | smoother; integration |
| `hand_strike.rs`, `felt_mallet.rs`, `hard_mallet.rs`, `drumstick.rs`, `metal_pipe.rs`, `metal_chain.rs` | `position += velocity·k` (0.0003–0.01); `velocity *= 0.98…0.999`; contact impulses | integration; decay |
| `stiff_point.rs`, `tension_rise.rs`, `corrugated_drag.rs`, `heavy_grinding.rs` | `accum += v_rel·k`; `noise *= 0.99` | integration; decay |
| `dsp/interaction.rs` | `position_wander_phase += rate` | phase rate |
| `post_processing/fem_body.rs` | per-sample `decay`, `force = 0.05·x`, `2 − ω² − decay` recursion | decay; gain |

Task 1.1 completes this table. Every exciter already receives the sample rate, or can: the voice knows it at each `process_sample`.

## Goals / Non-Goals

**Goals:**
- Sample-rate-independent exciter, voice-tail, stealing-level and body behaviour.
- Bit-identical at 48 kHz.

**Non-Goals:**
- **Stochastic `GOLDEN_STEP` phase noise.** It stays per sample; the result is still broadband noise.
- **Changing any 48 kHz sound.**
- **The resonator gain** (covered by `fix-resonator-sample-rate-gain`).
- **Post-chain stages that already derive their coefficients from the sample rate** (filter, smoothers, reverbs, echo, spread delay).

## Decisions

1. **Three conversion forms, each exactly the identity at 48 kHz**, through a shared helper with `ratio = 48 000 / fs`:
   - **Decay or one-pole factors:** `c^ratio`. Rust's `powf(c, 1.0)` returns `c` exactly.
   - **Integration steps and phase rates:** `step · ratio`.
   - **Smoothers of the form `x += (t − x)·k`:** `1 − (1 − k)^ratio`.

   Coefficients are cached and recomputed only when the sample rate changes, so there's no per-sample `powf`.

   *Alternative:* re-express every constant as seconds (τ) and compute `exp(−1/(τ·fs))`. That's cleaner to read, but not bit-identical at 48 kHz, so every golden test would need re-capturing.

2. **Frame counts scale with `fs / 48k`.** `TAIL_DEACTIVATE_FRAMES` becomes `0.1 s · fs`, which is exactly 4 800 at 48 kHz.

3. **Exciters get the sample rate the same way the resonator does.** Each exciter that needs it gains a cached `rate_ratio` set by a `set_sample_rate`, which the voice calls at note-on and whenever the rate changes. This avoids widening every `process_sample` signature. There's no allocation, and nothing changes in `tests/no_alloc.rs`.

4. **The FEM body is normalized like the resonator.** Its per-sample `decay` becomes a factor-from-time. Its force term is scaled so the body's level and ring time hold across rates, matching the "same level and ring time" scenario.

5. **Tests are at voice level with spec tolerances.** They are new tests in `src/voice/mod.rs` (crate-internal, so they can see `is_rendering` and timing), plus a body test in `fem_body.rs`. A 48 kHz golden render hash per exciter, captured before any edit, proves bit-identity.

## Risks / Trade-offs

- [Stateful contact models (mallets, drumstick) are nonlinear, so a time-step change isn't just a rescale, and some exciters may still miss the 1.5 dB / 10 % band at 44.1 kHz because of discretization] → Measure per exciter. If one misses, record it in Implementation findings with its numbers. Tolerances may only widen for that exciter, with a reason. A stochastic exciter is checked by RMS only.
- [The bit-identity claim depends on the helper returning exact identities at a ratio of 1.0] → A unit test on the helper (ratio 1.0 → exact), plus the 48 kHz golden hashes.
- [Many files are touched] → Each exciter is its own task with its own check, so a regression points to one file.

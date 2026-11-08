# Proposal

## Why

When an object has per-sample pitch dynamics (TautCable, SheetMetal), **or any note is held with a non-zero pitch bend**, `apply_dynamics` calls `set_frequency` on every mode, every sample. Each call rebuilds the mode's coefficients from scratch, with a `sin`, `cos` and `exp` in f64. A held bend gives the same factor every sample, so all that work recomputes identical numbers. And bending makes every object pay the cost the two dynamic objects were meant to pay. At Render quality with 16 voices this is a large CPU cliff, and no benchmark covers it (fable review 2026-07-07, finding **H12** and the tooling note on `benches/performance.rs`).

## What Changes

- **The bank remembers the pitch factor its coefficients were built for,** and skips the rebuild when the new factor is the same. A held bend costs nothing after its first sample.
- **For moving factors (dynamics, or a bend in motion), the rebuild runs only when the factor has moved by more than 0.05 cents** since the last rebuild. Below that, the pitch difference is inaudible.
- **A retune keeps the decay pole radius it already has** (it doesn't depend on frequency), and recomputes only the frequency-dependent terms.
- **New benchmarks:** 16 voices with a held bend, and 16 TautCable voices with dynamics, at Normal and Render.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `realtime-safety`: adds a requirement that bent and dynamic notes don't cost much more CPU than plain notes (bent at most 1.1×, dynamics at most 1.6× their twins, as the median of 5 benchmark runs).

## Impact

- **Code:** `src/dsp/resonators/core.rs` (`apply_dynamics`, `set_pitch_bend_factor`, a cheaper retune on `SecondOrderMode`).
- **Benches:** `benches/performance.rs` gains the held-bend and dynamics cases.
- **Sound:** a held bend is bit-identical. Dynamic objects follow their pitch in steps of at most 0.05 cents. The sheet-sweep click test and the dynamics tests guard it.
- **Docs:** the review's H12 status and its tooling note.

# Proposal

## Why

The per-object generators produce modes that can never be heard (fable review 2026-07-07, finding **H11**):
- **Modes far above the audible band.** Pipe's stiff-string series, Plate's `(m² + n²)` lattice and SheetMetal's `n²` series reach far above Nyquist at normal notes, more so at small sizes, where pitch is multiplied by up to 20×.
- **Modes with zero or negative decay.** Pipe's `1.5 − 0.15·n` goes to zero or below from mode 10, and Plate's `0.4 − 0.02·(m + n)` from `m + n = 20`. `max(ε)` rescues them into modes that die within one sample.

`resolve-exciter-tradeoffs` already silences the first kind with a fade at the top of the band, so they no longer alias. But both kinds still cost a full mode update every sample, and the dead ones still feed a one-sample direct path into the output and count in the bank's level normalization.

## What Changes

- **At note-on, the bank drops every mode that can never sound:**
  - its frequency is above the band top even at the lowest pitch a note can reach (per-note tuning goes down to ×0.25, so above 4 × 20 kHz);
  - or its decay time is under 0.1 ms, a ring that is gone in about five samples at 48 kHz.
- **Bank level normalization runs on the kept modes.**
- **The golden mode-bank fixture is re-captured.**
- **Audible change:** the dropped modes' one-sample feedthrough disappears from the attack. Note levels stay within the existing loudness tests' tolerances.
- **CPU:** Pipe and Plate banks at High and Render quality shrink substantially (Pipe keeps 9 of its sustain-dependent modes). The benchmarks record it.

## Capabilities

### New Capabilities
- `modal-bank`: which modes a resonator bank contains: only ones that can sound.

### Modified Capabilities
<!-- none: the resonator-level requirements (loudness across objects and registers) are unchanged and must keep passing -->

## Impact

- **Code:** `src/dsp/resonators/core.rs`, with a prune step between the transforms and the bank fill.
- **Generators:** not changed. The prune is one rule for every object.
- **Tests:** a new prune test; re-captured `tests/fixtures/mode_bank_hashes.txt`. The loudness tests (`objects_are_comparably_loud`, `loudness_does_not_depend_on_pitch`, and so on) must pass unchanged.
- **Docs:** the resonator model section in `docs/ARCHITECTURE.md`, and the review's H11 status.

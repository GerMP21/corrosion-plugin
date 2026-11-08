# Proposal

## Why

The rest of the fable review's medium findings (2026-07-07), and its tooling notes, are dead code, misleading names and documentation that no longer matches the code. None changes the sound; together they mislead whoever reads the code next:

- **M10.** `InteractionState::exciter_force` is written every sample and never read; the "bidirectional" interaction bus is feed-forward.
- **M11.** Its doc is fixed, but the tail-deactivation branch still re-assigns `active = false` on a voice that is already inactive.
- **M14.** Quality mode changes a voice's modal density only at note-on, while the post chain switches at once. Nothing tells the user.
- **M15.** `Preset::into_params` rebuilds parameters with its own helpers instead of the plugin's constructors: UI Scale loses its "50%…150%" strings, and every parameter built with the float helper gets a six-decimal display the plugin's own parameters don't have. A parameter built from a preset displays differently from the plugin's own.
- **M17.** The damage rattle's `highpass_noise` is not high-passed (it's `noise · 0.5`), and the rattle is identical in both channels. The rest of M17 was fixed earlier (proper white noise, no step at the gate).
- **M18.** Docs say "0-16" and "17 exciters" where the range is 1–16, and the fallback for an invalid exciter (HandStrike, the default) is undocumented.
- **Tooling.** `dsp` re-exports offline-render specs (an inverted dependency used only by its own tests), and `docs/CODE-REVIEW.md` (2026-05-27) is stale.
- **Review bookkeeping.** M2 (the editor now shows an AD envelope for hit exciters) and M19 (equal sync bands, the division shown) are resolved but still listed as open.

## What Changes

- Remove `exciter_force` and its setter; describe the bus as feed-forward.
- Remove the redundant assignment.
- Document quality mode's two timings (README, `docs/ARCHITECTURE.md`, and the parameter's description).
- **`into_params` builds every parameter with the same constructor, range and formatting as the plugin's own parameters.**
- Rename `highpass_noise` to what it is and document that the rattle is shared by both channels; no sound change.
- Correct the exciter range in docs and comments; document the fallback and pin it with a test.
- Drop the `dsp` re-export of offline specs (its tests import from `offline`); move `docs/CODE-REVIEW.md` to `docs/archive/` with a pointer to the fable review.
- Mark M2, M10, M11, M14, M15, M17, M18 and M19 in the review.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `preset-loading`: adds that parameters built from a preset display like the plugin's own, and how an out-of-range exciter loads.

## Impact

- **Code:** `src/dsp/interaction.rs`, `src/dsp/resonators/core.rs` (the two setter calls), `src/voice/mod.rs` (redundant assignment, rattle naming), `src/presets/mod.rs` (`into_params`), `src/params.rs` (constructor helpers shared with presets, exciter docs), `src/lib.rs` (exciter comment), `src/dsp/mod.rs` and `src/dsp/tests.rs` (re-export).
- **Docs:** README, `docs/ARCHITECTURE.md`, `docs/CODE-REVIEW.md` → `docs/archive/`, `FABLE-CODE-REVIEW-2026-07-07.md`.
- **Sound:** none; proven by render hashes against a clean worktree.

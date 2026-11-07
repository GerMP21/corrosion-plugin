# Proposal

## Why

The post-chain drive's **Drive Amount** knob runs from 0 to 5, but the plugin clamps the value (plus the Violence macro's offset) to 0..1 before passing it on. The drive then divides by 5 to get its wet share. So 80 % of the knob does nothing, and the most the drive can ever be is about 20 % wet (fable review 2026-07-07, finding **H1**).

## What Changes

- **The full knob range reaches the drive.**
  - The host-side clamp of `drive_amount + Violence offset` widens from 0..1 to the parameter's own 0..5.
  - The wet share stays `amount / 5`, so it runs from 0 to fully wet across the knob.
- **The drive's dry/wet mix becomes a crossfade.**
  - Today it is `wet·w + dry·(1 − w/2)`, which sums to 1.5 at full wet. It becomes `wet·w + dry·(1 − w)`.
  - This is the same rule the master Drive gets (finding H2).
- **Audible change:**
  - Settings above 1 (today identical to 1) now get progressively wetter.
  - The three factory presets that use the drive (amounts 0.4–0.55) keep their wet share. Their dry share drops from 0.95 to 0.90, about −0.4 dB.

## Capabilities

### New Capabilities
- `post-chain-drive`: how the post-chain drive's amount control maps to its wet/dry balance.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:**
  - `src/lib.rs`: the clamp in `apply_post_chain_params`, and the copy of that clamp in the test renderer;
  - `src/dsp/post_processing/lorenz_drive.rs`: the mix.
- **Presets:** no file changes. The three drive presets move by about 0.4 dB; `preset_levels_are_sane` confirms they still pass.
- **Docs:** the parameter docs in `src/params.rs`, and the review's H1 status.

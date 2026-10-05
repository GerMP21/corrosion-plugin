# Proposal

## Why

The post chain mixes the voice engine down to mono, then filters and drives that mono signal. It passes it *only* through the FEM body bank (8 fixed resonances, no dry term), and rebuilds "stereo" as `0.3·dry + 0.7·body`. So the filter and drive are heard only through a narrow, ringing resonator, and 70 % of the stereo image collapses to mono. The BODY fader is dead: `chassis_volume.max(body / 5)` lets Chassis Volume (0.5 in every preset) win over BODY (0.5–1.4 → 0.1–0.28). Eco quality also skips the body, spread and the selected reverb/echo, so it changes the sound rather than just the CPU cost (code review 2026-07-07, finding **C5**).

## What Changes

- Filter and drive run per channel (dual-mono), so the voice engine's stereo spread survives the post chain.
- The filtered/driven signal always reaches the output directly. The body resonator runs **in parallel** on the mono sum, and its output is mixed into both channels at the **BODY** amount.
- **BODY** becomes the body wet amount (0–5 maps to 0–100 %). **Chassis Volume** only sets the body's size (mode tuning) and no longer scales its gain. The `max()` conflict is removed.
- **Eco** runs the same chain as the other quality modes. It still saves CPU through 1× clipper oversampling and half-size modal banks.
- Factory presets are re-voiced so BODY gives roughly today's ~70 % body colour, and the `body` parameter default moves so a fresh instance keeps its current character.
- **BREAKING (sound):** every preset and the default patch sound different. The filtered signal is now heard directly, the stereo image widens, and Eco now includes the body, spread and space.

## Capabilities

### New Capabilities
- `post-chain-routing`: How the post chain routes stereo audio through filter, drive, body, spread and space, what the BODY / Chassis Volume controls do, and what quality modes may and may not change.

### Modified Capabilities
<!-- none in openspec/specs/ yet -->

## Impact

- Code: `src/dsp/post_processing/post_chain.rs` (topology, per-channel filter/drive, Eco path), `fem_body.rs` (gain no longer tied to volume), `src/lib.rs` (body/chassis parameter routing), `src/params.rs` (`body` default and doc), `src/bin/seed_presets.rs` plus a regenerated `presets/factory/*` (re-voiced `body`).
- Tests: new routing tests (stereo preserved, BODY 0 has no body colouring, Chassis Volume doesn't change body level, Eco keeps body/space). The existing Eco-differs test in `post_chain.rs` is re-based.
- Docs: README (Quality / Post chain), `docs/ARCHITECTURE.md` (signal-flow diagram, Eco row), `FABLE-CODE-REVIEW-2026-07-07.md` C5 status.
- CPU: one extra filter and drive instance per plugin (not per voice). Negligible.

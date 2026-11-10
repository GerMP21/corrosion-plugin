# Proposal

## Why

While building `add-asymmetric-diode-stage`, the factory presets' levels showed a large, slow offset at the output of many patches (2026-11-10, findings in that change's design). It comes from the voices: each mode is a coupled-form resonator whose output has a non-zero gain at 0 Hz, so a one-signed force (a bow's pressure, a mallet's long contact) leaves a static deflection under the ringing. It follows the force envelope (a felt mallet's offset rises over ~60 ms and fades over a second; a bow's stays while bowing) and is gone after note-off.

Across the factory bank, 21 of 50 presets carry at least 1 % of their output power at DC and 9 at least 20 % (Bow up to 47 %, StiffPoint 33 %, FeltMallet 26 %, CorrugatedDrag 22 %, HardMallet 16 %). The FEM body, fed the mono sum, scales it with the rest of the signal. The offset is inaudible, but it:
- reaches the host's output as DC;
- uses up headroom in the master drive, the post-chain drive and the clipper (an offset of +0.41 makes the positive half clip much earlier than the negative one);
- makes any later high-pass (the diode stage's DC blocker) change a patch's level by 4–9 dB RMS and move its peaks.

The maintainer chose to remove it once, on the voice mix, before the rest of the output path (2026-11-10), and to finish the diode stage after this change.

## What Changes

- **A DC blocker on the voice mix:** a one-pole high-pass per channel at about 5 Hz, at the host rate, between the voice manager's stereo sum and the master drive. Everything after it (master drive, post chain, body, Space, clipper, limiter) sees a centred signal.
- The audible low end is kept: the lowest notes' fundamentals lose under 0.2 dB.
- It applies to every patch, in the plugin and in the offline renderer (they share `render_frame_unlimited`).
- **Sound change** for patches with one-signed forces: no DC at the output, and the drives and clipper bite symmetrically where the offset used to push one half into them. Patches without the offset change by a negligible amount (the high-pass's effect above ~20 Hz). No migration: there's no setting to restore.

## Capabilities

### New Capabilities
- `voice-mix-dc`: the voice mix reaching the output path carries no DC, keeps its audible low end, and does so at every sample rate.

### Modified Capabilities
- None.

## Impact

- **Order:** a new step before `add-asymmetric-diode-stage`, which is paused until this lands; its DC blocker and level checks are then re-measured.
- **Code:** `src/lib.rs` (the blocker's state on `Corrosion`, `render_frame_unlimited`, `reset`, the panic clear), a small DC-blocker type (shared with the diode stage's later if it fits).
- **Factory presets:** levels of the presets with an offset drop (it was part of their RMS); `preset_levels_are_sane` is re-run and the light level pass (`preset_levels_table`) applied if needed. They are placeholders.
- **Docs:** `docs/ARCHITECTURE.md` (signal chain, voice output), README (a line on the output having no DC).

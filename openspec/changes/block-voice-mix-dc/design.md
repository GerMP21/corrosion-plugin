# Design

## Context

- `Corrosion::render_frame_unlimited` takes the voices' stereo sum (`VoiceManager::process_sample_stereo`), applies the master drive (`apply_drive`, asymmetric), the post chain, and the output gain. The offline renderer goes through the same function (H13).
- The voices' offset is a static modal deflection under a one-signed force (`add-asymmetric-diode-stage` design, "Findings"). It is present with BODY 0 (21–35 % of the output's power in the affected presets) and scaled by the FEM body.
- Making each mode zero-gain at DC (a velocity-like output) was considered and rejected by the maintainer: it tilts every patch's spectrum and touches the per-sample mode loop.

## Goals / Non-Goals

**Goals:** no DC from the voices anywhere in the output path; the audible low end kept; rate-independent; negligible cost.

**Non-Goals:**
- Removing DC that later stages create (the master drive's and the diode stage's asymmetric curves). The diode stage keeps its own blocker for its curve's DC.
- Changing the modal model or the exciters' forces.

## Decisions

1. **Where:** on the voice mix, per channel, before the master drive. Before the drive so its asymmetric shaper sees a centred signal; on the mix rather than per voice so it costs two filters, not thirty-two.
2. **Filter:** a one-pole DC blocker, `y[n] = x[n] − x[n−1] + R·y[n−1]`, `R = exp(−2π·f_c / f_s)` from the cutoff, so it's the same filter at every rate. *Alternative:* a second-order high-pass. Steeper, but it rings and costs more; the offset's content is far below the notes (under ~3 Hz), so one pole is enough.
3. **Cutoff: 5 Hz** (time constant 32 ms). The lowest note's fundamental (MIDI 0 is 8.2 Hz but the instrument's useful range starts near 30 Hz) loses 0.12 dB at 30 Hz. A felt mallet's offset rises over ~60 ms, so a 32 ms blocker removes most of it; a lower cutoff would leave more of the strike's slow thump. The tests measure how much is left and the bound is set from that (see the spec); **stop and ask** if 5 Hz can't meet both the strike bound and the low-end bound.
4. **State:** two `f32` pairs on `Corrosion`, cleared by `reset()` and by the panic clear, the coefficient recomputed when the host rate changes (`set_sample_rate` path at `initialize`, and per block from the block's rate). No allocation.
5. **Test switch:** a test-only `mix_dc_blocker_enabled` on `Corrosion`, to compare against the old path (for the bounds and for proving patches without an offset barely change).

## Risks / Trade-offs

- **[Every patch's sound changes slightly]** → Patches without an offset change only by the high-pass's effect on the audible band (≤ 0.2 dB at 30 Hz, less above). Patches with one change where the offset pushed the drives and the clipper.
- **[Factory preset levels]** → Placeholders; re-run the level checks and apply the existing light pass if needed.
- **[A strike's slow thump is partly musical]** → It's sub-audible (the offset's content is under ~3 Hz); what's audible of it is the drive and clipper reacting to it, which is the problem.

## Migration Plan

None: there's no parameter. User presets and projects play with the blocker.

## Open Questions

- Whether the diode stage's blocker can then filter only its curve's DC, or keep filtering its whole input; re-measured when that change resumes.

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
3. **Cutoff: 5 Hz** (time constant 32 ms). *As built: 10 Hz, second order; see below.* The lowest note's fundamental (MIDI 0 is 8.2 Hz but the instrument's useful range starts near 30 Hz) loses 0.12 dB at 30 Hz. A felt mallet's offset rises over ~60 ms, so a 32 ms blocker removes most of it; a lower cutoff would leave more of the strike's slow thump. The tests measure how much is left and the bound is set from that (see the spec); **stop and ask** if 5 Hz can't meet both the strike bound and the low-end bound.
4. **State:** two `f32` pairs on `Corrosion`, cleared by `reset()` and by the panic clear, the coefficient recomputed when the host rate changes (`set_sample_rate` path at `initialize`, and per block from the block's rate). No allocation.
*As built (decisions 2–3), 2026-11-10:* the one-pole at 5 Hz missed both DC bounds. The held bow kept 1.36 % of its RMS as DC (bound 1 %), and the felt mallet's largest 20 ms mean dropped only 6.8 dB (bound 12). The mallet's offset rises over ~60 ms, so it has content up to ~10 Hz, not only under ~3 Hz as assumed. No setting met all three bounds (sweep below). The maintainer chose a **second-order Butterworth high-pass at 10 Hz** (bilinear, prewarped) and relaxed the held-bow bound to **< 2 % of RMS** (DC at least 34 dB under the note). Its coefficients and state are `f64`: the poles sit 0.001 inside the unit circle, where an `f32` direct form would amplify its rounding noise about 10⁶ times at DC. The type is `dsp::StereoDcBlocker` (`src/dsp/utils/dc_blocker.rs`), rated in `initialize` and `begin_block`, cleared by `reset()` and the panic clear.

5. **Test switch:** a test-only `mix_dc_blocker_enabled` on `Corrosion`, to compare against the old path (for the bounds and for proving patches without an offset barely change).

## Risks / Trade-offs

- **[Every patch's sound changes slightly]** → Patches without an offset change only by the high-pass's effect on the audible band (≤ 0.2 dB at 30 Hz, less above). Patches with one change where the offset pushed the drives and the clipper.
- **[Factory preset levels]** → Placeholders; re-run the level checks and apply the existing light pass if needed.
- **[A strike's slow thump is partly musical]** → It's sub-audible (the offset's content is under ~3 Hz); what's audible of it is the drive and clipper reacting to it, which is the problem.

## Migration Plan

None: there's no parameter. User presets and projects play with the blocker.

## Open Questions

- Whether the diode stage's blocker can then filter only its curve's DC, or keep filtering its whole input; re-measured when that change resumes.

## Measurements

### Before (task 1.1)

`voice_mix_dc_tests` (`src/lib.rs`), with the blocker a pass-through. Plugin output, BODY 0, Space off, no drive, Diode Asymmetry 0 (which bypasses the clipper's own 2 Hz blocker, so it can't hide the voices' offset); `print_voice_mix_dc`:

| Scenario | 44.1 kHz | 48 kHz | 96 kHz | Bound |
|---|---|---|---|---|
| Held bow (Plate, C3, Drone): mean over the second half, % of RMS | 98.3 % | 98.2 % | 98.3 % | < 1 % |
| Felt mallet strike (Plate, C3): largest 20 ms mean, first 400 ms | 0.1138 | 0.1138 | 0.1138 | ≥ 12 dB lower with the blocker |
| 30 Hz / 100 Hz sine through the blocker | +0.000 / +0.000 dB | +0.000 / +0.000 dB | +0.000 / +0.000 dB | within 0.2 dB |

The DC tests fail and the low-end tests pass, as expected. The hand strike on the Pipe trivially passes (both paths are the same).

*As built (test):* the held bow plays in **Drone**. In Tonal, with the default bow on the Plate, the note barely oscillates: its AC level is about 0.0005 (−67 dBFS) under a 0.03 offset, so "mean under 1 % of RMS" would measure the blocker's residue against near-silence. In Drone the bow sounds (AC 0.017) with an offset 4.5× larger, which is what the spec means by a held bow note.

### Cutoff sweep (task 2.1)

The bypassed output, filtered offline (the one-pole at 5 Hz reproduced the in-plugin values exactly: 1.36 %, 0.05173). 48 kHz, same scenarios as above. The 20 ms metric isn't the limit: the strike with only its DC removed (a 3-period moving average subtracted) reads 0.0040, 29 dB under the raw strike.

| Filter | Held bow, DC % of RMS | Strike, largest 20 ms mean | 30 Hz | 50 Hz |
|---|---|---|---|---|
| One-pole 2 Hz | 4.09 % | −3.3 dB | −0.018 dB | −0.006 dB |
| One-pole 5 Hz (designed) | 1.36 % | −6.8 dB | −0.116 dB | −0.040 dB |
| One-pole 8 Hz | 0.20 % | −9.6 dB | −0.294 dB | −0.105 dB |
| One-pole 10 Hz | 0.14 % | −11.1 dB | −0.452 dB | −0.165 dB |
| One-pole 12 Hz | 0.32 % | −12.4 dB | −0.638 dB | −0.236 dB |
| One-pole 20 Hz | 0.47 % | −16.4 dB | −1.586 dB | −0.633 dB |
| Butterworth 5 Hz | 2.49 % | −11.9 dB | −0.003 dB | −0.000 dB |
| Butterworth 8 Hz | 2.24 % | −15.2 dB | −0.022 dB | −0.003 dB |
| **Butterworth 10 Hz (chosen)** | **1.77 %** | **−16.6 dB** | **−0.053 dB** | **−0.007 dB** |
| Butterworth 15 Hz | 0.96 % | −20.6 dB | −0.263 dB | −0.035 dB |

The held bow's residue isn't monotonic in the cutoff: it's the signed mean of the offset's slow drift after filtering, and the second-order filters leave more of it than the one-poles.

### After (task 2.1)

`print_voice_mix_dc`, the plugin with the blocker:

| Scenario | 44.1 kHz | 48 kHz | 96 kHz | Bound |
|---|---|---|---|---|
| Held bow, DC % of RMS | 1.79 % | 1.77 % | 1.77 % | < 2 % |
| Strike, largest 20 ms mean | 0.01681 (−16.6 dB) | 0.01682 (−16.6 dB) | 0.01679 (−16.6 dB) | ≥ 12 dB lower |
| 30 Hz / 100 Hz sine | −0.053 / −0.000 dB | −0.053 / −0.000 dB | −0.053 / −0.000 dB | within 0.2 dB |
| Hand strike on the Pipe (defaults), RMS | | −0.0002 dB | | within 0.1 dB |

`tests/no_alloc.rs` passes in debug (9 tests). `StereoDcBlocker` has its own tests: a constant is removed, `reset` clears it, and the cutoff is −3 dB at 10 Hz at 44.1, 48, 96 and 192 kHz.

### Output DC across the factory bank (task 2.2)

`print_factory_output_dc` (release): each factory preset, C4 on and off at 1 s, 2 s at 48 kHz, worst channel. The presets are placeholders; the bank is used only as a sample of patches with one-signed forces.

| | Presets with ≥ 1 % of output power at DC | ≥ 20 % | Worst |
|---|---|---|---|
| Diode Asymmetry 0, blocker off | 21 of 50 | 9 | 46.7 % (Industrial Plate Bow) |
| Diode Asymmetry 0, blocker on | 1 | 0 | 1.21 % (Chain Bell Tower) |
| Stored Asymmetry 0.25, either | 0 | 0 | 0.001 % |

With the stored Asymmetry the diode stage's own 2 Hz blocker already removes the offset at the end of the chain, but only there: the drives and the clipper still saw it. Chain Bell Tower's remaining 1.2 % (and Spring Reverb Tap's 0.66 %) is DC made after the mix, by the asymmetric stages (a non-goal). The largest 20 ms mean isn't a useful DC figure for whole presets: loud low notes leave a large windowed mean of their own (0.58 for Cog Mesh Grind with 0.010 % DC), so it is recorded in the test's table but not summarised here.

### CPU (task 2.2)

`plugin_output_path_sixteen_voices` and its Render twin, this change against a clean worktree of `b930d23`, five alternating runs of both frozen binaries, per-run ratios: Normal 1.022, 0.957, 0.909, 1.027, 0.978 (median **0.978×**); Render 1.093, 1.002, 0.994, 0.900, 0.952 (median **0.994×**). Noise: two biquads per sample on the mix.

### Preset levels (task 3.1)

`preset_levels_are_sane` fails after the blocker, as expected: the offset was part of some presets' RMS. Cable Bow is −13.6 dB, Suspension Cable Pluck −15.1 dB and Workshop Plate Scrape −15.3 dB under the median (bound −12). The light level pass is **not** applied: the factory presets are placeholders that the maintainer will regenerate (2026-11-10), and the diode stage, which resumes next, moves levels again. The presets still load and pass the CI tests (`tests/preset_roundtrip.rs`).

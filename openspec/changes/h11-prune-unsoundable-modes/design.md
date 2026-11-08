# Design

## Context

- **Note-on:** `rebuild_in_place` runs the generator into `raw_specs`, the transforms into `transformed_specs`, then fills `self.modes` (preallocated to `MAX_MODES`) and normalizes the strike level.
- **Decay:** `ModalModeSpec` stores `decay_seconds` after a `max(f32::EPSILON)` rescue, so a dead mode has a decay of about 1e-7 s.
- **Pitch at most moves by `pitch_bend_factor ∈ [0.25, 4]`** (per-note tuning clamps at ±24 semitones) times a dynamic factor ≥ 1 (cable tension drop and sheet warp only raise pitch).
- **Band top:** `band_fade` silences modes at or above `min(20 kHz, 0.45·fs)`.
- **Normalization:** the strike level estimate sums `|w·fade·b0/ω|^1.3` over the modes. Dead modes contribute their `b0/ω`, even though they ring for one sample.

## Goals / Non-Goals

**Goals:**
- No mode that can never sound.
- Unchanged loudness.
- No sound change under any later bend.

**Non-Goals:**
- Changing the generators' formulas or adding modes to replace dropped ones.
- Pruning modes in 20–80 kHz that a 2-octave bend could bring into band.

## Decisions

1. **The frequency limit is `20 kHz / 0.25 = 80 kHz`, a constant independent of `fs`.**
   - A mode above it can't reach the band at any reachable pitch. Using 20 kHz (not `0.45·fs`) keeps the bank the same at every sample rate, so the golden fixture is rate-independent.
   - *Alternative:* prune at the current band top and re-add on bend. That needs a variable-size bank on the audio thread.
2. **The decay limit is 0.1 ms.**
   - At 48 kHz that's under 5 samples (a 60 dB fall in 0.7 ms), so the mode is a feedthrough tick, not a resonance. The rescued ε decays are about 1e-7 s.
   - *Alternative:* 1 ms. That would remove short but real clicky modes that some objects rely on for attack.
3. **Prune after the transforms, before the fill.**
   - Transforms change frequency and decay (Heat, Sludge, Damage). Pruning on the final values is what the spec describes.
   - It's an in-place `retain` on the preallocated `transformed_specs`, so it allocates nothing.
4. **Normalize on the kept modes.**
   - Dropping dead modes slightly lowers the estimate, so normalization raises the bank slightly.
   - That is the correct level for what actually rings. The loudness scenario bounds the change at 0.5 dB.

## Risks / Trade-offs

- **[The attack loses the dead modes' one-sample feedthrough]** → It's a sub-millisecond tick spread over the inaudible top of the band for the high modes. The level scenario bounds it, and the golden fixture is re-captured with the reason given.
- **[A future generator change could prune a mode someone relies on]** → The rule is physical (can't ring, or can't be heard), and the prune test runs across every object and quality.

## Measurements

### Unsoundable modes before the prune (task 1.1)

`print_unsoundable_mode_counts`: banks at size 1 over C2/C4/C6 × every quality × damage 0 and 1, and one C6 bank at the smallest size (0.05) at Render.

| Object | Over the size-1 sweep | Worst size-1 bank | C6, size 0.05, Render |
|---|---|---|---|
| Pipe | 27 of 270 | C2 Render damage 1: 6 of 24 (decays under 0.1 ms) | 7 of 12 |
| Plate | 0 of 360 | — | 14 of 16 |
| Tank | 0 of 360 | — | 4 of 16 |
| Chain | 0 of 450 | — | 0 of 20 |
| IBeam | 0 of 360 | — | 9 of 16 |
| TautCable | 0 of 360 | — | 9 of 16 |
| CoilSpring | 0 of 450 | — | 6 of 20 |
| SheetMetal | 36 of 360 | C6 Render damage 1: 16 of 32 (above 80 kHz) | 15 of 16 |
| IndustrialCog | 0 of 432 | — | 0 of 12 |

So at normal sizes only Pipe (its dead high-mode decays) and SheetMetal (its `n²` series) lose modes; the other objects only reach 80 kHz at small sizes. The proposal's expectation that Pipe and Plate banks "shrink substantially" at High and Render holds only at small sizes.

### Level (task 2.1)

`pruning_does_not_change_how_loud_notes_are` (HandStrike, velocity 100, every object at C2/C4/C6, Render, sizes 1 and 0.05): largest change +0.003 dB (Pipe C2, size 1, 9 of 12 modes kept). Notes pitched above the band at the smallest size are silent with the full bank and stay silent.

### Golden fixture (task 2.2)

3 of 108 cases change, all Pipe at Render losing modes (24 → 18, 24 → 18, 12 → 9). The other 105 are bit-identical, which shows the kept modes' brightness weights and the level normalization are unchanged when nothing is pruned. Matches on `x86_64-unknown-linux-gnu` and `x86_64-unknown-linux-musl`.

### Benchmarks (task 2.3)

Same session, same container. "Before" is a clean worktree of `048d138`; Criterion medians, two runs each.

| Benchmark | Before | After |
|---|---|---|
| `voice_manager_sixteen_voices` (Normal) | 10.21 ms, 9.91 ms | 10.22 ms, 9.67 ms |
| `voice_manager_sixteen_voices_render` (damaged Plate, size 1) | 21.68 ms, 19.98 ms | 20.42 ms, 19.20 ms |

Render is no slower (within noise). The benchmark's size-1 Plate loses no modes, so no saving is expected there.

### As built

- The prune compacts `transformed_specs` in place rather than calling `retain`, so it can record each kept mode's brightness weight from the full bank in a parallel preallocated `mode_weights` buffer. `set_live_resonator_controls` reads those weights; without them, dropping modes would shift the brightness tilt of every kept mode at any Brightness other than 0.5.
- The test-only switch is `ModalResonator::prune_enabled`.

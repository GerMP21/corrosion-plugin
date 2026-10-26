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

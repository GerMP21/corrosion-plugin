# Design

## Context

- **`FactoryReverb`:**
  - four combs in `[[f32; 2048]; 4]`, with base delays 1553/1657/1789/1913 samples scaled by `0.5 + 1.5·size` and clamped to 100–2000;
  - two allpasses at a fixed 200 and 300 samples in `[f32; 512]`;
  - `set_sample_rate` stores the rate but doesn't use it for delays;
  - `diffusion_y` is never read (M16).
- **`FactoryEcho`:** two `[f32; 16384]` buffers; delay `(0.02 + t·0.78)·fs` with modulation, clamped to 16 383.
- **`SpringReverb`:** one `[f32; 8192]` per channel; delay `0.08·fs·(0.5 + tank)`, clamped to 1000–8000.
- **`HrtfSpread`:** `[f32; 64]` per channel, read with `.min(63)`. Its ITD is dead code (M8, out of scope).
- **Real-time safety:** buffers must not be allocated on the audio thread. `PostProcessingChain::new()` runs at plugin construction or initialize, and `reset()` clears in place.

## Goals / Non-Goals

**Goals:**
- Time-defined delays.
- No silent clamps up to 192 kHz.
- Distinct combs at every size.
- Still allocation-free while processing.

**Non-Goals:**
- Re-voicing the reverbs (the feedback and damping laws stay).
- Echo interpolation (M7).
- The HRTF ITD (M8).

## Decisions

1. **Delays in seconds, converted with `fs`.**
   - The comb bases become `[1553, 1657, 1789, 1913] / 48 000` s, and the allpasses 200 and 300 / 48 000 s, so 48 kHz delays are exactly the old values before any clamp.
   - The size scaling (`0.5 + 1.5·size`) is unchanged.
2. **Buffers sized for 192 kHz at construction, as `Box<[f32]>` heap slices.**
   - **Comb:** 1913·2·4 = 15 304 samples, rounded up.
   - **Allpass:** 300·4.
   - **Echo:** `0.8·(1 + max modulation)·192 000` plus a margin.
   - **Spring:** `0.08·1.5·192 000`.
   - **HRTF:** 1 ms at 192 kHz, rounded to 256.
   - Each processor keeps a `capacity` and computes delays with `min(capacity − 1)` only as a guard. The spec scenarios prove the guard is never reached.
   - *Alternative:* resize in `set_sample_rate`. That allocates at a rate change; nih-plug's `initialize` is off the audio thread, but sizing once is simpler and can't regress.
3. **Echo stays integer-read.** Interpolation is M7. Delays are rounded to whole samples, which is within the spec's one-sample tolerance.
4. **The Factory presets get a level pass with `preset_levels_table`'s rules.** Un-collapsing the combs changes the tail's density and its 48 kHz decay (the combs that were clamped to 2000 get longer). The rules catch any level outliers; the presets are placeholders, so they don't get a re-voice.

## Risks / Trade-offs

- **[The five Factory presets sound different at 48 kHz]** → Intended: they were metallic from a clamp, not by design. The level pass keeps them within the bank's rules, and `levels.md` records it.
- **[The memory per post chain grows]** → About 1.5 MB, allocated once. Acceptable for a desktop plugin.
- **[Longer comb delays with unchanged feedback give a longer decay]** → That's the uncollapsed room, as designed. The rate scenarios fix it across rates, not its absolute length.

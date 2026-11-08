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
   - *As built:* each delay is kept as its 48 kHz sample count and scaled by `fs / 48 000` (`space::delay_samples`). That is the same delay in time, and the factor is exactly 1 at 48 kHz, so 48 kHz delays are the old values bit for bit.
2. **Buffers sized for 192 kHz at construction, as `Box<[f32]>` heap slices.**
   - **Comb:** 1913·2·4 = 15 304 samples, rounded up.
   - **Allpass:** 300·4.
   - **Echo:** `0.8·(1 + max modulation)·192 000` plus a margin.
   - **Spring:** `0.08·1.5·192 000`.
   - **HRTF:** 1 ms at 192 kHz, rounded to 256. *As built:* kept as two inline `[f32; 256]` arrays (2 KB) rather than a heap slice, since they are small.
   - Each processor keeps a `capacity` and computes delays with `min(capacity − 1)` only as a guard. The spec scenarios prove the guard is never reached.
   - *Alternative:* resize in `set_sample_rate`. That allocates at a rate change; nih-plug's `initialize` is off the audio thread, but sizing once is simpler and can't regress.
3. **Echo stays integer-read.** Interpolation is M7. Delays are rounded to whole samples, which is within the spec's one-sample tolerance.
4. **Synced divisions are folded onto the beat, not clamped** (added after verification). The echo's buffer stays at 0.8 s; a longer synced division is halved until it fits. *Alternative:* a buffer long enough for 2/1 at slow tempos (4 s at 120 BPM, 24 s at 20 BPM) would cost tens of MB per post chain at 192 kHz.
5. **The Factory presets get a level pass with `preset_levels_table`'s rules.** Un-collapsing the combs changes the tail's density and its 48 kHz decay (the combs that were clamped to 2000 get longer). The rules catch any level outliers; the presets are placeholders, so they don't get a re-voice.

## Risks / Trade-offs

- **[The five Factory presets sound different at 48 kHz]** → Intended: they were metallic from a clamp, not by design. The level pass keeps them within the bank's rules, and `levels.md` records it.
- **[The memory per post chain grows]** → About 1.5 MB, allocated once. Acceptable for a desktop plugin.
- **[Longer comb delays with unchanged feedback give a longer decay]** → That's the uncollapsed room, as designed. The rate scenarios fix it across rates, not its absolute length.

## Measurements

### Before (task 1.1, on the code before this change)

All five new tests failed:

| Test | Measured |
|---|---|
| Factory T30 at 44.1 / 48 / 96 / 192 kHz | 0.610 / 0.560 / 0.280 / 0.140 s (the room halves with each doubling of the rate) |
| Echo, Delay Time 1 (0.8 s) | first echo at 16 383 samples at every rate: 0.371 s at 44.1 kHz, 0.341 s at 48 kHz, 0.171 s at 96 kHz, 0.085 s at 192 kHz |
| Echo sweep at 96 kHz, Delay Time 0 → 1 | 0.020, 0.059, 0.098, 0.137 s, then stuck at 0.171 s from Delay Time 0.2 up |
| Spring, tank 1 | 0.120 s at 48 kHz, 0.042 s at 192 kHz (8 000-sample clamp) |
| Factory comb delays at 48 kHz | size 0: 776/828/894/956; 0.25: 1358/1449/1565/1673; 0.5: 1941/2000/2000/2000 (three combs on the clamp) |

### After (tasks 2.1–2.4)

| Test | Measured |
|---|---|
| Factory T30 at 44.1 / 48 / 96 / 192 kHz | 0.616 / 0.617 / 0.617 / 0.617 s (longer than before at 48 kHz: the three clamped combs are uncollapsed) |
| Echo, Delay Time 1 | first echo at 35 280 / 38 400 / 76 800 / 153 600 samples: 0.8000 s at every rate |
| Echo sweep at 96 kHz | 0.020 to 0.800 s in 21 rising steps of 0.039 s |
| Spring, tank 1 | 0.120 s at both 48 and 192 kHz |
| Factory comb delays at 48 kHz | size 0: 776/828/894/956; 0.25: 1358/1449/1565/1673; 0.5: 1941/2071/2236/2391; 0.75: 2523/2692/2907/3108; 1: 3106/3314/3578/3826 |
| HRTF full-width delay | 48 / 96 / 192 samples at 48 / 96 / 192 kHz (1 ms) |

The echo's delay is now rounded to the nearest sample instead of truncated: in f32, 0.8 s at 48 kHz is 38 399.998 samples. Hanging Chain Echo's 0.215 s is exactly 10 320 samples either way, and it has no modulation, so it is unchanged at 48 kHz.

Buffers, allocated once in `new()`: 4 combs × 15 312, 2 allpasses × 1 208, 2 springs × 23 048, 2 echo lines × 168 968 samples, plus HRTF's two inline 256-sample arrays: 447 696 samples, about 1.8 MB per post chain.

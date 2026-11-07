# Design

## Context

- **Plumbing exists:** `VoiceControls` already carries `exciter_pressure`, `exciter_speed` and `exciter_roughness` from the host parameters.
- **Note-on:** `Voice::note_on_with_controls` calls each exciter's `set_parameters` from its dedicated controls, then `trigger`.
- **Force scaling:** `Voice::force_scale()` multiplies the force (drive level × test drive × governor gain). Contact exciters see the same scale through `contact_prediction`, so their solve stays consistent.
- **Live updates:** `update_live_controls` pushes a few live controls into held voices once per buffer.
- **Safety net:** the drive governor (`resolve-exciter-tradeoffs`) bounds the pre-clamp signal, so a larger force can't overdrive.

## Goals / Non-Goals

**Goals:**
- Each control does something on every exciter it can.
- The defaults are bit-identical.
- Nothing overdrives.

**Non-Goals:**
- Replacing the dedicated parameters.
- Speed or Roughness on exciters with no motion or grit parameter (they're inert there, and documented as such).
- Live Speed or Roughness on held notes: dedicated parameters are note-on only today.

## Decisions

1. **Pressure: `2^((p − 0.5)·4)`, applied in `force_scale`.**
   - The factor is 0.25× at 0 (−12 dB), 1× at 0.5 and 4× at 1 (+12 dB). It is exactly 1.0 at 0.5 in f32, so the default stays bit-identical.
   - Because it's part of `force_scale`, contact exciters see it in their prediction, and the governor follows.
   - It's pushed live through `update_live_controls`, smoothed with a one-pole of about 10 ms (through `rate::smoother`) to avoid zipper noise.
2. **Speed: multiplier `2^((s − 0.5)·2)`, ×0.5 to ×2, on motion-speed parameters at note-on:**
   - bow speed;
   - `scrape_speed`, `grind_speed`, `drag_speed`, `pull_speed`;
   - ParticleRain flow.

   Spreads and durations (WireBrush spread, MetalChain drop spread) take the inverse, since faster motion means a shorter spread. Each scaled value is clamped to its parameter's range. The implementation's first task confirms the list by reading each exciter.
3. **Roughness: multiplier `r / 0.3`, from 0 to ×3.33, on grit and randomness parameters at note-on:**
   - bow roughness, `surface_grit`, `slip_stochasticity`, `turbulence_chaos`;
   - `amplitude_randomization`, `internal_rattle`;
   - ParticleRain mass variance.

   A linear factor keeps 0 meaning "no grit", which is musically useful, and 1.0 at 0.3 exactly. Values are clamped to each parameter's range.
4. **Apply the multipliers in one place.** A helper scales the dedicated controls in a copy of `VoiceControls` before the per-exciter `set_parameters` dispatch, so the 16 call sites don't change.
   - *Alternative:* scale inside each exciter. That's 16 edits, and each exciter would need to know about globals.

## Risks / Trade-offs

- **[Raising Pressure engages the governor more]** → That's its job. The no-overdrive scenario pins it at the maximum settings.
- **[Speed changes friction exciters' level, not just their character]** → Accepted. The governor bounds the peak, and the levels are tuned at the default.
- **[Bit-identity at the defaults relies on exact factors]** → `2^0 = 1` and `0.3/0.3 = 1` are exact in f32, and the default-unchanged scenario tests it.

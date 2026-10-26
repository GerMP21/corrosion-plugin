# Design

## Context

- **Output stage** in `Corrosion::process`:
  - post chain (its clipper clips at `analog_ceiling`);
  - then output gain;
  - then either the `LookaheadLimiter` (threshold = `analog_ceiling`, clamped to 0.1..1) or `apply_output_limiter` (fixed clamp at `LIMITER_THRESHOLD` = 0.9661).
- **Parameter:** Analog Ceiling ranges 0.5–1.0, default 0.9661.
- **Other callers of `apply_output_limiter`:** the test renderer, `render_presets`, and the preset level tests.

## Goals / Non-Goals

**Goals:** one ceiling, honoured by both modes, after gain.

**Non-Goals:**
- Changing the clipper or the lookahead limiter's dynamics.
- Output gain's taper (finding M5).

## Decisions

1. **Pass the ceiling into the hard clamp.**
   - `apply_output_limiter(sample, ceiling)` clamps to `±ceiling`.
   - The process loop passes the same per-buffer `limiter_threshold` the lookahead limiter already uses (`analog_ceiling.clamp(0.1, 1.0)`), so the two modes share one value by construction.
   - *Alternative:* a separate "hard ceiling" parameter. That's more surface for no gain: the comment already states the intent.
2. **Keep `LIMITER_THRESHOLD`** as the documented default ceiling. Offline tools and the preset level checks call `apply_output_limiter(x, LIMITER_THRESHOLD)`; their presets all store that value, so nothing they measure changes.

## Risks / Trade-offs

- **[Sessions that lowered Analog Ceiling and used Hard mode get quieter peaks]** → That's what the control says it does. Lookahead users already had this behaviour.
- **[Public API change]** → The function is `pub` only for the repository's own binaries and tests, and all callers are updated together.

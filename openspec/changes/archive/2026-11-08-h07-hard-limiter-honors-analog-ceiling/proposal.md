# Proposal

## Why

The two output limiter modes stop at different levels:
- **Lookahead** uses the **Analog Ceiling** parameter as its threshold. The code comments say both modes "target the same effective peak".
- **Hard**, the default, clamps at the fixed `LIMITER_THRESHOLD` (0.9661) and ignores the ceiling.

So with Analog Ceiling at 0.5, Lookahead stops at 0.5 and Hard at 0.9661. Output gain is applied after the clipper, which clips at the ceiling, so in Hard mode a gain above 1 can push the signal back over the ceiling (fable review 2026-07-07, finding **H7**).

## What Changes

- **Hard mode clamps at the Analog Ceiling,** the same threshold Lookahead uses. The ceiling becomes the guaranteed maximum output in both modes, after output gain.
- **`apply_output_limiter` takes the ceiling as an argument.** `LIMITER_THRESHOLD` stays as the ceiling's default and the reference that tests and tools use.
- **No preset changes.** All 50 factory presets set the ceiling to 0.9661, so Hard mode behaves exactly as today for them.

## Capabilities

### New Capabilities
- `output-limiting`: the plugin's output ceiling and how both limiter modes respect it.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:** `src/lib.rs` (the output stage, `apply_output_limiter`'s signature, the test renderer); `src/bin/render_presets.rs` (call site).
- **API:** `corrosion::apply_output_limiter` gains a ceiling argument. Its only callers are in this repository.
- **Docs:** the `analog_ceiling` parameter doc, and the review's H7 status.

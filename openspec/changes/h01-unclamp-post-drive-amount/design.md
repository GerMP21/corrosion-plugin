# Design

## Context

- **Signal path:** `apply_post_chain_params` (`src/lib.rs`) clamps `drive_amount + drive_amount_offset` to 0..1 and passes it to `PostProcessingChain::set_drive_params`. `LorenzDrive::process` then mixes `wet·(amount/5) + input·(1 − amount/10)`.
- **The same clamp is copied** in the test-only renderer (`render_frames` in `src/lib.rs`), so the two must change together.
- **Saturator:** `tube_saturate` is the identity below |x| = 0.5. The drive's other stages (chaos modulation, starvation gate) scale the signal before it, so "linear region" in the spec means small inputs with chaos and starvation at 0.

## Goals / Non-Goals

**Goals:**
- Every knob position reaches the drive.
- Mixing never changes the level of an unsaturated signal.

**Non-Goals:**
- Re-voicing the drive's character: the saturator, chaos and starvation are unchanged.
- The starvation gate's hard switch. That is finding M6, a separate change.

## Decisions

1. **Clamp to the parameter range, keep `amount / 5`.**
   - The wet share for amounts in 0..1 is unchanged, so existing settings keep their saturation balance. Only the dry weight moves (Decision 2).
   - *Alternative:* change the parameter to 0..1. That breaks automation, and saved sessions store 0..5 values.
2. **Crossfade mix, `dry = 1 − w`.**
   - The old `1 − w/2` dry weight made the drive louder as it got wetter (×1.5 at full wet).
   - The crossfade matches the master Drive's new rule (H2), so both drive stages behave alike.
   - *Alternative:* keep `1 − w/2`. Full wet would then add 3.5 dB and push the clipper.

## Risks / Trade-offs

- **[Presets at 0.4–0.55 lose 0.4 dB of dry]** → `preset_levels_are_sane` (no limiter hits, none more than 12 dB under the median) and `tests/preset_roundtrip.rs` must still pass. The review's preset level rules apply if they don't.
- **[The Violence macro now reaches further]** → Intended: its +0.4 offset was mostly clipped by the 0..1 clamp at high settings.

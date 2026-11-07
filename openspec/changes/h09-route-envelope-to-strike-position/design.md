# Design

## Context

- **Live controls:** the voice passes `strike_position`, `position_wander`, `position_envelope` and `fundamental_anchor` to `ModalResonator::set_interaction_params`, at note-on and per buffer through `update_live_controls`. That computes `wander_rate = 0.1 + position_envelope·4.9` and calls `InteractionState::set_position_wander(amount, rate, fs)`.
- **Per-sample position:** each sample, `interaction_bus.update()` advances the wander phase and computes the effective position. The strike weights are recomputed only when the position, lock or mode count changes (the cached key).
- **The envelope:** the voice's force envelope (`process_force_envelope`, 0..1) is known every sample in `process_exciter`, before the resonator runs.
- **Preset schema** v4 has `position_envelope` and no wander rate.

## Goals / Non-Goals

**Goals:**
- Position Envelope does what its name says.
- Wander's rate is its own control.
- Old presets keep their sound.

**Non-Goals:**
- A bipolar or negative envelope amount: the parameter stays 0..1.
- Routing other modulators to position.

## Decisions

1. **Scale 0.5, clamped.** At full envelope the position moves half the bar, from its base toward the far end. That's enough to sweep through the bank's nodal pattern without pinning every note at the end.
   - *Alternative:* a full ±1 sweep. It clamps at the end for most base positions, which wastes range.
2. **The voice feeds the envelope each sample.**
   - `InteractionState` gets an `envelope_offset` set by the voice before `prepare_sample`. The effective position adds it before the wander and the clamp.
   - It's a store per sample. Recomputing the weights happens only when the cached key changes, which is the existing caching rule.
3. **A new `position_wander_rate` parameter**, 0.1–5 Hz and default 0.1, with a skewed range so the low rates get more knob travel.
   - **Migration:** a v4 preset without the field gets `0.1 + position_envelope·4.9` as its rate, and Position Envelope 0. That reproduces its old sound exactly.
   - *Alternative:* keep the rate tied to Position Envelope and add an envelope amount elsewhere. That keeps the misleading name, and the user chose the documented behaviour.
4. **The preset schema version stays at 4.** The new field is optional, and the migration runs when it's absent.
   - *Alternative:* bump to v5. That's more ceremony for one optional field. Revisit if more fields land in the same release.

## Risks / Trade-offs

- **[Sessions that automated Position Envelope as a wander rate change sound]** → Marked BREAKING in the proposal. Factory presets are unaffected (all 0), and presets are migrated.
- **[A moving position on struck notes re-weights the modes during the ring]** → That is the intent. At 0 nothing changes (the zero-is-neutral scenario).

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

## Measurements (task 2.2)

Linux x86_64 container (4 vCPU), `cargo bench --no-default-features --bench performance -- voice_manager_sixteen_voices`, Criterion median. "Before" is a clean worktree of the commit before this change (`d24ea80`), run twice; "after" is this change.

| Benchmark | Before | After |
|---|---|---|
| `voice_manager_sixteen_voices` (Normal) | 10.78 ms, 10.91 ms | 10.04 ms, 9.46 ms |
| `voice_manager_sixteen_voices_render` | 19.07 ms, 20.63 ms | 20.61 ms, 21.85 ms |

Both are within run-to-run noise (about ±5 % here). A first version called `set_envelope_offset` on every sample even at Position Envelope 0; it measured 24.2–25.7 ms on the Render benchmark, so the voice now only touches the offset while Position Envelope is above 0 (turning it to 0 clears the offset once).

**Bit-identity at Position Envelope 0:** Pipe C4, velocity 100, strike position 0.3, one second held and one released, stereo, for all 16 exciters at Wander 0 and at Wander 0.3 (0.1 Hz). All 32 render hashes match the clean worktree's.

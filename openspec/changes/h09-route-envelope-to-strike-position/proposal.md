# Proposal

## Why

`position_envelope` is documented as "How much envelope affects strike position", but no envelope reaches the strike position. The parameter only sets the rate of the position-wander LFO (`0.1 + position_envelope·4.9` Hz), so with Wander at 0 it does nothing at all (fable review 2026-07-07, finding **H9**). The user chose to implement the documented behaviour.

## What Changes

- **Position Envelope moves the strike position with the note's force envelope.**
  - The effective strike position becomes `strike_position + position_envelope · envelope · 0.5`, clamped to 0..1.
  - A strike or a bow swell moves the contact point (and so the timbre) as the envelope rises and falls.
- **Wander gets its own Wander Rate parameter** (`position_wander_rate`, 0.1–5 Hz). Its default of 0.1 Hz is today's rate at Position Envelope 0.
- **Presets and sessions:**
  - **Factory presets:** all 50 hold Position Envelope 0 and Wander 0, so none changes.
  - **Older presets:** they have no `position_wander_rate`, so they load the default. Any older preset that used Position Envelope to set the wander rate is migrated to the equivalent Wander Rate.
- **BREAKING (host automation):** a session that automated Position Envelope to change the wander rate now moves the strike position instead.

## Capabilities

### New Capabilities
- `strike-position`: how the strike position is set at note-on and moved during a note by wander and by the force envelope.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:**
  - `src/params.rs`: the new parameter, and docs;
  - `src/presets/mod.rs`: the field and its migration;
  - `src/voice/mod.rs` and `manager.rs`: pass the envelope and the new rate;
  - `src/dsp/resonators/core.rs` and `src/dsp/interaction.rs`: the effective position;
  - `src/gui/editor.rs`: a Wander Rate knob.
- **Schema:** the preset gains a field. Older presets get the default, or the migrated rate.
- **CPU:** the strike-position weights are cached (`resolve-exciter-tradeoffs`) and recomputed only while the effective position moves. With Position Envelope above 0 that is every sample of an envelope's attack and release, at the same cost as Wander today.
- **Docs:** `docs/ARCHITECTURE.md` (interaction bus), README, and the review's H9 status and "Missing features" item 6.

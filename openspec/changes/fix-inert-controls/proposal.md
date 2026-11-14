# Proposal

## Why

Writing the guide's exciter pages (`write-user-guide-content` 2.3) needed each control's effect measured. One note at each control's minimum and maximum (`print_slot_effects`) shows a dozen exciter controls that change the note by under 0.3 dB, with no change in ring or spectrum, and six help texts that describe the opposite of what is heard. A guide can't describe controls that do nothing, and users who move them hear nothing. The maintainer chose to fix the engine before the exciter pages are written (2026-11-14).

## What Changes

- **Measure every control in its context.** A measurement (an ignored test, extending `print_slot_effects`) plays each sound-shaping control at its minimum and maximum on a patch where it applies (its exciter or object, the effect switched on, the drive engaged, a held or released note, two velocities for velocity controls), and reports level, ring length and spectral change. Every control below an audibility floor is listed with its cause.
- **Make each inert control act**, fixing the model where a control is computed but cancelled (for example Stiff Point Scrape slips every sample, so Chatter Pitch and Chatter Damping never shape its force; force-scaling controls that the drive level and governor normalise away). Where a control can't act without breaking the model, remove it with a preset and host migration, or document why it only acts in a narrower context.
- **Make each help text point the way the control sounds:** Mute Decay, Flesh Damping, Rattle Color, Stick Mass, Pipe Mass and Voltage Sag today describe the opposite or another effect. For each, fix the text or the behaviour, whichever matches the control's name and the model.
- **Guard it:** a CI test fails when a sound-shaping control stops changing its measured note, so a later change can't silently disconnect one.
- **Sound changes** for presets that set the fixed controls away from their defaults: the factory bank is reseeded and re-levelled, and the guide clips regenerated. Default settings render bit-identically where the fix allows; where not, the change says what moved.

## Capabilities

### New Capabilities
- `control-response`: every sound-shaping control audibly changes a note in the context where it applies, in the direction its help text says, and a test guards it.

### Modified Capabilities
<!-- Filled in once the measurement shows which model requirements change. -->

## Impact

- `src/dsp/exciters/*` (Stiff Point Scrape, Bow, Metal Chain, Hand Strike, the mallets, Drumstick, Heavy Grinding at least), possibly the resonator's position controls (`strike_position`, `position_envelope`, `fundamental_anchor`) and Coil Spring's dispersion, which also measured 0.00 on their clip patch.
- `src/gui/controls.rs` help texts and `web/guide/src/data/parameters.es.json`.
- Factory presets (reseed), `levels.rs`, guide clips, the golden render hashes if a default changes.
- Blocks `write-user-guide-content` batch 2.3 (exciter pages) and the control descriptions of 2.4 for any object control found inert.

# Proposal

## Why

Several parameters that change the sound have no control in the editor, so they can only be reached through host automation:
- resonator Brightness and the Thickness transform;
- tempo Sync;
- Play Mode and Limiter Mode;
- the four macros.

Two finished pieces are also unreachable. The Save button is drawn permanently disabled, so there are no user presets. And the randomizer module is complete but nothing calls it (fable review 2026-07-07, finding **H5**).

## What Changes

- **Missing controls.** The editor gets controls for:
  - Brightness (next to Damping);
  - Thickness (with Heat and Sludge);
  - Sync, showing "Off" or the division from H3;
  - Play Mode (Tonal/Kit/Drone) and Limiter Mode (Hard/Lookahead);
  - the four macros (Mass, Corrosion, Violence, Brightness) in a macro strip.
- **User presets.**
  - Save writes the current sound as a named preset to a per-user preset folder.
  - The preset list shows the factory bank, then the user presets.
  - Loading a user preset uses the same path as a factory one (H4).
  - Saving over an existing user preset asks for confirmation. Errors (folder not writable, invalid name) are shown in the editor.
- **Randomizer.**
  - A Randomize button and a mode selector (Safe, Object, Damage, Full) apply the existing randomizer's ranges and safety constraints to the current sound, as one undoable host gesture per parameter.
  - The randomizer's Body range is rescaled to the 0–5 BODY parameter introduced in C5.
- **The exciter Pressure, Speed and Roughness controls are not in this change.** They become meaningful in H6, which adds their knobs.

## Capabilities

### New Capabilities
- `editor-controls`: which parameters the editor must expose.
- `user-presets`: saving, listing and loading the user's own presets.
- `patch-randomizer`: randomizing the current sound from the editor within safe ranges.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:**
  - `src/gui/editor.rs`: the new controls, the macro strip, the save dialog, the user preset list and the randomize button;
  - `src/presets/mod.rs`: the user preset folder, listing and file-name sanitizing;
  - `src/randomizer/mod.rs`: the BODY range, and a seeded randomness source.
- **Dependencies:** none new. The preset folder comes from the platform's standard environment variables.
- **Depends on:** H4 (`h04-load-every-preset-field-in-gui`) for loading user presets with every field, and H3 for the Sync display. Apply those first.
- **Docs:** README (user presets, randomizer), `docs/ARCHITECTURE.md` (GUI section), and the review's H5 status.

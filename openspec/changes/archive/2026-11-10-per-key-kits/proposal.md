# Proposal

## Why

Kit play mode was meant to be a drum kit (PRD §9.2: "each MIDI note maps to a different object preset"). What exists only swaps the resonator in fixed 10-note slots, with one exciter and one set of parameters for the whole keyboard:
- the pitch still rises key by key within a slot;
- the General MIDI drum keys (kick 36, snare 38, closed hat 42) all land on Plate;
- the user has no way to see or change what a key plays.

The maintainer wants a real kit, like the AFX mode of the Bass Station II: every key holds its own sound, assigned visually on the editor's new keyboard strip.

## What Changes

- **BREAKING:** Kit mode is replaced. In Kit mode each key plays its own **pad**:
  - a stored sound with its own exciter, object and every voice-level parameter, at a fixed pitch (the key's pitch plus the pad's tune);
  - keys without a pad are silent;
  - the old 10-note slot mapping is removed.
  - Host automation of Play Mode → Kit now plays the new kit.
- **Pads are assigned in the editor.** In Kit mode a bar above the keyboard strip shows the selected key, **Assign** (stores the editor's current sound on that key), **Clear**, and **Tune**.
  - Selecting a key that has a pad loads the pad's sound into the editor, so it can be tweaked and assigned again.
  - Assigning keeps the pitch the sound was last auditioned at, by setting the tune.
- **The kit zone.** In Kit mode the strip greys out and locks the keys outside MIDI 36–59, the General MIDI drum zone. Note names: the maintainer's C1–B2, with middle C as C3; C2–B3 in the plugin's naming, with middle C as C4. A **Full keyboard** toggle unlocks every key.
- **Shared post chain.** Filter, drive, body, spread, Space, diode stage and limiter stay one chain for all voices, so they are shared by the whole kit.
- **The keyboard strip widens to MIDI 36–84** (C2–C6 in the plugin's naming) so the kit zone is on screen. In Kit mode the computer keys `A W S E D F T G Y H U J K` play MIDI 36–48, so the kit can be played without a mouse.
- **Kits are saved** in preset files (a new optional `kit` field) and in the host's project state.
- **Migration:** presets and projects saved with the old Kit mode and no kit load as Tonal, so they keep sounding.
- **Factory `drums/` folder:** 10 drum kits, designed by hand on the General MIDI layout.

Depends on `add-editor-keyboard` (the strip; to be archived first, as this change modifies its spec) and `reorganize-factory-presets` (the folder bank).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `kit-mode`: the slot mapping is removed; Kit mode plays per-key pads that are assigned in the editor and limited to the kit zone unless unlocked. Kits are saved in presets and project state, and old Kit state migrates to Tonal.
- `editor-keyboard`: the strip covers MIDI 36–84, and in Kit mode the computer keys play MIDI 36–48.
- `factory-preset-bank`: adds the `drums` folder with 10 kits.

## Impact

- `src/kit.rs` (new): the pad data, the lock-free pad table the audio thread reads at note-on, and its serialisation.
- `src/lib.rs`:
  - `handle_note_event` builds the voice from the key's pad in Kit mode;
  - silence for keys without a pad;
  - pad voices skip live global controls;
  - `filter_state` migration;
  - the kit persisted in the plugin state.
- `src/params.rs`: Kit's doc, a persisted kit field.
- `src/presets/mod.rs`: the optional `kit` field, `migrate_preset_json` (old Kit → Tonal), save/load of the kit with the preset.
- `src/gui/editor.rs`: the kit bar, the locked zone and the Full keyboard toggle, key selection, loading a pad into the editor, the 36–84 strip, kit-mode computer keys.
- `src/gui_notes.rs`: the key mapping depends on the mode.
- `presets/factory/drums/` and `presets::factory` (10 kits).
- Tests (pads, silence, live-control isolation, zone, persistence, migration, no_alloc with pad note-ons), README, `docs/ARCHITECTURE.md`, PRD §9.2 note.

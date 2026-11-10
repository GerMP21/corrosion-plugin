# Design

## Context

- **Kit mode today:** `handle_note_event` (`src/lib.rs`) swaps the object with `note_to_kit_object` and takes everything else from the live parameters. It builds `VoiceControls` (about 100 fields), resolves the macro bias and calls `note_on_addressed` with the profile, size, rust, damage, exciter and controls.
- **Live controls:** `global_controls_enabled` lets live controls (Brightness, Damping, Thickness, …) reach sounding voices.
- **The post chain** processes the voice mix once (filter, drive, body, spread, Space, diode, limiter).
- **The strip** (`add-editor-keyboard`):
  - covers MIDI 48–84;
  - sends editor notes through `gui_notes::EditorNotes`;
  - the computer keys map to 60–72.
- **Two state paths:** presets go through `migrate_preset_json`, and host state through `filter_state`. nih-plug persists `#[persist]` fields of `CorrosionParams` through `PersistentField`, as it does `editor_state`.
- **Note naming:** the plugin calls MIDI 60 "C4"; the maintainer's "C1–B2" is MIDI 36–59, the General MIDI drum zone, in the "middle C = C3" convention. This design uses MIDI numbers.

## Goals / Non-Goals

**Goals:**
- An AFX-style kit: every key can hold a complete voice-level sound at a fixed pitch, assigned and edited visually on the strip.
- Real-time safe pad playback and assignment.
- Kits saved in presets and projects, with old Kit state migrated.
- Ten factory drum kits.

**Non-Goals:**
- A post chain per pad (one chain is shared; per-pad chains would multiply the post-chain CPU).
- Choke groups (for example open hat choked by closed hat), per-pad output routing, or velocity layers. Possible follow-ups.
- Copy and paste of pads, or loading a pad from a preset file directly (the editor's current sound is the source).

## Decisions

### 1. What a pad holds
A `Pad` is a snapshot of everything `handle_note_event` reads for a voice:
- exciter and object;
- size, rust and damage, with the macro bias already applied at assignment;
- the full `VoiceControls`;
- the strike position and global exciter controls already inside it;
- `tune` (semitones, −48…+48) and `level` (dB, default 0).

It is plain `Copy` data, so there are no allocations in it. The post-chain parameters are not in the pad.
- *Alternative:* store a whole `Preset` per pad. Rejected: it carries post-chain values that can't apply per pad, and it would need converting at note-on.

### 2. Audio-side pad table: a seqlock per key
`KitTable`: 128 slots, allocated once in `Corrosion::default()`. Each slot is a sequence counter (`AtomicU32`) plus the `Pad` in an `UnsafeCell`, with a present flag.
- **Writes** come from the GUI or a state restore, serialised by a writer `Mutex` the audio thread never takes. A write bumps the sequence to odd, copies, and bumps it to even.
- **Reads** happen at note-on on the audio thread. A reader copies the pad and retries while the sequence is odd or changed. It is bounded: after 4 failed tries it plays nothing for that note, which in practice needs a write racing four times within microseconds.
- No allocation or lock on the audio side; a test checks it under `assert_no_alloc`.
- *Alternatives:* an `Arc` swap (dropping the old `Arc` on the audio thread would free memory); an SPSC queue of pad updates (works, but needs an audio-side copy of 128 pads and a resync path for state restore).

### 3. Persistence
A `#[persist = "kit"]` field on `CorrosionParams` holds the serialisable kit, `KitState` (a map of key → `Pad`) behind a `RwLock`, which nih-plug serialises with the plugin state.
- The GUI writes `KitState` and the `KitTable` together.
- After nih-plug restores state, the table is rebuilt from `KitState`. Where: in `Plugin::initialize`, or on a "kit dirty" flag checked at the start of `process()` that triggers a rebuild off the audio thread. **To verify during implementation:** when nih-plug calls `initialize` around a state restore. If `process()` would have to rebuild, a GUI-side or background-task rebuild is used instead, so the audio thread never deserialises.

Preset files get an optional `kit: [{ key, pad }]` field (schema stays "4": optional and additive). Loading a preset writes `KitState` and the table, and clears every pad when the field is absent.

### 4. Note-on in Kit mode
`handle_note_event`, Kit mode:
- read the key's pad from `KitTable`; none means return (silent);
- else call `note_on_addressed` with the pad's profile, size, rust, damage, exciter and controls, at the pitch of `note + tune` (through the existing per-note tuning: the voice's `note_tuning_semitones` is set to `tune`, so the pitch-bend handling stays one code path), with the velocity scaled by `level`.

The voice is marked `pad_voice = true`, and `global_controls_enabled` skips it, so live knob moves don't reach pad voices. Tonal and Drone are unchanged.

### 5. Editor
When Play Mode is Kit, a kit bar sits between the panels' scroll view and the strip: `KEY C2 (36) · Tune ±48 · [Assign] [Clear] · [Full keyboard]`.
- **Selection:** clicking a playable key selects it (and plays it).
- **Loading a pad:** selecting a key with a pad sets every voice-level parameter to the pad's values through the `ParamSetter`, in one host gesture per parameter as preset loading does.
- **Assign** builds a `Pad` from the live parameters. Its tune is `last_auditioned_note − key`; the last auditioned note is the last note the editor played on a key without a pad, or in Tonal mode.
- **Drawing:**
  - keys with pads get a tinted top band;
  - the selected key gets an outline;
  - keys outside 36–59 are drawn greyed and ignore the mouse and the computer keys unless Full keyboard is on.
- **Strip and computer keys:** the strip covers 36–84 in every mode. The computer-key base note is 36 in Kit mode and 60 otherwise (`gui_notes::key_note` takes the base).

The bar is about 24 points tall at 100 %. It takes that space from the panels' scroll view, so the window keeps its size when the mode changes (`BASE_EDITOR_HEIGHT` is unchanged).

### 6. Migration
- **Presets:** `migrate_preset_json` turns `play_mode: Kit` without `kit` into Tonal.
- **Host state:** `filter_state` does the same when the persisted `kit` field is absent.
- Host automation can't be migrated: **BREAKING**, as in the proposal.

### 7. Factory drum kits
The ten kits are built in `presets::factory` like the curated presets, on the General MIDI layout:
- 36 kick, 38 snare, 37 or 39 side stick or clap;
- 42 / 44 / 46 hats;
- 41, 43, 45, 47, 48 and 50 toms;
- 49 and 57 crash, 51 ride.

Each kit has its own character (for example "Foundry Kit": tank kick, sheet-metal snare, chain hats). The post chain is set per kit. They are levelled like the others, and each pad must meet the quality floor alone.

## Risks / Trade-offs

- **State-restore timing** (decision 3) is the one unverified piece of nih-plug behaviour. Stop and ask if the only option left would rebuild on the audio thread.
- **Kit preset size:** 24 pads × about 110 floats is a few kB of JSON per kit preset; fine.
- **Loading a pad moves every voice parameter** of the editor, which a host may record as automation if it is recording. Same as loading a preset; documented.
- **Shared post chain:** a kit's hats and kick go through the same filter and drive. Accepted (Non-Goals), and stated in the README.
- **Wider strip:** 49 keys make white keys about 22 points at 75 % (were about 28); check in REAPER.
- **Code size in `handle_note_event`:** the pad path shares the voice start with the live path, so the two can't drift.

## Measurements

(to be filled during implementation: state-restore behaviour found, pad note-on cost vs live note-on, no_alloc results, kit preset sizes, REAPER check)

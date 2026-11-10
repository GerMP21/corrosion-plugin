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

## As built

- **What a pad holds (decision 1).** The stored form is `KitPad { key, tune, level_db, values }`: the plain value of every parameter except the post chain, master, quality, sync and UI ones (`kit::NON_PAD_PARAMS`), keyed by parameter ID. `KitPad::build` turns it into the audio-side `Pad` (`NoteSetup` from `note_setup`, the same function the live path uses, plus tune and gain) when the table is loaded. Storing values rather than the `Copy` snapshot keeps the files readable and lets a pad load back into the editor. The macros' values are stored (their voice-side bias is in the pad) but are not loaded into the editor (`PAD_MACRO_PARAMS`), since they also move post-chain controls.
- **Pitch (decision 4).** The pad voice is started at note `key + tune` into the mode bank, not through the ±24 st per-note tuning: tune reaches ±48 st, and a pad then renders bit-identically to a live note at that pitch (`pads_keep_their_pitch`). The voice's note is set back to the key (`Voice::mark_pad`) so note-offs, chokes and expression still find it.
- **Level (decision 4).** `level_db` is a gain on the voice output after its clamp (`pad_gain`), not a velocity scale, so velocity keeps its timbre meaning. The shared post chain still compresses above about −20 dB of pad level (the diode knee), so a level step is linear only below that.
- **Quality.** A pad follows the live Quality setting at note-on (`mode_count_scale`), like every voice.
- **Seqlock (decision 2).** The slot holds `MaybeUninit<Option<Pad>>` and is copied with volatile reads, so a torn read is never interpreted as an enum before the sequence check rejects it.
- **Last auditioned note (decision 5).** Set by any editor note outside Kit mode, and by selecting a key in Kit mode (key + its pad's tune), so Assign on a selected pad keeps its tune.
- **Factory kits (decision 7).** Twelve pads per kit: 36 kick, 37 side stick, 38 snare, 39 clap, 41 / 45 / 48 toms, 42 / 44 / 46 hats, 49 crash, 51 ride (43, 47, 50 and 57 were left out to keep the kits even). Pads are tuned for their role: kicks −12, snares and claps +12, side sticks +24, hats +48 (the pedal hat +43), cymbals +36, toms 0 so they rise with their keys. At their own keys the hats rang at 300–1000 Hz: the engine's pitch follows the key ÷ Size. Kits default to BODY 0.4 (2.62 added low partials to every hat). A hard mallet's strike that high dies in 0.2 s whatever the damping, so those kits open the hat with a drumstick.
- **Pad levels.** Each pad is levelled alone by the seeder (`level_pad`: up to 5 steps, capped for headroom at velocity 127) and stored in `levels.rs` (`PAD_LEVELS`). The floor is the 2 s held-note RMS, and one-shot drums put little energy in 2 s, so levels are high: +0.1 to +37.5 dB for kicks, snares, claps and toms, up to +56.7 dB for hats and cymbals, which sit far above the normalized strike range. `PAD_LEVEL_RANGE_DB` is therefore −40…+60 dB (first set to ±40, which left four hard-mallet hats 6–11 dB under the floor).

## Measurements

- **State restore (decision 3, verified).** nih-plug calls `initialize()` again after loading state, under the plugin lock (CLAP wrapper `clap/wrapper.rs` 1805–1817, the VST3 wrapper the same). The table is rebuilt there from `KitState`; nothing deserialises on the audio thread. `project_round_trip_keeps_the_kit` covers it.
- **Note-on cost.** `print_pad_note_on_cost` (Felt Mallet on Tank, through `handle_note_event`, median of 7 interleaved runs of 2 000 note-ons): pad ÷ live = 0.974, 0.890, 0.950 over three runs (0.83–1.45 µs per note-on). A pad note-on costs no more than a live one; the difference is within this container's noise.
- **Real-time safety.** `pad_note_ons_are_alloc_free_while_pads_are_assigned` passes (and fails when an allocation is put on the pad path).
- **Tonal and Drone unchanged.** All 204 non-kit factory presets render bit-identically through the full output path before and after the change (one-off render-hash comparison against a clean worktree at `e70cbd1`).
- **Kit preset size.** About 50 kB per factory kit (12 pads × about 130 values, pretty-printed JSON), against 3.9 kB for a plain preset. The proposal guessed a few kB; the size is harmless for loading.
- **Drum character** (`drum_kits_have_drum_character`, all ten kits, at their stored levels): kick ≥ 70 % of its energy below 500 Hz and down 30 dB within 1.5 s; snare down 30 dB within 1.5 s with a centroid above the kick's; closed and open hats with centroids ≥ 2 kHz and ≤ 30 % below 500 Hz; closed hat down within 1.0 s, open hat longer; toms rising. The first draft (all pads at tune 0) failed in every kit: hat centroids of 70–1000 Hz, kicks and snares ringing past 2 s, toms that did not rise; `drum_pad_measurements_table` prints the per-pad values.
- **Pad level step.** `a_pads_level_scales_its_output`: −24 dB against −36 dB drops the output by more than 10 dB. Before the stereo voice path applied `pad_gain` it measured 0.00 dB.
- **REAPER check** (Linux, CLAP, editor at 75 %, 2026-11-10):
  - The browser lists Drums between Percussion and Experimental; loading Foundry Kit switches Play to Kit, shows the kit bar, keeps 36–59 playable and greys the rest, and marks exactly the 12 pads with bands.
  - Clicking C2 selects it (outline, KEY C2 (36), Tune −12 st), loads its sound (Felt Mallet on Plate, Size 2.0) and plays it (meter −4.1 dB peak). The computer key D plays the snare (38). A grey key neither plays nor selects; Full keyboard unlocks the whole strip.
  - Assign on the empty E2 stores the current sound at −16 st: the last auditioned note was the kick at 24, and an empty key plays nothing, so it doesn't count. The new pad then sounds at the pitch the sound was heard at, as the spec asks.
  - Saved as a user preset: 13 pads in the file with their tunes and levels. Saved in a project, REAPER closed and reopened: Kit mode and all 13 pads come back, and the restored E2 plays.
  - A project saved by the previous version (`e70cbd1`) with Play Mode Kit opens as Tonal, with its other values kept.

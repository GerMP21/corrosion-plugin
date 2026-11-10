# Tasks

## 0. Prerequisites

- [x] 0.1 `add-editor-keyboard` and `reorganize-factory-presets` are archived (this change modifies `editor-keyboard` and `factory-preset-bank`)

## 1. Tests first

- [x] 1.1 Pad playback: a pad renders bit-identically to a live note with the same sound at the pad's pitch; keys without a pad start no voice; tune +24 plays the pitch 24 semitones up; moving a live control doesn't change a pad voice; Tonal and Drone render unchanged (hash against a clean worktree)
- [x] 1.2 `KitTable` concurrency: reads never see a torn pad under a writer thread hammering a slot; a pad note-on and a GUI assign under `assert_no_alloc` on the audio side
- [x] 1.3 Persistence: preset round trip (pads on 36, 38 and 42 only), host-state round trip, a preset without `kit` clears the pads
- [x] 1.4 Migration: an old Kit preset and old Kit host state load as Tonal with every other value kept
- [x] 1.5 Editor (GUI lane): kit bar only in Kit mode; Assign with tune from the last auditioned note; Clear; selecting a pad loads its sound and re-assigning without changes leaves it as it was; keys outside 36–59 greyed and locked until Full keyboard; strip 36–84; computer keys play 36–48 in Kit mode with their letters there
- [x] 1.6 Remove the slot-mapping tests (`kit_note_to_object_covers_full_range`, `kit_notes_*`, `kit_mode_overrides_object_param`) with the slot mapping

## 2. Engine

- [x] 2.1 `src/kit.rs`: `Pad`, `KitTable` (seqlock slots, writer mutex), `KitState` (serde)
- [x] 2.2 `handle_note_event` Kit path: pad lookup, silence, pad pitch via per-note tuning, level, `pad_voice` excluded from live global controls; remove `note_to_kit_object`
- [x] 2.3 Persist `KitState` (`#[persist = "kit"]`), rebuild the table after a state restore (verify nih-plug's `initialize` timing; record it)
- [x] 2.4 Preset `kit` field (save, load, clear when absent); `migrate_preset_json` and `filter_state` turn old Kit into Tonal

## 3. Editor

- [x] 3.1 Strip covers 36–84; computer-key base 36 in Kit mode
- [x] 3.2 Kit bar (selected key, Tune, Assign, Clear, Full keyboard) taking its space from the scroll view
- [x] 3.3 Key selection, the zone lock and greying, pad markers, loading a pad into the parameters, Assign with tune from the last auditioned note

## 4. Factory kits

- [x] 4.1 Ten drum kits in `presets::factory` on the General MIDI layout, levelled, every pad meeting the quality floor; regenerate `presets/factory/drums/`

## 5. Docs and close-out

- [x] 5.1 Measurements in `design.md` (state-restore behaviour, pad vs live note-on cost, kit preset sizes)
- [x] 5.2 REAPER check at 75 %: build a kit by assigning, play it from MIDI and the computer keys, save and reload it as a preset and in a project, the zone lock and the toggle, an old Kit project migrating; screenshot
- [x] 5.3 README (Kit mode, the shared post chain, the BREAKING note), `docs/ARCHITECTURE.md`, PRD §9.2 note
- [x] 5.4 `cargo fmt --check`, clippy `-D warnings`, tests on gnu and musl, GUI lane, no_alloc, `cargo doc`; update CLAUDE.md "Current state"; commit and push to `dev`

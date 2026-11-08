# Design

## Context

- `NoteEvent::NoteOn/NoteOff/PolyPressure/PolyTuning/Choke` all carry `voice_id: Option<i32>`, `channel: u8` and `note: u8`. `handle_note_event` ignores `voice_id` and `channel`, handles `PolyPressure` by key, and drops `PolyTuning` and `Choke`.
- `VoiceManager::note_off(note)` releases the oldest held voice for that key. Channel pitch bend reaches every voice through `set_pitch_bend_semitones`, which ends in `ModalResonator::set_pitch_bend_factor` (H12's exact retune for a held factor).
- The plugin doesn't declare CLAP polyphonic modulation.

## Goals / Non-Goals

**Goals:** address notes the way CLAP and VST3 hosts do; per-note tuning; chokes.

**Non-Goals:**
- MIDI-channel MPE (per-channel pitch bend and pressure zones). Channel messages stay global, as today.
- Polyphonic modulation of parameters, and the `VoiceTerminated` event that only that feature needs.
- Per-note volume, pan, vibrato or brightness expressions.

## Decisions

1. **`Voice` stores `voice_id: Option<i32>` and `channel: u8`** from its note-on.
2. **Addressing rule, shared by note-off, choke, pressure and tuning:** if the event has a voice ID, match the voice with that ID; if none matches, or the event has no ID, match by key and channel (note-off keeps the oldest-first rule). This follows nih-plug's documented convention for CLAP note IDs.
3. **Per-voice tuning in semitones,** stored on the voice and added to the channel bend before `pitch_bend_semitones_to_factor` (clamped to ±24 semitones as today). A tuning change retunes only that voice's bank, through the existing exact path, so a held tuning is skipped like a held bend.
4. **Choke = a 2 ms linear fade, then the voice is idle.** *Alternative:* stop dead. A choke on a ringing metal object would click.
5. **Key-only hosts are unaffected:** with no IDs every rule falls back to today's behaviour; existing tests guard it.

## Risks / Trade-offs

- **[Hosts that send IDs on note-on but not on note-off]** → The fallback to key and channel releases the right voice in the usual case.
- **[Tuning far from the bank's base can push modes past the band]** → The ±24 semitone clamp and H11's pruning bounds (built for ±24) already cover it.

## Migration Plan

None: no parameter or saved state changes.

# Design

## Context

- `handle_note_event` routes `NoteEvent::MidiCC { cc: 1 }` to the mod wheel and drops every other CC. The plugin declares `MidiConfig::MidiCCs`, so CCs already arrive, with `value` normalized to 0–1 (127 → 1.0).
- `VoiceManager::note_off(note)` releases the oldest held voice for that note. `steal_index` tiers voices as released tails (0), held notes past the protection time (1) and younger held notes (2).
- `Voice` tracks `active` (key held) and `rendering` (still sounding).

## Goals / Non-Goals

**Goals:** pedal behaviour as on a piano-style synth, channel-mode messages that unstick notes, no allocation.

**Non-Goals:**
- Half-pedalling or sostenuto (CC66) and soft pedal (CC67).
- Per-channel state: the plugin treats all channels as one, as it does for pitch bend today (MPE is `m12-per-note-expression`).

## Decisions

1. **Pedal state lives in `VoiceManager`.** `sustain_down: bool`, and each voice gets `pedal_held: bool`. A note-off while the pedal is down sets `pedal_held` instead of releasing; the voice stays `active`, so it keeps its envelope's sustain. Pedal up releases every `pedal_held` voice. *Alternative:* keep a list of deferred note numbers. That needs a bounded buffer and duplicates state the voices already have.
2. **Threshold at 64.** `value ≥ 0.5` is down, the usual MIDI on/off split. Repeated "down" messages change nothing.
3. **Re-striking a sustained note releases the older voice** (a normal note-off on the pedal-held instance), so repeated notes under the pedal don't pile up 16 copies.
4. **Stealing order:** tails (0), pedal-held notes (1), held keys past the protection time (2), younger held keys (3). Inside a tier the existing rules apply (quietest, then oldest).
5. **All Notes Off** calls the note-off path for every held key, so the pedal still applies. This follows the MIDI 1.0 rule that All Notes Off is subject to the sustain pedal.
6. **All Sound Off is a panic fade in `Corrosion`:** the output gain ramps to 0 over 5 ms; when it reaches 0, voices and the post chain are reset in place (the existing allocation-free `reset` paths) and the gain returns to 1. The fade runs inside `render_frame_unlimited`, so `process()` and the offline renderer share it. *Alternative:* reset immediately. Cutting a ringing reverb mid-tail clicks.
7. **Reset All Controllers** lifts the pedal (releasing pedal-held notes) and resets mod wheel, pitch bend and channel pressure to rest, through the existing setters.
8. **`Plugin::reset`** clears `sustain_down` and every `pedal_held` flag (its voices are reset anyway).

## Risks / Trade-offs

- **[A pedal left down holds notes forever]** → That's how a sustain pedal works; All Notes Off doesn't override it by design (decision 5), but All Sound Off and Reset All Controllers do.
- **[The fade costs one multiply per sample]** → Only while a panic is in progress; otherwise the gain is exactly 1 and the branch is skipped (keep it out of line, per the hot-loop rules in `CLAUDE.md`).
- **[Drone mode notes already ring until release]** → The pedal just delays the release, the same as for other notes.

## Migration Plan

None: no parameters or saved state change.

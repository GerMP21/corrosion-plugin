# Proposal

## Why

The plugin ignores the voice ID and channel that hosts attach to notes (fable review 2026-07-07, finding **M12**). Note-offs and per-note pressure are matched by key number alone, so a host that plays two overlapping notes on the same key (CLAP note IDs, or a step sequencer with overlapping steps) can't tell the plugin which one to release. Per-note tuning (`PolyTuning`, sent by CLAP and VST3 hosts for note expressions and by MPE-capable editors) is dropped, although a code comment says it overrides the channel bend. Host chokes are ignored too. The FIFO note-off added later fixed the plain "same key twice" case, but not addressed notes.

## What Changes

- **Voices remember the voice ID and channel of the note that started them.**
- **Note-off, choke and per-note pressure address a voice by its ID** when the host gives one, and by key and channel otherwise (today's FIFO rule for key-only note-offs stays).
- **Per-note tuning:** a `PolyTuning` event retunes only the addressed voice, in semitones, on top of the channel pitch bend.
- **Choke:** a host choke stops the addressed voice (or every voice on that key and channel) within 2 ms, with a fade instead of a click.
- No change for hosts that send plain MIDI.

## Capabilities

### New Capabilities
- `note-expression`: how the plugin addresses individual notes from the host: voice IDs, per-note tuning, per-note pressure and chokes.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:** `src/lib.rs` (`handle_note_event`: `PolyTuning`, `Choke`, voice-ID routing), `src/voice/manager.rs` (lookup by voice ID or key and channel), `src/voice/mod.rs` (stored ID and channel, per-voice tuning combined with the channel bend, a short choke fade).
- **Real-time safety:** no allocation (`tests/no_alloc.rs` gains the new events).
- **Docs:** README (MIDI and note expressions), `docs/ARCHITECTURE.md` (voice lifecycle), the review's M12 status.

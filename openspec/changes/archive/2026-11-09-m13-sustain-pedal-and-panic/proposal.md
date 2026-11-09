# Proposal

## Why

The only MIDI controller the plugin acts on is CC1 (mod wheel). It ignores the sustain pedal (CC64), so a pianist can't hold a chord with the foot, and it ignores the channel-mode messages hosts and controllers send to stop stuck notes: All Sound Off (CC120), Reset All Controllers (CC121) and All Notes Off (CC123). For a playable instrument the missing pedal is a real gap (fable review 2026-07-07, finding **M13**).

## What Changes

- **Sustain pedal (CC64).** While the pedal is down (value ≥ 64), releasing a key keeps its note sounding as if held; lifting the pedal releases every note whose key is up. Re-striking a sustained note releases the older instance.
- **All Notes Off (CC123)** releases every held note, as if each key were lifted. Notes held by the pedal keep sounding until the pedal lifts.
- **All Sound Off (CC120)** silences everything within a few milliseconds, voices and Space tails included, with a short fade instead of a click.
- **Reset All Controllers (CC121)** lifts the pedal and returns mod wheel, pitch bend and channel pressure to rest.
- **Voice stealing** takes pedal-held notes before notes whose keys are still down.
- A host reset (transport stop) also lifts the pedal.

## Capabilities

### New Capabilities
- `midi-control`: how the plugin responds to MIDI controllers and channel-mode messages (sustain pedal, all notes off, all sound off, reset all controllers).

### Modified Capabilities
- `voice-allocation`: adds where pedal-held notes fall in the stealing order.

## Impact

- **Code:** `src/lib.rs` (`handle_note_event` CC routing; the panic fade in the output path), `src/voice/manager.rs` (pedal state, deferred releases, stealing tier), `src/voice/mod.rs` (a per-voice "held by pedal" flag).
- **Real-time safety:** all of it runs on the audio thread and must not allocate (`tests/no_alloc.rs` gains the pedal and panic paths).
- **Docs:** README (MIDI section), `docs/ARCHITECTURE.md` (voice lifecycle), the review's M13 status.

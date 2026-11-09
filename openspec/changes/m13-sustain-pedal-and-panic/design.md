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

## As built

- **Pedal:** `Voice::pedal_held` (cleared by note-on, note-off and reset) and `VoiceManager::sustain_down`; key-ups go through `VoiceManager::release_key`, which holds or releases. The stealing tiers are as in decision 4.
- **All Sound Off keeps the controllers:** the end of the fade calls a new `VoiceManager::silence()` instead of `reset()`. Silence resets voices and the frame counter but keeps the pedal, mod wheel, bend and pressure, since All Sound Off mutes sound and doesn't move controllers. `Plugin::reset` cancels a fade in progress.
- **The fade multiplies the output after output gain**, in `Corrosion::panic_frame` (`#[cold] #[inline(never)]`), reached only while `panic_step > 0`.
- **No-alloc check location:** the allocation scenario is a lib test (`controller_messages_are_alloc_free`), not `tests/no_alloc.rs`, because the CC routing (`handle_note_event`) is crate-private. It drives `process_block` into a preallocated buffer inside `assert_no_alloc`. A deliberate allocation inside its zone aborts the test process, so the checker is active there.

## Migration Plan

None: no parameters or saved state change.

## Measurements

### Tests on the code before the change (task 1.1)

With HandStrike (the default exciter) a key-up changes nothing audible, so the pedal and All Notes Off tests use Bow, a sustaining exciter. On the old code, 7 of the 11 tests failed: pedal chord, repeated note, both All Notes Off tests, the panic fade, playing after a panic and Reset All Controllers. Three passed because the old behaviour already matches them: a key held through pedal-up, a pedal pressed with nothing held, and stealing (released notes were tails, stolen first anyway). They guard the new code. The allocation test passed, as task 1.2 asks.

### After the change

- All 11 `midi_control_tests` pass, and the 19 voice-manager tests are unchanged. `tests/no_alloc.rs` (debug) passes 9 of 9.
- **Playing after a panic:** the first comparison differed from frame 3 703, about 1 100 samples into the note. Each note's noise seed comes from the voice manager's frame counter, which restarts at the end of the fade, mid-block. Comparing against a reset plugin that reaches the note at the same frame count gives bit-identical renders, so no state survives the panic.
- **No change without the new messages:** a render hash of `sheet_lightning` (16 notes, released at 20 000 samples, 1 s) is `d776c098061dfd55` on the base commit (`9507936`, clean worktree) and on this change.

### After verification

- **Repeated note:** `repeated_note_under_the_pedal_keeps_one_voice` now counts pedal-held voices for the note after every strike and every release (at most one each time), and still requires the last strike to be held at the end.
- **`Plugin::reset` lifts the pedal:** new test `plugin_reset_lifts_the_pedal`. It fails when `VoiceManager::reset` stops clearing `sustain_down` (checked by removing that line).
- **Cost of the panic branch:** a release render of `sheet_lightning` (16 notes, 2 s, median of 9) against the base commit (`9507936`, clean worktree), in three alternating rounds: after/before 0.995, 0.986, 1.011, with identical output hashes.

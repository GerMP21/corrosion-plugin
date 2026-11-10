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

## Measurements

### Tests first (task 1.1)

`note_expression_tests` (`src/lib.rs`) on the code before this change: six scenario tests fail, each for the expected reason, and the plain-MIDI reference passes.

| Test | Scenario | Before |
|---|---|---|
| `note_off_releases_the_addressed_voice` | Overlapping notes on one key | fails: the older voice (ID 1) is released |
| `plain_midi_note_off_releases_the_oldest` | Plain MIDI host | passes (reference) |
| `tuning_retunes_only_the_addressed_note` | One note of a chord bent | fails: E moved ×1.00000, expected ×1.05946 (`PolyTuning` dropped) |
| `tuning_adds_to_the_channel_bend` | Tuning plus channel bend | fails: ×1.12246 (bend only), expected ×1.18921 |
| `pressure_reaches_only_the_addressed_voice` | Pressure on one of two same-key notes | fails: ID 1 got 0.8 (key match, first voice) |
| `choke_fades_within_two_ms_and_frees_the_voice` | Choked ringing note | fails: still sounding at sample 193 (`Choke` dropped) |
| `choke_without_matching_id_chokes_the_key` | (fallback for chokes) | fails: key 60 still sounds |

*As built (test):* the voices are identified by start order and note rather than by stored ID, so the tests compile against the old code. The `tests/no_alloc.rs` test (`note_expression_is_alloc_free`) needs the new manager API, so it was written with it in task 3.2 rather than run red first.

### After (tasks 2.1–3.2)

All seven pass, and `note_expression_is_alloc_free` passes in debug (allocation checker active). The choke test passes with the linear fade of decision 4: on a ringing Pipe C3 the output is zero at sample 96 (2 ms), and no step after the choke is larger than the largest step in the 2 ms before it, for both a key choke and an ID choke.

*As built (API):* `VoiceManager::note_on_addressed(NoteAddress, …)` takes the address; `note_on_with_controls` and the key-only `note_off` / `set_poly_pressure` stay for existing callers and tests. `handle_note_event` routes every note event through the addressed versions, so a plain-MIDI note-off now matches key **and channel** (before: key on any channel). Single-channel hosts see no difference.

### Defaults unchanged

`print_48k_exciter_render_hashes` (one note per exciter, no voice IDs), this change against a clean worktree of the previous commit: identical for all 16 exciters on gnu.

### CPU

`voice_manager_sixteen_voices` group, this change against a clean worktree of the previous commit, five alternating runs of both frozen binaries, median of per-run ratios: plain 1.001×, render 1.010×, bent 1.021×, cable dynamics 1.025×, cable static 0.992× (per-run ratios 0.955–1.051): noise. The choke check is one well-predicted branch per voice sample; the fade itself is `#[cold] #[inline(never)]`.

### Verification follow-up (2026-11-10)

`/opsx:verify` found that disabling the ID lookup for tuning and chokes left all seven tests green: their events also carried the right key, so the key-and-channel fallback gave the same result. Three tests were added and run red against that mutation (ID lookup removed from `for_addressed_voices`):

| Test | Mutation result |
|---|---|
| `tuning_by_id_moves_one_of_two_same_key_notes` | fails: "ID 1: mode 0 moved ×1.05946, expected ×1.00000" |
| `choke_by_id_stops_one_of_two_same_key_notes` | fails: "both or neither voice was choked" |
| `reused_id_addresses_the_held_note_not_the_tail` | fails |

*As built (reused IDs):* the plugin sends no voice-terminated event, so a host may reuse an ID while the old note's tail sounds. `voice_with_id` used to take the first matching slot; it now prefers a held note over a tail, then the newest (`max_by_key((is_active, start_frame))`). `reused_id_addresses_the_held_note_not_the_tail` failed on the previous code (the tuning reached the tail) and passes now. All ten tests and `no_alloc` pass.

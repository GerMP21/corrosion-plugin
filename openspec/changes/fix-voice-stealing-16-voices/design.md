# Design

## Context

`VoiceManager` (`src/voice/manager.rs`) owns a fixed `[Voice; MAX_VOICES]` array, with `MAX_VOICES = 8`, built by listing `Voice::new()` eight times.

**Note-on today:**
1. Take the first voice that isn't rendering.
2. If every voice is rendering, steal the one with the lowest `peak_hold`, with the earlier `start_frame` losing ties.

**What `peak_hold` actually is:** the max of the voice's clamped output, decayed by 0.999 every sample (a time constant of about 21 ms). It works as a near-instantaneous level, not a long-term hold. `note_on` resets it to 0, so a voice is at its quietest right after it starts. Friction exciters (bow, scrape) also take tens of milliseconds to build up. That is how fast notes over a full pool end up stealing each other (see proposal.md, "Why").

**Voice state:**
- A voice is *active* while its note is held, and *rendering* while it is held or tailing.
- `note_off` clears `active`.
- `frame_counter` advances one per rendered sample and records each note's `start_frame`.
- The manager is never told the sample rate except as the argument to `process_sample(_stereo)`.

**Measured (release build, development machine, 48 kHz):**
- A `Voice` is 1,008 bytes inline; `VoiceManager` is 8,088 bytes.
- 8 voices cost 8.8 % of one core (Normal quality, Pipe), 15.9 % (Render, Cog) and 19.6 % (Render, Plate).

## Goals / Non-Goals

**Goals:**
- Stealing order: a free voice first, then a tailing voice, then an old enough held voice, and only then the oldest held voice.
- 16 voices with no other behaviour change: the same per-voice sound, no mix normalization, no allocation.

**Non-Goals:**
- **Fading out a stolen voice.** It is cut at the next note-on, as today. A fade needs a spare voice per steal or a crossfade buffer. With 16 voices and the new order, a held note is stolen far less often. A fade can follow as its own change.
- **Retriggering the same pitch** into the voice that already holds it. That changes how physical objects layer when struck again, so it is a separate sound-design decision.
- **Making the voice count a host parameter**, or any CPU-adaptive voice limit.
- **Changing `peak_hold` itself**, which also drives tail deactivation.

## Decisions

1. **Three-tier steal selection, all in one pass over the pool.** Each rendering voice gets a tier key:
   - 0 = tailing (not active);
   - 1 = held and at least `PROTECT_FRAMES` old;
   - 2 = held and younger than that.

   The stolen voice is the lowest tier. Within tiers 0 and 1, the quietest `peak_hold` goes first, then the earliest `start_frame`. Within tier 2, the earliest `start_frame` goes. Non-finite peaks still count as quietest (`comparable_peak_hold`). This is a single `min_by` with a composite key: no allocation, and O(16).

   *Alternatives:*
   - Always steal the oldest held note. That's simple, but it cuts a loud, ringing old note before a quiet, nearly decayed one, which is worse for struck metal.
   - A smoothed loudness estimate. That adds per-voice state and per-sample cost to fix a problem that age protection already fixes.

2. **Protection window of 100 ms, in samples at the current rate.**
   - The manager stores the sample rate it is last called with in `process_sample(_stereo)`, defaulting to 48 kHz.
   - `PROTECT_FRAMES = 0.1 · sample_rate`.
   - 100 ms covers the default friction attack (10 ms onset + 50 ms attack) with margin, and is short enough that a held chord of older notes stays eligible.

   *Alternative:* a fixed frame count, rejected because it halves the window at 96 kHz.

3. **Tier 0 before tier 1 regardless of level.** A released tail is musically less important than a held note, even if it is momentarily louder. This is the usual synth convention.

4. **`MAX_VOICES = 16`, with the pool built by `std::array::from_fn(|_| Voice::new())`.**
   - Everything that loops over `MAX_VOICES` (tests, the no-alloc suite, automation stress) scales automatically.
   - `from_fn` avoids a 16-line literal, and the array is about 16 KB inline, which is fine on any host's construction stack.
   - The voice sum is not normalized, so a single note keeps its level. The output limiter (already ≈ −0.3 dBFS) handles dense 16-voice peaks.

5. **Tests drive the manager through its public API, plus `voices_for_test`.** The stealing scenarios from the spec become unit tests in `manager.rs`.
   - They use `note_on_with_controls`, `process_sample` to advance time and set `peak_hold` naturally, and `note_off`.
   - They observe through `active_notes()` (added in C3) and `voices_for_test()`.
   - The "quiet young note" scenario needs a held note that is quieter than the rest: a quiet exciter at low velocity, measured, not assumed.

## Risks / Trade-offs

- [Worst-case CPU doubles (≈ 40 % of one core for 16 Render-quality Plate voices on the development machine)] → Only when 16 voices are actually sounding, since idle voices are skipped. Eco and Normal are lighter. Record 16-voice benchmark numbers (Normal and Render) in this change, and mention the cost in the README polyphony line.
- [Dense passages hit the limiter more] → No normalization, by design (Decision 4). The factory presets are re-verified with one note each by the existing C5 harness, which polyphony doesn't change. Chord-heavy playing just reaches the limiter sooner, the same trade any polyphonic synth makes.
- [A 100 ms window could, in theory, steal a louder old note while a quieter young one keeps sounding] → That is intended: the young note is the one the player just played.
- [Tests depending on exact `peak_hold` values could be brittle] → Assert the ordering the spec defines (tier and age), and measure levels in the test rather than hard-coding them.

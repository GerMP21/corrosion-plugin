# Proposal

## Why

The 6-stage MSEG used by friction-family exciters recurses into `process_sample()` on every stage transition. With `loop_mode = Forward` and `loop_start_stage = 4` (Sustain) — or 5, which is clamped to 4 — the Sustain branch jumps to Sustain and re-enters itself forever, overflowing the audio-thread stack and crashing the host (code review 2026-07-07, finding **C2**). Both parameters are host-automatable, so any user or automation lane can trigger it.

## What Changes

- Replace the recursive stage-transition pattern with a bounded, non-recursive transition step so that no parameter combination can loop without consuming time.
- Define loop semantics for every `loop_start_stage` / `loop_end_stage` pair in the host range 0–5, including degenerate ones (start ≥ end, start = Sustain), so they hold rather than spin.
- Honor `loop_end_stage` (currently ignored; the loop always ends at Sustain). The default and Drone-forced loop (Decay → Sustain, 3 → 4) keeps its exact current behavior.
- Add regression tests covering every `(loop_mode, start, end)` combination for bounded per-sample work and finite output.
- Out of scope: real PingPong behavior (it stays an alias of Forward-from-Decay), the velocity floor bug (C4).

## Capabilities

### New Capabilities
- `mseg-envelope`: Stage sequencing and loop behavior of the multi-stage friction envelope, including real-time safety of every loop configuration.

### Modified Capabilities
<!-- none: no existing specs in openspec/specs/ -->

## Impact

- Code: `src/dsp/envelopes/mod.rs` (`process_sample`, `advance_stage`, `handle_loop` — replaced by `loop_region`/`loop_stage` — and `set_loop`); `src/params.rs` doc comments for `loop_start_stage` / `loop_end_stage`.
- Behavior: existing presets all use `loop_mode = 0`, start 3, end 4; Drone mode forces Forward 3 → 4. Their output must be sample-identical before/after.
- Tests: new unit tests in the envelope module plus a voice-level test that plays a friction note with the previously crashing settings.
- Docs: `FABLE-CODE-REVIEW-2026-07-07.md` C2 status.

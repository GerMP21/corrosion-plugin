# Design

## Context

`MsegEnvelope::process_sample()` (`src/dsp/envelopes/mod.rs`) advances `stage_time`, then matches on the stage. Whenever a stage is finished it calls `advance_stage()` or `handle_loop()` and then `return self.process_sample()`. Each re-entry adds another `dt · time_scale` to `stage_time`. The Sustain arm with any loop mode calls `handle_loop()` unconditionally (`stage_time >= 0.0` is always true); if `handle_loop()` maps back to Sustain (start stage 4, or 5 clamped to 4), recursion never terminates.

`set_loop()` stores `end_stage` but nothing reads it. `release()` already jumps straight to Release from any non-idle stage, so gate-off cannot be blocked by a loop.

Constraint: 19 factory presets use Drone mode, which forces Forward 3 → 4; all 50 presets store start 3 / end 4. Their output must not change.

## Goals / Non-Goals

**Goals:**
- No recursion; bounded transitions per sample regardless of parameters.
- `loop_end_stage` honored with clear semantics (see spec).
- Sample-identical output for the default/Drone 3 → 4 loop and for loop mode Off.

**Non-Goals:**
- True PingPong (reverse traversal). It keeps today's Forward-from-Decay behavior; the param doc keeps saying "partial".
- The velocity/level floor and release-end discontinuity (review C4).
- Fixing the extra `dt` added on zero-length stage chains beyond what falls out naturally (see Decision 2).

## Decisions

1. **Iterative transition loop with a hard cap.** Restructure `process_sample()` as: advance time once; then `for _ in 0..MAX_TRANSITIONS { match stage { … finished → transition; continue, … active → break } }`, then compute the level. `MAX_TRANSITIONS = 8` (more than the 6 stages, so any legitimate chain of zero-length stages completes). If the cap is hit, stay in the current stage for this sample. Alternative: keep recursion but add a depth counter — rejected; still recursion on the audio thread and harder to reason about.

2. **Do not re-add `dt` on each transition.** Today every recursive call adds `dt` again. To stay sample-identical for the default configs, measure the current behavior first (task 1.1 golden capture). If the golden test shows the extra `dt` is observable for 3 → 4 loops, preserve it explicitly for that path and document it; otherwise drop it. The golden test decides, not assumption.

3. **Normalize loop bounds; derive the region at each transition.** `set_loop()` keeps clamping start and end to 1..=4, and looping is active only when `mode != Off && start < end`. *Implementation note:* the region is computed by `loop_region()` at each stage transition, not cached as a flag in `set_loop()`, because `scrape_envelope()` writes the loop fields directly without calling `set_loop()`. Transitions are rare, so this costs nothing measurable. PingPong maps to the fixed region 3 → 4, matching its old behavior.

4. **Loop trigger = entering the end stage.** When a transition would enter `end` with the gate held and looping active, enter `start` instead with `stage_time = 0`. For end = Sustain this reproduces today's 3 → 4 behavior; for end < Sustain it truncates the held part of the envelope as the spec requires. Because `start < end` is required and the jump goes strictly backwards, a loop always passes through at least one stage with non-zero duration or hits the transition cap — it can never spin.

5. **Zero-duration loop regions.** If every stage in `[start, end)` has zero duration, the loop would immediately re-trigger each sample. The transition cap bounds the work; the output simply holds that stage's level. Covered by the exhaustive sweep test.

## Implementation findings

- **The release-build symptom is a hang, not a crash.** Optimized builds turn the self-recursive tail call into an endless loop, so the audio thread spins forever instead of overflowing its stack. The regression tests therefore run the envelope on a separate thread with a deadline, so they fail on either symptom.
- **A second path to the same bug.** With any loop on and `t_decay = 0`, the old code recursed forever even for the default 3 → 4 loop (Decay → Sustain → Decay …). The host can't reach it (`mseg_decay` has a 10 ms minimum), but the API can; the transition cap now bounds it, and the sweep test covers zero-length stages.
- **Decision 2 resolved:** each transition starts the new stage at `stage_time = step`. That equals the old `0 + dt` re-entry exactly, and the golden test confirms bit-identical output for Off, Forward 3 → 4 (two velocities), `scrape_envelope()`, and PingPong.

## Risks / Trade-offs

- [Subtle timing drift changes Drone presets] → Golden-output test (task 1.1) captured from the current code for loop Off and Forward 3 → 4 before any edit; must match bit-for-bit after.
- [Honoring `loop_end_stage` changes sessions that automated it] → It was dead before, so only sessions with non-default end values change; the GUI already shows the knob, so behavior now matches the UI.
- [Transition cap masks a future bug silently] → A `debug_assert!` would fire on the legitimate zero-duration case (Decision 5), so instead a unit test asserts the cap is never hit for the default and Drone configurations with the default stage times.

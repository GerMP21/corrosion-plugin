# Design

## Context

- `Corrosion::initialize` returns `true` and does nothing. `InitContext` offers `set_latency_samples`, and `BufferConfig` carries the sample rate.
- `process()` reports latency after `process_block`, comparing the limiter mode's latency with `reported_latency_samples` (starts at 0).
- `Voice::set_rate` (per-sample constants, exciter rates) runs only from `process_sample[_stereo]`, which returns early for a voice that isn't rendering. A voice's `sample_rate` is therefore 48 kHz until it first plays, and afterwards whatever rate it last played at.
- `ModalResonator::rebuild_in_place` ends with `set_live_resonator_controls(damping, brightness, 48_000)`, which builds every mode's coefficients at 48 kHz and caches that rate; `SecondOrderMode::ensure_rate` then rebuilds them on the first sample at any other rate. The friction MSEG takes the voice's rate at note-on but is re-rated every sample, so it self-corrects.

## Goals / Non-Goals

**Goals:** latency known at initialization; one coefficient build per note-on at any rate; voices that agree on the rate whether or not they have played.

**Non-Goals:**
- Changing per-sample constants or any DSP at 48 kHz.
- Supporting a rate change in the middle of a block.

## Decisions

1. **Report latency in `initialize`.** Compute it from `params.limiter_mode` (already restored by the host when `initialize` runs), call `context.set_latency_samples`, and store it in `reported_latency_samples` so `process()` only reports changes. *Alternative:* report on every `process()`. Hosts may re-negotiate buses on each call (the reason for the existing change check).
2. **A `VoiceManager::set_sample_rate(rate)` that rates every voice.** It calls `Voice::set_rate` on all 16 voices (cheap when unchanged: one comparison each) and stores the rate. `Corrosion` calls it from `initialize` and from `begin_block` when the block's rate differs from the last one. No allocation: `set_rate` only rewrites fixed-size constants.
3. **Note-on builds at the voice's rate.** `rebuild_in_place` takes the sample rate and passes it to `set_live_resonator_controls`, so `ensure_rate` finds the cached rate correct and does nothing on the first sample. `Voice` passes its `sample_rate`, which decision 2 keeps current. *Alternative:* build lazily on the first sample. That is what happens now, and it doubles the work.
4. **Proof of no change at 48 kHz:** the golden mode-bank fixture and the existing sample-rate tests must pass unchanged, and a render hash at 48 kHz must match a clean worktree (one-off, recorded in Measurements).

## Risks / Trade-offs

- **[Rating idle voices costs work on a rate change]** → It happens once per change, not per sample, and only rewrites constants.
- **[`initialize` may run before the host restores state in some hosts]** → `process()` still reports any later change, as today, so the worst case is today's behaviour.
- **[The fresh-versus-primed equality may also expose exciter state that depends on the rate at trigger time]** → If the spec test fails for that reason after decisions 2–3, record which exciter and ask before widening the change.

## Migration Plan

None: no parameter or state changes.

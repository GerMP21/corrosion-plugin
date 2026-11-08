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

## As built

- **Latency logic extracted:** `Corrosion::latency_change(limiter_mode)` holds the "report only changes" rule; `process()` and the tests use it. `initialize` reports the restored mode's latency (0 for Hard) and seeds `reported_latency_samples`.
- **Voice rate:** `VoiceManager::set_sample_rate` calls the (now `pub(crate)`) `Voice::set_rate` on all 16 voices; `initialize` calls it with `BufferConfig::sample_rate`, and `begin_block` when the block's rate differs from the manager's. `rebuild_in_place` takes the sample rate; the voice passes its own, the standalone constructor and tests pass 48 000.
- **Exciter state between notes (scope added by the maintainer):** the fresh-versus-primed test showed that four exciters carried state from the previous note on the same voice, at every rate, so the risk named above applied. The maintainer chose to fix it here rather than relax the scenario. Each `trigger` now restarts that state: HeavyGrinding's noise filter states, PneumaticJet's band filter, ElectromagneticHum's phase, and ParticleRain's spawn accumulator and in-flight particles (`clear` keeps the preallocated capacity). Repeated notes on those exciters now start identically; other exciters are unchanged.

## Migration Plan

None: no parameter or state changes.

## Measurements

### Tests on the code before the change (tasks 1.1–1.2)

- **Latency at initialization (Lookahead):** fails; nothing is reported (`[]`, expected `[48]`).
- **Hard at initialization, and a Hard→Lookahead change between blocks:** pass (the change test exercises `Corrosion::latency_change`, extracted from `process()` with no behaviour change).
- **First note at 96 kHz, fresh versus primed voice:** 12 of the 16 exciters are already bit-identical, so the doubled coefficient build costs time but doesn't change the output. Four differ from frame 0: HeavyGrinding (10), PneumaticJet (13), ElectromagneticHum (14) and ParticleRain (16). Their exciters take the voice's rate when they trigger, and a never-played voice still has 48 kHz then.
- **Bend before the first sample (44.1 kHz):** already passes. The coefficients are rebuilt at the right rate on the first sample, from the bent frequency.

### After the change (tasks 2.1–2.5)

- All five spec tests pass. The fresh-versus-primed test passes for all 16 exciters at 96 kHz (and at 48 kHz in a probe), once the four exciters restart from rest; rating idle voices alone left the same four failing, which is how the state leak was found.
- How much the leak changed a note (fresh versus primed, before the exciter fix, as diff energy relative to the note): HeavyGrinding −76 dB at 48 kHz (−64 dB at 96 kHz), PneumaticJet −81 dB (−73 dB), ParticleRain −13 dB (−18 dB), ElectromagneticHum −0.6 dB (+5.5 dB): the hum restarted at whatever phase the last note left.
- **No sound change at 48 kHz for fresh voices:** a hash of `plugin_output_path_sixteen_voices`'s scenario (`sheet_lightning`, 16 notes, 4 096 samples) is `6dd5b638bff89d13` on both the base commit (`1984355`, clean worktree) and this change.
- **Coefficient builds** (temporary counter in `CoupledCoefficients::with_radius`, 16 note-ons of `sheet_lightning` and their first sample, 208 modes):

  | Rate | Before | After |
  |---|---|---|
  | 48 kHz | 416 | 416 |
  | 96 kHz | 624 | 416 |

  At 96 kHz every mode was built three times (bank fill, live controls at 48 kHz, rebuild on the first sample); now twice, as at 48 kHz.
- `tests/no_alloc.rs` (debug): 9 pass, including the new `rate_change_and_retrigger_are_alloc_free`. CI lane: 360 tests pass; the golden mode-bank fixture is unchanged.

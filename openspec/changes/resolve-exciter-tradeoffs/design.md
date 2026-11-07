# Design

## Context

See proposal.md for the trade-offs being removed. The relevant current state (after `normalize-object-loudness`):

- **The voice loop.** Each sample, the voice calls the exciter with the resonator's previous-sample displacement and velocity. The exciter returns a force, which is multiplied by the force envelope and `EXCITER_DRIVE_TRIM[exciter]` and fed to the resonator.
  - The coupling is explicit, with a one-sample lag.
  - The trims were tuned so the single worst object and note stays under 0.9.
- **The modes** run in coupled form: `z ← p·z + g·x`, `y = Im z`, with `g = b0·(cot ω + i)`, so the ring amplitude is `b0/sin ω`.
  - Modes at or above `min(20 kHz, 0.45·fs)` get `b0 = 0`.
  - Banks are normalized at note-on by `‖w·b0/sin ω‖_1.3`.
- **The interaction bus** recomputes `sin(nπ·position)` for every mode on every sample, even when the position never moves. That is most of the extra per-mode cost.
- **Noise.** The exciters generate noise as `fract(sin(phase)·43758)` with an f32 phase that grows each call. As the phase grows, its f32 resolution coarsens, so the noise's spectrum depends on note length and sample rate.
- **Per-exciter findings** are in `fix-sample-rate-time-constants/design.md` and `normalize-object-loudness/design.md`, including:
  - HandStrike's 5-sample pulse on the attack ramp;
  - drumstick chatter;
  - the ParticleRain spawn-train aliasing;
  - the MetalChain filter that keeps no state;
  - the runaway thresholds.

## Goals / Non-Goals

**Goals:**
- Meet the specs with no per-exciter exceptions (see the spec deltas).
- 16-voice benchmarks at or below the pre-normalization cost (`748d832`), measured on the same machine in the same session. See the findings.
- Stay allocation-free on the audio thread (`tests/no_alloc.rs` in debug).

**Non-Goals:**
- **Re-voicing exciters** beyond what each fix needs. Exciter parameters keep their meanings.
- **Bit-identical 48 kHz output.** Every exciter's sound changes, and the presets are placeholders.
- **A soft saturator in place of the clamp.** The governor keeps voices out of it.
- **Sample rates above 96 kHz** in the specs. 192 kHz is measured and recorded, not required.

## Decisions

1. **Drive levels target the median, and a per-voice governor catches the rest.**
   - `EXCITER_DRIVE_TRIM` becomes `EXCITER_DRIVE_LEVEL`, set so each exciter's median pre-clamp peak over the 27-note sweep (9 objects × C2/C4/C6) is about 0.35. Levels above 1 are allowed: HandStrike and MetalChain are raised.
   - The governor is a per-voice gain on the exciter force. Because it scales the force rather than the output, the note stays clean, and it lowers the loop gain of any exciter↔resonator feedback.
   - **It acts on a prediction, not on the output.**
     - A follower on the output can't catch a strike. The force pulse (0.1–2 ms) is over before the output peaks: about 4 ms later at C2.
     - The resonator tracks a ring bound `B = Σ w_i·|z_i|`, where `w_i = fade_i·(1 + boost·|H_hp(ω_i)|)` and `H_hp` is the voice's brightness high-pass.
     - Without more force, no later sample (either stereo channel) can exceed `B`. So `B` is known on the strike sample itself.
     - It also tracks `per_force = Σ w_i·|g_i|·|b_i|`: how far one unit of force can raise `B`.
     - `B` is summed exactly on every 8th tracked sample. In between it grows by `per_force·|f|`, which is still an upper bound.
   - **Smooth gain.** A follower on `B` (1 ms attack, 200 ms release, through `rate::smoother`) gives a gain of `(0.7 / level)^8` above 0.7, and 1 below it.
     - The gain depends only on the current level, so a long ring that keeps `B` up after the force drops can't wind it down.
     - A note driven 12 dB over settles about 1.3 dB above the threshold.
   - **Ceiling.** Each sample's force is capped so that `B` stays at or under 0.9. This is the same margin `stabilize-mallet-and-grinding-contact` set for HardMallet and HeavyGrinding.
     - The cap is what catches a strike faster than the follower.
     - Contact exciters see it in their prediction (`ContactPrediction::limit`), so their solve uses the force that is really applied.
   - A test-only switch (`governor_enabled`) turns the governor off for the stability scenario.
   - *Alternatives:*
     - A follower on the pre-clamp output with gain `0.7 / envelope`. It can't catch a strike. Its steady state is a square-root compromise (a note 12 dB over settles 6 dB over). And dividing the output by the gain, to follow the ungoverned level, winds the gain down to nothing while a long ring holds the output up.
     - A look-ahead of up to 1 ms on the force path. A strike's output peaks several milliseconds after its force, so it doesn't help.
     - Worst-case trims (the current state, rejected by the user).
     - Per-object × exciter level tables, which go stale, and still can't handle velocity and controls.
     - Output limiting, which colours the sound and leaves runaway loops running.

2. **Contact is solved together with the resonator's immediate response.**
   - **The prediction.** `ModalResonator::contact_prediction` gives this sample's contact displacement as `free + compliance·F`.
     - `free` is where the resonator goes on its own.
     - `compliance` is `Σ w_i·b_i·fade_i·Im(g_i)`, from the strike-position weights, the coupling blend and the band fade.
     - The voice scales the compliance by everything it multiplies the force by (envelope, drive level, governor gain), and passes the governor's force cap.
   - **The solve.** Each position-dependent exciter finds the force that satisfies its contact law at the predicted displacement, instead of reacting to the last sample's.
     - The shared helper is `dsp::utils::contact::solve_force`: an Illinois regula falsi on `F − h(F)`, with a bisection step whenever the bracket doesn't halve. It handles springs, Hertz contacts and saturating laws alike, as long as they are monotone in the displacement.
     - This is the zero-delay feedback pattern already used for the filter (C1). The loop can no longer gain energy from the one-sample lag, so the runaway thresholds go away.
     - The exciters' `process_sample` takes the prediction instead of the last displacement.
   - **Exciters covered:** HandStrike, FeltMallet, HardMallet, Drumstick, MetalPipe, MetalChain, ParticleRain, HeavyGrinding and TensionSnap.
   - **Exciters left explicit:** Bow, StiffPoint, CorrugatedDrag, TensionRise and PneumaticJet keep reading the resonator's velocity.
     - Their friction and flow laws aren't monotone, which the solver needs.
     - None of them runs away at 12 dB over their drive with the governor off.
     - StiffPoint did diverge, but from negative damping: its damping term had the wrong sign, and that is fixed.
   - **Length units.**
     - A position-driven exciter can't push the surface further than its own travel. At the drive needed for the common level, HandStrike, MetalPipe and ParticleRain became displacement-limited: their output stopped following the drive, and their level followed the sample grid.
     - Each now sees the resonator in its own length units (`ContactPrediction::in_units`; `HAND_SCALE`, `PIPE_SCALE`, `RAIN_SCALE`).
     - Their contact then behaves as it did at the drive where it was rate-matched, and their output scales linearly with the drive.
   - *Alternative:* oversampling the contact. That costs more CPU and only moves the threshold.

3. **Struck exciters come to rest.** When a drumstick or hard mallet reaches its bounce limit, or its rebound velocity falls below a rest threshold, it becomes inactive. Today the stick is instead pinned to the resonator and pulled back in by "gravity" each sample. The timing test then uses the whole-note peak.

4. **One integer noise generator.**
   - `dsp::utils::noise::Noise` is xorshift32 seeded per voice and note, returning uniform [−1, 1). It replaces every `sin(phase)·43758` generator: PneumaticJet, MetalChain (noise and link randomization), WireBrush, ParticleRain, HeavyGrinding, the voice rattle, and any others the inventory finds.
   - Per-sample noise forces go through `rate::white_noise`, so their audible-band density holds at every rate.
   - Random event parameters keep their per-event draws.
   - *Alternative:* keep the hash, using an f64 phase that wraps. That is still not white, and it costs more.

5. **Short events are timed between samples.**
   - An event whose time falls at fraction `φ` into a sample adds `(1−φ)·A` to that sample and `φ·A` to the next. Its area and centre of mass then match continuous time. This covers ParticleRain spawns, WireBrush impulses, the tension-snap and tension-rise spikes, and MetalChain impacts.
   - HandStrike's approach: the snap sample's force is scaled by the fraction of the step that was still in contact, and the approach force is taken at mid-step.
   - MetalChain's links follow their exact fall trajectory, so the moment they reach the surface doesn't depend on the grid. Each impact uses the penetration half a reference sample into the contact, so its area depends only on the link's speed.
   - The Hit envelope's attack is evaluated at sample centres, `level += dt/attack` starting from `0.5·dt/attack`, so a short strike on the ramp has the same area at every rate.
   - *Alternative:* oversample the exciters only. That costs more, and still rounds.

6. **Mode amplitude, and a fade at the top of the band.**
   - Mode gain becomes `g = b0·(sin ω / ω)·(cot ω + i)`, so the ring amplitude is `b0/ω`: the continuous oscillator's, and the same at every rate. It no longer blows up near Nyquist.
   - The modes' output is weighted by a fade that is 1 below 0.9·top and 0 at or above the top, raised-cosine in between. The top is `min(20 kHz, 0.45·fs)`, so the fade runs from 18 to 20 kHz at 44.1 kHz and above. The fade is evaluated from the mode's current frequency.
   - The weight is applied to the output only, so a mode retuned upward fades out instead of switching off or ringing on as an alias.
     - Weighting the input as well would square the fade.
     - The output fade alone already silences the mode and keeps retuning smooth.
     - The contact prediction uses the same output weight, so exciters see the faded response.
   - The note-on estimate uses the same amplitude and fade.
   - The `MODE_CEILING_HZ` hard cut is removed. A generator still need not create modes above 20 kHz; that is a later clean-up, not required.
   - *Alternative:* keep the hard cut. It clicks on retune, and it masked the contact fault that Decision 2 fixes.

7. **Strike-position weights are cached.** The bus recomputes `mode_coefficients` only when the effective position changes, that is, when wander or the position envelope is active, or a parameter is set. Otherwise it reuses the cached weights. The per-mode weighted force, `x·((1−c) + w·c)`, is computed once per mode with precomputed blend weights.

8. **MetalChain rattle.**
   - It becomes a one-pole high-pass with kept state and pole `exp(−2π·fc/fs)`, so it is rate-correct.
   - The form is `a·y₋₁ + (1+a)/2·(x − x₋₁)`, which has unit gain at Nyquist and is −3 dB at the cutoff. The `a·(y₋₁ + x − x₋₁)` form is −3.6 dB at 1 kHz on 44.1 kHz.
   - The rattle is no longer rectified, since rectified white noise has a DC part that grows with √fs.
   - The rattle level is then set by the chain's drive level (Decision 1).

## Risks / Trade-offs

- **[The governor can audibly pump on sustained notes near the threshold]** → The 200 ms release and the 0.7 threshold leave the median note (about 0.35) about 6 dB below the threshold.
  - The ring bound sits above the actual peak, so in practice the governor engages on 2–12 of the 27 sweep notes per exciter, by up to −23 dB (see the findings). That is more than outliers alone.
  - The excess-drive scenario bounds the peak's change to 3 dB. It doesn't bound the RMS: a note quieter than the threshold at default drive can only be pulled down to about the threshold.
  - Record how many sweep notes engage the governor, and by how much.
- **[The implicit contact changes each struck exciter's tone]** → Expected. The non-goal on bit-identity covers it. Struck timing and level tests guard the behaviour.
- **[Chaotic exciters (Bow stick-slip, TensionRise) may still miss 1.5 dB at some rate]** → Do not widen tolerances. Measure first at velocity 100 and averaged over velocities. If the miss is chaos, not a fault, stop and bring the numbers to the user.
  - The outcome is in the findings. A noise-driven exciter's single note varies by several dB from its seed alone, and Bow locks into a different regime at velocity 100 on 44.1 kHz only.
  - The user chose to compare the expected level (mean power over velocities and seeds), keeping 1.5 dB.
- **[The `sin ω / ω` correction changes high-mode balance at 48 kHz]** → At 48 kHz it lowers a 5 kHz mode by 0.6 dB and a 15 kHz mode by 6.5 dB (more at 44.1 kHz, less at 96 kHz). That is intended: the discrete gain was that much too high, and differently so at each rate. The bank estimate uses the same amplitude, so normalization stays consistent.
- **[CPU goal not met by caching alone]** → Fallback: store the mode state and coefficients as struct-of-arrays so the inner loop vectorizes. Record the numbers either way.

## Migration Plan

Archive the four completed changes first (see proposal), so the deltas apply. After this change:
- re-capture the golden mode-bank fixture;
- re-run the preset level pass;
- update `levels.md` (in this change), ARCHITECTURE and README.
There's no data migration; presets keep their parameters.

## Implementation findings

### Baseline (task 1.1)

`openspec list --specs` shows `resonator-level` (5 requirements) and `sample-rate-independence` (3), both archived on 2026-10-07.

The 16-voice benchmarks (4 096 samples = 85 ms at 48 kHz, gnu release) ran in this session's container, which is a different machine from the one the original figures came from:

| | Pre-normalization (`748d832`) | Current (`d6a01b7`, two runs) |
|---|---|---|
| Normal (Pipe) | 8.57 ms | 10.03 / 10.97 ms |
| Render (damaged Plate, 2× modes) | 21.10 ms | 22.51 / 22.73 ms |

The old absolute targets (10.7 and 25.4 ms) came from another machine, so the CPU goal is now relative: the pre-normalization commit measured on the same machine in the same session. Runs vary by about ±5 %.

### Failing tests on the current code (task 1.2)

The per-exciter tolerances, the drumstick's 20 ms timing window and the 1e-3 "silent" threshold are removed, so the existing tests state the spec again. The new tests and the restored ones, on `d6a01b7`:

- **`exciters_play_at_a_common_level`.** The common median is 0.076, and 8 exciters are out of the ±6 dB band:

  | Exciter | Median vs common |
  |---|---|
  | HandStrike | −19.0 dB |
  | Drumstick | +6.0 dB |
  | WireBrush | +8.3 dB |
  | MetalChain | −47.9 dB |
  | HeavyGrinding | −11.5 dB |
  | CorrugatedDrag | −9.0 dB |
  | TensionSnap | +6.7 dB |
  | ParticleRain | −11.6 dB |
- **`default_notes_do_not_overdrive_the_voice`**, now covering all 16 exciters: **passes**, because the current trims are sized for the worst case.
- **`excess_drive_is_reduced_not_clipped`:** fails for all 16.
  - Each exciter's loudest note gets +1.8 to +14.3 dB louder at 12 dB of extra drive, with pre-clamp peaks up to 5.8.
  - The exception is ParticleRain (−3.1 dB): its feedback changes regime.
- **`contact_is_stable_without_the_governor`** (12 dB of extra drive, governor off): only ParticleRain diverges, on SheetMetal C6 (209). Every other exciter stays bounded.
- **`strikes_come_to_rest`:** the drumstick still applies force at 0.55 s.
- **`struck_attack_timing_does_not_depend_on_sample_rate`**, whole-note peak: the drumstick at 44.1 kHz peaks at 63 ms against 343 ms at 48 kHz.
- **`every_exciter_has_the_same_level_at_any_sample_rate`** at 1.5 dB, with no exceptions:

  | Exciter | 44.1 kHz | 96 kHz |
  |---|---|---|
  | Bow | +1.56 | — |
  | HandStrike | — | −2.48 |
  | WireBrush | +1.97 | — |
  | MetalChain | −2.09 | −1.87 |
  | TensionRise | +2.71 | — |
  | PneumaticJet | — | +2.87 |
- **`sixteen_simultaneous_notes_all_sound`** at the 0.01 threshold: a bowed voice peaks at 0.0084.
- **`a_high_mode_keeps_its_level_at_every_sample_rate`:** +1.51 dB at 44.1 kHz. The test stops at the first failure, and by the formula 96 kHz is about −5 dB.
- **`a_mode_swept_through_the_top_of_the_band_does_not_click`:** **passes on the current code.**
  - The coupled form already retunes without a step, and the 20 kHz cut gates only a mode's input, not its ringing, so sweeping a ringing mode across it doesn't click.
  - It stays as a guard for the output fade (Decision 6).

### Foundations (tasks 2.1–2.4)

- **Noise (2.1).** `dsp::utils::noise::Noise` (xorshift32) replaces all nine hash generators, and `grep 43_758 src` is empty. Its sites:
  - the jet;
  - the MetalChain rattle and link draws;
  - the WireBrush impulse draws;
  - the ParticleRain mass draws;
  - the TensionRise slip jitter;
  - HeavyGrinding's Brownian walk;
  - the voice's damage rattle;
  - the ladder filter's component-tolerance jitter.

  How each kind of noise is handled:
  - **White noise forces and output noise** (jet, rattles) go through `rate::white_noise`.
  - **The Brownian walk's step** grows as √ratio, since a leaky walk's variance is `step²/(2·ratio·(1 − leak))`.
  - **Seeding:** the voice reseeds every consumer at note-on from `note_seed(note, velocity, start_frame)`, salted per consumer.

  Two notes on the tests and the effect:
  - **Spectrum test:** a 64-bin spectrum estimate scattered by ±0.5 dB, so the test averages the periodograms of 32 blocks (Welch).
  - **Exciter levels:** clean noise changes several exciters' sample-rate figures. WireBrush passes now. MetalChain, ParticleRain, TensionRise and HeavyGrinding move, and are re-measured after the event-timing and level work (3.3, 4.2).
- **Mode amplitude and fade (2.2).**
  - The gain is now `b0·(sin ω/ω)·(cot ω + i)`, so the ring amplitude is `b0/ω`.
  - The output is weighted by `band_fade`, raised-cosine from 0.9·top to the top, where the top is `min(20 kHz, 0.45·fs)`: 18–20 kHz at 44.1 kHz and above.
  - **Deviation from Decision 6:** the fade weights the output only, not the input as well. Weighting both would square it, and the output fade alone already silences a mode and keeps retuning smooth.
  - `STRIKE_LEVEL_REF` 0.594507 → 0.594266 (−0.04 %). Pipe C4's modes are low, where `sin ω ≈ ω`.
  - The 15 kHz test passes, and so do the pitch, object, quality and damage loudness tests and the sustained-level test up to 192 kHz.
  - The mode-bank fixture is re-captured and matches on musl and gnu.
- **Cached strike weights (2.3).**
  - The bus skips recomputing its coefficients when (position, lock, lock minimum, mode count) is unchanged; the wander phase still advances.
  - The 16-voice benchmark went from 10.0 to **5.5 ms** at Normal and from 22.5 to **12.8 ms** at Render, well under the same-machine pre-normalization figures (8.6 and 21.1 ms). The per-mode blend precompute and the struct-of-arrays fallback aren't needed.
  - The interaction, reuse and prediction tests pass, and so does `no_alloc` in debug.
- **Prediction fade (2.4).** `contact_prediction` weights each mode's free response and compliance by its fade, and the prediction test passes.

### Exciters (tasks 3.1–3.4)

- **Implicit contact (3.1).**
  - HandStrike, FeltMallet, Drumstick, MetalPipe, MetalChain, ParticleRain and TensionSnap solve their force through `contact::solve_force` against the voice's `contact_prediction`. HardMallet and HeavyGrinding already did.
  - **Deviation:** Bow, StiffPoint, CorrugatedDrag, TensionRise and PneumaticJet stay explicit. Their friction and flow laws are non-monotone in the resonator's velocity, which the monotone solver can't bracket, and none of them diverges at 12 dB of extra drive with the governor off. The stability sweep passes.
  - `second_order_mode_obeys_difference_equation` now expects the first output `b0·sin ω/ω`, from 2.2's gain.
- **Strikes come to rest (3.2).** Drumstick goes inactive at its bounce limit or below the rest velocity, and a 12 000-frame (48 kHz, rate-scaled) cap ends a strike that never rebounds. The rest scenario passes, and Drumstick and HardMallet are timed on the whole-note peak.
- **Sub-sample events (3.3).** `rate::EventSplitter` places each event's area across two samples. The unit tests (`dsp::exciters::event_timing_tests`, `hit_attack_has_the_same_area_at_every_rate`) render one event at 44.1, 48 and 96 kHz and check its area within 2 % and its centre of mass within one 44.1 kHz sample plus 10 µs. Three exciters needed more than the splitter:
  - **MetalChain:** a link's force came from its penetration on the first contact sample, which depends on where the grid falls (−24 % area at 44.1 kHz). The link now follows its exact fall trajectory, and the impact uses the penetration half a reference sample into the contact, less the resonator's push-back. The area then depends only on the link's speed.
  - **ParticleRain:** a particle still pushed on the sample its velocity passed the rebound value, a one-sample tail worth `h/2` of the ramp (−4.8 % at 96 kHz). It is now removed before the solve.
  - **HandStrike:** the approach force is evaluated at the middle of each step instead of its start (−2.1 % at 96 kHz before).
- **MetalChain rattle (3.4).**
  - The rattle runs through a one-pole high-pass with kept state and pole `exp(−2π·fc/fs)`.
  - The `a·(y₋₁ + x − x₋₁)` form is −3.6 dB at 1 kHz on 44.1 kHz. The form used, `a·y₋₁ + (1+a)/2·(x − x₋₁)`, has unit gain at Nyquist and is −3 dB at the cutoff within 0.5 dB at 200 Hz and 1 kHz on every tested rate.
  - The output is still rectified (a link only pushes).
  - The §12 note is removed.
- **Sample-rate figures after group 3** (lib run, before the governor and drive levels):

  | Exciter | 44.1 kHz | 96 kHz |
  |---|---|---|
  | Bow | +1.56 | — |
  | HeavyGrinding | −1.75 | −5.57 |
  | TensionRise | +4.47 | — |
  | PneumaticJet | +1.58 | +1.82 |
  | ParticleRain | — | +1.63 |

  "—" means the rate passes. MetalChain now passes. HeavyGrinding's worst pre-clamp peak is 5.2, so its output clamp distorts the comparison. All of these are re-measured in 4.2.

### Drive governor and levels (tasks 4.1–4.2)

- **Governor (4.1), with three deviations from Decision 1:**
  - **It follows a prediction, not the output.** A follower on the pre-clamp output can't catch a strike: the force pulse (0.1–2 ms) is over before the output peaks (about 4 ms later at C2).
    - The resonator tracks a ring bound `B = Σ w_i·|z_i|`, with `w_i = fade_i·(1 + boost·|H_hp(ω_i)|)`, where `H_hp` is the voice's brightness high-pass.
    - It also tracks `per_force = Σ w_i·|g_i|·|b_i|`.
    - Without more force, no later sample can exceed `B`. Over the sweep it was 1.0–1.5× the actual peak for most notes, and at most about 3×.
    - `ring_bound_rises_by_at_most_per_force` checks the bound on every object. It caught a first version that summed the signed strike-position blends.
  - **A per-sample ceiling.** Each sample's force is capped so that `B + per_force·|f| ≤ 0.9`. Contact exciters see the cap in their prediction (`ContactPrediction::limit`), so their solve uses the force really applied.
    - 0.9 keeps `stabilize-mallet-and-grinding-contact`'s HardMallet/HeavyGrinding margin, for every exciter.
    - `B` is summed exactly on every 8th tracked sample. In between it grows by `per_force·|f|`, which is still an upper bound.
  - **The gain law is `(0.7/env)^8`, not `0.7/env`.**
    - `env` follows `B` (1 ms attack, 200 ms release, through `rate::smoother`).
    - With `0.7/env`, a note driven `x` times over settles at `√(0.7·x)` of the threshold.
    - Dividing the bound by the gain to follow the ungoverned level wound down to −60 dB, because a long ring keeps `B` up after the force drops.
    - With order 8, 12 dB over settles about 1.3 dB above the threshold.
  - The test-only off switch is `governor_enabled`. `no_alloc` passes in debug.
- **Exciter fixes found while levelling (4.2).** Raising quiet exciters to a common level exposed faults that a low drive had hidden:
  - **Displacement-limited contacts.** HandStrike, MetalPipe and ParticleRain:
    - With the contact solved implicitly, an exciter can't push the surface further than its own travel. At the drive needed for 0.35, their output stopped following the drive, and their level followed the sample grid (HandStrike +8.4/−4.2 dB, MetalPipe −3.2 dB at 96 kHz, ParticleRain +2.5 dB at 96 kHz).
    - Each now sees the resonator in its own length units (`ContactPrediction::in_units`): `HAND_SCALE` 55.4, `PIPE_SCALE` 5.06, `RAIN_SCALE` 1000. Their strikes keep the shape they had at their old, rate-matched drive, and their output scales linearly.
  - **MetalPipe's internal ring** wasn't rate-converted. Its decay is now `rate::decay`, and its input gain is scaled by `ratio²`.
  - **TensionRise drew its break threshold every sample**, so a faster rate broke earlier and softer (−2.0 dB at 96 kHz, averaged). It now draws once per break (Decision 4: per-event draws).
  - **StiffPoint subtracted its damping term:** negative damping, which diverged at ×4 drive with the governor off. It is now `spring + damping`.
  - **The MetalChain rattle is no longer rectified.** Rectified white noise has a DC part that grows with √fs.
- **Drive levels (4.2).** `EXCITER_DRIVE_LEVEL` is set by `print_exciter_overdrive`. HandStrike, MetalPipe and TensionSnap were bracketed, since they respond non-linearly. Every exciter's median is 0.342–0.363 (target 0.35).

  How often the governor engages at default drive (of 27 sweep notes), and its deepest reduction:

  | Exciter | Notes | Deepest |
  |---|---|---|
  | Bow | 9 | −20.8 dB |
  | HandStrike | 2 | −9.9 dB |
  | FeltMallet | 11 | −17.5 dB |
  | HardMallet | 9 | −17.4 dB |
  | Drumstick | 8 | −17.5 dB |
  | WireBrush | 0 | — |
  | MetalPipe | 10 | −17.5 dB |
  | MetalChain | 10 | −17.7 dB |
  | StiffPoint | 12 | −23.1 dB |
  | HeavyGrinding | 9 | −17.5 dB |
  | CorrugatedDrag | 11 | −22.8 dB |
  | TensionRise | 6 | −11.1 dB |
  | PneumaticJet | 4 | −8.9 dB |
  | ElectromagneticHum | 7 | −17.5 dB |
  | TensionSnap | 2 | −17.4 dB |
  | ParticleRain | 4 | −17.5 dB |

  That is more than the "only outliers" the Risks expected: the ceiling of 0.9 is on the bound, which is above the actual peak.
- **Passing:**
  - `exciters_play_at_a_common_level`, `default_notes_do_not_overdrive_the_voice` (all 16), `contact_is_stable_without_the_governor`, `strikes_come_to_rest`;
  - `struck_attack_timing_does_not_depend_on_sample_rate` with no exceptions;
  - `mallet_and_grinder_play_at_a_struck_exciter_level`, `sixteen_simultaneous_notes_all_sound` at 0.01;
  - `hosted_normalized_note_velocity_reaches_output_chain`. Its reference moved from 0.118 to 0.452, because the default patch is about 12 dB louder.
- **Still failing (stopped here, per Risks):**
  - **`every_exciter_has_the_same_level_at_any_sample_rate`** (Pipe C4, velocity 100, one note):

    | Exciter | 44.1 kHz | 96 kHz |
    |---|---|---|
    | Bow | +1.56 | — |
    | MetalChain | −7.91 | −1.81 |
    | HeavyGrinding | −1.75 | −5.57 |
    | PneumaticJet | +1.57 | +1.81 |

    These aren't scaling faults:
    - **Seed noise.** MetalChain, HeavyGrinding and PneumaticJet are noise-driven, and their 1-second level varies by −5.8…+3.9, −7.0…+6.5 and −5.5…+2.7 dB from the note seed alone at 48 kHz. Averaged over 128 seeds (`print_seed_averaged_rate_levels`), they are −0.56/−0.54, −0.30/−0.39 and +0.55/+0.40 dB, and every exciter is within ±0.56 dB at both rates, except Bow.
    - **Bow** is deterministic. Its 44.1 kHz miss happens only at velocity 100, where it locks into a different stick-slip regime. Averaged over velocities 40–127 it is +0.20/−0.19 dB, and at 32, 40, 46, 50, 88.2, 96 and 192 kHz it is within 0.25 dB.
  - **`excess_drive_is_reduced_not_clipped`:** every peak is ≤ 0.9, but four notes come out more than 3 dB louder than at default drive.

    | Exciter | Note | Level vs default |
    |---|---|---|
    | Bow | IndustrialCog C2 | +4.0 dB |
    | MetalPipe | Tank C2 | +7.2 dB |
    | CorrugatedDrag | IndustrialCog C2 | +3.3 dB |
    | TensionRise | SheetMetal C6 | +3.0 dB |

    A governor that acts above a threshold can only pull a ×4 note down to about that threshold. When the note's level at default drive is well under it, that is more than 3 dB louder. For struck exciters, capping the force also lengthens the contact: MetalPipe Tank C2's 10–100 ms window is the same at both drives, but its 0.1–1 s tail is 7 dB louder.
- **16-voice benchmark, same session:**

  | Build | Normal | Render |
  |---|---|---|
  | `748d832` | 8.44 ms | 20.22 ms |
  | Pre-governor commit | 9.05 ms | 18.06 ms |
  | Now | 8.26 ms | 18.15 ms |

  The machine is slower than at 2.3, so these figures aren't comparable with the earlier ones.

### Spec decisions after task 4.2

The user accepted both recommendations, and the delta specs are updated:

- **Sample-rate level (`sample-rate-independence`).** The Pipe C4 scenario now compares mean power over velocities 40/70/100/127 × 32 note seeds, still within 1.5 dB.
  - With 8 seeds, HeavyGrinding's mean scattered by up to 2.7 dB.
  - With 64 seeds, every velocity was within 1.1 dB.
  - The test passes for all 16 exciters.
- **Excess drive (`exciter-drive`).** The scenario compares the note's peak at ×4 drive with its peak at default drive (within 3 dB), not its RMS, and still requires ≤ ±1.0 before the clamp. It passes for all 16 exciters.

### Presets (task 5.1)

- **Raises reset first.** `preset_levels_table` first resets outputs above 1 to 1. Those were the earlier pass's raises, up to ×40.7 on Falling Chain Cluster, and they compensated for exciters that are now at the common level. The floor rule then re-derives any raise.
- **Before the pass**, on this engine:
  - four presets hit the limiter: Falling Chain Cluster (57 848 samples), Suspension Cable Pluck, Sheet Lightning and Sheet Metal Thunder;
  - Cable Tension Rise sat 15.5 dB under the median.
- **After the pass:**
  - no preset hits the limiter;
  - six presets are raised to the floor (RMS 0.04374);
  - Sheet Lightning and Sheet Metal Thunder are trimmed to 0.95.
- Per-preset numbers are in `levels.md`. `preset_levels_are_sane` and `tests/preset_roundtrip.rs` pass.

### Integration gate (task 6.1)

All green:

| Check | Result |
|---|---|
| Debug workspace tests (musl) | all pass; lib 239 passed in 139 s at first, 21 s with the test profile at `opt-level = 1` (see below); `tests/no_alloc.rs` 8 passed |
| Release workspace tests (musl) | all pass; lib 239 passed |
| gnu lib tests (default features) | 248 passed |
| `cargo fmt --check` | clean |
| clippy `-D warnings`, 1.99 and 1.97 (`-A unknown_lints`) | clean |
| Render smoke (`render --suite family`, `scripts/check_wav.py`) | peak 0.702, RMS 0.304, no NaNs |
| `preset_levels_are_sane`, `tests/preset_roundtrip.rs` | pass |
| `cargo doc --no-deps --no-default-features` | no warnings |

16-voice benchmark, gnu release, same session:

| Build | Normal | Render |
|---|---|---|
| `748d832` | 8.44 ms | 20.22 ms |
| This change | 8.26 ms | 18.15 ms |

The struct-of-arrays fallback isn't needed.

The default-feature build prints `f32`-fallback future-incompatibility warnings from `src/gui/editor.rs`. They predate this change, and the file isn't touched by it.

### Verification follow-ups

`/opsx:verify` found no critical issues. Its warnings and suggestions are addressed:

- **Design text.** Decisions 1, 2, 5, 6 and 8 and two Risks bullets now describe what was built. The deviations were previously only in these findings:
  - the predictive governor;
  - the solver, and which exciters stay explicit;
  - the length scales;
  - the output-only band fade;
  - the high-pass form.
- **Spec wording.** The sample-rate requirement now says "expected level", to match its averaged scenario.
- **192 kHz (informational).** Seed-averaged over 128 notes at velocity 100 (`print_seed_averaged_rate_levels`), every exciter is within 0.7 dB of 48 kHz:

  | Exciter | 192 kHz |
  |---|---|
  | Bow | +0.01 dB |
  | HandStrike | −0.39 dB |
  | FeltMallet | −0.00 dB |
  | HardMallet | +0.02 dB |
  | Drumstick | +0.01 dB |
  | WireBrush | +0.11 dB |
  | MetalPipe | +0.68 dB |
  | MetalChain | −0.44 dB |
  | StiffPoint | −0.06 dB |
  | HeavyGrinding | −0.13 dB |
  | CorrugatedDrag | +0.05 dB |
  | TensionRise | +0.10 dB |
  | PneumaticJet | −0.02 dB |
  | ElectromagneticHum | +0.03 dB |
  | TensionSnap | +0.10 dB |
  | ParticleRain | +0.45 dB |

- **`HAND_SCALE` is HandStrike's drive level.** `EXCITER_DRIVE_LEVEL[2]` now refers to `HAND_SCALE`, so the two can't drift apart. The searched level was 55.44, against 55.4.
- **TensionRise's "release burst" removed.** It was scaled by the tension the break had just zeroed, so it was always silent. The pause after a break remains, so the output is unchanged.
- **Debug test time.** `[profile.test] opt-level = 1` keeps debug assertions and overflow checks, and IEEE results don't depend on the opt level. The debug lib suite went from 139 s to 21 s. The debug workspace tests, the gnu lib tests (248), fmt and clippy pass.
- **Not addressed here:** the build-directory binaries from commit `aa9bff0` are still in the branch history. Squash-merging the branch keeps them out of `main`.

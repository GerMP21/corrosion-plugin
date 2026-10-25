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
   - The governor is a per-voice gain on the exciter force:
     - Its envelope follower tracks the pre-clamp peak, with a 1 ms attack and 200 ms release, converted through `rate::`.
     - Above 0.7, the gain falls as `0.7 / envelope`. Below it, the gain recovers.
   - Because it scales the force rather than the output, the note stays clean. It also lowers the loop gain of any exciter↔resonator feedback.
   - A test-only switch turns the governor off for the stability scenario.
   - A fast strike can overshoot before the follower reacts, and the drive levels absorb that. If the sweep still shows overshoot, the follower gets a short look-ahead, at most 1 ms, on the force path.
   - *Alternatives:*
     - Worst-case trims (the current state, rejected by the user).
     - Per-object × exciter level tables, which go stale, and still can't handle velocity and controls.
     - Output limiting, which colours the sound and leaves runaway loops running.

2. **Contact is solved together with the resonator's immediate response.**
   - The resonator exposes its instantaneous compliance `C = Σ_i w_i·Im(g_i)`: how far this sample's force moves this sample's contact displacement. That is `Σ w_i·b0_i`, plus the high-band fade weights.
   - Each position-dependent exciter computes its contact force from the predicted displacement `x_r + C·F`, instead of the last sample's `x_r`, using a shared helper:
     - a closed form for linear springs: `F = k·d / (1 + k·C)`;
     - two Newton steps for Hertzian `k·d^1.5`.
   - This is the zero-delay feedback pattern already used for the filter (C1). The loop can no longer gain energy from the one-sample lag, so the runaway thresholds go away.
   - The exciters' `process_sample` takes the compliance as one more argument.
   - Exciters covered: HandStrike, FeltMallet, HardMallet, Drumstick, MetalPipe, MetalChain, ParticleRain, HeavyGrinding, StiffPoint, CorrugatedDrag, TensionRise and TensionSnap. That is every exciter whose force reads the resonator's displacement or velocity; Bow and PneumaticJet use velocity, through the same helper. The inventory task confirms the list.
   - *Alternative:* oversampling the contact. That costs more CPU and only moves the threshold.

3. **Struck exciters come to rest.** When a drumstick or hard mallet reaches its bounce limit, or its rebound velocity falls below a rest threshold, it becomes inactive. Today the stick is instead pinned to the resonator and pulled back in by "gravity" each sample. The timing test then uses the whole-note peak.

4. **One integer noise generator.**
   - `dsp::utils::noise::Noise` is xorshift32 seeded per voice and note, returning uniform [−1, 1). It replaces every `sin(phase)·43758` generator: PneumaticJet, MetalChain (noise and link randomization), WireBrush, ParticleRain, HeavyGrinding, the voice rattle, and any others the inventory finds.
   - Per-sample noise forces go through `rate::white_noise`, so their audible-band density holds at every rate.
   - Random event parameters keep their per-event draws.
   - *Alternative:* keep the hash, using an f64 phase that wraps. That is still not white, and it costs more.

5. **Short events are timed between samples.**
   - An event whose time falls at fraction `φ` into a sample adds `(1−φ)·A` to that sample and `φ·A` to the next. Its area and centre of mass then match continuous time. This covers ParticleRain spawns, WireBrush impulses, the tension-snap and tension-rise spikes, and MetalChain impacts.
   - HandStrike's approach: the snap sample's force is scaled by the fraction of the step that was still in contact.
   - The Hit envelope's attack is evaluated at sample centres, `level += dt/attack` starting from `0.5·dt/attack`, so a short strike on the ramp has the same area at every rate.
   - *Alternative:* oversample the exciters only. That costs more, and still rounds.

6. **Mode amplitude, and a fade at the top of the band.**
   - Mode gain becomes `g = b0·(sin ω / ω)·(cot ω + i)`, so the ring amplitude is `b0/ω`: the continuous oscillator's, and the same at every rate. It no longer blows up near Nyquist.
   - The modes' output is weighted by a fade that is 1 below 18 kHz and 0 at or above 20 kHz, raised-cosine in between, evaluated from the mode's current frequency. The weight is applied to the output, so a mode retuned upward fades out instead of switching off or ringing on as an alias. Its input is weighted the same way.
   - The note-on estimate uses the same amplitude and fade.
   - The `MODE_CEILING_HZ` hard cut is removed. A generator still need not create modes above 20 kHz; that is a later clean-up, not required.
   - *Alternative:* keep the hard cut. It clicks on retune, and it masked the contact fault that Decision 2 fixes.

7. **Strike-position weights are cached.** The bus recomputes `mode_coefficients` only when the effective position changes, that is, when wander or the position envelope is active, or a parameter is set. Otherwise it reuses the cached weights. The per-mode weighted force, `x·((1−c) + w·c)`, is computed once per mode with precomputed blend weights.

8. **MetalChain rattle.** It becomes a one-pole high-pass with kept state and coefficient `exp(−2π·fc/fs)`, so it is rate-correct. The rattle level is then set by the chain's drive level (Decision 1).

## Risks / Trade-offs

- **[The governor can audibly pump on sustained notes near the threshold]** → The 200 ms release and the 0.7 threshold leave the median note (about 0.35) about 6 dB below the threshold, so only outliers engage it. The excess-drive scenario bounds the level change to 3 dB. Record how many sweep notes engage the governor, and by how much.
- **[The implicit contact changes each struck exciter's tone]** → Expected. The non-goal on bit-identity covers it. Struck timing and level tests guard the behaviour.
- **[Chaotic exciters (Bow stick-slip, TensionRise) may still miss 1.5 dB at some rate]** → Do not widen tolerances. Measure first at velocity 100 and averaged over velocities. If the miss is chaos, not a fault, stop and bring the numbers to the user.
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

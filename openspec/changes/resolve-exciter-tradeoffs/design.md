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
- 16-voice benchmarks at or below the pre-normalization cost: Normal 10.7 ms, Render 25.4 ms per 4 096 samples on the development machine.
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

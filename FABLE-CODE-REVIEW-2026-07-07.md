# Corrosion — Full Codebase Review (Signal Flow, Bugs, Inconsistencies, Missing Features)

**Date:** 2026-07-07
**Scope:** entire `src/` tree (plugin entry, params, voice engine, DSP core, resonators,
exciters, post-processing, presets, GUI, offline/render bins, randomizer), plus the
factory preset bank, test suite expectations, and build configuration.
**Method:** line-by-line read of the audio path; targeted reads of the GUI and offline
paths; numeric simulation used to verify the ladder-filter stability finding.
**Status (2026-10-05):** C1–C6 are FIXED (OpenSpec changes archived under `openspec/changes/archive/`, specs in `openspec/specs/`), including the follow-ups from their verification pass.
**Status (2026-11-08):** H1–H14 are FIXED too. Their OpenSpec changes are archived under `openspec/changes/archive/2026-11-08-h*`, and their requirements are in `openspec/specs/`. The medium findings (M1–M20) are open except M16.
**Status (2026-11-10):** M1–M20 are closed: fixed by their OpenSpec changes, or, for M2, M14 and M19, resolved or documented as noted on each. M17's decorrelation is deliberately not done (it would change the sound).
**Note:** the earlier review, now `docs/archive/CODE-REVIEW-2026-05-27.md` (2026-05-27), is partially stale — its Finding 1
(clipper oversampling no-op) and the factory-reverb delay compounding have since been
fixed (regression tests exist for both). This document supersedes it.

---

## Signal-flow summary (as implemented)

```
MIDI → handle_note_event (lib.rs)
  → VoiceManager (8 voices) → Voice
      exciter (1 of 16) ⇄ ModalResonator (algorithmic per-object mode bank
                          + interaction bus + dynamics hooks)
      × force envelope (one-shot AR | ADSR | 6-stage MSEG, by exciter family)
      + damage rattle noise, velocity HF boost, expression gain, clamp ±1
  → per-voice stereo sum
→ apply_drive (master drive waveshaper, per-sample smoothed)
→ PostProcessingChain:
     mono-sum → WdfLadderFilter → LorenzDrive → FemBodyResonator
     → remix (input·0.3 + processed·0.7) → HrtfSpread
     → Space (Factory / Spring / Echo, wet-add) → OversampledClipper
→ output gain (smoothed) → Hard clamp or LookaheadLimiter
```

Key structural observation: the WDF filter, Lorenz drive, and FEM body operate on a
**mono sum** of the voice mix, and the chain then reconstitutes "stereo" as
`0.3·dry_stereo + 0.7·processed_mono`. Several critical findings below fall out of
this topology.

---

## CRITICAL

### ~~C1. Ladder filter is unstable for cutoff ≥ fs/4 — including the default 20 kHz — and latches to a DC rail~~ — FIXED
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-ladder-filter-prewarp`](openspec/changes/archive/2026-10-05-fix-ladder-filter-prewarp/proposal.md). Prewarp is now `tan(π·fc/fs)` with `fc ≤ 0.49·fs`. The stages are also rebuilt as unity-gain TPT one-poles, because the old stage's 0.5 DC gain (−24 dB over four stages) left a default note at ≈ −42 dBFS once the rail was gone. Regression tests: `wdf_filter::tests::*`, `tests/post_chain_default_dc.rs`. Commit `d33bb26`.
> Follow-up after verification: with unity-gain stages, the one-sample-delayed resonance feedback made the filter ring forever from resonance ≈ 0.3 at the 20 kHz default. The feedback is now zero-delay with `k = 4·res/0.9`, so tails decay below 90 % resonance and the filter self-oscillates from 90 % up, at any cutoff and sample rate.
> Correction found while scoping: below `fs/4` the bug puts the effective cutoff about an octave *above* the displayed value, not half of it.

`src/dsp/post_processing/wdf_filter.rs:106-112, 144-159` (pre-fix line numbers)

```rust
let wc = 2.0 * PI * self.cutoff_hz / self.sample_rate;
self.g = wc.tan();
```

The TPT one-pole prewarp is `g = tan(π·fc/fs)`; this code uses `tan(2π·fc/fs)` —
twice the correct argument. Consequences, per stage
(`y[n] = (g·x + (1−g)·y[n−1]) / (1+g)`, recursion pole = `(1−g)/(1+g)`):

- `fc < fs/4`: stable but the cutoff mapping is wrong (effective cutoff ≈ half the
  displayed value even before considering the tan warp).
- `fc = fs/4` (12 kHz @ 48 k): `tan(π/2)` → `g ≈ 1.6e16`, response degenerates.
- `fs/4 < fc < fs/2`: `g < 0` → **pole magnitude > 1 → each stage is an unstable
  recursion**. At `fc = 3fs/8` (18 kHz @ 48 k) `g = −1` and the stage divides by
  zero (`1+g = 0`) → inf/NaN.
- The per-stage `transistor_saturate` masks the divergence by clamping, so instead of
  NaNs the filter **latches at ±1.0 and stays there forever** (with zero input the
  unstable recursion re-amplifies the railed state each sample).

Verified by numeric simulation of the exact code (sine input, amplitude 0.1, 48 kHz):

| cutoff | g | steady-state output |
|---|---|---|
| 1 kHz | 0.13 | 0.006 (normal) |
| 11 kHz | 7.6 | 0.005 (wrong but stable) |
| 12 kHz | 1.6e16 | 0.97 railed |
| 18 kHz | −1.0 | 1.0 railed (div-by-zero boundary) |
| **20 kHz (default)** | −0.577 | **1.0 railed** |

The default `filter_cutoff` is 20 000 Hz (`src/params.rs:1643-1651`) and **all 50
factory presets ship `filter_cutoff: 20000.0`**. So in any host at 44.1/48/96 kHz,
the first note ever played drives the post-chain filter into a permanent ±1 DC rail,
which then feeds the drive and body stages and puts a large DC offset on the output
(the body stage integrates the constant, the clipper passes it) until the plugin is
reset. This went unnoticed because (a) no test asserts output values at high cutoff
(the WDF tests only exercise 100 Hz–5 kHz responses), and (b) the shipped preset
renders bypass the post chain entirely (see H14).

**Fix:** `g = (PI * cutoff_hz / sample_rate).tan()` and clamp cutoff to
`< 0.49·fs`; add a regression test asserting no DC and bounded output at
`fc = 20 kHz` and at `fc` swept through `fs/4`.

### ~~C2. MSEG: infinite recursion (audio-thread stack overflow) when `loop_start_stage = 4`~~ — FIXED
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-mseg-loop-recursion`](openspec/changes/archive/2026-10-05-fix-mseg-loop-recursion/proposal.md). `process_sample()` now resolves stage changes in a capped loop (no recursion), `loop_end_stage` is honored, and loop start ≥ end holds sustain. Output for loop Off and the default/Drone 3 → 4 loop was bit-identical to before (commit `3540048`), until C4 changed the velocity gain and the release start level. Two findings while fixing: optimized builds *hang* the audio thread here rather than overflowing the stack, and `t_decay = 0` with any loop hit the same recursion (API-only; the host's decay minimum is 10 ms). PingPong is still partial. Regression tests: `envelopes::tests::*`, `tests/mseg_loop_safety.rs`.

`src/dsp/envelopes/mod.rs:279-289, 347-365`

`Stage::Sustain` with any loop mode calls `handle_loop()` and recurses:

```rust
Stage::Sustain => {
    if self.loop_mode != LoopMode::Off && self.stage_time >= 0.0 {  // always true
        self.handle_loop();
        return self.process_sample();
    }
    ...
}
```

`handle_loop()` for `Forward` maps `loop_start_stage == 4` to `Stage::Sustain` —
which immediately re-enters the same branch. `process_sample()` recurses forever →
stack overflow in the audio callback. Reachable from the host: `loop_start_stage`
is an automatable `IntParam` with range 0–5 (`src/params.rs:1415-1419`); values 4
and 5 (clamped to 4 by `set_loop`) trigger the crash the moment a friction-family
note plays with `loop_mode = Forward`.

Related defects in the same code:

- **`loop_end_stage` is completely ignored** — the loop always spans Decay→Sustain.
  The parameter (params.rs:223-225, GUI knob included) is dead.
- **PingPong is not a ping-pong**: `LoopMode::PingPong` unconditionally jumps to
  `Stage::Decay`, making it behaviorally identical to Forward-from-Decay. (The
  params doc admits "PingPong (partial)".)
- `loop_start_stage` values 0 and 5 silently clamp to 1..4, mismatching the
  documented 0–5 range.
- The `advance_stage`/`return self.process_sample()` re-entry pattern adds
  `dt·time_scale` to `stage_time` on every re-entry, so zero-length stage chains
  skew timing slightly.

### ~~C3. Note-on allocates on the audio thread~~ — FIXED
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`remove-note-on-audio-allocations`](openspec/changes/archive/2026-10-05-remove-note-on-audio-allocations/proposal.md), commit `d16781b`. Each voice preallocates its worst-case bank (`MAX_MODES` = 96) and `rebuild_in_place` refills it at note-on. `reset()` clears voices and the post chain in place, and the mode banks are bit-identical to before (golden fixture). Two findings while fixing: `tests/no_alloc.rs` only checks in **debug** builds (release runs pass vacuously), and three exciters also allocated on the audio thread (MetalChain, WireBrush, ParticleRain), now preallocated. `tests/no_alloc.rs` now runs note-on, stealing, expression and reset inside the check.

`src/dsp/resonators/core.rs:221-268, 536-572`, `src/dsp/profile.rs (damaged())`,
`src/dsp/interaction.rs:138-141`

`handle_note_event` runs inside `process()`. Its call chain
`Voice::note_on_with_controls → ModalResonator::with_algorithm_controls_and_note`
performs, per note-on:

- `generate_modes(...)` → `Vec<ModalModeSpec>` allocation (every resonator algorithm),
- `transform_algorithm_modes` → a second `Vec` plus **one `vec![...]` per mode**
  inside `ModalModeSpec::damaged()` (even at damage = 0: `return vec![self]`),
- `SecondOrderMode` collection into a third `Vec`,
- `BidirectionalInteractionBus::initialize` → `Vec::resize`.

This directly contradicts the module header ("No heap allocations in the audio
callback", lib.rs:50) and the AGENTS/test story: `tests/no_alloc.rs` deliberately
triggers all notes *before* entering the `assert_no_alloc` zone, so the suite never
observes the real-world path. Worse, `Cargo.toml` enables nih-plug's
`assert_process_allocs` on non-Windows — a **debug build will abort the DAW process
on the first MIDI note**. In release it "merely" risks allocator locks/priority
inversion under load. `Corrosion::reset()` (lib.rs:833-840) similarly reconstructs
`VoiceManager` and `PostProcessingChain` (large arrays, several `Vec`s) and can be
called from the audio thread by hosts.

**Fix:** preallocate a max-size mode pool per voice and generate modes into it, or
defer bank construction to a lock-free swap; make the no-alloc test cover note-on.

### ~~C4. MSEG velocity/level formula puts a floor under the envelope and steps to zero at release end~~ — FIXED
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-mseg-velocity-scaling`](openspec/changes/archive/2026-10-05-fix-mseg-velocity-scaling/proposal.md). Velocity is now a pure gain, `level · ((1 − vtl) + vtl · velocity_scale)`, so zero level is silence and Release ends without a step (the click measured 0.175 at velocity 0.3). Full-velocity output matches the old formula within f32 rounding (≤ 1.5e-8) for `velocity_to_level = 1`. Commit `45dd964`. Correction found while scoping: with `velocity_to_level = 1` (all presets) the old formula at full velocity is mathematically `level`, so the floor and release-end click only occur below full velocity or with `velocity_to_level < 1`. Follow-up after verification: Release now ramps from the level the envelope had when the gate closed, so releasing during Attack or Decay no longer jumps to the sustain level first.

`src/dsp/envelopes/mod.rs:315-319` (pre-fix line numbers)

```rust
let velocity_adjusted = 0.5
    + (self.current_level - 0.5) * self.velocity_scale * self.velocity_to_level
    + self.current_level * (1.0 - self.velocity_to_level);
```

- With `velocity_to_level = 1` (the default) and velocity < max
  (`velocity_scale = 0.5 + v/2`), a stage level of 0 outputs
  `0.5·(1 − velocity_scale)` — e.g. **0.25 at velocity 0** instead of silence.
- With `velocity_to_level = 0`, the output is `0.5 + current_level`: a constant
  **+0.5 offset**, so the envelope can never go below half scale and saturates at 1.
- The Release→Idle transition (pre-fix lines 292-295) returns `l_end·env_amount = 0`
  directly, so the envelope **jumps discontinuously** from its floored value to 0 —
  an audible click at the end of every friction-voice release, and friction
  exciters keep being driven at ≥ 0.25 force during the entire release.

The intended math is presumably
`level · ((1−vtl) + vtl·velocity_scale)` — a multiplicative velocity scale that
preserves zeros. The unit test `velocity_affects_level` only checks that peaks
differ, so it passes despite the broken floor.

### ~~C5. Post chain: filter/drive are only audible through the FEM body bank, and the stereo image is 70 % mono-collapsed~~ — FIXED
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-post-chain-body-routing`](openspec/changes/archive/2026-10-05-fix-post-chain-body-routing/proposal.md), commit `d04d252`. Filter and drive now run per channel. The body resonance is added in parallel at BODY / 5, Chassis Volume only tunes the body, and Eco runs the full chain. All 50 factory presets were re-voiced per preset (BODY, plus an `output` trim on 11) to within 0.5 dB of their old RMS, with no more limiter hits than before. The `body` default moved to 2.62 to keep the default patch's level. Found while fixing: the dry voice signal is very quiet on struck presets (a default note peaks at −51 dBFS with BODY 0), because the always-on body bank used to supply nearly all of their level. HRTF spread's own mono remix stays open.

`src/dsp/post_processing/post_chain.rs:138-161`, `fem_body.rs:57-78`

1. `FemBodyResonator::process` returns **only** the resonator-bank sum — there is no
   dry term. In the chain, `bodied = body.process(driven)` then
   `bodied_left = left·0.3 + bodied·0.7`. So the entire filtered/driven signal
   reaches the output **only after being convolved with 8 fixed resonances at
   220–1400 Hz** (scaled ±30 % by material/volume). Turning `filter_cutoff` or
   `drive_amount` mostly changes how a narrowband ringy bank is excited — not the
   broadband mix. Nothing in the params/docs suggests body is always 70 % wet and
   in series; `chassis_volume` (default 0.5!) only scales the wet gain 0.5–1.0 and
   can never bypass the stage.
2. The `0.3/0.7` remix collapses 70 % of the voice engine's per-mode stereo spread
   (the `width` parameter's entire output) to mono on every non-Eco quality mode.
3. **Eco mode changes topology, not just cost** (post_chain.rs:147-151): it skips
   body, spread, *and the user's selected Space/reverb entirely*. A preset with
   Factory reverb sounds dry in Eco. Quality modes should be CPU/fidelity trades,
   not effect on/off switches.
4. `lib.rs:920-923` conflates two params: `set_body_params(material,
   chassis_volume.max(body/5))` — `body` and `chassis_volume` fight through a
   `max()`, so automating `body` below the current `chassis_volume` does nothing.

### ~~C6. Spring reverb: one mono state shared by both channels~~ — FIXED
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-spring-reverb-stereo`](openspec/changes/archive/2026-10-05-fix-spring-reverb-stereo/proposal.md), commit `7d35794`. The post chain now runs one spring per channel, so each tail comes only from its own input and the first echo arrives one full delay length later (80 ms at the default tank size, previously 40 ms). A mono input still gives identical tails. `SpringReverb::reset()` also rewinds the delay line. The two Spring presets (Distant Plate Roll, Spring Reverb Tap) now echo at twice the old spacing, at the same level and with no limiter hits. Spring Reverb Tap's BODY was re-matched from 1.60 to 2.26 to hold that level (`openspec/changes/archive/2026-10-05-fix-spring-reverb-stereo/audition.md`).

`src/dsp/post_processing/post_chain.rs:175-182`, `space.rs:133-212` (pre-fix line numbers)

`SpaceMode::Spring` calls `self.spring_reverb.process(spread_left)` and then
`.process(spread_right)` on the **same instance**. `SpringReverb` owns a single
delay line, write index, and dispersion state, so L and R samples are interleaved
into one buffer: the effective delay is halved, each channel's echo returns on the
opposite call, and the feedback path mixes channels sample-by-sample. Needs either
two instances or true stereo state (like `FactoryEcho` has).

---

## HIGH

### ~~H1. `drive_amount` knob: 80 % of its range is dead, and its maximum is ~20 % wet~~ — FIXED
> **Status: FIXED** (2026-10-27, branch `claude/eloquent-dirac-fc0sqm`): OpenSpec change [`h01-unclamp-post-drive-amount`](openspec/changes/archive/2026-11-08-h01-unclamp-post-drive-amount/proposal.md).
> - **Range:** the host-side clamp of `drive_amount + Violence offset` is now the parameter's own 0–5, so the wet share (`amount / 5`) reaches 100 % at the top of the knob.
> - **Mix:** the drive's mix is a crossfade (`dry·(1 − w)`, not `1 − w/2`), so it no longer adds up to +3.5 dB at full wet. The drive's input gain (`1 + amount/2`) is unchanged.
> - **Correction to this finding:** the Violence offset is −0.2 to +0.2 around its neutral 0.5, not +0.4.
> - **Presets:** the three that use the drive moved by −0.3 to −0.4 dB RMS, from the crossfade alone. Beam Grind went 0.567 → 0.540, Ironforge Anvil Strike 0.164 → 0.159, and Spring Grind 0.450 → 0.432. All of them still pass `preset_levels_are_sane`.
- Param range is 0–5 (`src/params.rs:1663-1668`), but `process()` clamps
  `(drive_amount + violence_bias)` to **0..1** (`src/lib.rs:915-919`). Knob values
  1.0–5.0 are indistinguishable.
- Inside `LorenzDrive::process` (`lorenz_drive.rs:112-116`) the wet mix is
  `drive_amount / 5.0`. With the host-side clamp at 1.0, the maximum achievable wet
  mix is **0.2**. Either the clamp or the `/5` normalization is wrong; together
  they make the post drive nearly inaudible.

### ~~H2. `apply_drive` mix math assumes drive ∈ 0..1 but the param is 0..5~~ — FIXED
> **Status: FIXED** (2026-10-27, branch `claude/eloquent-dirac-fc0sqm`): OpenSpec change [`h02-normalize-master-drive-mix`](openspec/changes/archive/2026-11-08-h02-normalize-master-drive-mix/proposal.md).
> - **Mix:** the master drive now crossfades, `shaped·m + dry·(1 − m)` with `m = min(drive, 1)`. The weights always sum to 1 and the dry weight is never negative. Above 1 only the shaper's input gain rises.
> - **Correction to this finding:** the old negative dry weight never flipped the output's sign, because the shaped term always dominated. The cost was the loudness jump.
> - **Presets:** the 11 presets that use Drive 1.0–1.6 are 0.3–1.6 dB quieter (`levels.md`) and needed no trims.
`src/lib.rs:412-414`: `shaped·drive + sample·(1 − drive·0.5)`.
At drive > 2 the dry coefficient goes **negative** (phase-inverted dry mixed with
5×-scaled wet, then clamped at ±1.5) — the top half of the master Drive knob is a
clamp-dominated, phase-smeared regime rather than "more saturation". The weights
also don't sum to 1 anywhere above drive ≈ 0.67, giving a built-in loudness jump.

### ~~H3. Tempo sync is ON by default and silently disables the Delay Time knob~~ — FIXED
> **Status: FIXED** (2026-10-27, branch `claude/eloquent-dirac-fc0sqm`): OpenSpec change [`h03-default-tempo-sync-off`](openspec/changes/archive/2026-11-08-h03-default-tempo-sync-off/proposal.md).
> - **Default and display:** `sync_rate` defaults to 0.0 (Off), and the host shows "Off" or the division name ("1/16" … "2/1"). The name comes from the same band function the Echo uses (`params::sync_division`), so the display can't drift from the behaviour.
> - **Presets:** the 49 factory presets that don't use sync now store 0.0. Hanging Chain Echo keeps its 0.5 (1/4).
> - **Still open:** the editor control for sync is part of H5.
`sync_rate` defaults to **0.25** (`src/params.rs:1423-1427`) while the documented
"Off" region is `< 0.05` (lib.rs:108-138). In any host that reports tempo, Echo
mode defaults to a 1/8-note sync and the `delay_time` knob does nothing, with no UI
indication. Default should be 0.0 (Off). (The GUI doesn't even expose `sync_rate`
— see H5 — so a GUI user can't turn sync off at all.)

### ~~H4. GUI preset loading drops v4 fields~~ — FIXED
> **Status: FIXED** (2026-11-07, branch `claude/eloquent-dirac-fc0sqm`): OpenSpec change [`h04-load-every-preset-field-in-gui`](openspec/changes/archive/2026-11-08-h04-load-every-preset-field-in-gui/proposal.md).
> - **Generic load:** the editor's `apply_preset` now walks the parameter map and applies `Preset::parameter_values` (every parameter except `ui_scale`), each as a begin/set/end gesture. The six skipped IDs (play mode, limiter mode, the four macros) load, and so will any parameter added later.
> - **Values:** they are normalized against the live parameter, because `into_params` builds some parameters with other ranges.
> - **UI Scale:** loading a preset no longer resizes the editor.
> - **Tests:** `tests/preset_loading.rs`.
`src/gui/editor.rs:1929-2085` (`apply_preset`) sets every parameter **except**
`play_mode`, `limiter_mode`, `macro_mass`, `macro_corrosion`, `macro_violence`,
`macro_brightness`. A factory/user preset that stores Kit/Drone mode, the lookahead
limiter, or non-neutral macros loads incompletely from the GUI — those six params
keep whatever values were previously live. (The host state path via nih-plug
persists them fine, which makes the divergence extra confusing.)

### ~~H5. GUI is missing controls for shipped, audible parameters~~ — FIXED
> **Status: FIXED** (2026-11-07, branch `claude/eloquent-dirac-fc0sqm`): OpenSpec change [`h05-expose-missing-gui-controls`](openspec/changes/archive/2026-11-08-h05-expose-missing-gui-controls/proposal.md).
> - **Controls:** the editor now has resonator Brightness, Thickness, Sync ("Off" or its division), Play Mode, Limiter Mode, and a macro strip (Mass, Corrosion, Violence, Brightness).
> - **Coverage test:** `editor_covers_every_parameter` renders the editor headlessly for every exciter, object and space mode, and fails for any parameter that is neither drawn nor on `HOST_ONLY_PARAMS`. That list holds only `exciter_pressure/speed/roughness` until H6.
> - **User presets:** Save writes to a per-user folder (sanitized names, confirmation before overwriting), and they are listed after the factory bank.
> - **Randomizer:** a Randomize button and mode selector apply the randomizer within its safety constraints. Its BODY range is rescaled to the 0–5 parameter, and the unregistered `mode_param` is removed.
Diffing widgets (`&params.<field>` occurrences in editor.rs) against `CorrosionParams`:

- **`res_brightness`** — resonator Brightness has no knob (Damping does).
- **`thickness`** — the Thickness transform has no knob (Heat and Sludge do).
- **`sync_rate`** — no way to enable/disable tempo sync from the UI (see H3).
- **`play_mode`**, **`limiter_mode`** — Tonal/Kit/Drone and Hard/Lookahead are
  host-automation-only.
- **The four macros** (PRD §12) — implemented in DSP, absent from the UI.
- `exciter_pressure/speed/roughness` — absent, but they're dead anyway (H6).
- The **Save** button is rendered permanently disabled (editor.rs:1919-1924): there
  is no user-preset saving at all, only the embedded factory bank.
- The `randomizer` module (`src/randomizer/mod.rs`) is fully implemented
  (modes, ranges, safety constraints, even a `mode_param()`) but **nothing
  references it** — no GUI button, no parameter. Dead feature.

### ~~H6. Three host-facing exciter params are dead~~ — FIXED
> **Status: FIXED** (2026-11-07, branch `claude/zen-dirac-vq38u4`): OpenSpec change [`h06-wire-global-exciter-controls`](openspec/changes/archive/2026-11-08-h06-wire-global-exciter-controls/proposal.md).
> - **Pressure** trims every exciter's force by −12 to +12 dB (neutral at 0.5) inside `Voice::force_scale`, so contact predictions and the drive governor follow it. It is live on held notes, smoothed over ~10 ms.
> - **Speed** (×0.5 to ×2, neutral 0.5) and **Roughness** (×0 to ×3.33, neutral 0.3) scale each exciter's motion-speed and grit/randomness parameters at note-on, clamped to their host ranges. Exciters with no such parameter are documented as inert.
> - **Defaults are bit-identical:** `default_global_controls_are_bit_identical` covers all 16 exciters, and the 48 kHz render hashes are unchanged. No factory preset changes.
> - **Editor:** Pressure, Speed and Roughness knobs sit under the exciter selector, and `HOST_ONLY_PARAMS` is now empty. The values display as dB or ×factor.
> - **Overdrive:** `maximum_global_controls_do_not_overdrive_the_voice` plays every exciter × object at C2/C4/C6, velocity 127, with all three at 1, and stays under 1.0 pre-clamp.

`exciter_pressure`, `exciter_speed`, `exciter_roughness` (params.rs:258-272) are
captured into `VoiceControls` (lib.rs:485-487) and then **never read** — no exciter
consumes them (each uses its dedicated params instead). They're automatable,
documented ("Overall intensity of excitation"), shown in presets, and do nothing.

### ~~H7. The two limiter modes target different ceilings~~ — FIXED
> **Status: FIXED** (2026-11-07, branch `claude/zen-dirac-vq38u4`): OpenSpec change [`h07-hard-limiter-honors-analog-ceiling`](openspec/changes/archive/2026-11-08-h07-hard-limiter-honors-analog-ceiling/proposal.md).
> - **One ceiling:** `apply_output_limiter(sample, ceiling)` takes the ceiling, and `process` passes the same per-buffer Analog Ceiling the Lookahead limiter uses. Both modes now stop at it after output gain.
> - **Tests:** `hard_mode_stays_under_a_low_ceiling` (ceiling 0.5, gain 4) and `both_limiter_modes_reach_the_same_ceiling` (ceiling 0.7: both peak in 0.65–0.7) run the real output stage. Both failed before the fix.
> - **Defaults unchanged:** every factory preset stores 0.9661, and `factory_preset_is_bit_identical_at_the_default_ceiling` checks the output is bit-identical to the old fixed clamp. `LIMITER_THRESHOLD` stays as the default ceiling for offline tools.

lib.rs:884-905 sets the lookahead limiter threshold to `analog_ceiling`
("Threshold tracks the analog ceiling so both limiter modes target the same
effective peak") — but Hard mode uses `apply_output_limiter`, a fixed clamp at
`LIMITER_THRESHOLD = 0.9661` that ignores `analog_ceiling`. With
`analog_ceiling = 0.5`: Lookahead caps at 0.5, Hard at 0.9661. Also note the
clipper already clips at `analog_ceiling` *before* output gain — so output gain
> 1 can push the clipped signal back above the ceiling into the final limiter,
making the "analog ceiling" a soft suggestion in Hard mode.

### ~~H8. `get_state`/`load_state` are dead and dangerous~~ — FIXED
> **Status: FIXED** (2026-11-07, branch `claude/zen-dirac-vq38u4`): OpenSpec change [`h08-remove-dead-state-functions`](openspec/changes/archive/2026-11-08-h08-remove-dead-state-functions/proposal.md). Both methods are deleted; nothing called them. Host state save and restore still go through nih-plug's `Params` derive, and the preset schema migration stays in `Preset::from_json_str`. The only remaining `get_state` is nih-plug's `GuiContext` trait method on the editor tests' `NullGui`.

lib.rs:761-783. Nothing calls them (nih-plug persists params via the `Params`
derive). If anything ever did call `load_state`, the `self.params = Arc::new(...)`
swap would disconnect the host and GUI (which hold the original Arc from
`params()`) from the audio thread's parameter set. Delete or rewrite via setters.

### ~~H9. `position_envelope` doesn't do what it says~~ — FIXED
> **Status: FIXED** (2026-11-07, branch `claude/zen-dirac-vq38u4`): OpenSpec change [`h09-route-envelope-to-strike-position`](openspec/changes/archive/2026-11-08-h09-route-envelope-to-strike-position/proposal.md).
> - **Routing:** the effective strike position is `strike_position + position_envelope · envelope · 0.5` (plus wander), clamped to 0..1, every sample. The voice sets the offset from its force envelope before the resonator prepares the sample (`force_envelope_moves_the_strike_position`: 0.2 → 0.7 on a Bow swell; `position_envelope_is_audible_without_wander`).
> - **Wander Rate:** a new `position_wander_rate` parameter (0.1–5 Hz, default 0.1, skewed) sets the wander's rate, independent of Position Envelope (`wander_period_follows_wander_rate`). The editor has a Wander Rate fader beside Wander.
> - **Presets:** a preset without the field loads `0.1 + position_envelope · 4.9` Hz and Position Envelope 0, so it sounds as before (`preset_without_wander_rate_keeps_its_wander`). The 50 factory presets only gained the field at its default.
> - **Neutral at 0:** at Position Envelope 0 the voice never touches the offset; 32 renders (16 exciters, with and without wander) are bit-identical to before, and the 16-voice benchmarks are within noise (numbers in the change's design).
> - **Saved DAW projects:** `Plugin::filter_state` applies the same migration to host state that predates Wander Rate, since the host restores parameters by ID and never goes through the preset loader (`old_host_state_keeps_its_wander`).
> - **BREAKING (host automation):** a session that automated Position Envelope to change the wander rate now moves the strike position instead. Automation can't be migrated.

Documented as "How much envelope affects strike position" (params.rs:527-530).
Actually (`core.rs:644-649`) it sets the **rate of the wander LFO**
(`0.1 + position_envelope·4.9` Hz) — there is no envelope→position routing at all.
With `position_wander = 0`, `position_envelope` does nothing.

### ~~H10. Sample-rate dependence throughout the space section~~ — FIXED
> **Status: FIXED** (2026-11-07, branch `claude/zen-dirac-vq38u4`): OpenSpec change [`h10-scale-space-effects-with-sample-rate`](openspec/changes/archive/2026-11-08-h10-scale-space-effects-with-sample-rate/proposal.md).
> - **Time-defined delays:** the Factory combs and allpasses, the spring tank, the echo and the HRTF delay are tuned at 48 kHz and scaled by `fs / 48 000`, so 48 kHz delays are the tuned values before any clamp.
> - **Buffers for 192 kHz,** allocated in `new()` (about 1.8 MB per post chain), so no knob position clamps: the combs stay distinct at every size, Delay Time reaches 0.8 s at every rate, and the spring and HRTF keep their timing.
> - **Tests:** Factory T30 is 0.616–0.617 s at 44.1/48/96/192 kHz (was 0.610/0.560/0.280/0.140 s); the 0.8 s echo arrives within a sample at every rate; the 96 kHz Delay Time sweep rises from 0.02 to 0.8 s; the spring's tank-1 echo is 0.120 s at 48 and 192 kHz; HRTF's 1 ms holds at 192 kHz.
> - **Presets:** the five Factory-reverb presets lose the metallic collapsed-comb tail and stay within the bank's level rules with no trim. Hanging Chain Echo's echo comes one sample later (rounded instead of truncated). Numbers in the change's `levels.md`.

- `FactoryReverb` comb delays are in raw samples (1553–1913 base, clamp 100–2000)
  with **no sample-rate scaling** despite `set_sample_rate` claiming to recompute
  them (space.rs:64-78). Room decay/size halves at 96 kHz.
- The 2048-sample comb buffers can't hold the intended scaling: at `size ≥ ~0.3`,
  three of the four combs rail at the 2000 clamp → **identical delays → strongly
  periodic, metallic reverb** for most of the size range.
- `FactoryEcho` buffers are 16384 samples: 0.8 s max echo at 48 k, but only 170 ms
  at 96 k — the top ~80 % of the Delay Time range silently clamps at 96 kHz.
- `HrtfSpread` delay buffer is 64 samples but "1 ms max" needs 96 at 96 kHz
  (silently clamped via `.min(63)`).

### ~~H11. Modal banks generate modes above Nyquist and with negative decay formulas~~ — FIXED
> **Status: FIXED** (2026-11-07, branch `claude/zen-dirac-vq38u4`): OpenSpec change [`h11-prune-unsoundable-modes`](openspec/changes/archive/2026-11-08-h11-prune-unsoundable-modes/proposal.md).
> - **Aliasing half:** already fixed by `resolve-exciter-tradeoffs`, whose top-of-band fade silences every mode at or above `min(20 kHz, 0.45·fs)`, so out-of-band modes no longer alias.
> - **Wasted modes:** note-on now drops every mode at or above 80 kHz (the band top at the lowest reachable pitch) or with a decay under 0.1 ms (the rescued negative-decay modes), so they no longer cost a mode update every sample or feed the one-sample direct path. Kept modes keep their brightness weights, and a bent note keeps its whole bank.
> - **Tests:** the small high Pipe at Render, a sweep over every object, quality and damage, a 24-semitone retune, and a pruned-vs-full level comparison (largest change +0.003 dB). The golden fixture changes only for the three Pipe Render cases that lose modes. Render is no slower; at normal sizes only Pipe and SheetMetal lose modes, so the CPU saving is modest outside small sizes.

`src/dsp/resonators/basic.rs`, `sheet_metal.rs`, etc.:
- No frequency clamp anywhere: Pipe (`n·f1·√(1+Bn²)`), Plate (`k·(m²+n²)`),
  SheetMetal (`base·n²`) routinely produce modes far above fs/2 for normal notes.
  `ResonatorCoefficients::for_mode` happily builds `cos(ω)` with ω > π, so these
  modes **alias to arbitrary reflected frequencies** — audible inharmonic garbage
  that changes with sample rate. High notes + small Size (which further divides
  by `size_scale`, i.e. multiplies pitch by up to 20×) make it much worse.
- Decay formulas go negative: Pipe `1.5 − 0.15n` (< 0 for n ≥ 10), Plate
  `0.4 − 0.02(m+n)`. They're rescued by `.max(f32::EPSILON)` into effectively dead
  modes — silent second-order sections that still burn CPU every sample.

### ~~H12. Per-sample coefficient rebuild for dynamic objects and bent notes~~ — FIXED
> **Status: FIXED** (2026-11-08, branch `claude/zen-dirac-vq38u4`): OpenSpec change [`h12-skip-redundant-mode-retunes`](openspec/changes/archive/2026-11-08-h12-skip-redundant-mode-retunes/proposal.md).
> - **Skip:** the bank remembers the pitch factor its coefficients were built for and skips the rebuild while the factor stays within 0.05 cents. A held bend is free after its first sample, and bit-identical to rebuilding every sample.
> - **Incremental rotation:** a moving factor (TautCable tension drop, SheetMetal warp, a bend in motion) rotates each mode's cached sine and cosine with a Taylor step, with no transcendental call or division, refreshing exactly every 256 steps. Every mode's frequency and pole angle stay within 0.05 cents of the target at every sample.
> - **Cost (median of 5 runs, `scripts/bench_ratios.py`):** 16 bent Render voices cost about 1.06× unbent (was about 5×); 16 TautCable voices with tension drop cost about 1.56× their static twin (bound 1.6×). The static twin itself went from 43 ms to about 11 ms.
> - **Also fixed:** `SecondOrderMode::ensure_rate`'s rebuild branch is out of line. Inlined, it made plain notes swing by up to 30 % with bit-identical output whenever the coefficient code changed size.

`core.rs:658-672`: when dynamics are active (TautCable, SheetMetal) **or any
non-unity pitch bend is held**, `apply_dynamics` calls `set_frequency` on *every
mode every sample*, each doing a `cos` + `exp` (`ResonatorCoefficients::for_mode`).
For a Render-quality bank (2× modes, ×8 voices) this is an enormous CPU cliff —
and pitch bend makes it apply to *all* objects, not just the two dynamic ones.
Should be rate-limited (per block) or use incremental rotation.

### ~~H13. Shipped preset renders bypass the post chain~~ — FIXED
> **Status: FIXED** (2026-11-08, branch `claude/nice-mccarthy-9fm1p6`): OpenSpec change [`h13-render-presets-through-post-chain`](openspec/changes/archive/2026-11-08-h13-render-presets-through-post-chain/proposal.md).
> - **One output path:** `process()`'s per-buffer code is `Corrosion::begin_block` + `Corrosion::process_block` (voices → master drive → post chain → output gain → limiter). `process()` calls it with the host's buffer; `offline::render_preset` calls it with a real plugin instance holding the preset, in 512-sample blocks.
> - **Tools and tests use it:** `render_presets` writes stereo WAVs through it (the private drive/body/limiter path is gone), and the preset level checks measure it (`preset_levels_are_sane` and `preset_levels_table` print the same numbers as before).
> - **Proof:** `offline_render_equals_plugin_path_for_a_post_chain_preset` (Space + master drive + BODY + Lorenz drive) and `offline_render_equals_plugin_path_with_the_lookahead_limiter` (also checks the limiter's delay) are bit-identical against a host-style `process_block` driver; `written_wav_equals_the_quantized_offline_render` checks the WAV writer. `output/*.wav` is re-rendered.
> - **Also fixed:** `reset()` started the master-drive smoother at Drive without the Violence bias, so the first ~60 ms after a host reset ramped the drive (`reset_starts_the_drive_smoother_at_the_biased_drive`).

`src/bin/render_presets.rs` renders `VoiceManager` + a standalone body resonator
only — no `PostProcessingChain`, no master drive, no limiter. The `output/*.wav`
audition set therefore does not represent what the plugin outputs (and is exactly
why C1 — now FIXED — was never heard). The renderer's own comment says it exists so presets are
auditioned "as the plugin would sound".

### ~~H14. Kit-mode notes ignore per-slot pitch documentation and wrap oddly~~ — FIXED
> **Status: FIXED** (2026-11-08, branch `claude/nice-mccarthy-9fm1p6`): OpenSpec change [`h14-wrap-low-kit-notes`](openspec/changes/archive/2026-11-08-h14-wrap-low-kit-notes/proposal.md). `note_to_kit_object` is `(note − 24).div_euclid(10).rem_euclid(9)`: the 10-note slots repeat below MIDI 24 as they do above the block (14–23 → object 8, 4–13 → 7, 0–3 → 6), and notes from 24 up map as before. Proven by `kit_notes_inside_the_block_map_to_their_slot`, `kit_notes_above_the_block_wrap` and `kit_notes_below_the_block_wrap` (which failed before: MIDI 23 played Pipe). The doc comment and README describe the wrap. Per-slot pitch handling stays out of scope (design non-goal).

`note_to_kit_object` (lib.rs:94-97): `.max(0)` clamps everything below MIDI 24
into slot 0 (Pipe) rather than "wrapping" as the doc claims; wrapping only happens
above the block. Minor, but the doc and tests promise symmetric behavior.

---

## MEDIUM

- ~~**M1. `mseg_onset` doubles as time and level.** `voice/mod.rs:561-575` passes
  `controls.mseg_onset` (a 0.001–1.0 s *time*) to both `set_stage_times` *and*
  `set_stage_levels` (as the onset *level*, clamped 0..1). The VoiceControls doc
  even calls it "Onset stage *level*" while params.rs calls it "onset *time*". One
  knob silently controls two unrelated things.~~ — FIXED (2026-11-09, [`m01-m04-m05-clarify-parameters`](openspec/changes/archive/2026-11-10-m01-m04-m05-clarify-parameters/proposal.md)): new parameter Onset Level (`mseg_onset_level`, default 0.01) sets the onset stage's level, and Onset (now "Onset Time") only its length. The default keeps the old level, so all 16 exciters' default 48 kHz renders hash identically before and after. Older presets (`migrate_preset_json`) and host state (`filter_state`) get `clamp(mseg_onset, 0, 1)`. Proven by `onset_time_and_level_are_separate`, `default_onset_level_is_the_old_shared_level`, `old_host_state_keeps_its_onset_level` and `preset_without_onset_level_keeps_its_onset`.
- ~~**M2. Hit-family envelope ignores `env_decay`/`env_sustain`.** Hits use a pure
  AR one-shot (voice/mod.rs:540-547) — attack then immediate release, even while
  the key is held. The GUI shows full ADSR knobs for hit exciters; Decay/Sustain
  do nothing there (only Specialty uses them).~~ — RESOLVED (marked 2026-11-10 by [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)): the editor shows hit exciters a two-fader "Force Envelope (AD)" panel (Attack, Release) instead of ADSR knobs, so no control does nothing; the AR one-shot is the hit family's design.
- ~~**M3. Hard-coded 48 kHz at note-on.**
  `with_algorithm_controls_and_note` → `set_live_resonator_controls(damping,
  brightness, 48_000)` (core.rs:569-571). Coefficients are wrong for one call at
  other rates (self-heals on first `process` via the cached-rate check, but the
  live-controls path also caches `Some(48_000)`, masking the first rebuild).
  Similarly `Voice::set_pitch_bend_semitones` uses `self.sample_rate`, which is
  48 kHz until the first sample renders.~~ — FIXED (2026-11-08, branch `claude/nice-mccarthy-9fm1p6`, [`m03-m20-start-at-host-rate-and-report-latency`](openspec/changes/archive/2026-11-09-m03-m20-start-at-host-rate-and-report-latency/proposal.md)): voices are rated for the host rate at initialization and on rate changes, and a note-on builds its coefficients once at that rate (96 kHz: 624 → 416 builds for 16 notes, as at 48 kHz). `first_note_does_not_depend_on_voice_history` also found four exciters carrying state between notes; each now restarts from rest on trigger.
- ~~**M4. Duplicate parameter display names.** Two params named "Wire Stiffness"
  (`brush_wire_stiffness`, `wire_stiffness`), two named "Brightness"
  (`res_brightness`, `macro_brightness`), plus "Mass" (macro) vs the many masses.
  IDs are unique so state is safe, but host automation lists show
  indistinguishable duplicates.~~ — FIXED (2026-11-09, [`m01-m04-m05-clarify-parameters`](openspec/changes/archive/2026-11-10-m01-m04-m05-clarify-parameters/proposal.md)): "Resonator Brightness" / "Macro Brightness" and "Brush Wire Stiffness" / "Spring Wire Stiffness", also in `into_params`. "Mass" was already unique (the other masses are "Pipe Mass", "Hand Mass", …), so it stays. Proven by `parameter_names_are_unique` and `into_params_uses_the_live_names_and_output_range`.
- ~~**M5. `output` param is linear-in-gain 0..100 (0 → −∞, 100 → +40 dB), default
  1.0.** No dB display/units, and the linear taper means the useful −20..+6 dB
  region occupies ~1 % of the knob travel. Also +40 dB of clean digital gain into
  a clipper is an odd ceiling.~~ — FIXED (2026-11-09, [`m01-m04-m05-clarify-parameters`](openspec/changes/archive/2026-11-10-m01-m04-m05-clarify-parameters/proposal.md)): Output is shown and typed in dB with a gain-skewed taper; 0 dB moved from 1.0 % to 57.5 % of the travel and −20..+6 dB from 1.9 % to 18.9 %. The stored value is still the gain, so presets and projects keep their level; the +40 dB maximum stays (maintainer's choice). BREAKING for existing Output automation. Proven by `output_is_shown_in_decibels`, `output_accepts_typed_decibels`, `output_taper_reaches_the_useful_range` and `old_project_keeps_its_output_gain`.
- ~~**M6. `LorenzDrive` bias-starvation gate is a hard 1.0↔0.3 gain switch**
  (lorenz_drive.rs:98-107) keyed off the chaotic z state — produces stepped gain
  discontinuities (clicks/crackle), which may be intended "circuit starvation" but
  is undocumented and un-smoothed. `tube_state` field is dead.~~ — FIXED (2026-11-09, [`m06-m09-smooth-starvation-and-oversample-clipper`](openspec/changes/archive/2026-11-09-m06-m09-smooth-starvation-and-oversample-clipper/proposal.md)): a 5 ms one-pole follower ramps the gain, so the largest output step is 0.99–1.005× the starvation-0 step at 44.1, 48 and 96 kHz (it was up to 28× at 48 kHz and 57× at 96 kHz); starvation 0 is bit-identical to the drive without the stage; `tube_state` is gone. Measuring it also showed the gate only switched for Bias Starvation of about 0.2–0.7 (a fixed −10.5 dB cut above 0.71); the threshold now spans the attractor's `z` range, starved 5 % of the time at 0.1 to 87 % at 1 (maintainer's choice). Proven by `starvation_does_not_click`, `starvation_does_not_click_at_other_rates`, `starvation_off_is_bit_identical`, `starvation_keeps_switching_across_its_range` and `more_starvation_starves_more_often` in `lorenz_drive.rs`.
- ~~**M7. `FactoryEcho` modulated delay is un-interpolated** (space.rs:273) —
  integer-truncated read index with a moving modulation produces zipper noise as
  `machinery_movement` increases. `high_frequency_damping` is a broadband wet gain,
  not a filter — the name promises air absorption; it just turns the echo down.~~ — FIXED (2026-11-09, branch `claude/nice-mccarthy-9fm1p6`, [`m07-m08-interpolate-echo-and-tidy-spread`](openspec/changes/archive/2026-11-09-m07-m08-interpolate-echo-and-tidy-spread/proposal.md)): modulated reads use Hermite interpolation (largest step of a swept 1 kHz sine 0.242 → 0.077, vs 0.066 for the dry sine), and High Frequency Damping is a low-pass in the feedback loop (at 1, 8 kHz repeats 14 dB below 200 Hz; 200 Hz level unchanged within 0.1 dB), proven by `echo_modulation_glides` and the HF Damping tests in `space.rs`.
- ~~**M8. `HrtfSpread` ITD is dead code.** Both delay buffers store the same mono
  signal and are read at the same index (hrtf_spread.rs:66-73): `delayed_left ==
  delayed_right` always, so there is no interaural time difference; width comes
  only from the side-signal gain and crossfeed. Two 64-float buffers where one
  would do; the "1 ms ITD" comment is not implemented.~~ — FIXED (2026-11-09, same change): the redundant buffer is gone and the comments describe a common mid delay, not an ITD. The output is bit-identical (render hashes at 44.1, 48 and 192 kHz match the previous code). Keeping the sound, rather than implementing a real ITD, was the maintainer's choice.
- ~~**M9. `OversampledClipper` isn't oversampling in the DSP sense.** It linearly
  interpolates between two consecutive *input* samples (adds no bandwidth) and
  box-filter decimates (adds ~half-sample delay and HF droop). It's a "transient
  smoother", not the true-peak/anti-aliasing stage the docs describe. Its
  `set_sample_rate` stores a value that is never used.~~ — FIXED (2026-11-09, same change): the clipper up-samples through cascaded 2× half-band IIR stages, clips at the high rate and comes back down (Normal/High/Render use 2/3/4 stages); `set_sample_rate` is gone. Flat to 20 kHz (0.000 dB; the box filter lost 8 dB at 18 kHz in Render); a 5 kHz sine at twice the ceiling folds back at −51.0 dB in Normal and −75.8 dB in Render (−32.4 and −33.0 before), also through the default hard clamp, because the oversampled curve starts at the ceiling ÷ 1.3 (maintainer's choice). The half-bands delay 1 kHz by 4.2–5.0 samples, not reported (maintainer's choice). Proven by `clipper_is_transparent_below_its_threshold`, `clipper_alias_bounds_hold`, `clipper_alias_bounds_hold_through_the_hard_clamp`, `clipper_fold_back_falls_with_quality`, `clipper_delays_less_than_eight_samples` (`offline/mod.rs`), `eco_is_plain_clipping` and the half-band stage test in `half_band.rs`.
- ~~**M10. `InteractionState.exciter_force` is write-only** (`set_exciter_force`
  is called each sample; nothing reads the field). The "bidirectional" bus is
  feed-forward: exciters read displacement/velocity, but the force distribution
  path is vestigial.~~ — FIXED (2026-11-10, [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)): `exciter_force` and `set_exciter_force` are removed. The bus is bidirectional where it matters: the resonator's contact displacement and velocity are read back by the exciter every sample (`Voice::process_exciter`), so the docs keep saying bidirectional; the force goes straight into the modes. Render hashes unchanged.
- ~~**M11. `Voice::is_active()` doc contradicts behavior** — "voice can remain
  audible briefly after note_off" but `note_off()` sets `active = false`
  immediately; tails are tracked by `rendering`. The tail-deactivation branch
  redundantly re-assigns `active = false`.~~ — FIXED (doc earlier; the redundant `active = false` in both tail branches removed 2026-11-10, [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)). Render hashes unchanged.
- ~~**M12. `VoiceManager::note_off` releases only the first matching voice.** Two
  note-ons of the same note (host retrigger without note-off, or after a steal)
  leave the second instance sounding until its envelope dies naturally. `voice_id`
  and `channel` are ignored throughout (no MPE per-note pitch bend — only channel
  bend; poly *pressure* is supported, poly *tuning* is not despite comments
  mentioning it).~~ — FIXED (2026-11-10, [`m12-per-note-expression`](openspec/changes/m12-per-note-expression/proposal.md)): voices store the host's voice ID and channel; note-off, poly pressure, `PolyTuning` (per-note semitones on top of the channel bend) and `Choke` (2 ms fade, then idle) address a voice by ID, else by key and channel, with the oldest-first rule for key-only note-offs. Proven by `note_expression_tests` (10 tests: tuning and chokes addressed by ID alone are told apart from the key fallback, and a reused ID reaches the held note, not the old tail) and `tests/no_alloc.rs::note_expression_is_alloc_free`; plain-MIDI renders are bit-identical for all 16 exciters. Channel MPE (per-channel bend and pressure zones) stays out of scope.
- ~~**M13. No sustain pedal (CC64), no all-notes-off (CC123) handling** — CC1 is the
  only CC routed (lib.rs:636-638). For a physical-modeling instrument, missing
  sustain-pedal handling is a real playability gap.~~ — FIXED (2026-11-09, branch `claude/nice-mccarthy-9fm1p6`, [`m13-sustain-pedal-and-panic`](openspec/changes/archive/2026-11-09-m13-sustain-pedal-and-panic/proposal.md)): CC64 sustain with a pedal-held stealing tier, CC123 All Notes Off, CC120 All Sound Off (5 ms fade, then voices and tails cleared) and CC121 Reset All Controllers, proven by `midi_control_tests` (11 tests, including an allocation check).
- ~~**M14. Quality mode affects voices only at note-on** (`mode_count_scale`
  captured into the bank), but the post chain switches instantly — flipping
  Eco↔Render mid-note changes the space topology immediately while held notes
  keep their old modal density. Mixed semantics, worth documenting or unifying.~~ — DOCUMENTED (2026-11-10, [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)): the README, `docs/ARCHITECTURE.md` (Quality modes) and the parameter's doc say the clipper's oversampling changes at once while a voice's mode count is fixed at note-on. Rebuilding held notes on a mode change is left as a possible later change.
- ~~**M15. Preset `apply_to`/`into_params` rebuild params without their display
  metadata** — e.g. `ui_scale` loses its "50 %…150 %" strings after
  `Preset::into_params` (presets/mod.rs:555-557 uses a bare `int_param`), and
  `filter_cutoff` loses its formatter. Only affects the (dead) `load_state` path
  and `sanitized()` round-trips, but it's an asymmetry waiting to bite.~~ — FIXED (2026-11-10, [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)): every parameter has one constructor in `params.rs` (`<id>_param(value)`, 136 generated from the old inline ones plus the existing shared builders), used by both `CorrosionParams::default()` and `Preset::into_params`; the bare helpers are gone. `preset_built_parameters_display_like_the_live_ones` checks name, normalization and display text for every parameter of all 50 factory presets (76 parameters differed before), and `ui_scale_and_filter_cutoff_display_like_the_live_ones` the two named here.
- ~~**M16. `FactoryReverb.diffusion_y` is dead state**; allpass delays are
  hard-coded 200/300 samples (also not sample-rate scaled).~~ — FIXED (H10:
  `diffusion_y` removed; the allpass delays are sample-rate scaled).
- ~~**M17. `rattle_noise` "noise"** (`voice/mod.rs:1130-1148`) is
  `sin(φ)·43758.547` fract with φ stepping by the golden ratio — a deterministic
  chirpy pattern, identical in both stereo channels, gated by a threshold that
  produces on/off discontinuities at the boundary. Fine as a texture, but it's
  neither highpassed (despite `highpass_noise` naming — it's just `noise·0.5`)
  nor decorrelated.~~ — PARTLY FIXED: the noise is real white noise and the gate has no step (earlier changes); on 2026-11-10 ([`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)) `highpass_noise` was renamed to what it is (`half_noise`) and the shared, mono rattle documented. **Not done:** high-passing or decorrelating it, which would change the sound.
- ~~**M18. `ExciterType::from_int(0)` maps to HandStrike** while the param's range
  is 1–16; docs across lib.rs/params.rs repeatedly say "0-16" and "17 exciters".
  Off-by-one documentation everywhere; the `0` fallback silently masks bad preset
  data instead of rejecting it.~~ — FIXED (2026-11-10, [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)): docs and comments say 1–16; `ExciterType::from_int` documents that any other value gives Hand Strike, the default, so one bad field doesn't reject a whole user preset (maintainer's design choice), pinned by `out_of_range_exciter_loads_as_hand_strike` and `every_valid_exciter_loads_and_saves_as_itself` (`tests/preset_roundtrip.rs`). The README and ARCHITECTURE also said 8 voices; corrected to 16.
- ~~**M19. `sync_rate_to_delay_time` quantizes divisions with `(normalized·6) as
  usize`** — the six bands are uneven at the edges (band boundaries at 0.05 +
  k·0.158…), and there's no triplet/dotted support; fine, but the param tooltip
  doesn't tell the user where the boundaries are (and there's no UI at all, H5).~~ — RESOLVED (marked 2026-11-10 by [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)): `params::sync_division` splits the range above Off into six equal bands, used by both the echo and the display (`h03-default-tempo-sync-off`), and the editor's Sync fader shows the division it plays (H5). Triplet and dotted divisions remain unsupported, by design.
- ~~**M20. Latency is only reported when limiter mode changes** (lib.rs:897-905) —
  correct, but the initial `reported_latency_samples = 0` means a session loaded
  with Lookahead already active reports latency only on the first `process()`
  call, after the host has typically finished PDC negotiation. Report it in
  `initialize()` too.~~ — FIXED (2026-11-08, same change): `initialize` reports the restored Limiter Mode's latency (`lookahead_latency_is_reported_at_initialization`).

---

## Tests & tooling observations

- The test suite is heavy on "A differs from B" energy assertions and light on
  invariants: ~~nothing pins the filter's frequency response at high cutoffs (C1)~~ (FIXED),
  ~~envelope continuity (C4)~~ (FIXED), DC at the output, sample-rate invariance (H10), or
  Nyquist safety (H11).
- ~~`tests/no_alloc.rs` scopes note-on *outside* the no-alloc zone, so the strongest
  RT-safety claim in the docs is untested where it actually breaks (C3).~~ — FIXED (note: the check only runs in debug builds)
- ~~`dsp::mod` re-exports `crate::offline::*` specs (mod.rs:43-46) — an inverted
  dependency (DSP core depending on the offline module) that exists only for
  convenience re-export; it drags offline render specs into every DSP consumer.~~ — FIXED (2026-11-10, [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)): the re-export is gone; `src/dsp/tests.rs` imports the specs from `crate::offline`.
- ~~`benches/performance.rs` exists but nothing benchmarks the dynamic-object /
  pitch-bend per-sample rebuild path (H12), which is the actual worst case.~~ — FIXED
  (H12: `voice_manager_sixteen_voices_bent` and `_cable_dynamics` / `_cable_static`,
  compared with their twins by `scripts/bench_ratios.py`).
- ~~`docs/CODE-REVIEW.md` should be updated or archived — two of its headline
  findings are fixed, and readers may assume the rest of the file is current.~~ — FIXED (2026-11-10, [`m10-m18-tidy-dead-code-and-docs`](openspec/changes/m10-m18-tidy-dead-code-and-docs/proposal.md)): moved to `docs/archive/CODE-REVIEW-2026-05-27.md` with a superseded header; the README points at this review.

## Missing features (code-evident, not backlog speculation)

1. ~~User preset save/load — Save button permanently disabled; no file dialog, no
   user bank directory (GUI loads only the embedded `include_dir` factory bank).~~ — FIXED (H5)
2. ~~Randomizer — fully implemented module, zero integration (no UI, no param).~~ — FIXED (H5)
3. MSEG PingPong loop — placeholder (C2/H10 above). *(Not covered by the C2 fix; PingPong remains open.)*
4. ~~Sustain pedal / CC64, all-sound-off~~ (FIXED, M13); ~~MPE per-note bend~~ (FIXED, M12: per-note tuning by voice ID; channel MPE out of scope).
5. ~~GUI exposure of brightness, thickness, sync rate, play mode, limiter mode,
   macros (H5).~~ — FIXED
6. ~~Envelope→strike-position routing promised by `position_envelope` (H9).~~ — FIXED
7. ~~Post-chain wet/dry or bypass for the body stage; true stereo processing of
   filter/drive (C5).~~ — FIXED

## Suggested priority order

1. ~~**C1** (filter instability — plugin is broken at default settings in a DAW)~~ — FIXED,
2. ~~**C2** (crash)~~ — FIXED, ~~**C3** (RT allocation; debug abort)~~ — FIXED,
3. ~~**C4/C5/C6** (audible correctness of envelope, chain topology, spring reverb)~~ — FIXED,
4. ~~**H1–H4**~~ (FIXED: parameter ranges, defaults, and preset loading),
5. ~~**H5**~~ (FIXED: GUI gaps) and the remaining H items,
6. Medium items opportunistically, alongside regression tests for each fix.

## Later finding: voice overdrive and clipping

Found during the voice-stealing work (`fix-voice-stealing-16-voices`):
- A bowed Tank's raw resonator output reached about 280× full scale. The voice's hard clamp hid this as clipping, which lasted about 18 s after release.
- Loudness also varied 34 dB across objects and +6 dB per octave down.

> **Status: FIXED** (2026-10-06, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`normalize-object-loudness`](openspec/changes/normalize-object-loudness/proposal.md). What changed:
> - Mode banks are normalized to one strike level at note-on.
> - Modes run in coupled form, which also fixed SheetMetal diverging under its warp.
> - Modes above 20 kHz aren't driven.
> - Each exciter has a drive trim.
>
> Every default-control note on every object, C2–C6 at full velocity, now stays under full scale before the clamp.


# Corrosion — Full Codebase Review (Signal Flow, Bugs, Inconsistencies, Missing Features)

**Date:** 2026-07-07
**Scope:** entire `src/` tree (plugin entry, params, voice engine, DSP core, resonators,
exciters, post-processing, presets, GUI, offline/render bins, randomizer), plus the
factory preset bank, test suite expectations, and build configuration.
**Method:** line-by-line read of the audio path; targeted reads of the GUI and offline
paths; numeric simulation used to verify the ladder-filter stability finding.
**Status (2026-10-05):** C1–C6 are FIXED (OpenSpec changes under `openspec/changes/`).
**Note:** `docs/CODE-REVIEW.md` (2026-05-27) is now partially stale — its Finding 1
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
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-ladder-filter-prewarp`](openspec/changes/fix-ladder-filter-prewarp/proposal.md). Prewarp is now `tan(π·fc/fs)` with `fc ≤ 0.49·fs`. The stages are also rebuilt as unity-gain TPT one-poles, because the old stage's 0.5 DC gain (−24 dB over four stages) left a default note at ≈ −42 dBFS once the rail was gone. Regression tests: `wdf_filter::tests::*`, `tests/post_chain_default_dc.rs`.
> Correction found while scoping: below `fs/4` the bug puts the effective cutoff about an octave *above* the displayed value, not half of it.

`src/dsp/post_processing/wdf_filter.rs:106-112, 144-159`

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
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-mseg-loop-recursion`](openspec/changes/fix-mseg-loop-recursion/proposal.md). `process_sample()` now resolves stage changes in a capped loop (no recursion), `loop_end_stage` is honored, and loop start ≥ end holds sustain. Output for loop Off and the default/Drone 3 → 4 loop is bit-identical to before. Two findings while fixing: optimized builds *hang* the audio thread here rather than overflowing the stack, and `t_decay = 0` with any loop hit the same recursion (API-only; the host's decay minimum is 10 ms). PingPong is still partial. Regression tests: `envelopes::tests::*`, `tests/mseg_loop_safety.rs`.

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
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`remove-note-on-audio-allocations`](openspec/changes/remove-note-on-audio-allocations/proposal.md). Each voice preallocates its worst-case bank (`MAX_MODES` = 96) and `rebuild_in_place` refills it at note-on. `reset()` clears voices and the post chain in place, and the mode banks are bit-identical to before (golden fixture). Two findings while fixing: `tests/no_alloc.rs` only checks in **debug** builds (release runs pass vacuously), and three exciters also allocated on the audio thread (MetalChain, WireBrush, ParticleRain), now preallocated. `tests/no_alloc.rs` now runs note-on, stealing, expression and reset inside the check.

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
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-mseg-velocity-scaling`](openspec/changes/fix-mseg-velocity-scaling/proposal.md). Velocity is now a pure gain, `level · ((1 − vtl) + vtl · velocity_scale)`, so zero level is silence and Release ends without a step (the click measured 0.175 at velocity 0.3). Full-velocity output matches the old formula within f32 rounding (≤ 1.5e-8). Correction found while scoping: full-velocity output (presets use `velocity_to_level = 1`) is already exactly `level`, so the floor and release-end click only occur below full velocity or with `velocity_to_level < 1`.

`src/dsp/envelopes/mod.rs:315-319`

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
- The Release→Idle transition (line 292-295) returns `l_end·env_amount = 0`
  directly, so the envelope **jumps discontinuously** from its floored value to 0 —
  an audible click at the end of every friction-voice release, and friction
  exciters keep being driven at ≥ 0.25 force during the entire release.

The intended math is presumably
`level · ((1−vtl) + vtl·velocity_scale)` — a multiplicative velocity scale that
preserves zeros. The unit test `velocity_affects_level` only checks that peaks
differ, so it passes despite the broken floor.

### ~~C5. Post chain: filter/drive are only audible through the FEM body bank, and the stereo image is 70 % mono-collapsed~~ — FIXED
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-post-chain-body-routing`](openspec/changes/fix-post-chain-body-routing/proposal.md). Filter and drive now run per channel. The body resonance is added in parallel at BODY / 5, Chassis Volume only tunes the body, and Eco runs the full chain. All 50 factory presets were re-voiced per preset (BODY, plus an `output` trim on 11) to within 0.5 dB of their old RMS, with no more limiter hits than before. The `body` default moved to 2.62 to keep the default patch's level. Found while fixing: the dry voice signal is very quiet on struck presets (a default note peaks at −51 dBFS with BODY 0), because the always-on body bank used to supply nearly all of their level. HRTF spread's own mono remix stays open.

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
> **Status: FIXED** (2026-10-05, branch `claude/eloquent-dirac-fc0sqm`) — OpenSpec change [`fix-spring-reverb-stereo`](openspec/changes/fix-spring-reverb-stereo/proposal.md). The post chain now runs one spring per channel, so each tail comes only from its own input and the first echo arrives one full delay length later (80 ms at the default tank size, previously 40 ms). A mono input still gives identical tails. `SpringReverb::reset()` also rewinds the delay line. The two Spring presets (Distant Plate Roll, Spring Reverb Tap) now echo at twice the old spacing, at the same level and with no limiter hits. Spring Reverb Tap's BODY was re-matched from 1.60 to 2.26 to hold that level (`openspec/changes/fix-spring-reverb-stereo/audition.md`).

`src/dsp/post_processing/post_chain.rs:175-182`, `space.rs:133-212`

`SpaceMode::Spring` calls `self.spring_reverb.process(spread_left)` and then
`.process(spread_right)` on the **same instance**. `SpringReverb` owns a single
delay line, write index, and dispersion state, so L and R samples are interleaved
into one buffer: the effective delay is halved, each channel's echo returns on the
opposite call, and the feedback path mixes channels sample-by-sample. Needs either
two instances or true stereo state (like `FactoryEcho` has).

---

## HIGH

### H1. `drive_amount` knob: 80 % of its range is dead, and its maximum is ~20 % wet
- Param range is 0–5 (`src/params.rs:1663-1668`), but `process()` clamps
  `(drive_amount + violence_bias)` to **0..1** (`src/lib.rs:915-919`). Knob values
  1.0–5.0 are indistinguishable.
- Inside `LorenzDrive::process` (`lorenz_drive.rs:112-116`) the wet mix is
  `drive_amount / 5.0`. With the host-side clamp at 1.0, the maximum achievable wet
  mix is **0.2**. Either the clamp or the `/5` normalization is wrong; together
  they make the post drive nearly inaudible.

### H2. `apply_drive` mix math assumes drive ∈ 0..1 but the param is 0..5
`src/lib.rs:412-414`: `shaped·drive + sample·(1 − drive·0.5)`.
At drive > 2 the dry coefficient goes **negative** (phase-inverted dry mixed with
5×-scaled wet, then clamped at ±1.5) — the top half of the master Drive knob is a
clamp-dominated, phase-smeared regime rather than "more saturation". The weights
also don't sum to 1 anywhere above drive ≈ 0.67, giving a built-in loudness jump.

### H3. Tempo sync is ON by default and silently disables the Delay Time knob
`sync_rate` defaults to **0.25** (`src/params.rs:1423-1427`) while the documented
"Off" region is `< 0.05` (lib.rs:108-138). In any host that reports tempo, Echo
mode defaults to a 1/8-note sync and the `delay_time` knob does nothing, with no UI
indication. Default should be 0.0 (Off). (The GUI doesn't even expose `sync_rate`
— see H5 — so a GUI user can't turn sync off at all.)

### H4. GUI preset loading drops v4 fields
`src/gui/editor.rs:1929-2085` (`apply_preset`) sets every parameter **except**
`play_mode`, `limiter_mode`, `macro_mass`, `macro_corrosion`, `macro_violence`,
`macro_brightness`. A factory/user preset that stores Kit/Drone mode, the lookahead
limiter, or non-neutral macros loads incompletely from the GUI — those six params
keep whatever values were previously live. (The host state path via nih-plug
persists them fine, which makes the divergence extra confusing.)

### H5. GUI is missing controls for shipped, audible parameters
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

### H6. Three host-facing exciter params are dead
`exciter_pressure`, `exciter_speed`, `exciter_roughness` (params.rs:258-272) are
captured into `VoiceControls` (lib.rs:485-487) and then **never read** — no exciter
consumes them (each uses its dedicated params instead). They're automatable,
documented ("Overall intensity of excitation"), shown in presets, and do nothing.

### H7. The two limiter modes target different ceilings
lib.rs:884-905 sets the lookahead limiter threshold to `analog_ceiling`
("Threshold tracks the analog ceiling so both limiter modes target the same
effective peak") — but Hard mode uses `apply_output_limiter`, a fixed clamp at
`LIMITER_THRESHOLD = 0.9661` that ignores `analog_ceiling`. With
`analog_ceiling = 0.5`: Lookahead caps at 0.5, Hard at 0.9661. Also note the
clipper already clips at `analog_ceiling` *before* output gain — so output gain
> 1 can push the clipped signal back above the ceiling into the final limiter,
making the "analog ceiling" a soft suggestion in Hard mode.

### H8. `get_state`/`load_state` are dead and dangerous
lib.rs:761-783. Nothing calls them (nih-plug persists params via the `Params`
derive). If anything ever did call `load_state`, the `self.params = Arc::new(...)`
swap would disconnect the host and GUI (which hold the original Arc from
`params()`) from the audio thread's parameter set. Delete or rewrite via setters.

### H9. `position_envelope` doesn't do what it says
Documented as "How much envelope affects strike position" (params.rs:527-530).
Actually (`core.rs:644-649`) it sets the **rate of the wander LFO**
(`0.1 + position_envelope·4.9` Hz) — there is no envelope→position routing at all.
With `position_wander = 0`, `position_envelope` does nothing.

### H10. Sample-rate dependence throughout the space section
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

### H11. Modal banks generate modes above Nyquist and with negative decay formulas
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

### H12. Per-sample coefficient rebuild for dynamic objects and bent notes
`core.rs:658-672`: when dynamics are active (TautCable, SheetMetal) **or any
non-unity pitch bend is held**, `apply_dynamics` calls `set_frequency` on *every
mode every sample*, each doing a `cos` + `exp` (`ResonatorCoefficients::for_mode`).
For a Render-quality bank (2× modes, ×8 voices) this is an enormous CPU cliff —
and pitch bend makes it apply to *all* objects, not just the two dynamic ones.
Should be rate-limited (per block) or use incremental rotation.

### H13. Shipped preset renders bypass the post chain
`src/bin/render_presets.rs` renders `VoiceManager` + a standalone body resonator
only — no `PostProcessingChain`, no master drive, no limiter. The `output/*.wav`
audition set therefore does not represent what the plugin outputs (and is exactly
why C1 — now FIXED — was never heard). The renderer's own comment says it exists so presets are
auditioned "as the plugin would sound".

### H14. Kit-mode notes ignore per-slot pitch documentation and wrap oddly
`note_to_kit_object` (lib.rs:94-97): `.max(0)` clamps everything below MIDI 24
into slot 0 (Pipe) rather than "wrapping" as the doc claims; wrapping only happens
above the block. Minor, but the doc and tests promise symmetric behavior.

---

## MEDIUM

- **M1. `mseg_onset` doubles as time and level.** `voice/mod.rs:561-575` passes
  `controls.mseg_onset` (a 0.001–1.0 s *time*) to both `set_stage_times` *and*
  `set_stage_levels` (as the onset *level*, clamped 0..1). The VoiceControls doc
  even calls it "Onset stage *level*" while params.rs calls it "onset *time*". One
  knob silently controls two unrelated things.
- **M2. Hit-family envelope ignores `env_decay`/`env_sustain`.** Hits use a pure
  AR one-shot (voice/mod.rs:540-547) — attack then immediate release, even while
  the key is held. The GUI shows full ADSR knobs for hit exciters; Decay/Sustain
  do nothing there (only Specialty uses them).
- **M3. Hard-coded 48 kHz at note-on.**
  `with_algorithm_controls_and_note` → `set_live_resonator_controls(damping,
  brightness, 48_000)` (core.rs:569-571). Coefficients are wrong for one call at
  other rates (self-heals on first `process` via the cached-rate check, but the
  live-controls path also caches `Some(48_000)`, masking the first rebuild).
  Similarly `Voice::set_pitch_bend_semitones` uses `self.sample_rate`, which is
  48 kHz until the first sample renders.
- **M4. Duplicate parameter display names.** Two params named "Wire Stiffness"
  (`brush_wire_stiffness`, `wire_stiffness`), two named "Brightness"
  (`res_brightness`, `macro_brightness`), plus "Mass" (macro) vs the many masses.
  IDs are unique so state is safe, but host automation lists show
  indistinguishable duplicates.
- **M5. `output` param is linear-in-gain 0..100 (0 → −∞, 100 → +40 dB), default
  1.0.** No dB display/units, and the linear taper means the useful −20..+6 dB
  region occupies ~1 % of the knob travel. Also +40 dB of clean digital gain into
  a clipper is an odd ceiling.
- **M6. `LorenzDrive` bias-starvation gate is a hard 1.0↔0.3 gain switch**
  (lorenz_drive.rs:98-107) keyed off the chaotic z state — produces stepped gain
  discontinuities (clicks/crackle), which may be intended "circuit starvation" but
  is undocumented and un-smoothed. `tube_state` field is dead.
- **M7. `FactoryEcho` modulated delay is un-interpolated** (space.rs:273) —
  integer-truncated read index with a moving modulation produces zipper noise as
  `machinery_movement` increases. `high_frequency_damping` is a broadband wet gain,
  not a filter — the name promises air absorption; it just turns the echo down.
- **M8. `HrtfSpread` ITD is dead code.** Both delay buffers store the same mono
  signal and are read at the same index (hrtf_spread.rs:66-73): `delayed_left ==
  delayed_right` always, so there is no interaural time difference; width comes
  only from the side-signal gain and crossfeed. Two 64-float buffers where one
  would do; the "1 ms ITD" comment is not implemented.
- **M9. `OversampledClipper` isn't oversampling in the DSP sense.** It linearly
  interpolates between two consecutive *input* samples (adds no bandwidth) and
  box-filter decimates (adds ~half-sample delay and HF droop). It's a "transient
  smoother", not the true-peak/anti-aliasing stage the docs describe. Its
  `set_sample_rate` stores a value that is never used.
- **M10. `InteractionState.exciter_force` is write-only** (`set_exciter_force`
  is called each sample; nothing reads the field). The "bidirectional" bus is
  feed-forward: exciters read displacement/velocity, but the force distribution
  path is vestigial.
- **M11. `Voice::is_active()` doc contradicts behavior** — "voice can remain
  audible briefly after note_off" but `note_off()` sets `active = false`
  immediately; tails are tracked by `rendering`. The tail-deactivation branch
  redundantly re-assigns `active = false`.
- **M12. `VoiceManager::note_off` releases only the first matching voice.** Two
  note-ons of the same note (host retrigger without note-off, or after a steal)
  leave the second instance sounding until its envelope dies naturally. `voice_id`
  and `channel` are ignored throughout (no MPE per-note pitch bend — only channel
  bend; poly *pressure* is supported, poly *tuning* is not despite comments
  mentioning it).
- **M13. No sustain pedal (CC64), no all-notes-off (CC123) handling** — CC1 is the
  only CC routed (lib.rs:636-638). For a physical-modeling instrument, missing
  sustain-pedal handling is a real playability gap.
- **M14. Quality mode affects voices only at note-on** (`mode_count_scale`
  captured into the bank), but the post chain switches instantly — flipping
  Eco↔Render mid-note changes the space topology immediately while held notes
  keep their old modal density. Mixed semantics, worth documenting or unifying.
- **M15. Preset `apply_to`/`into_params` rebuild params without their display
  metadata** — e.g. `ui_scale` loses its "50 %…150 %" strings after
  `Preset::into_params` (presets/mod.rs:555-557 uses a bare `int_param`), and
  `filter_cutoff` loses its formatter. Only affects the (dead) `load_state` path
  and `sanitized()` round-trips, but it's an asymmetry waiting to bite.
- **M16. `FactoryReverb.diffusion_y` is dead state**; allpass delays are
  hard-coded 200/300 samples (also not sample-rate scaled).
- **M17. `rattle_noise` "noise"** (`voice/mod.rs:1130-1148`) is
  `sin(φ)·43758.547` fract with φ stepping by the golden ratio — a deterministic
  chirpy pattern, identical in both stereo channels, gated by a threshold that
  produces on/off discontinuities at the boundary. Fine as a texture, but it's
  neither highpassed (despite `highpass_noise` naming — it's just `noise·0.5`)
  nor decorrelated.
- **M18. `ExciterType::from_int(0)` maps to HandStrike** while the param's range
  is 1–16; docs across lib.rs/params.rs repeatedly say "0-16" and "17 exciters".
  Off-by-one documentation everywhere; the `0` fallback silently masks bad preset
  data instead of rejecting it.
- **M19. `sync_rate_to_delay_time` quantizes divisions with `(normalized·6) as
  usize`** — the six bands are uneven at the edges (band boundaries at 0.05 +
  k·0.158…), and there's no triplet/dotted support; fine, but the param tooltip
  doesn't tell the user where the boundaries are (and there's no UI at all, H5).
- **M20. Latency is only reported when limiter mode changes** (lib.rs:897-905) —
  correct, but the initial `reported_latency_samples = 0` means a session loaded
  with Lookahead already active reports latency only on the first `process()`
  call, after the host has typically finished PDC negotiation. Report it in
  `initialize()` too.

---

## Tests & tooling observations

- The test suite is heavy on "A differs from B" energy assertions and light on
  invariants: ~~nothing pins the filter's frequency response at high cutoffs (C1)~~ (FIXED),
  ~~envelope continuity (C4)~~ (FIXED), DC at the output, sample-rate invariance (H10), or
  Nyquist safety (H11).
- ~~`tests/no_alloc.rs` scopes note-on *outside* the no-alloc zone, so the strongest
  RT-safety claim in the docs is untested where it actually breaks (C3).~~ — FIXED (note: the check only runs in debug builds)
- `dsp::mod` re-exports `crate::offline::*` specs (mod.rs:43-46) — an inverted
  dependency (DSP core depending on the offline module) that exists only for
  convenience re-export; it drags offline render specs into every DSP consumer.
- `benches/performance.rs` exists but nothing benchmarks the dynamic-object /
  pitch-bend per-sample rebuild path (H12), which is the actual worst case.
- `docs/CODE-REVIEW.md` should be updated or archived — two of its headline
  findings are fixed, and readers may assume the rest of the file is current.

## Missing features (code-evident, not backlog speculation)

1. User preset save/load — Save button permanently disabled; no file dialog, no
   user bank directory (GUI loads only the embedded `include_dir` factory bank).
2. Randomizer — fully implemented module, zero integration (no UI, no param).
3. MSEG PingPong loop — placeholder (C2/H10 above). *(Not covered by the C2 fix; PingPong remains open.)*
4. Sustain pedal / CC64, all-sound-off, MPE per-note bend (M12/M13).
5. GUI exposure of brightness, thickness, sync rate, play mode, limiter mode,
   macros (H5).
6. Envelope→strike-position routing promised by `position_envelope` (H9).
7. ~~Post-chain wet/dry or bypass for the body stage; true stereo processing of
   filter/drive (C5).~~ — FIXED

## Suggested priority order

1. ~~**C1** (filter instability — plugin is broken at default settings in a DAW)~~ — FIXED,
2. ~~**C2** (crash)~~ — FIXED, ~~**C3** (RT allocation; debug abort)~~ — FIXED,
3. ~~**C4/C5/C6** (audible correctness of envelope, chain topology, spring reverb)~~ — FIXED,
4. **H1–H4** (parameter ranges, defaults, and preset loading — cheap, high-impact),
5. **H5** (GUI gaps) and the remaining H items,
6. Medium items opportunistically, alongside regression tests for each fix.

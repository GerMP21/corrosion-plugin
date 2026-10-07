# Corrosion Architecture (current state)

This document describes the **as-built** structure, signal flow, and engineering
contracts of the Corrosion industrial physical-modeling synthesizer. It is
written against the code in `src/` as of 2026-05 and is intended to be accurate
to what actually ships, not to the aspirational DSP described in
`docs/detailed-specs/`. Where the implementation is a deliberate approximation
of a spec, that is called out explicitly.

> Companion documents:
> - `docs/code-review.md` — critical review and a doc-vs-code conformance matrix.
> - `docs/detailed-specs/` — DSP design notes (some aspirational; see status banners).
> - `docs/corrosion_plugin_prd_and_specs.md` — product requirements (MVP-era, see status section).

---

## 1. Signal Chain

The audio path is a strict per-sample loop inside the host `process()` callback
(`src/lib.rs`). The exciter↔resonator interaction is per-sample by design; the
global post chain is **also** per-sample today (see §9).

```text
MIDI Note Event
      │
      ▼
┌──────────────────┐
│  VoiceManager     │  16 fixed voices; steal tails, then held notes ≥100 ms
│ (MAX_VOICES = 16) │  only voices with `is_rendering()` are summed
└────────┬──────────┘
         │  per active voice:
         ▼
┌──────────────────┐   force/displacement/velocity   ┌──────────────────┐
│   Exciter (1/16) │◀────────────────────────────────│  Interaction Bus  │
│  family dispatch │────────────────────────────────▶│  spatial coeffs    │
└──────────────────┘                                  └─────────┬─────────┘
                                                                │
                                                                ▼
                                                  ┌──────────────────────────┐
                                                  │     ModalResonator        │
                                                  │  N second-order modes      │
                                                  │  + per-mode strike coeffs  │
                                                  └─────────────┬─────────────┘
                                                                │
                          rattle (damage) + velocity highpass boost + clamp/denormal
                                                                │
                              Σ voices  ─────────────────────────┘
                                       │
                                       ▼
                         apply_drive() (master/exciter drive, L & R)
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       PostProcessingChain                              │
│  L/R: WDF filter → Lorenz drive (one pair per channel) ─┐             │
│  FEM body on (L+R)/2, × BODY amount, added to both ─────┤             │
│  → HRTF spread → Space(Factory|Spring|Echo) →            │             │
│  OversampledClipper (per channel)      (all quality modes)             │
└─────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
                        × output_gain → apply_output_limiter()
                                       │  hard clamp to ±0.9661  (≈ −0.3 dBFS)
                                       ▼
                                 Stereo Output
```

### Per-Sample Voice Loop (`Voice::process_sample[_stereo]`)

1. **Render gate** — return early if `!rendering` (idle/decayed voices cost nothing).
2. **Force envelope** — one of three families (see §6): one-shot AR (Hit), ADSR
   (Specialty), or 6-stage MSEG (Friction).
3. **Exciter** — selected model is fed the resonator's current displacement and
   velocity (`get_displacement()` / `get_velocity()`), producing a force.
4. **Resonator** — force is distributed across modes by strike-position
   coefficients and the coupling blend, then each second-order mode is advanced.
5. **Damage rattle** — amplitude-gated noise injected when `damage > 0`.
6. **Velocity highpass boost** — velocity-scaled spectral lift.
7. **Clamp + denormal flush** — output bounded to [-1, 1]; `DENORMAL_FLUSH` trick.
8. **Peak-hold / tail tracking** — drives voice stealing and tail deactivation
   (`TAIL_ENERGY_THRESHOLD = 1e-4`, `TAIL_DEACTIVATE_FRAMES = 4800`).

---

## 2. Module Map

```text
src/
  lib.rs              Plugin trait impls, MIDI routing, apply_drive, output limiter,
                      per-sample post-chain parameter updates
  params.rs           127 host parameters; ExciterType (16), Object (9),
                      QualityMode (Eco/Normal/High/Render), 14 per-object character params
  voice/
    mod.rs            Voice: 16 exciter slots + ModalResonator + 3 envelope models,
                      VoiceControls snapshot, midi_to_hz, tail/rendering lifecycle
    manager.rs        VoiceManager: 16-voice pool, tiered stealing (tails → held ≥100 ms → oldest), idle gating
  dsp/
    mod.rs            Re-exports
    interaction.rs    BidirectionalInteractionBus, InteractionState,
                      mode_coefficient_1d/2d/circular, fundamental lock, wander
    transforms.rs     Newtypes: SizeScale, RustAmount, DamageAmount,
                      ThicknessAmount, HeatAmount, SludgeAmount (bounded/sanitized)
    profile.rs        ModalModeSpec + transform methods (scaled_for_size, corroded,
                      thickened, heated, sludge_loaded, damaged, retuned, adjusted_for_controls)
    profiles/         Per-object modal mode tables (pipe, plate, tank, chain, ibeam,
                      taut_cable, coil_spring, sheet_metal, industrial_cog)
    resonators/       core.rs (ModalResonator, SecondOrderMode, ResonatorCore trait,
                      ResonatorCoefficients, CharacterParams) + per-object
                      ResonatorAlgorithm generators (sole resonator path)
    envelopes/        mod.rs: 6-stage MSEG, Stage, LoopMode
    exciters/         16 exciters (bow, hand_strike, felt_mallet, hard_mallet,
                      drumstick, wire_brush, metal_pipe, metal_chain, stiff_point,
                      heavy_grinding, corrugated_drag, tension_rise, other_specialty
                      [pneumatic_jet, electromagnetic_hum, tension_snap, particle_rain])
    post_processing/  wdf_filter, lorenz_drive, fem_body, hrtf_spread, space
                      (FactoryReverb/SpringReverb/FactoryEcho), oversampled_clipper,
                      post_chain (PostProcessingChain + PostQualityMode)
    utils/            budget (mode-count estimates), body, deterministic_excitation
  gui/
    editor.rs         egui editor: knobs/faders, UI scale (50–150%), quality selector
  presets/
    mod.rs            Preset save/load + sanitization (out of scope for this review)
  offline/
    mod.rs            OfflineRenderer, comparison/rust/damage suites,
                      AliasingReport + analyze_post_chain_aliasing (DFT residual proxy)
  randomizer/
    mod.rs            Parameter randomization helpers
  bin/
    render.rs         CLI: --suite family|rust|damage|all|aliasing
    render_presets.rs CLI: preset batch renderer (auto-discovered bin)
```

---

## 3. Real-Time Safety Contract

The audio callback guarantees:

- **No heap allocation** — covers note-on (every object × quality mode × exciter),
  voice stealing, note-off tails, live expression, and `reset()`. Each voice's
  `ModalResonator` preallocates `MAX_MODES` (largest profile × 4 scale cap × 2
  damage companions = 96) and `rebuild_in_place` refills it at note-on; streaming
  exciters (wire brush, metal chain, particle rain) preallocate to their clamps.
  Verified by `tests/no_alloc.rs` in **debug** builds only — the checking
  allocator is installed under `debug_assertions`, so `--release` runs pass
  vacuously. The non-Windows build enables nih_plug's `assert_process_allocs`
  feature (`Cargo.toml` target cfg).
- **No locks / blocking / I/O / logging** in `process()`.
- **Fixed voice array** — 8 `Voice` instances, each owning all 16 exciter structs
  so dispatch is a `match` on an integer (no trait objects, no allocation).
- **Bounded loops** — modal banks have fixed sizes; MSEG stages are bounded.
- **Denormal protection** — `DENORMAL_FLUSH` add/subtract at the voice boundary;
  resonator coefficients clamp `decay_seconds` to `f32::EPSILON`.
- **NaN/Inf guards** — every voice output is checked with `is_finite()` and
  flushed to zero before leaving the voice.
- **NaN-safe stealing** — within its stealing tier (released tails, then held
  notes past the 100 ms protection window), a non-finite `peak_hold` sorts as the
  quietest voice (`comparable_peak_hold`), so a corrupted voice is reclaimed rather
  than panicking. Young held notes are ordered by age only.

> Caveat: parameter *application* is not control-rate. `lib.rs` calls every
> `post_chain.set_*` setter once per sample, which recomputes coefficients at
> audio rate. This is real-time-*safe* (no allocation) but wasteful, and it is
> the proximate cause of the FactoryReverb delay bug (see `docs/code-review.md`).

---

## 4. Voice Lifecycle

```text
Idle ──note_on──▶ Active(rendering) ──note_off──▶ Tail(rendering) ──peak<thr for 4800 frames──▶ Idle(!rendering)
                       │                               │
                       └─ steal if pool exhausted      └─ resonator decays naturally; exciter released
```

- **note_on**: snapshot parameters into `VoiceControls`, rebuild the resonator
  via the algorithmic generator configured by the object's character params
  (see §5), configure interaction params, trigger the selected exciter, init the
  envelope. Sets both `active` and `rendering` true.
- **note_off**: clears `active`, releases the exciter and force envelope; the
  resonator tail keeps `rendering` true until it decays.
- **Tail deactivation**: when `!active` and `peak_hold < TAIL_ENERGY_THRESHOLD`
  for `TAIL_DEACTIVATE_FRAMES`, both `active` and `rendering` go false and the
  voice stops costing CPU. (`is_rendering()` is the idle-CPU optimization.)
- **Stealing**: `VoiceManager` prefers an inactive (`!is_rendering`) slot; if none,
  it steals the lowest `peak_hold`, breaking ties toward the oldest `start_frame`.

---

## 5. Resonator Model

`ModalResonator` holds `Vec<SecondOrderMode>`. Each mode is a damped
two-pole resonator, run in coupled (complex one-pole) form:

```text
z[n] = p·z[n-1] + g·x[n],   y[n] = Im z[n]
p = r·e^{iω},   g = b0·(cot ω + i)
ω = 2π·f / sample_rate
r = exp(−1 / (decay_seconds · sample_rate))
b0 = gain · (48 000 / sample_rate)²
```

Its impulse response, `b0·rⁿ·sin(ω(n+1)) / sin ω`, is the direct-form biquad's
(`ResonatorCoefficients`: `a1 = −2r·cos ω`, `a2 = r²`). The coupled form is
used for two reasons:
- Retuning a mode (sheet warp, cable drop, pitch bend) rotates its state
  without changing its magnitude, so it can't pump energy in. In direct form
  the sheet warp diverged.
- The frequency comes from `r·sin ω`, not `−2r·cos ω ≈ −2`, so low,
  long-ringing modes stay accurate in f32 up to 192 kHz. The coefficients are
  computed in f64.

The `(48 000 / fs)²` input scale keeps the level independent of the host
sample rate: the mode stands in for a force-driven oscillator, whose
discretized force term scales with `dt²`. Without it, a force lasting a fixed
time rang the mode at a level ∝ fs² (+12 dB at 96 kHz). At 48 kHz the factor
is exactly 1. `decay_seconds` is a 1/e time constant (T60 ≈ 6.9·τ).

Modes at or above `min(20 kHz, 0.45·fs)` (`MODE_CEILING_HZ`) get no input.
A mode's gain grows as `1 / sin ω`, so modes next to Nyquist blew up through
contact feedback, and above it they only alias.

**Bank normalization.** At note-on, after the material transforms, every bank
is scaled to one strike level (`STRIKE_LEVEL_REF`, the default Pipe C4
patch's), whatever the object, pitch, quality mode, damage or character
settings. Relative mode gains, and so the timbre, are kept.
- The level is estimated as `‖A‖_1.3`, with `A_i = |w_i · b0_i / sin ω_i|`:
  mode i's ring amplitude under a one-sample impulse, weighted by the
  strike-position coefficient `w_i` that `process_sample` applies.
- The p = 1.3 norm sits between the incoherent (p = 2) and the coherent (p = 1)
  sum of the modes, and tracks the measured peak within about 4 dB.
- It is O(modes) and allocation-free (`normalize_strike_level`).
- Live damping and brightness, and a non-default strike position, act after
  normalization, so they still change the level as they should.

The resonator is built by a single path (algorithmic), then transformed:

- **Algorithmic generator**: each object's `ResonatorAlgorithm`
  (`PipeResonator`, `PlateResonator`, … `IndustrialCogResonator`), configured by
  that object's curated **character** params, generates the modal bank for the
  MIDI pitch.
- **Transform chain**: the generated modes then run the same transforms in order
  (rust → thickness → heat → sludge → damage) plus `res_damping`/`res_brightness`.

Coefficients are cached and rebuilt on sample-rate change. Transformation macros
and character params are applied at note-on and stay fixed for the duration of the
note; the Cable/Sheet **dynamic hooks** are the exception (see below).

> **Approved change (decided 2026-05, implemented 2026-05-27).** The former dual
> path was consolidated: the **algorithmic path is now the only path**,
> `complex_algo` was removed, and a **curated set of 14 per-object "character"
> parameters** is exposed (Pipe Diameter, Plate Aspect/Stiffness, Tank
> Volume/Cavity Mix, Chain Link Mass/Instability, IBeam Shear, Cable Braid/Tension
> Drop, Spring Dispersion/Slosh, Sheet Thinness, Cog Dissonance). Profile tables
> are kept as `mode_count`/budget/test metadata only and no longer drive sound.
> The two generator pitch gaps (Chain ignored the note; Tank's cavity was a fixed
> Hz) are fixed, and the per-sample dynamic hooks (`TautCable` amplitude→pitch,
> `SheetMetal` warp) run in `ModalResonator::process_sample[_stereo]`, scaling each
> mode from its immutable base frequency (allocation-free). The default object
> timbres changed as a result. See `docs/backlog.md` → "Algorithmic resonator
> engine".

Nine object profiles ship: **Pipe, Plate, Tank, Chain, IBeam, TautCable,
CoilSpring, SheetMetal, IndustrialCog**.

---

## 6. Exciter System

Sixteen exciters dispatch by integer (`ExciterType`, IDs 1–16). Each owns its
DSP state and is fed the resonator's contact displacement/velocity each sample so
the interaction is genuinely bidirectional (not feed-through). Families drive the
force-envelope choice:

| Family | Members | Envelope |
|---|---|---|
| Hit | HandStrike, FeltMallet, HardMallet, Drumstick, WireBrush, MetalPipe, MetalChain | one-shot AR |
| Friction | Bow, StiffPoint, HeavyGrinding, CorrugatedDrag, TensionRise | 6-stage MSEG (loopable) |
| Specialty | PneumaticJet, ElectromagneticHum, TensionSnap, ParticleRain | ADSR |

The per-exciter DSP models implement the formulas in
`docs/detailed-specs/exciter-algorithms.md` (Kelvin–Voigt, Hertzian contact,
micro-bouncing, Poisson impulse clusters, Stribeck friction, etc.).

### Drive trims

Each exciter's force is scaled by a fixed trim (`EXCITER_DRIVE_TRIM` in
`voice/mod.rs`), so a default-control note at full velocity stays under full
scale before the voice clamp, on every object from C2 to C6. The trims were
measured with `print_exciter_overdrive`, and `default_notes_do_not_overdrive_the_voice`
checks the struck, bowed and jet cases.

The trims leave continuous exciters (bow, scrapes, jet, hum) much quieter than
before, when they were mostly hard-clipped inside the voice. HardMallet and
HeavyGrinding are effectively silent (about −47 dBFS median): above a small
drive their contact loop runs away, so their trim sits at half of that
threshold. Making them usable needs their contact models fixed.

### Sample-rate rule

Per-sample constants in the exciters, the voice (output high-pass, peak decay,
tail frame count), the resonator's velocity and dynamics trackers and the FEM
body are **48 kHz references**. Each is converted to the host rate through
`dsp::utils::rate`, with `ratio = 48 000 / fs`:

| Constant | Conversion |
|---|---|
| decay or one-pole pole `c` | `c^ratio` (`rate::decay`) |
| smoother step `x += (t − x)·k` | `1 − (1 − k)^ratio` (`rate::smoother`) |
| integration step, acceleration, rate | `× ratio` (`rate::step`) |
| frame count | `/ ratio` (`rate::frames`) |
| one-sample force spike | amplitude `/ ratio`, so it keeps its area (`rate::impulse`) |
| per-sample white noise | amplitude `/ √ratio` (`rate::white_noise`) |

Every conversion is exactly the identity at 48 kHz, so 48 kHz output is
bit-identical to the unconverted code. Exciters cache their converted
constants in `set_sample_rate`, which the voice calls when the rate changes.
The resonator reports velocity in 48 kHz-sample units, which is what the
exciters' contact laws assume. New per-sample code should follow the same
rule.

Levels match 48 kHz within 1.5 dB at 44.1 and 96 kHz, except where sample-grid
rounding or chaotic contact dominates, which needs sub-sample timing to fix.
The voice test `every_exciter_has_the_same_level_at_any_sample_rate` widens
the tolerance for these exciters:

- HandStrike, 3 dB: a ~0.1 ms strike on the envelope's attack ramp;
- Drumstick, 3 dB: chatter after its bounce limit;
- WireBrush, 2.5 dB: impulse times rounded to the grid;
- MetalChain, 3 dB: near-silent rattle and hash noise;
- TensionRise, 3 dB: clipped stick-slip;
- ParticleRain, 8 dB: its grid-rounded spawn train aliases onto the modes.

The measurements behind each are in the archived `fix-sample-rate-time-constants`
design ("Implementation findings").

---

## 7. Interaction Bus

`BidirectionalInteractionBus` / `InteractionState` implement the spec in
`docs/detailed-specs/exciter-resonator-interaction.md`:

- Per-mode 1D strike coefficients `c_n(P) = sin((n+1)·π·P)`.
- `fundamental_anchor` clamps `c_0` to a minimum so the pitch never fully nulls.
- `coupling_stiffness` blends feed-forward (`0`) ↔ position-weighted (`1`) force.
- `position_wander` adds a bounded sinusoidal drift to the strike position.
- The resonator reports summed displacement and a first-difference velocity back
  to the exciter each sample.

---

## 8. Transformations

`profile.rs` implements all six transforms from
`docs/detailed-specs/transformation-algorithms.md`, applied to mode specs:

| Transform | Effect (implemented) |
|---|---|
| Size | `f /= scale`, `decay *= scale`, low-mode gain tilt |
| Rust | high-mode-weighted brightness loss + decay loss |
| Thickness | stiffness `√(1 + (t−0.5)·0.15·n²)` raises/inharmonicizes partials |
| Heat | pitch drop (`·(1 − heat·0.05)`) + brightness loss |
| Sludge | mass loading `f·√(1/(1+sludge))` + broadband damping |
| Damage | mode splitting into detuned pairs + amplitude-gated rattle (in the voice) |

Velocity expressiveness (force `V^1.5`, brightness, decay) is applied in the
exciters and envelope velocity response.

---

## 9. Post-Processing Chain

`PostProcessingChain` runs, per channel, WDF ladder filter → Lorenz drive. It then adds
the FEM body resonance (run once on the mono sum, mixed at BODY / 5) to both channels,
followed by HRTF spread → Space → oversampled clipper. Chassis Volume only retunes the
body. Every quality mode runs every stage (C5, `fix-post-chain-body-routing`). These are
**lightweight, real-time-safe approximations** of the high-fidelity algorithms in
`docs/detailed-specs/post-processing.md`; the module docstrings say as much. See
`docs/code-review.md` §"Post-processing conformance" for the exact gap per stage.

> **Decision (2026-05-28).** The approximations described here are the shipped
> contract. The detailed-specs files are aspirational design notes; promoting
> any single stage to its full PDE/FDTD / HRIR description would be a feature
> proposal, not a finish-the-partial task. The cutoff/drive smoothers added in
> the P1 sweep live inside `WdfLadderFilter` and `LorenzDrive` and ramp at
> audio rate (~20 ms tau) with first-call snap so static-config flows are
> unchanged.

Stage summary (as implemented):

| Stage | Reality |
|---|---|
| WDF ladder filter | 4-pole ladder, soft transistor clip, hash-noise "tolerance" — not a Newton-Raphson WDF circuit |
| Lorenz drive | real Lorenz attractor state used as a mild gain modulator + soft tube curve |
| FEM body | 8 fixed modal resonators — not a finite-element mesh |
| HRTF spread | ≤1 ms delay + 1-pole LP/HP + crossfeed — not HRIR convolution |
| Factory reverb | 4 comb + 2 allpass (Schroeder) — not 3D FDTD |
| Spring reverb | one delay line + 1-pole dispersion per channel — not a helical PDE |
| Factory echo | modulated stereo delay (gentle Doppler-ish) |
| Oversampled clipper | soft diode clip; **the oversample factor is currently a no-op** (see code review) |

### Quality modes (`QualityMode` → `PostQualityMode`)

| Mode | Oversample factor requested | Chain behavior |
|---|---|---|
| Eco | 1× | full chain |
| Normal (default) | 4× | full chain |
| High | 8× | full chain |
| Render | 16× | full chain |

The mapping is wired end-to-end (param → `set_quality_mode` → `set_oversample_factor`),
and every mode runs the same stages. **However the clipper's
oversampling math does not actually oversample** (it zero-order-holds a single
input and averages identical copies), so Normal/High/Render are sonically
identical at the clipper. This is the top defect in `docs/code-review.md`.

### Output protection

Final stage is `apply_output_limiter`: a hard clamp to `±LIMITER_THRESHOLD`
where `LIMITER_THRESHOLD = 0.9661` (≈ **−0.3 dBFS**). The clipper's
`analog_ceiling` parameter defaults to the same 0.9661 and is clamped to
[0.5, 1.0].

---

## 10. Build, Test, and QA Infrastructure

| Component | Purpose |
|---|---|
| `.github/workflows/ci.yml` | fmt, clippy (`-D warnings`), no-default-features tests, native gnu lib tests, `render --suite family` smoke, WAV check, `bundle.sh release` |
| `scripts/verify-local.sh` | local mirror of CI |
| `scripts/validate-plugins.sh` | optional `pluginval` + `clap-validator` wrapper |
| `scripts/check_wav.py` / `analyze_wav.py` / `check_clicks.py` | offline artifact validation |
| `benches/performance.rs` | Criterion harness (voice, dense excitation, post chain, idle, offline) |
| `tests/` | integration tests: no_alloc, automation_stress, preset_roundtrip, body, drive, stereo, velocity, chain distinctness, limiter, damage rattle, bow, plugin metrics |
| `src/bin/render.rs` | offline QA renderer incl. `--suite aliasing` → `aliasing_report.txt` |

Default local target is `x86_64-unknown-linux-musl` (`.cargo/config.toml`); the
bundle/validator flow uses `x86_64-unknown-linux-gnu`.

---

## 11. Status of Previously-Claimed Work (verified in code)

| Feature | Status | Evidence |
|---|---|---|
| Quality modes (Eco/Normal/High/Render) | Implemented | `params.rs` `QualityMode`, `post_chain.rs` `set_quality_mode`, `lib.rs` mapping |
| Configurable oversampling 1/4/8/16 | Wired but **ineffective** | `oversampled_clipper.rs` factor is a no-op (see code review) |
| Idle rendering optimization | Implemented | `Voice::rendering` flag + `is_rendering()` gating in `manager.rs` |
| Aliasing measurement | Implemented (DFT proxy) | `offline/mod.rs::analyze_post_chain_aliasing`, `render --suite aliasing` |
| Output ceiling −0.3 dBFS | Implemented | `LIMITER_THRESHOLD = 0.9661`, `analog_ceiling` default 0.9661 |

---

## 12. Known Architectural Decisions & Gaps

- **Per-sample everything.** The exciter↔resonator loop must be per-sample
  (bidirectional coupling). The post chain is also per-sample today, including
  parameter setters — block processing remains a future optimization.
- **All exciters resident per voice.** Trades memory (~all 16 structs × 16 voices)
  for allocation-free, branch-only dispatch.
- **Resonator mode count scales with QualityMode.** Each profile reports a base
  mode count (roughly 6–12); the active QualityMode multiplies that at note-on
  (Eco 0.5× / Normal 1.0× / High 1.5× / Render 2.0×), clamped to ≥1 mode. The
  multiplier is snapshotted into `VoiceControls.mode_count_scale` and only
  re-evaluates on the next note-on.
- **Approximate post DSP.** The post/space/output stages are intentionally
  cheaper than `detailed-specs/post-processing.md`. Treat that spec as a design
  target, not a description of shipped behavior.
- **Oversampled clipper is effectively a soft clipper.** Until the no-op is fixed,
  nonlinear-stage aliasing is unmitigated (and is exactly what the aliasing
  harness measures).
- **MetalChain's rattle filter keeps no state.** `apply_hpf` runs after
  `noise_state` is written back, so the "high-pass" is a plain gain of about
  `(1000/fs)/cutoff`, now evaluated at 48 kHz. That is why the chain is nearly
  silent (about −80 dBFS on a Pipe). Fixing it changes the 48 kHz sound.
- **Single algorithmic resonator path.** The former dual path was retired
  (implemented 2026-05-27, see §5 and `docs/backlog.md`): `complex_algo` is gone,
  the per-object algorithmic generators are the sole engine, 14 per-object
  character params are exposed, and the profile tables are demoted to
  `mode_count`/budget/test metadata.

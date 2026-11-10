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

Below the host API the path is one function, `Corrosion::process_block`:
`begin_block` (per-buffer setup: drive and output-gain smoother targets,
limiter configuration, post-chain parameters with macro biases, live controls
of held voices), then for each sample the MIDI events due at it,
`render_frame_unlimited` (voices → mix DC blocker → master drive → post chain → output gain) and
`limit_frame`. `process()` calls it with the host's buffer and events, and
`offline::PresetRenderer` / `offline::render_preset` with a real plugin
instance holding a preset, in 512-sample blocks. So `render_presets`, the audition WAVs in `output/` and the
preset level checks hear exactly what the plugin outputs
(`h13-render-presets-through-post-chain`). `reset()` starts the master-drive
smoother at Drive plus the Violence bias, where `begin_block` aims it.

**The voice mix carries no DC** (`block-voice-mix-dc`). Each mode is a
coupled-form resonator whose output has a non-zero gain at 0 Hz, so a
one-signed force (a bow's pressure, a mallet's long contact) leaves a static
deflection under the ringing: up to ~47 % of a bowed patch's output power,
and a felt mallet's ~60 ms thump. `dsp::StereoDcBlocker`, a second-order
Butterworth high-pass at 10 Hz (`VOICE_MIX_DC_CUTOFF_HZ`, bilinear with a
prewarped cutoff, `f64` coefficients and state), removes it from the voices'
stereo sum before the master drive, so the drives and the clipper see a
centred signal. It costs 0.05 dB at 30 Hz; a held bow keeps under 2 % of its
RMS as DC and a felt mallet's largest 20 ms mean drops 16.6 dB. It is rated
in `initialize` and `begin_block` and cleared by `reset()` and the panic
clear. DC that later stages create (the asymmetric drives and the diode
curve) is theirs to handle.

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
                         StereoDcBlocker (10 Hz, 2nd-order Butterworth, L & R)
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
│  OversampledClipper (per channel, half-band IIR 1/4/8/16×)            │
└─────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
                        × output_gain → Hard clamp or Lookahead limiter
                                       │  both at ±Analog Ceiling (default 0.9661 ≈ −0.3 dBFS)
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
    utils/            budget (mode-count estimates), body, dc_blocker (voice mix),
                      deterministic_excitation
  gui/
    editor.rs         egui editor: knobs/faders, UI scale (50–150%, default 75%: 810×576;
                      a change asks the host to resize; the layout never exceeds the
                      window's width), quality,
                      play and limiter mode selectors, a macro strip (Mass,
                      Corrosion, Violence, Brightness), resonator Brightness,
                      Thickness, and Sync (shows "Off" or its division).
                      `editor_covers_every_parameter` renders the editor
                      headlessly for every exciter, object and space mode, and
                      fails if a parameter has no control and isn't on
                      `HOST_ONLY_PARAMS` (with its reason).
  presets/
    mod.rs            Preset save/load + sanitization (out of scope for this review)
  offline/
    mod.rs            OfflineRenderer, comparison/rust/damage suites,
                      AliasingReport + analyze_post_chain_aliasing (DFT residual proxy)
    preset.rs         render_preset: a preset through the plugin's own output
                      path (Corrosion::process_block, 512-sample blocks)
  randomizer/
    mod.rs            Parameter randomization helpers
  bin/
    render.rs         CLI: --suite family|rust|damage|all|aliasing
    render_presets.rs CLI: preset batch renderer → stereo WAVs via offline::render_preset
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
- **Sustain pedal** (`VoiceManager::set_sustain`, CC64 ≥ 64): a key-up while
  the pedal is down marks the voice `pedal_held` instead of releasing it, so it
  stays `active`; lifting the pedal releases every pedal-held voice. A re-strike
  of a pedal-held note releases the older instance. Stealing tiers are released
  tails, then pedal-held notes, then held keys past the protection time, then
  younger held keys. CC123 (`all_notes_off`) releases every held key through the
  same key-up path, so the pedal still applies.
- **All Sound Off** (CC120): `Corrosion::start_panic` fades the output over
  `PANIC_FADE_SECONDS` (5 ms) in `render_frame_unlimited` (an out-of-line branch,
  skipped when idle), then clears voices (`VoiceManager::silence`, which keeps
  pedal, mod wheel, bend and pressure), the post chain and the lookahead
  limiters in place. CC121 lifts the pedal and returns mod wheel, bend and
  channel pressure to rest (`m13-sustain-pedal-and-panic`).
- **Note addressing** (`m12-per-note-expression`): each voice stores the host's
  `voice_id` and `channel` from its note-on (`VoiceManager::note_on_addressed`,
  `NoteAddress`). Note-off, `PolyPressure`, `PolyTuning` and `Choke` share one
  rule: the voice with the event's voice ID, or, when the event has none or no
  voice has it, key and channel (note-off: the oldest held voice; pressure: the
  first sounding voice; tuning and choke: every sounding voice on that key).
  Per-note tuning is stored on the voice and added to the channel bend before
  `pitch_bend_semitones_to_factor` (±24 semitones together), so it retunes
  through the same exact path and a held value is skipped like a held bend.
  A choke releases the note and fades the voice linearly over
  `CHOKE_FADE_FRAMES` (96 at 48 kHz, 2 ms at any rate), then resets it to idle;
  the fade is an out-of-line branch in the per-sample voice loop. Channel MPE
  is out of scope: channel bend, pressure and CC1 stay global.
- **Object per note** (`handle_note_event`): in Tonal and Drone play modes the
  Object parameter picks the object. In **Kit** mode the note does
  (`note_to_kit_object`): 10-note slots in object order from MIDI 24, repeating
  in both directions, slot = `(note − 24).div_euclid(10).rem_euclid(9)`. So
  114–123 is Pipe again, and below the block 14–23 is object 8, 4–13 object 7
  and 0–3 object 6 (`h14-wrap-low-kit-notes`; every note under 24 used to play
  Pipe).

---

## 5. Resonator Model

`ModalResonator` holds `Vec<SecondOrderMode>`. Each mode is a damped
two-pole resonator, run in coupled (complex one-pole) form:

```text
z[n] = p·z[n-1] + g·x[n],   y[n] = Im z[n]
p = r·e^{iω},   g = b0·(sin ω / ω)·(cot ω + i)
ω = 2π·f / sample_rate
r = exp(−1 / (decay_seconds · sample_rate))
b0 = gain · (48 000 / sample_rate)²
```

Its impulse response, `(b0/ω)·rⁿ·sin(ω(n+1))`, has the direct-form biquad's
poles (`ResonatorCoefficients`: `a1 = −2r·cos ω`, `a2 = r²`). The ring
amplitude is `b0/ω`, the continuous oscillator's, rather than the discrete
`b0/sin ω`, so a mode's level doesn't depend on the rate and doesn't blow up
next to Nyquist (`resolve-exciter-tradeoffs`). The coupled form is used for
two reasons:
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

**Top of the band.** A mode's output is weighted by `band_fade`: 1 below
0.9·top, 0 at or above the top, raised-cosine in between, where the top is
`min(20 kHz, 0.45·fs)` (18–20 kHz at 44.1 kHz and above). It is evaluated
from the mode's current frequency, so a mode retuned across it fades instead
of switching on or off. (It replaced a hard cut at the top, which clicked on
retune and hid the contact runaway that implicit contact now fixes.)

**Unsoundable modes are pruned.** At note-on, after the material transforms
and before the bank is filled, every mode that can never sound is dropped
(`h11-prune-unsoundable-modes`):
- its frequency is at or above `PRUNE_MAX_FREQUENCY_HZ` = 80 kHz, the band top
  (20 kHz) at the lowest pitch a note can be tuned to (×0.25). Object dynamics
  only raise pitch, so such a mode can't reach the band during the note. The
  limit is in hertz, so the bank is the same at every sample rate;
- or its decay time constant is under `PRUNE_MIN_DECAY_SECONDS` = 0.1 ms (gone
  in about five samples at 48 kHz). The generators' negative decay formulas
  (Pipe's `1.5 − 0.15·n`, Plate's `0.4 − 0.02·(m + n)`) land here after their
  `max(ε)` rescue.

The prune compacts the preallocated spec buffer in place (no allocation), and
each kept mode keeps its brightness weight from the full bank, so dropping
modes doesn't re-tilt the ones that stay. A pitch bend never rebuilds the bank,
so a bent note keeps every mode it started with. At normal sizes only Pipe and
SheetMetal lose modes; small sizes push the other objects' upper modes past
80 kHz too.

**Bank normalization.** At note-on, after the material transforms, every bank
is scaled to one strike level (`STRIKE_LEVEL_REF`, the default Pipe C4
patch's), whatever the object, pitch, quality mode, damage or character
settings. Relative mode gains, and so the timbre, are kept.
- The level is estimated as `‖A‖_1.3`, with `A_i = |w_i · fade_i · b0_i / ω_i|`:
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

The friction MSEG's onset stage rises from 0 to **Onset Level**
(`mseg_onset_level`, 0–1, default 0.01) over **Onset Time** (`mseg_onset`,
0.001–1 s). Onset Time used to set the level too, clamped to 0–1; the new
default keeps that level, so default notes are bit-identical
(`m01-m04-m05-clarify-parameters`). Presets (`migrate_preset_json`) and saved
host state (`Plugin::filter_state`) without Onset Level get
`legacy_onset_level(mseg_onset)`, the level they had.

The per-exciter DSP models implement the formulas in
`docs/detailed-specs/exciter-algorithms.md` (Kelvin–Voigt, Hertzian contact,
micro-bouncing, Poisson impulse clusters, Stribeck friction, etc.).

### Global exciter controls

Three host parameters act on every exciter on top of its dedicated controls
(`h06-wire-global-exciter-controls`). Each is neutral at its default, where a
note renders bit-identically to one that ignores it
(`default_global_controls_are_bit_identical`).

| Control | Default | Factor | Applies | Acts on |
|---|---|---|---|---|
| Pressure | 0.5 | `2^((p − 0.5)·4)`: −12 dB to +12 dB | live, smoothed ~10 ms | every exciter's force, through `Voice::force_scale` |
| Speed | 0.5 | `2^((s − 0.5)·2)`: ×0.5 to ×2 | note-on | motion speeds: Bow Speed, Scrape/Grind/Drag/Pull Speed, ParticleRain Flow Rate; WireBrush and MetalChain spreads are divided by it |
| Roughness | 0.3 | `r / 0.3`: ×0 to ×3.33 | note-on | grit and randomness: Bow Rosin Grip, WireBrush Amplitude Randomization, MetalChain Internal Rattle, HeavyGrinding Surface Grit, TensionRise Slip Stochasticity, PneumaticJet Turbulence Chaos, ParticleRain Mass Variance |

- **Pressure** is part of `force_scale`, so contact exciters see it in their
  prediction and the drive governor still bounds the voice. It reaches held
  notes through `update_live_controls`.
- **Speed and Roughness** scale a copy of `VoiceControls` before the
  per-exciter dispatch (`apply_global_exciter_controls`). Each scaled value is
  clamped to its host parameter's range. Exciters with no motion or grit
  parameter are inert to them: HandStrike, FeltMallet, HardMallet, Drumstick
  and MetalPipe for both;
  PneumaticJet, ElectromagneticHum and TensionSnap for Speed; StiffPoint,
  CorrugatedDrag, ElectromagneticHum and TensionSnap for Roughness.
- `maximum_global_controls_do_not_overdrive_the_voice` plays every exciter on
  every object at C2/C4/C6, velocity 127, with all three at 1: no pre-clamp
  sample exceeds 1.0.

### Drive levels and the drive governor

Each exciter's force is scaled by a drive level (`EXCITER_DRIVE_LEVEL` in
`voice/mod.rs`), set so its median note over every object at C2/C4/C6
(velocity 127, default controls) peaks at about 0.35 before the voice clamp.
`print_exciter_overdrive` measures the medians and suggests the levels, and
`exciters_play_at_a_common_level` keeps every median within 6 dB of the
others. Louder notes are handled per voice by the drive governor, instead of
turning an exciter down everywhere (`resolve-exciter-tradeoffs`).

The governor scales the exciter force (never the output), so a note stays
clean:

- **Ring bound.** The resonator tracks `B = Σ w_i·|z_i|`, with
  `w_i = fade_i·(1 + boost·|H_hp(ω_i)|)`, where `H_hp` is the voice's
  brightness high-pass. With no more force, no later sample (either stereo
  channel) can exceed `B`, so it is known on the strike sample itself, before
  the output peaks. It also tracks `per_force`, how much one unit of force can
  raise `B`. `B` is summed exactly on every 8th tracked sample; in between it
  grows by `per_force·|f|`.
- **Smooth gain.** A follower on `B` (1 ms attack, 200 ms release) gives a
  gain of `(0.7 / level)^8` above 0.7. The gain depends only on the current
  level, so a long ring can't wind it down.
- **Ceiling.** Each sample's force is capped so that `B` stays under 0.9.
  Contact exciters see the same cap in their prediction
  (`ContactPrediction::limit`).

`default_notes_do_not_overdrive_the_voice` checks all 16 exciters, and
`excess_drive_is_reduced_not_clipped` drives each exciter's loudest note 12 dB
over: its peak stays under 1.0 and within 3 dB of the default-drive peak.

### Implicit contact

Every exciter whose force depends on the resonator's position solves its
contact together with the resonator's response to this sample's force,
instead of reacting to the previous sample: HandStrike, FeltMallet,
HardMallet, Drumstick, MetalPipe, MetalChain, HeavyGrinding, ParticleRain and
TensionSnap.
- `ModalResonator::contact_prediction` gives this sample's contact
  displacement as `free + compliance·F`;
- `dsp::utils::contact::solve_force` finds the force that satisfies the
  contact law.

The voice scales the compliance by everything it multiplies the force by
(envelope, drive level, governor gain) and passes the governor's force cap,
so the exciter sees its force's real effect. `contact_is_stable_without_the_governor`
plays every exciter 12 dB over its drive with the governor off.

- **Strikes come to rest.** HardMallet leaves after its rebound, and
  Drumstick goes inactive at its bounce limit or below its rest velocity.
- **Length units.** A position-driven exciter can't push the surface further
  than its own travel. HandStrike, MetalPipe and ParticleRain see the
  resonator in their own units (`ContactPrediction::in_units`; `HAND_SCALE`,
  `PIPE_SCALE`, `RAIN_SCALE`), so at their drive levels their contact stays
  force-limited and their output follows the drive.
- Bow, StiffPoint, CorrugatedDrag, TensionRise and PneumaticJet read the
  resonator's velocity explicitly: their friction and flow laws aren't
  monotone, which the solver needs, and none of them runs away.

### Noise

Every random source uses `dsp::utils::noise::Noise`, a xorshift32 generator
that is white and uniform in [−1, 1). The voice reseeds each consumer at
note-on from `note_seed(note, velocity, start_frame)`, salted per consumer.
Per-sample noise forces go through `rate::white_noise`. Random event
parameters (impulse times, link drops, TensionRise's break threshold) are
drawn once per event, so their statistics don't depend on the rate.

### Sub-sample events

Short events land between samples at their true time: a `rate::EventSplitter`
puts `early·A` of an event's area on this sample and the rest on the next.
The event's area and centre of mass then match continuous time (one sample
late). This covers WireBrush impulses, MetalChain impacts (the links follow
their exact fall), TensionSnap and TensionRise spikes, and ParticleRain
spawns (aged by their fraction of the sample). HandStrike scales its landing
sample by the part of the step it was still in contact, and takes its force
at mid-step. The Hit envelope's attack is sampled at sample centres.
`dsp::exciters::event_timing_tests` checks each at 44.1, 48 and 96 kHz.

### Sample-rate rule

Every voice, idle ones included, is rated for the host rate before it plays:
`Plugin::initialize` passes `BufferConfig::sample_rate` to
`VoiceManager::set_sample_rate`, and `begin_block` does again when the rate
changes. A note-on therefore builds its bank's coefficients once, at the host
rate, and its exciter starts at that rate; each exciter's `trigger` also
restarts its per-note state, so a note doesn't depend on what the voice played
before (`m03-m20-start-at-host-rate-and-report-latency`).

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

Every conversion is exactly the identity at 48 kHz. Exciters cache their converted
constants in `set_sample_rate`, which the voice calls when the rate changes.
The resonator reports velocity in 48 kHz-sample units, which is what the
exciters' contact laws assume. New per-sample code should follow the same
rule.

Levels match 48 kHz within 1.5 dB at 44.1 and 96 kHz for every exciter, with
no exceptions (`every_exciter_has_the_same_level_at_any_sample_rate`). The test
compares the expected level, mean power over four velocities and 32 note
seeds each: a noise-driven exciter's single note varies by several dB from its
seed alone. Struck attacks peak within 10 % of the 48 kHz time over the whole
note.

---

## 7. Interaction Bus

`BidirectionalInteractionBus` / `InteractionState` implement the spec in
`docs/detailed-specs/exciter-resonator-interaction.md`:

- Per-mode 1D strike coefficients `c_n(P) = sin((n+1)·π·P)`.
- `fundamental_anchor` clamps `c_0` to a minimum so the pitch never fully nulls.
- `coupling_stiffness` blends feed-forward (`0`) ↔ position-weighted (`1`) force.
- **Effective strike position**, every sample:
  `clamp(strike_position + position_envelope · envelope · 0.5 + wander, 0, 1)`
  (`h09-route-envelope-to-strike-position`).
  - **Position Envelope** moves the strike point with the note's force envelope
    (0..1, set by the voice each sample before the resonator prepares the
    sample), so a strike or a bow swell sweeps the contact point and the
    timbre. At 0 the voice never touches the offset, and the output is
    bit-identical to before the routing existed.
  - **Wander** (`position_wander`) adds a bounded sine drift at **Wander Rate**
    (`position_wander_rate`, 0.1–5 Hz, default 0.1), independent of Position
    Envelope. Presets and saved host state (DAW projects, through
    `Plugin::filter_state`) that predate Wander Rate load the rate their
    Position Envelope used to set (`0.1 + position_envelope · 4.9`), with
    Position Envelope 0.
  - The strike weights are cached on the effective position, so they are
    recomputed only while it moves.
- The resonator reports summed displacement and a first-difference velocity back
  to the exciter each sample.

---

## 8. Transformations

`profile.rs` implements all six transforms from
`docs/detailed-specs/transformation-algorithms.md`, applied to mode specs:

| Transform | Effect (implemented) |
|---|---|
| Size | `f /= scale`, `decay *= scale`, low-mode gain tilt |
| Rust | damping increase: `decay /= 1 + rust·(0.2 + 0.54·w²)`, `gain /= 1 + rust·0.6·w^1.5` (`w` = mode position, 0 fundamental → 1 top); never reaches zero, highs die first, the fundamental always rings (`fix-rust-damage-heat-response`) |
| Thickness | stiffness `√(1 + (t−0.5)·0.15·n²)` raises/inharmonicizes partials |
| Heat | pitch drop (`·(1 − heat·0.05)`), spectral darkening (`gain /= 1 + heat·2·w^1.5`, `decay /= 1 + heat·0.5·w`) and a thermal warble (pitch dynamic, up to about ±8 cents at Heat 1, two LFOs at 0.37 and 0.83 Hz) |
| Sludge | mass loading `f·√(1/(1+sludge))` + broadband damping |
| Damage | mode splitting into detuned pairs, driven by a saturating amount `3·(1 − e^(−d/3))` so the fundamental keeps its note and no decay reaches zero, + amplitude-gated rattle (in the voice) |

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
| Lorenz drive | real Lorenz attractor state used as a mild gain modulator + soft tube curve. Bias Starvation starves the input gain to 0.3 while the attractor's `z` is above `38 − 22·starvation` (starved about 5 % of the time at 0.1, 87 % at 1, switching at every value), through a 5 ms one-pole follower so it sags rather than clicks; at 0 the stage is bit-identical to no starvation (`m06-m09-smooth-starvation-and-oversample-clipper`) |
| FEM body | 8 fixed modal resonators — not a finite-element mesh |
| HRTF spread | mid (L+R) delayed ≤1 ms with Width, common to both channels (not an ITD), through 1-pole LP/HP proximity shaping, plus the undelayed side, then crossfeed — not HRIR convolution; one delay buffer (`m07-m08-interpolate-echo-and-tidy-spread`) |
| Factory reverb | 4 comb + 2 allpass (Schroeder) — not 3D FDTD |
| Spring reverb | one delay line + 1-pole dispersion per channel — not a helical PDE |
| Factory echo | modulated stereo delay; Machinery Movement's depth is smoothed (τ 20 ms) so knob changes don't jump the delay; unmodulated reads are whole samples, modulated reads use 4-point Hermite interpolation (no zipper); High Frequency Damping is a one-pole low-pass in the feedback loop (20 kHz bypassed at 0 → 1.5 kHz at 1), so each repeat is darker; wet level fixed at 0.7 (`m07-m08-interpolate-echo-and-tidy-spread`) |
| Oversampled clipper (diode stage) | per-polarity tanh-knee diode curve (see "Diode stage" below) run at 4/8/16× through cascaded 2× polyphase half-band IIR stages (`half_band.rs`: elliptic design, 9/5/4/4 all-pass coefficients, flat to 22/48 of the host rate, 93–107 dB stop band), up → curve → down → 2 Hz DC blocker; Eco runs the same curve at the host rate. The limit is the ceiling ÷ 1.3 in every mode, so the filters' overshoot stays under the output limiter. The half-bands delay low frequencies by 4–5 samples (not reported as latency) (`m06-m09-smooth-starvation-and-oversample-clipper`, `add-asymmetric-diode-stage`) |

### Quality modes (`QualityMode` → `PostQualityMode`)

| Mode | Clipper | Clips from | Alias, 5 kHz at 2× ceiling | Delay at 1 kHz |
|---|---|---|---|---|
| Eco | 1× (curve at the host rate) | ceiling ÷ 1.3 | −19.7 dB | −0.015 samples (the DC blocker's lead) |
| Normal (default) | 4× (2 half-band stages) | ceiling ÷ 1.3 | −67.0 dB | 4.2 samples |
| High | 8× (3 stages) | ceiling ÷ 1.3 | −94.2 dB | 4.7 samples |
| Render | 16× (4 stages) | ceiling ÷ 1.3 | −105.0 dB | 5.0 samples |

Alias figures at the defaults (Softness 0.5, Asymmetry 0.25). At Softness 0
(a hard knee, what migrated projects get) they are −50.7 dB Normal and
−75.5 dB Render.

The mapping is wired end-to-end (param → `set_quality_mode` →
`PostQualityMode::clipper_oversample_factor` → `set_oversample_factor`), and
every mode runs the same stages. Below its threshold the clipper is flat
(0.000 dB to 20 kHz at 48 kHz in every mode). The alias figures hold through
the default hard clamp too (`clipper_alias_bounds_hold`,
`clipper_alias_bounds_hold_through_the_hard_clamp`; bounds −40 dB Normal,
−60 dB Render). No fixed margin can keep every overshoot under the clamp: with
extreme drive in the top octaves, peaks reach 1.7× the curve's threshold and
the clamp clips them.

### Output gain and parameter names

**Output** (`output_param`, shared by the live parameters and
`Preset::into_params`) stores the linear gain, 0 to +40 dB, so presets and
host state keep their level. It's shown and typed in dB
(`v2s_f32_gain_to_db` / `s2v_f32_gain_to_db`, −inf at gain 0), with a
gain-skewed taper (`gain_skew_factor(−60, 40)`): −10 dB at the middle of the
travel, 0 dB at 0.575, −20 to +6 dB across 0.189 (it was 0.010 and 0.019 with
the linear range). Host automation of Output recorded before the change maps
to different gains (BREAKING, not migratable).

Every parameter's display name is unique (`parameter_names_are_unique`), and
`into_params` builds them with the live names
(`into_params_uses_the_live_names_and_output_range`): Resonator / Macro
Brightness, Brush / Spring Wire Stiffness, Onset Time / Onset Level
(`m01-m04-m05-clarify-parameters`).

### Output protection

The **Analog Ceiling** parameter (0.5–1.0, default 0.9661 ≈ **−0.3 dBFS**) is
the output ceiling. The diode stage limits at it ÷ 1.3 in every quality
mode (headroom for the half-bands' overshoot and the DC blocker's tilt), and after output gain
the selected limiter stops at it: **Hard** (default) is
`apply_output_limiter(sample, ceiling)`, a clamp to `±ceiling`; **Lookahead**
uses it as its threshold (`h07-hard-limiter-honors-analog-ceiling`). The Lookahead limiter's delay (`LOOKAHEAD_LIMITER_SAMPLES`) is reported to the host as latency in `initialize`, from the restored Limiter Mode, and again whenever the mode changes (`Corrosion::latency_change`). So no
output sample exceeds the ceiling, whatever the gain or mode
(`hard_mode_stays_under_a_low_ceiling`,
`both_limiter_modes_reach_the_same_ceiling`). `LIMITER_THRESHOLD = 0.9661` is
the ceiling's default, used by offline tools and tests that render without the
parameter.

### Diode stage (`add-asymmetric-diode-stage`)

`OversampledClipper` is the diode stage. For each polarity with limit `L` and
knee width `w`, the curve is linear below `K = L·(1 − w)` and
`K + (L − K)·tanh((x − K)/(L − K))` above it: slope 1 at `K`, never past `L`.

- **Limit:** `L = ceiling / 1.3` in every mode (`CEILING_HEADROOM`). 1.3 is
  the smallest swept value that keeps a 5 kHz tone at twice the ceiling under
  it (0.987×) and holds the alias bounds at Softness 0. At 7 kHz the
  half-bands still overshoot to 1.21×, which the output limiter clamps.
- **Diode Softness** sets `w = 0.8·softness` (`KNEE_WIDTH`): the knee starts
  14 dB under `L` at 1, 4.4 dB at the default 0.5, and is a hard clip at 0.
- **Diode Asymmetry** (`diode_asymmetry`, default 0.25) lowers the negative
  limit to `L·2^(−a)` and firms its knee to `w·(1 − a/2)`.
- **DC blocker:** a 2 Hz one-pole high-pass per channel after the
  down-sampling, weighted by `min(1, a/0.05)`; bypassed and cleared at
  `a = 0`, so a symmetric curve is unfiltered and bit-identical. It filters
  the whole signal, so it also removes DC that the asymmetric drive curves
  before it leave. With the voices' own offset gone
  (`block-voice-mix-dc`), it changes factory presets' RMS by at most 0.4 dB.
- **Smoothing:** Softness and Asymmetry glide with τ 20 ms. The per-polarity
  constants are recomputed only while they move. Both snap before the first
  sample and on `reset()`, so a preset loaded before playback doesn't glide.
- **tanh:** a 513-point table on [0, 8] with linear interpolation, built in
  `new()`: error 2.3e-5, monotonic, 0.63× libm's cost, and the same on gnu
  and musl.
- **Meter:** the clipper tracks its curve's input and output peaks per host
  block. The plugin publishes the reduction in dB to `DiodeMeter` (an
  `AtomicU32` holding the maximum since the editor last read it). The editor
  takes it every frame and brightens the Brickwall Limiter panel's red dot
  from 30 % to full `DANGER_RED` over 0.1–6 dB, releasing at 20 dB/s. It
  repaints only while the dot is above rest.
- **CPU:** the output path is 1.030× Normal and 1.003× Render against m06
  (median of 5).
- **Migration:** preset JSON and host state without `diode_asymmetry` get
  Softness 0 and Asymmetry 0 (`migrate_diode_stage`,
  `migrate_host_diode_stage`). That is a hard clip at `L`, about 0.15 dB under
  the old curve's top. Host automation of Softness is BREAKING.

---

### Editor inline help (`add-parameter-tooltips`)

Every widget helper (`industrial_knob[_text]`, `industrial_fader[_text]`, `industrial_vfader`, `vfader_global`) paints its label with `paint_label_with_badge`: the label plus a painted ⓘ badge (1 physical px circle and "i" in `MUTED_GREY`; egui's fonts have no U+24D8). Only the badge's own hover area shows the tooltip. The text comes from `help_for(label)`, one table keyed by display label, so slot help follows the selected exciter or object. When a control's behaviour changes, update its entry. `industrial_combo` shows `selector_help(label, selected)` for the selected option: a **trial**, removable by deleting that function and the marked block. `every_drawn_control_has_help` and `every_slot_spec_has_help` check coverage; the hover tests check that the badge, not the knob, opens the tooltip; `help_follows_the_selected_exciter` / `_object`, `selector_help_describes_the_selection` and `badge_scales_with_the_ui` cover the other scenarios. Because help is keyed by label, a slot label shared between models shares its tooltip: `shared_slot_labels_are_reviewed` fails unless such a label is listed in `SHARED_SLOT_LABELS` (today only "Mallet Mass") after checking it means the same in each model.

## 10. Build, Test, and QA Infrastructure

| Component | Purpose |
|---|---|
| `.github/workflows/ci.yml` | `quality-and-build`: fmt, clippy (`-D warnings`), no-default-features tests (musl), native gnu lib tests, `render --suite family` smoke, WAV check. `bundle`: VST3 and CLAP for Linux (`bundle.sh`) and Windows (`bundle-win.sh`, mingw-w64), zipped as artifacts. `prerelease`: replaces the rolling `nightly` pre-release on pushes to main and manual runs |
| `scripts/verify-local.sh` | local mirror of CI |
| `scripts/validate-plugins.sh` | optional `pluginval` + `clap-validator` wrapper |
| `scripts/check_wav.py` / `analyze_wav.py` / `check_clicks.py` | offline artifact validation |
| `benches/performance.rs` | Criterion harness (voice, dense excitation, post chain, idle, offline, the plugin's whole output path via `offline::PresetRenderer`) |
| `tests/` | integration tests: no_alloc, automation_stress, preset_roundtrip, body, drive, stereo, velocity, chain distinctness, limiter, damage rattle, bow, plugin metrics |
| `src/bin/render.rs` | offline QA renderer incl. `--suite aliasing` → `aliasing_report.txt` |

`.cargo/config.toml` defaults the target to `x86_64-pc-windows-gnu` (the maintainer's
Windows host); on Linux set `CARGO_BUILD_TARGET`. CI tests on `x86_64-unknown-linux-musl`,
and the Linux bundle/validator flow uses `x86_64-unknown-linux-gnu`.

---

## 11. Status of Previously-Claimed Work (verified in code)

| Feature | Status | Evidence |
|---|---|---|
| Quality modes (Eco/Normal/High/Render) | Implemented | `params.rs` `QualityMode`, `post_chain.rs` `set_quality_mode`, `lib.rs` mapping |
| Configurable oversampling 1/4/8/16 | Implemented | `oversampled_clipper.rs` + `half_band.rs`: half-band IIR up/down (`m06-m09-smooth-starvation-and-oversample-clipper`) |
| Idle rendering optimization | Implemented | `Voice::rendering` flag + `is_rendering()` gating in `manager.rs` |
| Aliasing measurement | Implemented (DFT proxy) | `offline/mod.rs::analyze_post_chain_aliasing`, `render --suite aliasing` |
| Output ceiling −0.3 dBFS | Implemented | `analog_ceiling` (default 0.9661 = `LIMITER_THRESHOLD`) is the ceiling in both limiter modes |

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
- **Only the clipper is oversampled.** The Lorenz drive and the WDF filter's
  soft clip run at the host rate, so their aliasing is part of the sound: the
  whole-chain aliasing harness (drive 2.5) measures about −23 dB in every
  oversampled mode, the drive's own figure, while the clipper alone reaches
  −51 to −76 dB.
- **Single algorithmic resonator path.** The former dual path was retired
  (implemented 2026-05-27, see §5 and `docs/backlog.md`): `complex_algo` is gone,
  the per-object algorithmic generators are the sole engine, 14 per-object
  character params are exposed, and the profile tables are demoted to
  `mode_count`/budget/test metadata.

# Corrosion — Work Backlog (missing, partial, and broken)

A single prioritized list of what needs solving, derived from the 2026-05 code
review (`docs/code-review.md`) and the as-built architecture
(`docs/ARCHITECTURE.md`). Scope excludes `src/presets/`.

Status key: **[BUG]** broken behavior · **[PARTIAL]** wired but incomplete ·
**[MISSING]** specced but absent · **[CHORE]** cleanup/quality · **[VERIFY]** not
yet read in review, confirm before acting.

Each item notes the relevant file(s) and an acceptance check.

---

## Proposed (2026-11-10, from `reorganize-factory-presets`)

- [ ] **[MISSING] Release Snap control for the friction exciters.** Releasing
  Bow, Stiff Point Scrape or Corrugated Drag snaps the object free from its
  static deflection: on the default patch the tail is up to +30 dB over the
  held note (Plate + Corrugated Drag −48.1 → −17.6 dB), in the bank up to
  +37 dB (Bow · Taut Cable). The maintainer keeps it as a feature. *Proposal:*
  a Release Snap parameter, from a sudden release (today's snap) to a ramp
  that follows the MSEG release, so the snap can be bounded (e.g. ≤ +6 dB over
  the held note); the seeder then also levels the released note.
  *Accept:* at Release Snap 0 the released note is no louder than the held
  one; the default keeps today's snap or a bounded one, by ear.
- [ ] **[VERIFY] Env Amount 0 silences the friction exciters.** The MSEG output
  is multiplied by Env Amount (`mseg-envelope` spec), so at 0 Bow, Stiff Point
  Scrape, Heavy Grinding, Corrugated Drag and Tension Rise make no sound, but
  the tooltip says "Lower flattens it". Decide which is right: fix the tooltip,
  or make Env Amount blend toward a flat envelope (a sound change for presets
  below 1). Found by `print_velocity_sweep` (`fix-pneumatic-jet-saturation`).
- [ ] **[VERIFY] The default patch sits at the voice clamp for ~3 s.** A struck
  Pipe, I-Beam or Taut Cable rings at the voice's safety clamp (~0.31 RMS) for
  2–3 s, the default patch included (`reorganize-factory-presets` design).
  The percussion presets add Rust and Sludge to make strikes decay. *Accept:*
  decide by ear whether the default should decay sooner.

## P0 — Correctness defects (fix first)

- [x] **[BUG] Oversampled clipper does not oversample.** *(fixed 2026-05-27)*
  `src/dsp/post_processing/oversampled_clipper.rs`. `process()` zero-order-holds a
  single input (`os_samples.fill(input)`) and averages identical clipped copies, so
  1×/4×/8×/16× are mathematically identical. No interpolation, no anti-aliasing,
  `upsample_state`/`downsample_state` unused.
  *Fix:* real upsampling (polyphase, or at least linear interp across prev/next
  input) → clip at high rate → low-pass decimate.
  *Accept:* Eco vs Render differ at the clipper; `analyze_post_chain_aliasing`
  shows lower `alias_ratio_db` at higher factors; re-base `quality_mode_changes_oversample_factor`
  to exercise the factor (not the Eco bypass).

- [x] **[BUG] FactoryReverb comb delays mutate cumulatively.** *(fixed 2026-05-27)*
  `src/dsp/post_processing/space.rs::update_delays` re-scales `comb_delays` from
  their own prior values, and `lib.rs` calls it every sample → delays rail to the
  clamp bounds; `factory_size` is effectively dead.
  *Fix:* keep immutable base delays and recompute `comb_delays = base · size_scale`
  each update (mirror `SpringReverb::update_delay`).
  *Accept:* sweeping `factory_size` changes the tail length monotonically; add a test.

- [x] **[BUG/PERF] Post-chain parameters set per sample.** *(fixed 2026-05-27)*
  `src/lib.rs::process` calls ~10 `post_chain.set_*` setters inside the sample loop;
  several recompute coefficients (`tan()`, FEM freqs, reverb delays) at audio rate.
  Root cause of the FactoryReverb bug and wasted CPU.
  *Fix:* apply setters once per buffer (or only on change); keep coefficient
  recompute out of the per-sample loop.
  *Accept:* `post_processing_chain` benchmark improves; FactoryReverb fix holds.

---

## Approved change — Algorithmic resonator engine (decided 2026-05, IMPLEMENTED 2026-05-27)

Decision (with sign-off): promote the per-object **algorithmic** resonator path to
the **only** path, remove the `complex_algo` toggle, and expose a curated set of
per-object "character" parameters. Profile tables (`src/dsp/profiles/`) are kept as
`mode_count` / budget / test metadata only — they stop driving sound. The default
timbre of every object will change (accepted). See `docs/ARCHITECTURE.md` §5 and
`docs/detailed-specs/resonator-algorithms.md` for the design.

*Implemented 2026-05-27: algorithmic path is now the sole resonator engine,
`complex_algo` is gone, all 14 character params are exposed and threaded, Chain/Tank
pitch-track the note, and the Cable/Sheet dynamic hooks run per sample (verified
allocation-free by `tests/no_alloc.rs`). All checks below pass.*

- [x] **[CHORE] Remove `complex_algo`.** Dropped the param + `complex_algo_param`
  (`src/params.rs`), the `VoiceControls` field + the `if controls.complex_algo != 0`
  branch (`src/voice/mod.rs`), the snapshot (`src/lib.rs`), and the GUI
  toggle (`src/gui/editor.rs`). Resonator construction is now unconditionally
  `with_algorithm_controls_and_note`.

- [x] **[MISSING] Expose 14 curated per-object character params** and thread them
  `CorrosionParams` → `VoiceControls` → `generate_algorithm_modes` (replacing the
  `::default()` generator construction in `src/dsp/resonators/core.rs:178`):

  | Object | UI name | id | range | default | generator field |
  |---|---|---|---:|---:|---|
  | Pipe | Pipe Diameter | `pipe_diameter` | 0–1 | 0.5 | `tube_diameter` |
  | Plate | Plate Aspect | `plate_aspect` | 0.1–4.0 | 1.0 | `aspect_ratio` |
  | Plate | Plate Stiffness | `plate_stiffness` | 0.25–3.0 | 1.0 | `metal_stiffness` * |
  | Tank | Tank Volume | `tank_volume` | 0–1 | 0.5 | `tank_volume` |
  | Tank | Cavity Mix | `tank_cavity_mix` | 0–1 | 0.6 | `cavity_mix` |
  | Chain | Link Mass | `chain_link_mass` | 0.1–1.0 | 0.5 | `link_mass` |
  | Chain | Instability | `chain_instability` | 0–1 | 0.3 | `instability` |
  | IBeam | Shear Density | `beam_shear` | 0–1 | 0.5 | `shear_density` |
  | TautCable | Braid Stiffness | `cable_braid` | 0–1 | 0.3 | `braid_stiffness` |
  | TautCable | Tension Drop | `cable_tension_drop` | 0–1 | 0.4 | `tension_drop` |
  | CoilSpring | Dispersion Chirp | `spring_dispersion` | 0–1 | 0.5 | `dispersion_chirp` |
  | CoilSpring | Spring Slosh | `spring_slosh` | 0–1 | 0.3 | `spring_slosh` |
  | SheetMetal | Metal Thinness | `sheet_thinness` | 0–1 | 0.4 | `metal_thinness` |
  | IndustrialCog | Tooth Dissonance | `cog_dissonance` | 0–1 | 0.1 | `tooth_dissonance` |

  \* `metal_stiffness` is already the Metal Pipe **exciter** param id, so the Plate
  control ships under id `plate_stiffness`. Dropped as redundant with global
  Size/Damping/Brightness: `sustain_time`, `wall_thickness`, `friction_decay`,
  `chain_length`, `beam_mass`, `rigidity_damping`, `cable_tension`, `coil_length`,
  `sheet_size`, `edge_damping`, `blade_radius`, `blade_thickness`.

- [x] **[BUG] Fix algorithmic pitch tracking.** `ChainResonator::generate_modes`
  now anchors its GOE cluster to `fundamental_hz` (heavier `link_mass` shifts it
  lower) and `TankResonator`'s cavity mode follows the note as a deep sub-octave
  (`basic.rs`); both track the MIDI note.

- [x] **[MISSING] Wire the dynamic hooks** into the per-sample resonator loop
  (`ModalResonator::process_sample[_stereo]`): the cable tension-drop
  (amplitude→pitch "boing") and sheet-metal buckling warp now run every sample.
  The resonator retains the relevant algorithm instance (`Dynamics` enum) plus a
  smoothed amplitude / low-frequency estimate, recomputing affected mode
  coefficients from each mode's immutable base frequency (no cumulative drift).
  Verified allocation-free by `tests/no_alloc.rs`.

- [x] **[CHORE] Preset migration.** A legacy `complex_algo` field is ignored on
  load (serde drops the unknown key); the 14 new fields carry serde defaults so
  existing presets still open (`src/presets/mod.rs`). Covered by
  `legacy_complex_algo_field_is_ignored_on_load`.

- [x] **[CHORE] Profiles → metadata only.** `*_MODAL_PROFILE_MODES` now feed only
  `mode_count` (via `ModalProfile::from_id(..).mode_count()` in
  `generate_algorithm_modes`); they no longer drive sound generation.

- [x] **[CHORE] Tests & docs.** Replaced `complex_algo_toggle_changes_resonator_output`
  with `character_param_changes_resonator_output`, added `chain_resonator_tracks_pitch`
  and `taut_cable_dynamic_hook_changes_output` (`src/voice/mod.rs`); updated
  `tests/preset_roundtrip.rs` (character-param roundtrip + legacy-field migration);
  reconciled docs (`ARCHITECTURE.md` §5, `resonator-algorithms.md`).

> **Interaction with P1 "held-note automation":** these new params are snapshotted
> at note-on like every other voice control, so they will not update during a
> sustained note until that separate limitation is addressed. The Cable/Sheet
> dynamic hooks above *do* update per sample because they are driven by the
> resonator's own running amplitude, not by parameter automation.

---

## P1 — Partially implemented (finish or formally scope down)

- [x] **[PARTIAL] Quality modes are shallow.** *(fixed 2026-05-28)*
  Eco bypassed body/spread/space via a fixed `0.3/0.7` mix and the oversample
  factor was broken (now fixed in P0). The resonator mode count was also fixed.
  *Fix:* QualityMode now multiplies the per-object mode count at note-on (Eco
  0.5×, Normal 1.0×, High 1.5×, Render 2.0×) via
  `VoiceControls.mode_count_scale`, threaded through
  `with_algorithm_controls_and_note`. Documented in `ARCHITECTURE.md` §12.
  *Accept:* `mode_count_scale_changes_resonator_density` proves Render produces
  more modes than Eco.

- [x] **[PARTIAL] Aliasing measurement is a proxy, with no budget.** *(fixed 2026-05-28)*
  `offline::analyze_post_chain_aliasing` reported residual energy with no
  asserted threshold or oversampled-reference comparison. *Fix:* refactored the
  analyzer into `analyze_post_chain_aliasing_at_quality(.., PostQualityMode)`
  and added two regressions in `src/offline/mod.rs::tests`:
  `render_mode_alias_ratio_stays_within_budget` fails if Render's
  `alias_ratio_db` exceeds -10 dB, and `higher_quality_reduces_alias_ratio`
  fails if Render does not strictly beat Eco — which is exactly what the P0
  oversampled-clipper fix unlocks.

- [x] **[PARTIAL] Held-note automation has no effect.** *(fixed 2026-05-28)*
  `VoiceControls` was snapshotted at note-on, so changing exciter/resonator
  parameters during a sustained note did nothing until the next note.
  *Fix:* `Voice::update_live_controls` and `VoiceManager::update_live_controls`
  push damping, brightness, strike position, coupling, wander, envelope, and
  fundamental anchor into active voices once per buffer from `lib.rs::process`.
  Tail voices keep their note-on snapshot so decays stay consistent. The
  resonator now stores `base_decay_seconds`/`base_gain` so damping/brightness
  re-derive from the note-on baseline instead of compounding
  (`src/dsp/resonators/core.rs`). Drive was already buffer-live in `apply_drive`.
  *Accept:* `held_note_damping_automation_shortens_decay` and
  `held_note_automation_skips_released_voices`; `no_alloc` still green.

- [x] **[PARTIAL] No parameter smoothing.** *(fixed 2026-05-28)*
  Neither NIH-plug smoothing nor internal smoothing was applied to audibly-stepping
  params (filter cutoff, drive, strike position) so rapid automation could zipper.
  *Fix:* added one-pole audio-rate smoothing (~20 ms tau) to the WDF filter
  (`cutoff_hz`/`resonance`), `LorenzDrive::drive_amount`, plus a top-level
  `OnePoleSmoother` in `lib.rs` smoothing master drive and output gain
  per-sample. First call after `new`/`reset` snaps so initial configuration
  takes immediate effect (no audible startup ramp). Held-note strike-position
  automation lands via the per-buffer `update_live_controls` path.
  *Accept:* new `mid_stream_cutoff_change_is_smoothed` test verifies the WDF
  smoother lags a snapped reference; all prior parameter-response tests still
  pass.

- [ ] **[PARTIAL] MSEG is not a routable modulation source.** *(scoped down
  2026-05-28 — promoted to P2)*
  `src/dsp/envelopes/mod.rs` MSEG only drives the force envelope for friction
  voices; making it a routable mod source needs per-destination depth params,
  a routing GUI, and per-voice MSEG instances on all families. *Decision:*
  treat this as a single deliverable with the P2 mod-matrix / macro work
  (PRD §12) rather than a P1 finish-it task — completing it here in isolation
  would ship half a routing system.

- [x] **[PARTIAL] `sync_rate` parameter appears unused.** *(fixed 2026-05-28)*
  `sync_rate` now drives the FactoryEcho. Below `0.05` the echo runs free
  with the existing `delay_time` knob. Above that the value selects one of
  six musical divisions (1/16 → 2/1) and the effective delay is computed
  from `context.transport().tempo`. If the host doesn't report a BPM the
  knob silently falls back to free-running mode. Wiring lives in
  `lib.rs::sync_rate_to_delay_time` (table-driven, allocation-free) and is
  consumed at the per-buffer `set_echo_params` call. Tests:
  `sync_rate_quarter_note_at_120bpm`, `sync_rate_below_threshold_returns_free_running_value`,
  `sync_rate_without_tempo_falls_back_to_knob`, `higher_sync_rate_gives_longer_delay`.
  MSEG tempo-sync remains future work — the sequencer feature (P2.3,
  scoped down) is the natural home for that.

- [x] **[PARTIAL] Post/space DSP are approximations of the detailed specs.**
  *(decision recorded 2026-05-28: keep approximations as the contract)*
  WDF (4-pole ladder, not a circuit solver), FEM body (8 modes), HRTF
  (delay+filter, no HRIR), Factory/Spring reverbs (Schroeder/single-delay, not
  FDTD/PDE). *Decision:* the lightweight approximations stay as the shipped
  contract. The spec docs in `docs/detailed-specs/post-processing.md` describe
  the high-fidelity target; the in-source module docstrings and
  `ARCHITECTURE.md` §9 are authoritative about what actually ships. Upgrading
  any single stage to the full PDE/FDTD / HRIR description would be a feature
  proposal in its own right, not a P1 finish-the-partial task.

---

## P2 — Missing features (specced in PRD/master-prompt, not present)

- [x] **[MISSING] MIDI expression beyond note on/off.** *(fixed 2026-05-28)*
  `MIDI_INPUT` upgraded to `MidiConfig::MidiCCs`. `handle_note_event` now
  routes `MidiPitchBend` (±2 semitones default, ±24 safety clamp),
  `MidiChannelPressure`, `PolyPressure`, and CC1 `MidiCC` mod wheel.
  `ModalResonator::set_pitch_bend_factor` rebuilds biquads from each mode's
  immutable `base_frequency_hz`; the bend layers on top of the cable/sheet
  dynamic hooks (`src/dsp/resonators/core.rs::apply_dynamics`). VoiceManager
  stores channel-state so notes triggered mid-bend pick it up. Tests:
  `pitch_bend_retunes_held_resonator_modes`, `channel_pitch_bend_reaches_all_active_voices`,
  `channel_pressure_increases_voice_output`, `mod_wheel_boosts_output_independently_of_pressure`,
  `poly_pressure_overrides_channel_pressure_when_higher`,
  `pitch_bend_persists_for_new_notes_on_channel`,
  `poly_pressure_targets_only_matching_note`.
- [x] **[MISSING] Macro controls** (Mass, Corrosion, Violence, Brightness; PRD §12).
  *(fixed 2026-05-28)*
  Four `0..=1` macro params (`macro_mass`, `macro_corrosion`, `macro_violence`,
  `macro_brightness`) sit at the host surface. `lib.rs::resolve_macro_bias`
  derives a `MacroBias` once per buffer from the current macro values; the
  bias is applied to the `VoiceControls` mass cluster + damping/brightness at
  note-on, to `size`/`rust`/`damage` at the note_on_with_controls call site,
  to the post-chain drive / drive_amount / chaos_depth / filter cutoff in
  `process()`, and to held-note damping/brightness via `update_live_controls`.
  At `0.5` (default) the bias is mathematically NEUTRAL so existing presets
  sound unchanged — see `neutral_macros_produce_neutral_bias`. Damage is the
  destination of the Violence macro (matches PRD intent). Preset roundtrip
  persists the macros; legacy presets default to neutral via
  `default_macro_value`.
- [ ] **[MISSING] Sequencer + per-step locks** (PRD §18). *(scoped down
  2026-05-28 — deferred from P2)*
  Implementing PRD §18 needs a full sequencer module: step storage, per-step
  parameter snapshots ("locks"), host-transport synchronization, a sequencer
  GUI panel, and preset persistence. *Decision:* defer to a dedicated
  feature initiative; building a half-sequencer in this sweep would ship a
  partial UI surface that would need to be redesigned. P1.6 sync_rate
  tempo-sync wiring (currently inert) is a prerequisite once the sequencer
  picks up host BPM. No code change in this iteration.
- [x] **[MISSING] Percussion Kit mode** (note→object mapping; PRD §9.2).
  *(fixed 2026-05-28)*
  New `PlayMode` enum (`Tonal` / `Kit` / `Drone`) and `play_mode` IntParam
  control note-on routing in `lib.rs::handle_note_event`. Kit mode uses
  `note_to_kit_object` to map MIDI notes into 9 contiguous slots of 10 notes
  each across the playable range; the note still drives pitch within each
  slot. Tests: `kit_note_to_object_covers_full_range`,
  `kit_mode_overrides_object_param`.
- [x] **[MISSING] Explicit Drone mode** (PRD §9.3). *(fixed 2026-05-28)*
  `PlayMode::Drone` forces `controls.loop_mode = Forward` with
  `loop_start_stage = 3` (Decay) and `loop_end_stage = 4` (Sustain) at
  note-on so friction MSEG voices ring indefinitely until note-off. Hit-style
  exciter families intentionally keep their one-shot envelope — looping
  them would require either a per-voice retrigger clock or a forced family
  swap, both of which were judged out of scope for this iteration. Test:
  `drone_mode_forces_mseg_loop_on`. `MSEG::is_loop_enabled` added so tests
  can confirm the wiring.
- [ ] **[MISSING] Effect-mode variant** (audio-in excites the resonator; PRD §9.4).
  *(scoped down 2026-05-28 — deferred from P2)*
  Requires switching `AUDIO_IO_LAYOUTS` to add a stereo input, threading the
  input buffer into the per-sample exciter dispatch, gating the existing
  MIDI exciter path when Effect mode is active, and stabilizing feedback
  (the resonator + audio-in form a closed loop). Each of those is a
  meaningful surface change. *Decision:* defer until the audio-in plumbing
  can be designed end-to-end — building it incrementally on top of the
  instrument I/O layout would either ship a non-working mode or commit to
  an interface we'd need to redesign once feedback safety is added.
- [x] **[MISSING] Output options:** lookahead limiter / output meter / selectable
  soft-clip modes (PRD §20.3 future). *(partially fixed 2026-05-28 —
  lookahead limiter shipped; meter + soft-clip variants remain)*
  New `LookaheadLimiter` (`src/dsp/post_processing/lookahead_limiter.rs`):
  48-sample window (~1 ms @ 48 kHz), instant attack, 50 ms one-pole release,
  threshold tracks `analog_ceiling`. New `limiter_mode` IntParam (`Hard` /
  `Lookahead`) selects which path the master output takes. When Lookahead
  is engaged the plugin reports its 48-sample latency to the host via
  `context.set_latency_samples`, and only when the value changes so hosts
  aren't asked to renegotiate buses every buffer. Tests live in the
  limiter module: `passes_quiet_signal_unchanged_after_latency`,
  `limits_peaks_above_threshold`, `detects_peak_before_it_arrives_at_output`,
  `reset_clears_delay_line`. The output meter is a GUI surface concern and
  the soft-clip variants (alternative tube/diode curves) remain future
  work — both intentionally out of scope for this iteration.
- [x] **[MISSING] Preset schema migration** (versioned remap of old IDs; currently
  sanitize-and-clamp only). *(fixed 2026-05-28)*
  `PRESET_VERSION` bumped to `"4"`. New `migrate_preset_json` runs on the
  raw `serde_json::Value` before typed deserialization so renames and
  value remaps that can't be expressed via `#[serde(default)]` have a place
  to live. Currently handles: unstamped legacy → `"1"`, `"1"`/`"2"` →
  `"3"` (inject empty `extra` object), `"3"` → `"4"` (additive macros /
  play_mode / limiter_mode, no JSON rewrite). `Preset::from_json_str` routes
  through the migration; host state goes through nih-plug's `Params`. Tests:
  `unstamped_legacy_preset_migrates_to_current_schema`,
  `v3_preset_migrates_to_v4_without_dropping_fields`,
  `migration_idempotent_on_current_version`.

---

## P3 — Code quality / cleanup chores

- [x] **[CHORE] Remove dead code.** *(fixed 2026-05-28)*
  Dropped the unreachable `exciter_type == 0` branch in
  `Voice::process_sample_stereo` (ids are 1..=16); removed
  `ModalResonator::{last_output, current_output}` fields and their writes
  (no external reader); removed `InteractionState::{update_resonator_state,
  distribute_force_to_modes}` and `BidirectionalInteractionBus::{distribute_force,
  per_mode_forces}` (allocated `Vec<f32>` on the audio thread, never called);
  removed `mode_coefficient_2d` and `mode_coefficient_circular` (specced for
  higher-fidelity object models that the algorithmic resonator path now
  supersedes). `OversampledClipper::{upsample_state, downsample_state}` were
  already gone (replaced by `prev_input` during the P0 oversample fix).
  Updated `bidirectional_bus_lifecycle` test to verify the live mode-coeff
  cache instead of the removed force-distribution API.
- [x] **[CHORE] Fix `src/lib.rs` docstring "17 different exciter types" → 16.**
  *(fixed 2026-05-28)*
- [x] **[CHORE] Fix `src/params.rs` docstring "70+ parameters" → 130+.**
  *(fixed 2026-05-28)* The actual count grew past 130 with the macros,
  play_mode, and limiter_mode additions.
- [x] **[CHORE] Declare `render_presets` in `Cargo.toml`** *(fixed 2026-05-28)*.
- [x] **[CHORE] Rename/reword the "WDF" filter** *(fixed 2026-05-28)*
  Type name retained for ABI/preset stability; module + struct docstrings
  rewritten to make clear it is a TPT-flavored four-pole ladder
  approximation with transistor saturation, not a circuit-solver WDF.
  Points at the post-processing spec for the contractual response.
- [x] **[CHORE] Bound `midi_to_hz`** input *(fixed 2026-05-28)*.
  `u8` already caps the upper end (note 127 → 12.5 kHz), but the function
  now explicitly clamps via `.min(127)` so the math is robust if a future
  caller passes a wider integer type.
- [x] **[CHORE] Extract DSP magic numbers** *(partially fixed 2026-05-29)*
  Named the eight drive-curve thresholds in `lib.rs::apply_drive`:
  `DRIVE_GAIN_SCALE`, `DRIVE_POS_SOFT_THRESHOLD` / `DRIVE_POS_HARD_THRESHOLD`
  / `DRIVE_POS_CEILING`, `DRIVE_NEG_SOFT_THRESHOLD` / `DRIVE_NEG_HARD_THRESHOLD`
  / `DRIVE_NEG_CEILING`, `DRIVE_OUTPUT_CLAMP`. Body/reverb constants live
  inside their own modules and are already commented at the call site; the
  decision is to extract them only when they recur across functions, which
  they do not today.

---

## P4 — Testing & release infrastructure

- [x] **[MISSING] Aliasing/spectral regression thresholds** *(fixed 2026-05-28
  as part of P1.2)*. `offline::tests::render_mode_alias_ratio_stays_within_budget`
  fails if Render's `alias_ratio_db` exceeds -10 dB; `higher_quality_reduces_alias_ratio`
  fails if Render does not strictly beat Eco. Harness lives in `src/offline/mod.rs`.
- [ ] **[MISSING] Windows CI lane / bundle smoke** (CI is Linux-only; `bundle-win.sh` untested in automation).
- [ ] **[MISSING] DAW smoke automation** (REAPER/Bitwig/Ardour/Live/FL) — currently manual.
- [ ] **[MISSING] GUI interaction/visual regression tests** (headless only today).
- [ ] **[CHORE] Persist benchmark baselines** so Criterion runs can flag regressions.
- [ ] **[CHORE] `assert_process_allocs` is disabled on Windows** (`Cargo.toml` target cfg) — document or enable.
  *Status note 2026-05-29:* the disabled lane is intentional — the
  `assert_no_alloc` upstream crate uses a thread-local allocator hook that
  conflicts with Windows' default CRT during nih-plug's test setup, so
  enabling it without a CRT swap causes process-wide hangs. Linux + macOS
  CI lanes still run `tests/no_alloc.rs` with the feature on. A Windows fix
  would require either swapping to `mimalloc`/`jemalloc` or upstreaming
  Windows support into `assert_no_alloc`; both are out of scope until a
  Windows CI lane lands.

---

## Needs verification before estimating (not read in this review)

- [ ] **[VERIFY] `src/dsp/envelopes/mod.rs`** — MSEG loop / ping-pong correctness
  (params docstring calls PingPong "partial").
- [ ] **[VERIFY] `src/gui/editor.rs`** — quality-mode selector wiring, knob drag,
  preset apply path.
- [ ] **[VERIFY] `src/randomizer/mod.rs`** — that randomization respects the safe
  ranges / constraints in PRD §17.3.
- [ ] **[VERIFY] Per-exciter DSP** for all 16 models and the algorithmic resonators
  (`coil_spring`, `ibeam`, `industrial_cog`, `sheet_metal`, `taut_cable`) — review
  trusted dispatch + docstrings, not every formula.

---

## Suggested order of attack

1. P0 #1–#3 (correctness; small, localized, unblocks quality modes + aliasing).
2. P1 held-note automation + smoothing (biggest playability/expressiveness win).
3. P3 cleanup (cheap, reduces confusion).
4. Decide the P1 "approximation vs upgrade" question for post DSP — it determines
   whether the detailed specs are a contract or a wishlist.
5. P2 features by product priority (MIDI expression and macros likely first).

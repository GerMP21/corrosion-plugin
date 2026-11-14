# Corrosion — Work Backlog

What is still open. Finished items are removed: their history is in the
OpenSpec archive (`openspec/changes/archive/`), the archived code reviews
(`docs/archive/CODE-REVIEW-2026-05-27.md`,
`docs/archive/FABLE-CODE-REVIEW-2026-07-07.md`) and git. The open findings of
the current review, `FABLE-CODE-REVIEW-2026-11-14.md`, are listed below with
their IDs; the review has the measurements and suggested fixes.

Status key: **[BUG]** broken behavior · **[PARTIAL]** wired but incomplete ·
**[MISSING]** specced but absent · **[CHORE]** cleanup/quality · **[VERIFY]**
needs a decision or a check before acting.

Each item notes the relevant file(s) and an acceptance check.

---

## Sound and behaviour

- [ ] **[MISSING] Release Snap control for the friction exciters.** Releasing
  Bow, Stiff Point Scrape or Corrugated Drag snaps the object free from its
  static deflection: on the default patch the tail is up to +30 dB over the
  held note (Plate + Corrugated Drag −48.1 → −17.6 dB), in the bank up to
  +37 dB (Bow · Taut Cable). The maintainer keeps it as a feature. *Proposal:*
  a Release Snap parameter, from a sudden release to a ramp (since
  `fix-inert-controls` the friction exciters keep their force through the
  MSEG release, so the snap follows MSEG Release), so the snap can be bounded (e.g. ≤ +6 dB over
  the held note); the seeder then also levels the released note.
  *Accept:* at Release Snap 0 the released note is no louder than the held
  one; the default keeps today's snap or a bounded one, by ear.
- [ ] **[VERIFY] Real stick-slip for the Bow.** The bow never sticks: the
  resonator moves at 0.0002–0.007 against a bow speed near 1, so it stays on
  the sliding branch (Slip Curve shapes that branch since
  `fix-inert-controls`). Seeing the resonator ×1000 makes it stick and slip,
  but the 12 bowed chord presets fall into the raucous regime (a ~20 Hz
  subharmonic, partials up to 600 cents off, −16 to −35 dB). *Accept:* a
  stick-slip bow with the bowed presets re-voiced by ear, passing the bank's
  character checks.
- [ ] **[VERIFY] Env Amount 0 silences the friction exciters.** The MSEG output
  is multiplied by Env Amount (`mseg-envelope` spec), so at 0 Bow, Stiff Point
  Scrape, Heavy Grinding, Corrugated Drag and Tension Rise make no sound. The
  tooltip said "Lower flattens it"; since 2026-11-14 it says what it does
  (`write-user-guide-content`). Left to decide: keep it, or make Env Amount
  blend toward a flat envelope (a sound change for presets below 1). Found by
  `print_velocity_sweep` (`fix-pneumatic-jet-saturation`).
- [ ] **[PARTIAL] Gesture Modifiers show for every exciter but act only on the
  friction ones.** Env Amount, Vel To Peak, Global Time, Vel To Level, Vel To
  Time and Curve configure the friction MSEG only (`Voice::configure_envelope`);
  the struck exciters' AD and the specialty exciters' ADSR ignore them, yet the
  panel is drawn for all 16. Found while writing the guide's exciter pages
  (`write-user-guide-content`; the help texts now say "friction envelope").
  *Accept:* the panel shows only for friction exciters, or the modifiers act
  on the AD and ADSR envelopes too (a sound change), with a test either way.
- [ ] **[VERIFY] The default patch sits at the voice clamp for ~3 s.** A struck
  Pipe, I-Beam or Taut Cable rings at the voice's safety clamp (~0.31 RMS) for
  2–3 s, the default patch included (`reorganize-factory-presets` design).
  The percussion presets add Rust and Sludge to make strikes decay. *Accept:*
  decide by ear whether the default should decay sooner.
- [ ] **[PARTIAL] MSEG PingPong loop ignores the loop stages.** It always loops
  Decay → Sustain, like Forward 3 → 4 (its help says so since 2026-11-14) (`src/params.rs` `loop_mode` doc,
  `src/dsp/envelopes/mod.rs`). *Accept:* PingPong runs forward then backward
  between Loop Start and Loop End, with a test; or the mode is removed with a
  preset and host migration.
- [ ] **[PARTIAL] The web editor's canvas is cut off at a device pixel ratio
  above 1.** `web/editor/src/lib.rs` sets the canvas's CSS height to
  `content_height() * zoom_factor()`, which is right only at a ratio of 1: at
  1.5 the 1280 px editor's canvas is 939 px tall instead of 1408, at 2 it is
  705 px, so the bottom panels and the keyboard are cut off and the text is
  blurry (HiDPI laptops, phones). Found while capturing the guide's
  screenshots (`write-user-guide-content`, which shoots at a ratio of 1).
  *Accept:* at ratios 1, 1.5 and 2 the whole editor shows, sharp, with a test
  or a Playwright check per ratio.
- [ ] **[VERIFY] The preset browser names a preset the editor hasn't loaded.**
  At startup the header shows "Bass / Bowed Cable Sub"
  (`selected_factory_preset: 0`, `src/gui/editor.rs`) while the default patch
  plays. *Accept:* the header shows the default patch (e.g. "Init") until a
  preset is loaded, and a restored project shows its own name or none.

## From the 2026-11-14 code review

IDs are the review's (`FABLE-CODE-REVIEW-2026-11-14.md`). When one is fixed,
mark it FIXED there and remove it here.

- [ ] **[BUG] C1. The specialty ADSR never releases with Sustain 0.** Pneumatic
  Jet, Electromagnetic Hum and Particle Rain released during Attack or Decay
  with Env Sustain 0 hold their level forever (the release slope is
  `sustain / release`); with a low sustain, Release lasts level ÷ sustain times
  longer than set (`src/voice/mod.rs:931-951`). *Accept:* the release ramps from
  the level at note-off; a note released in Decay with Sustain 0 ends within
  Release plus a margin, with a test; bank tests pass; Wiki ADSR and
  Troubleshooting updated.
- [ ] **[BUG] R1. Switching Space mode replays a stale tail and cuts the wet
  signal.** Inactive Space processors keep their buffers; Factory came back at
  −25 dBFS from silence (`src/dsp/post_processing/post_chain.rs:224-257`).
  *Accept:* a mode re-entered after silence starts silent, and the switch
  crossfades or clears, with a test.
- [ ] **[BUG] R2. Space Amount steps once per block.** It has no smoother, unlike
  BODY, Drive, Output and the filter (`post_chain.rs:160`). *Accept:* a 20 ms
  smoother, identical output when static, with a test.
- [ ] **[BUG] R3. Width is discontinuous at 0.** A 100 Hz mono tone drops
  3.7 dB between Width 0 and 0.001 (`src/dsp/post_processing/hrtf_spread.rs:59`).
  *Accept:* the level and tone move continuously from 0, with a test; Wiki
  Width text updated.
- [ ] **[BUG] R4. Rate-dependent filter constants.** The HRTF proximity
  one-poles (`hrtf_spread.rs:88-93`) and the spring's dispersion
  (`src/dsp/post_processing/space.rs:253`) use raw per-sample constants.
  *Accept:* converted with `dsp::utils::rate`, bit-identical at 48 kHz, covered
  by the rate-invariance tests.
- [ ] **[BUG] R5. Corrugated Drag's ridge position loses precision.** It grows
  without wrapping in `f32`, so the ridge rate drifts on notes held a minute or
  more (`src/dsp/exciters/corrugated_drag.rs:68`). *Accept:* the position wraps
  by the ridge spacing; bank tests pass.
- [ ] **[VERIFY] R6. Drone mode's forced loop steps the friction envelope.** The
  Decay→Sustain loop restarts Decay at peak: 0.5 → 0.75 every 0.3 s at the
  defaults (`src/lib.rs:716`, `src/dsp/envelopes/mod.rs:309`). *Accept:*
  decided by ear (smooth restart, ramp, or no forced loop), bank tests and Wiki
  Drone page updated.
- [ ] **[BUG] R7. A kit pad's level is unclamped from files.** `"level_db": 800`
  gives an infinite gain (`src/kit/mod.rs:197`). *Accept:* clamped on load,
  with a test in `kit/tests.rs`.
- [ ] **[VERIFY] R8. The Lorenz drive's saturator is non-monotonic.** Above 1.5
  in, more input gives less output (1.0 at 1.5, 0.71 at 5)
  (`src/dsp/post_processing/lorenz_drive.rs:193`). *Accept:* kept as a
  documented foldback (ARCHITECTURE, Wiki Drive page), or made monotonic with
  bank tests and a reseed.
- [ ] **[CHORE] O1–O4. Per-block and per-sample waste.**
  - O1: the post chain is reconfigured every block (`src/lib.rs:1052`).
  - O2: the FEM body divides per sample and runs at BODY 0.
  - O3: the filter calls `tan()` every sample while Component Tolerance is on.
  - O4: strike-position wander recomputes every mode's `sin` per sample.

  *Accept:* each change is bit-identical (render hashes) or measured, with
  `scripts/bench_ratios.py`, medians of 5.
- [ ] **[BUG] O5. Delay-length controls click when moved.** Echo Delay Time,
  Factory Size and Spring Tank Size jump their read heads (`space.rs:108`,
  `:231`, `:402`). *Accept:* the delay glides (the echo already interpolates);
  no step above the dry signal's in a sweep test.
- [ ] **[CHORE] O6. Split the largest files.** `editor.rs`, `voice/mod.rs`,
  `lib.rs`, `params.rs` and `core.rs` are 2 700–6 900 lines; move test modules
  to sibling files and split the editor by panel. *Accept:* no behaviour
  change, render hashes unchanged.
- [ ] **[CHORE] O7. Merge the duplicated mono and stereo render paths**
  (`voice/mod.rs:1832`/`:1908`, the loops in `core.rs`). *Accept:* one inner
  function, bit-identical output, CPU within noise.
- [ ] **[CHORE] O8. Enum mapping in `apply_post_chain_params`.** Move the
  `SpaceMode` and `PostQualityMode` integer mapping beside the enums
  (`src/lib.rs:1077-1092`).
- [ ] **[VERIFY] O9. Changing Quality clears the clipper's filters**, a short
  discontinuity. *Accept:* a short crossfade, or a line in the Quality tooltip.
- [ ] **[VERIFY] O10. The Factory reverb is mono** (`post_chain.rs:247`).
  *Accept:* kept by choice, or offset right-channel combs (a sound change).
- [ ] **[CHORE] N1–N11. Tidy-ups:**
  - N1: the dead `match self.profile` in `core.rs:1753`.
  - N2: the misleading defaults written by `rebuild_in_place`.
  - N3: the test-only `calculate_mode_coefficients` and the P3.1 comments.
  - N4: the `process_exciter_stereo` wrapper and its stale comment.
  - N5: the redundant `set_sample_rate` calls at note-on.
  - N6: the unreachable ADSR Sustain branch.
  - N7: the MSEG doc order and the per-sample `dt`.
  - N8: the empty plugin URL and EMAIL.
  - N9: `expect` on pad data.
  - N10: document the safety clamps.
  - N11: bound the size of imported files.

## Left open by the 2026-07-07 review

- [ ] **[VERIFY] M14. Quality changes don't reach held notes.** The clipper's
  oversampling changes at once, but a voice keeps the mode count it started
  with (documented). *Accept:* rebuild held notes on a Quality change without a
  click, or keep it documented by choice.
- [ ] **[VERIFY] M17. The damage rattle is shared mono white noise.**
  High-passing or decorrelating it was left undone because it changes the sound
  (`src/voice/mod.rs`, `rattle_noise`). *Accept:* decided by ear.
- [ ] **[VERIFY] HRTF spread's mono remix (noted under C5).** The spread rebuilds
  both channels from a filtered mid plus the side scaled by `1 − width/2`, with
  crossfeed growing with Width, so raising Width narrows the voices' own stereo
  image (`src/dsp/post_processing/hrtf_spread.rs:83-107`). *Accept:* Width
  widens or keeps the dry image, with a test (a sound change: bank tests), or
  the behaviour is documented in the Wiki.

## Features (deferred)

- [ ] **[MISSING] MSEG as a routable modulation source** (PRD §12). Today the
  MSEG only drives the force envelope of the friction voices. Needs
  per-destination depths, a routing GUI and per-voice MSEGs on every family;
  to be done as one mod-matrix change, not piecemeal.
- [ ] **[MISSING] Sequencer with per-step locks** (PRD §18): step storage,
  per-step parameter snapshots, host-transport sync, a GUI panel, preset
  persistence. A dedicated feature change.
- [ ] **[MISSING] Full MIDI CC implementation.** Only CC1 (mod wheel), CC64
  (sustain), CC120, CC121 and CC123 act today (`src/lib.rs`); every other CC
  is ignored. Needs CC-to-parameter mapping with MIDI learn, the mapping saved
  in presets and projects, and real-time-safe dispatch. *Accept:* any CC can
  be learned onto any automatable parameter, moves it on held notes, survives
  a preset and a project round trip, with tests and `no_alloc` coverage.
- [ ] **[MISSING] Effect mode** (PRD §9.4): audio input excites the resonator.
  Needs a stereo input in `AUDIO_IO_LAYOUTS` (today instrument-only), the
  input threaded into the exciter dispatch, the MIDI exciters gated, and
  feedback stabilised (resonator plus audio-in is a closed loop).
- [ ] **[MISSING] Output level meter** (PRD §20.3). The header has the Output
  fader and the diode stage's clip indicator (`DiodeMeter`), but no level
  meter. *Accept:* a peak/RMS meter fed lock-free from the audio thread, with
  a GUI test.

## Testing and release infrastructure

- [ ] **[MISSING] DAW smoke automation** (REAPER / Bitwig / Ardour / Live / FL).
  Manual today: REAPER on Linux, in the development sessions (`docs/REAPER_TESTING.md`).
- [ ] **[MISSING] Visual regression tests for the GUI.** The GUI lane drives
  the editor headlessly (interaction, hover, layout and a few pixel checks),
  but nothing compares rendered screenshots against references.
- [ ] **[CHORE] Persist benchmark baselines** so Criterion runs can flag
  regressions. `scripts/bench_ratios.py` compares twins within one run only.

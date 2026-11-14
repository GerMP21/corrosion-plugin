# Corrosion — Work Backlog

What is still open. Finished items are removed: their history is in the
OpenSpec archive (`openspec/changes/archive/`), the code reviews
(`docs/archive/CODE-REVIEW-2026-05-27.md`, `FABLE-CODE-REVIEW-2026-07-07.md`)
and git.

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
  a Release Snap parameter, from a sudden release (today's snap) to a ramp
  that follows the MSEG release, so the snap can be bounded (e.g. ≤ +6 dB over
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
- [ ] **[VERIFY] Metal Chain is almost silent at Internal Rattle 0.** With no
  rattle the chain's drop barely sounds: −62 dBFS RMS on the guide's base
  patch (Hand Strike on Plate, `param:internal_rattle:min`, declared quiet in
  `src/guide/clip_plan.rs`). Found by the guide's audibility check
  (`add-user-guide`). Decide whether the links should still ring on their own
  at 0, or whether the range should start above silence. *Accept:* Metal Chain
  at Internal Rattle 0 is audible (over the guide's silence floor) or the
  tooltip says that 0 mutes it; the clip's quiet declaration goes either way.
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

- [ ] **[MISSING] Windows test lane.** CI cross-compiles the Windows bundles
  (mingw-w64) but runs no tests on Windows and never loads the
  bundle.
- [ ] **[MISSING] DAW smoke automation** (REAPER / Bitwig / Ardour / Live / FL).
  Manual today: REAPER on Linux, in the development sessions (`docs/REAPER_TESTING.md`).
- [ ] **[MISSING] Visual regression tests for the GUI.** The GUI lane drives
  the editor headlessly (interaction, hover, layout and a few pixel checks),
  but nothing compares rendered screenshots against references.
- [ ] **[CHORE] Persist benchmark baselines** so Criterion runs can flag
  regressions. `scripts/bench_ratios.py` compares twins within one run only.

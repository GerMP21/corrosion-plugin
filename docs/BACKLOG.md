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
- [ ] **[PARTIAL] MSEG PingPong loop ignores the loop stages.** It always loops
  Decay → Sustain, like Forward 3 → 4 (`src/params.rs` `loop_mode` doc,
  `src/dsp/envelopes/mod.rs`). *Accept:* PingPong runs forward then backward
  between Loop Start and Loop End, with a test; or the mode is removed with a
  preset and host migration.

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

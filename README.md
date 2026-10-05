# Corrosion

Corrosion is an industrial physical-modeling instrument built as a VST3 and CLAP plugin. MIDI notes trigger modal resonators and post-processing tuned for damaged metal, friction, weight, and mechanical tension rather than synth-style oscillator framing.

Current package version: `0.1.0`. Preset schema version: `4` (with backwards-compatible migration for v1–v3 files — see `src/presets/mod.rs::migrate_preset_json`).

## What It Ships

- VST3 and CLAP plugin entrypoints via NIH-plug.
- Stereo instrument layout (no audio input — audio-in "Effect mode" is on the roadmap, not shipped).
- **Full MIDI expression**: notes, pitch bend (default ±2 semitones, ±24 safety clamp), channel + polyphonic pressure, and CC1 mod wheel. All routed live to active voices.
- 8-voice polyphony with quietest-peak + oldest stealing.
- 130+ host parameters covering exciters, objects, transformations, post-processing, four high-level macros, and three play modes.
- An egui editor with UI scaling (50/75/100/125/150%).
- Offline renderers for comparison suites, preset batch renders, and aliasing analysis with asserted budgets.
- 50 curated factory presets seeded via a typed Rust binary (`src/bin/seed_presets.rs`).

## Current Sound Surface

### Exciters (16)

`Bow`, `HandStrike`, `FeltMallet`, `HardMallet`, `Drumstick`, `WireBrush`, `MetalPipe`, `MetalChain`, `StiffPoint`, `HeavyGrinding`, `CorrugatedDrag`, `TensionRise`, `PneumaticJet`, `ElectromagneticHum`, `TensionSnap`, `ParticleRain`.

Grouped into three envelope families:

| Family | Members | Envelope |
|---|---|---|
| Hit | HandStrike, FeltMallet, HardMallet, Drumstick, WireBrush, MetalPipe, MetalChain | one-shot AR |
| Friction | Bow, StiffPoint, HeavyGrinding, CorrugatedDrag, TensionRise | 6-stage MSEG (loopable) |
| Specialty | PneumaticJet, ElectromagneticHum, TensionSnap, ParticleRain | ADSR |

### Objects (9)

`Pipe`, `Plate`, `Tank`, `Chain`, `IBeam`, `TautCable`, `CoilSpring`, `SheetMetal`, `IndustrialCog`.

Each object is driven by an algorithmic resonator generator (the single resonator path; legacy profile-driven mode was retired 2026-05-27) and exposes a curated per-object **character param** (e.g. Pipe Diameter, Plate Aspect/Stiffness, Tank Volume/Cavity Mix, Chain Link Mass/Instability, Cable Tension Drop, Spring Dispersion/Slosh, Cog Dissonance — 14 in total). `TautCable` and `SheetMetal` also run per-sample dynamic hooks (amplitude→pitch drop, low-frequency warp) in `ModalResonator::process_sample`.

### Main Control Groups

- **Sound**: `Exciter`, `Object`, `Size`, `Rust`, `Damage`, `Drive`, `Output`, `Width`, `Body`.
- **Macros** (PRD §12, neutral midpoint at 0.5): `Mass`, `Corrosion`, `Violence`, `Brightness`. Each layers a bias across a cluster of related destinations (mass macro hits the exciter mass cluster + resonator size; corrosion adds damping and rust; violence pushes drive/chaos/damage; brightness opens the post-chain filter).
- **Play mode**: `Tonal` (default — note → pitch), `Kit` (MIDI note range → object family, 10 notes per kit slot), `Drone` (force MSEG loop on for friction voices so notes ring until release).
- **Quality**: `Eco` / `Normal` / `High` / `Render`. Controls both post-chain oversampling factor (1×/4×/8×/16×) **and** per-voice modal density (0.5×/1.0×/1.5×/2.0× the object's base mode count). Every mode runs the full post chain (Eco no longer skips body, spread or Space), so quality modes trade fidelity and CPU, not effects.
- **Limiter**: `Hard` (default zero-latency hard clamp at ≈−0.3 dBFS) or `Lookahead` (48-sample window, instant attack, 50 ms release; reports its 1 ms latency to the host).
- **Post chain**: per-channel WDF-style 4-pole ladder filter → Lorenz drive (dual-mono, so the voice engine's stereo image survives), plus the FEM body resonance added in parallel on the mono sum → HRTF spread → Space (Factory / Spring / Echo) → oversampled clipper. **BODY** (0–5) sets the body's mix level; **Chassis Volume** only sets its size (larger = lower resonances). Cutoff, resonance, and master drive are smoothed at audio rate (~20 ms tau) with a first-call snap so static configurations are unchanged.
- **Tempo sync**: `sync_rate < 0.05` keeps the Echo free-running; above it, six musical divisions (1/16 → 2/1) drive the delay from `context.transport().tempo` when the host reports one.
- **Per-voice modulation**: damping, brightness, strike position, coupling, position wander, position envelope, and fundamental anchor automate through to held notes via `Voice::update_live_controls`; tail voices keep their note-on snapshot so decays stay consistent.

### Factory Presets

50 curated presets live under `presets/factory/`. Coverage:

- 6 Pipe, 6 Plate, 6 Tank, 6 Chain, 6 IBeam (36)
- 5 TautCable, 5 CoilSpring, 5 SheetMetal, 5 IndustrialCog (14)

Each anchors on a distinct Object × Exciter pair and tunes the relevant character params, exciter parameters, macros, and post-chain choices. Several use `PlayMode::Drone` for sustained drones, one uses `sync_rate` for tempo-synced echo. Regenerate with `cargo run --bin seed_presets`.

## Repository Layout

```text
.
├── Cargo.toml
├── bundle.sh
├── bundle-win.sh
├── docs/                  ARCHITECTURE.md, BACKLOG.md, code-review.md,
│                          corrosion_plugin_prd_and_specs.md, detailed-specs/
├── presets/factory/       50 .corrosion-preset JSON files
├── scripts/               verify-local.sh, validate-plugins.sh, check_wav.py …
├── src/
│   ├── lib.rs             Plugin trait impls, MIDI routing (incl. CCs), macros,
│   │                      play mode, lookahead wiring, drive/output smoothers
│   ├── params.rs          130+ host parameters (ExciterType×16, Object×9,
│   │                      QualityMode×4, PlayMode×3, character×14, macros×4 …)
│   ├── voice/             Voice + VoiceManager (8-voice pool, expression state)
│   ├── dsp/               Resonators, exciters, interaction bus, transforms,
│   │                      profiles, envelopes, post-processing
│   ├── presets/           Versioned schema, JSON migration, sanitize-and-clamp
│   ├── offline/           OfflineRenderer + analyze_post_chain_aliasing
│   ├── randomizer/        Parameter randomization helpers
│   ├── gui/               egui editor (feature-gated)
│   └── bin/
│       ├── render.rs          --suite family|rust|damage|aliasing|all
│       ├── render_presets.rs  Batch-render presets to WAV + manifest
│       └── seed_presets.rs    Regenerate the 50 factory presets
├── tests/                 no_alloc, automation_stress, preset_roundtrip, body,
│                          drive_dynamics, stereo_width, velocity_*,
│                          chain_distinct, limiter, damage_rattle, bow_exciter,
│                          plugin_metrics, daw/
└── .sisyphus/             Legacy roadmap/evidence notes
```

## Build And Run

```bash
# Test using the repo's default local target from .cargo/config.toml
cargo test --workspace --no-default-features

# Build the Linux bundle target used by bundle.sh
cargo build --target x86_64-unknown-linux-gnu

# Bundle Linux plugins
./bundle.sh

# Cross-compile Windows bundles
./bundle-win.sh

# Offline debug renderer (suites: family, rust, damage, aliasing, all)
cargo run --target x86_64-unknown-linux-gnu --bin render -- --suite all

# Render all presets to WAV + manifest
cargo run --target x86_64-unknown-linux-gnu --bin render_presets

# Regenerate the 50 factory presets
cargo run --bin seed_presets

# Run the benchmark harness on the supported native no-GUI lane
cargo bench --no-default-features --target x86_64-unknown-linux-gnu --bench performance
```

The repository's `.cargo/config.toml` defaults unqualified `cargo` commands to `x86_64-pc-windows-gnu` so the maintainer's Windows host doesn't need to type `--target` constantly. The Linux CI lane overrides this via `CARGO_BUILD_TARGET=x86_64-unknown-linux-musl` in the workflow env; the bundle scripts pass `--target x86_64-unknown-linux-gnu` explicitly. If you're on Linux/macOS and not building Windows DLLs locally, either pass `--target` per command or override the build target with `CARGO_BUILD_TARGET=<your-host-triple>`.

## Validation

If you have the tools installed, the local validation flow is:

```bash
# Full local verification lane that mirrors CI
./scripts/verify-local.sh

# Or run the major steps manually
cargo fmt --check
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
cargo test --workspace --no-default-features
cargo test --lib --target x86_64-unknown-linux-gnu
cargo run --target x86_64-unknown-linux-gnu --bin render -- --suite family --out-dir /tmp/corrosion-verify-render
python3 scripts/check_wav.py /tmp/corrosion-verify-render/pipe_comparison.wav
./bundle.sh release

# Optional host/plugin validators when installed locally
./scripts/validate-plugins.sh
pluginval --strictness-level 5 --validate target/bundled/Corrosion.vst3 --skip-gui-tests
clap-validator validate target/bundled/Corrosion.clap/Corrosion.clap --only-failed
```

The repository CI currently automates the formatter, clippy, the no-default-features workspace tests, the native Linux library lane, the offline renderer smoke test, WAV validation, and the Linux bundle script. DAW smoke scripts under `tests/daw/` and external plugin validators remain optional/manual because they depend on tools that are not guaranteed to exist in every environment. The aliasing test (`offline::tests::render_mode_alias_ratio_stays_within_budget`) asserts `alias_ratio_db ≤ −10 dB` at Render quality; `higher_quality_reduces_alias_ratio` requires Render to strictly beat Eco.

Benchmark coverage is local/manual for now. Use `cargo bench --no-default-features --target x86_64-unknown-linux-gnu --bench performance` to measure the current hot-path baseline for voice rendering, stochastic excitation, post-processing, and offline rendering.

## Real-Time Safety

- **No heap allocation** in `process()` — including note-on for every object, quality mode and exciter, voice stealing, live MIDI expression, and `reset()`. Each voice preallocates its worst-case modal bank (`MAX_MODES`) and rebuilds it in place at note-on. Verified by `tests/no_alloc.rs`, which must run in a **debug** build (`cargo test --no-default-features --test no_alloc`): the checking allocator only exists under `debug_assertions`, and a release run passes vacuously. The non-Windows build enables nih_plug's `assert_process_allocs` feature. The Windows lane is intentionally disabled (`assert_no_alloc`'s thread-local allocator hook conflicts with the Windows CRT during nih-plug test setup); document is in `docs/BACKLOG.md` → P4.
- **No locks / blocking / I/O / logging** in the audio callback.
- **Fixed voice array** — 8 `Voice` instances, each owning all 16 exciter structs (allocation-free dispatch).
- **Denormal + NaN/Inf guards** at every voice boundary; non-finite peak holds sort as quietest so a corrupted voice is reclaimed.
- **Per-buffer parameter setters.** Following the P0 fix, post-chain parameters update once per buffer (not per sample), with audio-rate one-pole smoothers (~20 ms tau, first-call snap) inside `WdfLadderFilter`, `LorenzDrive`, and the master drive / output gain paths.

## Docs

Active design and implementation references:

- `docs/ARCHITECTURE.md` — as-built architecture, signal chain, and status of claimed work.
- `docs/BACKLOG.md` — prioritized work tracker (P0–P4) with `[x] fixed`/`[ ]` status and decision banners for scoped-down items.
- `docs/CODE-REVIEW.md` — critical code review and doc-vs-code conformance matrix.
- `docs/PRD.md` — product requirements (see §0 for current-state reconciliation).
- `docs/MASTER-PROMPT.md` — production-hardening agent prompt (see "Current state" banner).
- `docs/detailed-specs/` — DSP design notes (each file carries an implementation-status banner; treat as design targets, not descriptions of shipped behavior — see ARCHITECTURE.md §9 decision banner).

Legacy roadmap/evidence notes still live under `.sisyphus/`, but the docs above are the current reference for the shipped surface.

## Roadmap (in priority order)

These are the headlines from `docs/BACKLOG.md`; the file has the full status and rationale per item.

**Shipped:**

- **P0** — Oversampled clipper, FactoryReverb comb delays, per-sample post-chain setters: fixed 2026-05-27.
- **P1** — Quality-mode modal density, aliasing regression budgets, held-note automation, parameter smoothing, tempo-synced echo (`sync_rate`).
- **P2** — MIDI expression (pitch bend / channel + poly pressure / mod wheel), Macros (Mass / Corrosion / Violence / Brightness), Percussion Kit mode, Drone mode, opt-in Lookahead limiter, preset schema migration (v4).
- **P3** — Dead code sweep, docstring corrections, `render_presets` declared in Cargo, WDF docstring clarification, `midi_to_hz` bound, drive curve constants named.

**Scoped down (documented decisions, not committed work):**

- **Sequencer + per-step locks** (PRD §18) — needs a dedicated module + GUI; deferred from P2.
- **Effect mode** (audio-in excites the resonator) — needs `AUDIO_IO_LAYOUTS` change and feedback safety design.
- **MSEG as routable mod source** — folded into the deferred mod-matrix / macro-routing work.

**Open:**

- Windows CI lane / bundle smoke automation.
- DAW smoke automation across REAPER / Bitwig / Ardour / Live / FL.
- GUI interaction / visual regression tests.
- Persisted Criterion benchmark baselines.

## Community & Discussions

GitHub Discussions are open for anything that isn't a concrete reproducible bug. Use them before opening an Issue if your topic is in any of these buckets:

- **Questions** — usage, build setup, signal-flow understanding.
- **Ideas** — feature proposals, preset concepts, parameter-tuning suggestions.
- **DSP review** — algorithmic critique, modal-synthesis theory, oversampling/aliasing trade-offs.
- **Architecture discussion** — module boundaries, voice/manager design, post-chain wiring.

Issues are reserved for **confirmed, reproducible bugs** (steps to reproduce + expected vs. actual). Anything labeled `question`, `idea`, `discussion`, `DSP review`, or `parameter tuning` is more likely to find traction in Discussions than in the issue tracker.

When you post, please distinguish between:

- **Confirmed bugs** — steps to reproduce, host/version, expected vs. actual output.
- **Subjective sound-design feedback** — "this preset sounds harsh" / "I'd like more warmth here".
- **DSP theory concerns** — references to specs, papers, or competitive plugins where they apply.
- **Implementation cleanup** — refactor or readability proposals, ideally with a sketch of the change.
- **Experimental ideas** — speculative directions worth chewing over before anyone commits to building them.
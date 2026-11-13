# Corrosion: notes for Claude

Corrosion is a physical-modeling instrument plugin (VST3 + CLAP, Rust, nih-plug, egui editor): MIDI notes excite modal resonators (pipes, plates, cables, …) through 16 exciter models, then a post chain (filter, drive, FEM body, HRTF spread, Space, clipper, limiter). `README.md` describes the sound surface; `docs/ARCHITECTURE.md` is the as-built reference and is kept current.

## ⚠️ Branch: USE `dev`, ALWAYS

**Every change goes on the `dev` branch. No exceptions.**

- **Start of every session:** `git fetch origin && git checkout dev && git pull origin dev`. Check `git branch --show-current` says `dev` before editing anything.
- **Commit on `dev`, push to `dev`:** `git push origin dev`.
- **Never** commit to `main`, and never create or push any other branch, including the `claude/*` branch a session may start on or be told to use. If the session starts on another branch, switch to `dev` first. Only the maintainer can make an exception, explicitly.
- **PRs** go from `dev` into `main`, and only when the maintainer asks for one.
- **After a PR merges**, bring `dev` up to date with `main` (`git fetch origin && git merge --ff-only origin/main`, then push) before any new work.

## Working with the maintainer

- **Talk to the maintainer in Spanish.** Code, comments, docs, commit messages and PR descriptions are in English.
- **Update a PR's description only when a change is finished**, not after every commit.
- When a plan, spec bound or design decision turns out not to hold, stop and ask, with the numbers and the options. Don't silently relax a spec.
- Report results faithfully: say what was measured, what failed, what was skipped.

## Factory presets

The factory bank (214 presets in folders under `presets/factory/`: 204 plus 10 drum kits) is built in code by `src/presets/factory/` and written by `cargo run --release --bin seed_presets`. Since `reorganize-factory-presets` it is real: **its levels and character matter.**

- Change a preset in `curated.rs` (or the generator in `experimental.rs`), then rerun the seeder. It re-levels every preset and rewrites the files and `src/presets/factory/levels.rs`. `generated_bank_equals_the_files_on_disk` fails if they drift.
- The bank tests (`presets::factory::tests`, CI lane) must pass: counts, the 16 × 9 grid, determinism, parameter variation, the quality floor for all 204 non-kit presets and for every drum pad alone, each curated folder's character checks and the drum kits' character (`drum_kits_have_drum_character`). `curated_measurements_table` (ignored) prints the per-preset table for the change's design.
- A change that moves levels or timbre engine-wide (a new stage, a gain change) can push presets off the floor or out of character: run the bank tests and reseed, and say what moved.
- Tuning notes for designing presets are at the top of `curated.rs`. Pitch is roughly the key ÷ Size, with an offset per object. Space and BODY add partials of their own. Pipe, I-Beam and Taut Cable strikes ring for seconds.
- A schema change that adds a field: reseed so the files stay valid.
- **User presets and saved DAW projects still matter: keep their migrations** (see "Migrations" below).
- Tests that need one specific preset read the old placeholder bank, kept as a corpus in `tests/fixtures/presets/legacy/` (also the benchmark's and `render_presets`' default `sheet_lightning`). Don't regenerate the corpus: it doubles as a check that old presets keep loading.

## User guide data and clips

The guide at `/wiki/` (`web/guide/`, `add-user-guide`) is partly generated from the plugin, like the factory bank:
- **After changing a parameter** (range, default, name, unit), a control's label or ⓘ text, a model, a mode or the factory bank: `cargo run --bin export_guide_data`, then add or fix the parameter's Spanish entry in `web/guide/src/data/parameters.es.json`. `guide_data_equals_the_files_on_disk` and `npm run check` (in `web/guide/`) fail until then.
- **After a change that moves the sound engine-wide** (the trigger that reseeds presets) or a change to `src/guide/clip_plan.rs`: `scripts/build-guide-clips.sh` (needs `opusenc`), and say in the close-out that the clips were regenerated. The fingerprint test only sees plan changes, not engine changes.
- A new parameter needs a clip plan entry or an exclusion with a reason (`guide_clip_plan_covers_every_parameter`); a minimum that renders silent needs a `QUIET` reason or a fix.
- A new model or mode needs a page declaring it in `covers:`, in English and Spanish (`npm run check`).

## Current state (2026-11-13)

The work follows the code review in `FABLE-CODE-REVIEW-2026-07-07.md`. Each fixed finding there is struck through, with a `Status: FIXED` note naming its OpenSpec change.

| Findings | State |
|---|---|
| C1–C6 (critical) | Fixed, archived in `openspec/changes/archive/` |
| H1–H10 | Fixed. All H changes archived 2026-11-08 |
| H11 (prune unsoundable modes), H12 (skip redundant retunes) | Fixed. H12 bounds re-verified: bent 1.019×, dynamics 1.535× (median of 5) |
| H13 `h13-render-presets-through-post-chain` | Fixed. Also fixed `reset()`'s unbiased drive start |
| H14 `h14-wrap-low-kit-notes` | Fixed |
| Medium M1–M20 | All closed (2026-11-10), by the 7 M changes applied in the order below, plus `block-voice-mix-dc`, `add-asymmetric-diode-stage` and `add-parameter-tooltips` in between. M16 fixed with H10; M2 and M19 confirmed resolved and M14 documented by `m10-m18-tidy-dead-code-and-docs`; M17's decorrelation deliberately not done (it would change the sound). **Next:** nothing in the review; `docs/BACKLOG.md` (Release Snap, Env Amount) and the diode knee by ear |
| Rust, Damage, Heat (reported by the maintainer, not in the review) | Fixed by `fix-rust-damage-heat-response`: Rust and Damage no longer silence notes or lose the fundamental (worst −3.2 dB across their ranges); Heat darkens spectrally and warbles. Softening the exciter for Heat was dropped (maintainer, 2026-11-09) |
| Site and guide (maintainer request) | `add-site-shell` done (2026-11-13), ready to archive: home at `https://corrosion-vst.org/`, the web app at `/play/`, themed 404; the Worker's only domain is `corrosion-vst.org`, and `preview.` 301s to `/play/` through dashboard settings (`docs/ARCHITECTURE.md`); tokens in `web/theme/tokens.css` checked against `src/gui/palette.rs`; Cloudflare Web Analytics beacon kept (maintainer). `add-user-guide` done (2026-11-13), ready to archive: Starlight guide at `/wiki/` and `/wiki/es/` (74 stub pages per language, generated parameter reference, 491 generated audio clips, home demos, `npm run check`, CI `guide` job; see "User guide data and clips" above). Home demos chosen by the maintainer (`HOME_DEMOS`; triads for Chords, a held note for Drones); clips at 48 kbps, each cut where its tail falls 60 dB under its peak, at most 8 s past the last note-off (`trim_to_silence`, `RELEASE_TAIL`), 19.5 MB. Metal Chain at Internal Rattle 0 (−62 dBFS, declared quiet) is in `docs/BACKLOG.md`. **Next:** `write-user-guide-content` (proposed) |
| Backlog | `docs/BACKLOG.md`, open items only (2026-11-13): Release Snap, Env Amount, default-patch decay, Metal Chain at Internal Rattle 0, MSEG PingPong; deferred features (mod matrix, sequencer, effect mode, output meter); Windows test lane, DAW smoke automation, visual GUI tests, benchmark baselines |

**Archived:** `h01`–`h14`, `resolve-exciter-tradeoffs` and `stabilize-mallet-and-grinding-contact` (2026-11-08); `m03-m20-start-at-host-rate-and-report-latency`, `m13-sustain-pedal-and-panic`, `m07-m08-interpolate-echo-and-tidy-spread`, `m06-m09-smooth-starvation-and-oversample-clipper` and `fix-rust-damage-heat-response` (2026-11-09); `m01-m04-m05-clarify-parameters`, `block-voice-mix-dc`, `add-asymmetric-diode-stage`, `add-parameter-tooltips`, `m12-per-note-expression`, `m10-m18-tidy-dead-code-and-docs`, `add-editor-keyboard`, `reorganize-factory-presets`, `fix-pneumatic-jet-saturation` and `per-key-kits` (2026-11-10). They are in `openspec/changes/archive/`, and their delta specs are synced into `openspec/specs/` (31 capabilities; `voice-mix-dc`, `diode-stage`, `host-parameters`, `note-expression`, `editor-keyboard` and `factory-preset-bank` new on 2026-11-10). **Pending from the diode stage:** the maintainer judges the knee by ear and may adjust the Softness/Asymmetry defaults (a small follow-up change if so). Every review finding's change is archived. **Active:** `add-web-app` (tasks 6.2, 6.3 left), `extend-ci-windows-linux-macos` (3.1–3.3 left), `add-site-shell` and `add-user-guide` (done, to archive), `write-user-guide-content` (proposed). A new change syncs its specs on archive as usual (`/opsx:archive`).

### Execution order for the M changes (and two feature changes)

Apply them one at a time, in this order, each closed out (tests, docs, review status, this file) before the next. The order puts cheap host fixes first and the BREAKING change late, and keeps changes that touch the same code apart.

| # | Change | Findings | Notes |
|---|---|---|---|
| 1 ✅ | `m03-m20-start-at-host-rate-and-report-latency` | M3, M20 | Latency at `initialize`; voices rated before they play. No sound change at 48 kHz. Stop and ask if an exciter turns out to depend on the rate at trigger time. |
| 2 ✅ | `m13-sustain-pedal-and-panic` | M13 | CC64, CC120 (5 ms panic fade in the output path), CC121, CC123; new stealing tier. |
| 3 ✅ | `m07-m08-interpolate-echo-and-tidy-spread` | M7, M8 | Echo sound changes; the spread stays bit-identical (maintainer's choice). |
| 4 ✅ | `m06-m09-smooth-starvation-and-oversample-clipper` | M6, M9 | Half-band IIR clipper (maintainer's choice). Bias Starvation also rescaled to act across its range. The clipper clips at the ceiling ÷ 1.3 when oversampling (headroom for the hard clamp) and delays 4–5 samples, unreported. Alias −51 dB Normal, −76 dB Render; CPU within noise (Normal 1.031×, Render 0.992×, median of 5). |
| 5 ✅ | `m01-m04-m05-clarify-parameters` | M1, M4, M5 | Onset Level with preset and host migrations (default renders bit-identical, all 16 exciters); unique names; Output in dB, +40 dB kept (0 dB at 57.5 % of the travel, −20..+6 dB over 18.9 %). **BREAKING** for Output automation. Factory presets regenerated. |
| 6 ✅ | `block-voice-mix-dc` | (found by the diode stage) | The voices carry a static modal deflection under one-signed forces (bow, mallet). A 10 Hz second-order Butterworth high-pass per channel on the voice mix, before the master drive (`dsp::StereoDcBlocker`, `f64` state). The designed 5 Hz one-pole missed both DC bounds (maintainer's choice, held-bow bound relaxed to < 2 % of RMS). 30 Hz −0.05 dB; strike's 20 ms mean −16.6 dB; presets with ≥ 1 % DC power 21 → 1 (Asymmetry 0); CPU noise. `preset_levels_are_sane` fails (3 presets 13.6–15.3 dB under the median); no level pass, the presets are placeholders. |
| 7 ✅ | `add-asymmetric-diode-stage` | (maintainer request) | The clipper as a feature: tanh knee per polarity (`w = 0.8·softness`), new Diode Asymmetry (default 0.25, `L₋ = L·2^(−a)`, 2 Hz whole-signal DC blocker faded in by `a`), limit ceiling ÷ 1.3 in every mode (Eco too), 513-point tanh table, the brickwall panel's red dot as a diode meter (`DiodeMeter`). Alias Normal −67.0, Render −105.0 dB; CPU 1.030× / 1.003×. Older presets and projects migrate to Softness 0 / Asymmetry 0. **BREAKING** for Softness automation. After `block-voice-mix-dc` its blocker moves preset RMS ≤ 0.4 dB, so it keeps filtering the whole signal. `preset_levels_are_sane` still fails on 3 placeholder presets (no level pass). Its `output-clipper` delta modifies the spec m06 created (archived). Judge the knee by ear before archive. |
| 8 ✅ | `add-parameter-tooltips` | (maintainer request) | Done: help in one table keyed by label (`help_for`), not per-spec fields (As built, for speed); coverage, hover and per-scenario tests (run red against mutations), plus a guard on slot labels shared between models; no label shortened. Checked in REAPER at 75 %: badges legible, tooltips open on the badge. GUI only: ⓘ badge (painted 1 px circle and "i" in `MUTED_GREY`) right of each label, hover on the badge only, 1–2 sentences; selector help is a removable trial. After m01 and the diode stage so texts use the final names and controls. |
| 9 ✅ | `m12-per-note-expression` | M12 | Voice-ID addressing (`NoteAddress`; ID first, else key and channel; a reused ID picks the held note, then the newest), PolyTuning on top of the channel bend, Choke as a 2 ms linear fade. Plain-MIDI renders bit-identical (16 exciters); CPU within noise (0.99–1.03×). Plain-MIDI note-offs now also match the channel. Channel MPE out of scope. |
| 10 ✅ | `m10-m18-tidy-dead-code-and-docs` | M10, M11, M14, M15, M17, M18 | One constructor per parameter (136 generated in `params.rs`, shared with `Preset::into_params`; preset-built parameters display like the live ones); `exciter_force` removed (the bus stays bidirectional: displacement and velocity feed back); redundant `active = false` removed; rattle naming; exciter range 1–16 and the Hand Strike fallback documented and tested; quality-mode timings documented; `dsp` re-export dropped; old review archived. Render hashes unchanged. Also marks M2 and M19. |
| 11 ✅ | `add-editor-keyboard` | (maintainer request) | Done, archived. A piano strip (C3–C6) pinned under the panels' scroll view (`render_window_body`; window 810×630 at 75 %, panel layout unchanged via `BASE_LAYOUT_HEIGHT`): mouse with velocity by height, drag glissando; computer keys `A W S E D F T G Y H U J K` for C4–C5. **As built (maintainer's choices):**
- Keys match **by letter**, because egui-baseview reports no physical key.
- Keys play only while the window is focused **and the pointer is over the editor**, and not while the preset name is being typed. baseview reports focus loss only on macOS; the pointer leaving releases held notes and clears egui's `keys_down`.
- X11 auto-repeat key-up/key-down pairs are coalesced into one repeat (`key_actions`).

Notes go through a lock-free SPSC ring (`src/gui_notes.rs`) drained after `begin_block` into `handle_note_event` as channel-0 notes. A close is seen by the audio thread on `editor_state.is_open()`'s falling edge. The held-key mask is published per block. Checked in REAPER (Linux): the stuck note is fixed, held keys don't retrigger, and closing the window with a note held releases it. 18 + 5 + 16 tests plus no_alloc, each red against its mutation. |
| 12 ✅ | `reorganize-factory-presets` | (maintainer request) | Done, archived (2026-11-10).
- 204 presets: `bass`, `lead`, `chords`, `drones`, `percussion` (12 each, by hand) plus `experimental` (16 × 9, seeded, every parameter varied). Built in `src/presets/factory/`, written by `seed_presets`, stored levels in `levels.rs`.
- Levelled to `TARGET_RMS` 0.13 at velocity 100, capped for headroom at 127 (maintainer's choice). The floor also rejects presets more than 12 dB under the target at 127; one re-draw left (`stiff_point_scrape/coil_spring`, chaotic stick-slip).
- `preset_levels_are_sane` now renders the held note (≤ 1 % over the threshold) and passes. Releasing Bow, Stiff Point Scrape or Corrugated Drag snaps the object free, up to +30 dB on the default patch (+37 dB in the bank): a feature (maintainer), with a Release Snap control in `docs/BACKLOG.md`.
- Browser with submenus, arrows within the folder, Save into user subfolders (never factory), Open folder; checked in REAPER (Linux).
- Open: the chord flatness bound (< 0.1) never binds (max 0.031); keep or replace, maintainer's call.
- **Found here:** `fix-pneumatic-jet-saturation` (done, archived). The jet's `v² − chaos·v³` stalled to silence above `1/chaos`, so loud notes or high Air Pressure went silent. It now holds its peak above `2/(3·chaos)`; default velocity-100 notes are bit-identical. A full-range sweep (`print_velocity_sweep`) found no other parameter that silences full velocity; Env Amount 0 silences friction exciters (tooltip says "flattens"), in the backlog. |
| 13 ✅ | `per-key-kits` | (maintainer request) | Done, archived (2026-11-10).
- **BREAKING:** Kit mode is an AFX-style kit: each key plays its own pad (every voice parameter, plus tune ±48 st and a level), keys without a pad are silent, the slot mapping (`note_to_kit_object`) is gone. Old Kit presets and projects load as Tonal.
- `src/kit/`: `KitPad` (values by parameter ID) → `Pad` (`note_setup`); seqlock `KitTable`, rebuilt in `initialize` (nih-plug calls it after a state restore). The pad voice starts at `key + tune` into the bank (bit-identical to a live note there), skips live controls, and its level is a gain after the voice clamp (`pad_gain`, mono and stereo).
- Editor: strip 36–84, kit bar (Assign, Clear, Tune, Full keyboard), zone 36–59 greyed outside; computer keys 36–48 in Kit mode. Assign's tune keeps the last auditioned pitch (empty Kit keys don't count).
- 10 factory drum kits (`drums.rs`, 12 GM pads, role-tuned), each pad levelled alone (`PAD_LEVELS`, −40…+60 dB: one-shot hats need up to +56.7 dB to reach the 2 s floor). Tonal/Drone: all 204 presets bit-identical; pad note-on 0.89–0.97× a live one.
- Checked in REAPER (Linux, 75 %): load, select, play, Assign, preset and project round trips, old Kit project → Tonal. |

## OpenSpec workflow

Changes live in `openspec/changes/<name>/` with `proposal.md`, `design.md`, `specs/<capability>/spec.md` and `tasks.md`. Use the `/opsx:*` skills (`/opsx:apply <name>`, `/opsx:verify`, `/opsx:archive`). `openspec validate <name>` checks the artifacts.

How each change has been done, and what the maintainer expects:
1. **Tests first.** Write the spec scenarios as tests and verify they fail on the current code (or that a reference test passes), before implementing.
2. **Implement**, keeping changes scoped to the tasks. Mark each task `- [x]` as soon as it is done and verified.
3. **Record numbers in the change's `design.md`** (a `## Measurements` section): before/after values, benchmark tables, anything the proposal guessed wrong. When the implementation differs from a design decision, add an *As built* note rather than leaving the design stale.
4. **Docs:** update `docs/ARCHITECTURE.md` (and the README when users see the change), and mark the finding FIXED in `FABLE-CODE-REVIEW-2026-07-07.md` with date, change link and what proves it.
5. **Close out:** `cargo fmt --check`, clippy with `-D warnings`, the full test suite, then commit and push.
6. **Update "Current state" in this file** when a change is finished, without being asked: the date in its heading, the change's row (fixed or done), what is next, and the archive note. Don't record PR numbers, branches or merge status here: GitHub and git already hold them. Commit it with the change's close-out.

## Build and test

`.cargo/config.toml` defaults the target to `x86_64-pc-windows-gnu` (the maintainer's Windows host) and puts build output in `../corrosion-target`. On Linux, always set a host target:

```bash
export CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu
```

CI (`.github/workflows/ci.yml`) uses `x86_64-unknown-linux-musl` with the latest stable Rust. The local toolchain must be **Rust 1.99 or newer**: `src/lib.rs` allows a lint (`semicolon_in_expressions_from_non_local_macros`) that older versions reject under `-D warnings` (`rustup update stable`).

| Check | Command |
|---|---|
| Format | `cargo fmt --check` |
| Lint (CI lane) | `cargo clippy --workspace --all-targets --no-default-features -- -D warnings` |
| Tests (CI lane) | `cargo test --workspace --no-default-features` (~340 tests) |
| GUI lane | `cargo test --lib` (~300 tests; needs the X11/GL dev packages from `ci.yml`: `apt-get install -y pkg-config libx11-dev libx11-xcb-dev libxau-dev libxcb1-dev libxcursor-dev libxext-dev libxfixes-dev libxft-dev libxi-dev libxinerama-dev libxkbcommon-dev libxrandr-dev libxrender-dev libxtst-dev libxxf86vm-dev libgl-dev`) |
| musl parity | `CARGO_BUILD_TARGET=x86_64-unknown-linux-musl cargo test --workspace --no-default-features` (`rustup target add x86_64-unknown-linux-musl`) |
| Real-time safety | `cargo test --no-default-features --test no_alloc` (debug only; the checking allocator doesn't exist in release) |
| Docs | `cargo doc --no-deps --no-default-features` (should add no warnings) |
| Full local lane | `./scripts/verify-local.sh` |

- `[profile.test]` uses `opt-level = 1`, so debug test runs aren't painfully slow.
- **Known noise:** clippy on the GUI lane (`cargo clippy --lib` with default features) reports 13 existing `falling back to f32` warnings in `src/gui/editor.rs`. CI doesn't run that lane; don't add new ones.
- **Ignored measurement tests** print tables, run with `-- --ignored --nocapture`: `preset_levels_are_sane`, `preset_levels_table`, `print_exciter_overdrive`, `print_48k_exciter_render_hashes`, `print_unsoundable_mode_counts`, `print_editor_content_height`, and others.
- **Checking in REAPER** (Linux, from a cloud session): `docs/REAPER_TESTING.md` has the recipe: Xorg dummy display, REAPER with a startup script that loads the CLAP plugin, a screenshot build with `srgb: false`, and `xdotool` with ImageMagick.
- **Golden mode-bank fixture** (`tests/fixtures/mode_bank_hashes.txt`): regenerate only for an intended sound change, with `CORROSION_UPDATE_GOLDEN=1 cargo test mode_banks_match_golden`, and add the reason to the doc comment above `mode_banks_match_golden_fixture` in `src/dsp/resonators/core.rs`. It must match on gnu and musl.

## Benchmarks

```bash
cargo bench --no-default-features --bench performance -- voice_manager_sixteen_voices
python3 scripts/bench_ratios.py 5   # the H12 cost bounds, as medians of within-run ratios
```

- **This container's run-to-run noise is ±10–15 % per benchmark.** Compare twins within one run, take the median of 5 runs (`scripts/bench_ratios.py`), and never conclude from one pair of numbers.
- **`cargo bench` compiles the working tree.** Editing `src/` while a benchmark builds mixes old and new code. For a "before" baseline, use a clean worktree with its own target dir:
  ```bash
  git worktree add --detach /tmp/base HEAD
  (cd /tmp/base && CARGO_TARGET_DIR=/tmp/base-target cargo bench --no-default-features --bench performance -- <filter>)
  ```
- **Code layout can swing the plain benchmarks by 30 % with bit-identical output.** H12 found the cause: a cold branch inlined into the per-sample mode loop. To tell code placement from real work, compare an output hash of the benchmark scenario between the two builds.

## Engine rules learned the hard way

- **Real-time safety.** `process()` and note-on never allocate. Every per-voice buffer is preallocated to `MAX_MODES` / `MAX_RAW_MODES` and rebuilt in place (`ModalResonator::rebuild_in_place`). The Space and HRTF buffers are sized in `new()` for 192 kHz (`space::MAX_SAMPLE_RATE`). `tests/no_alloc.rs` checks note-on, live controls and MIDI expression.
- **Keep the per-sample loop's data small.** `SecondOrderMode` is iterated for every mode of 16 voices every sample, and the working set sits near L1 size. Data a mode needs only occasionally goes in a parallel `Vec` on `ModalResonator`: `mode_weights` (H11) and `retune_cache` (H12). It doesn't go in `SecondOrderMode`.
- **Keep cold paths out of line in hot loops.** `SecondOrderMode::ensure_rate` inlines only the rate check; the rebuild is `#[cold] #[inline(never)]`. `ModalResonator::apply_dynamics` is `#[inline(never)]`. No float division by a constant in hot code: the compiler won't turn it into a multiplication.
- **Prove "defaults are unchanged".** When a change claims something stays bit-identical, prove it. Use a test-only switch on the struct (`Voice::global_controls_enabled`, `governor_enabled`, `drive_scale`; `ModalResonator::prune_enabled`, `retune_skip_enabled`) to compare the new and old paths, or a one-off render-hash comparison against a clean worktree. Hashes aren't portable across libm (gnu vs musl), so they can't be asserted in CI.
- **Sample-rate rule.** Per-sample constants are tuned at 48 kHz and converted with `crate::dsp::utils::rate`. Each must be exactly its 48 kHz value at 48 kHz.
- **Loudness.** Banks are normalized at note-on (`STRIKE_LEVEL_REF`), each exciter has a drive level (`EXCITER_DRIVE_LEVEL`), and a per-voice drive governor bounds the pre-clamp signal. Tests such as `default_notes_do_not_overdrive_the_voice` and `exciters_play_at_a_common_level` must keep passing.

## Migrations (user presets and DAW projects)

When a parameter's meaning changes, migrate both paths:
- **Preset files:** `src/presets/mod.rs::migrate_preset_json`, which runs on the raw JSON before typed parsing. The schema version stays at `"4"` for optional additive fields.
- **Saved DAW projects:** the host restores parameters by ID and never goes through the preset loader. Migrate them in `Plugin::filter_state` (`src/lib.rs`, `migrate_host_wander_rate`).
- Example (H9): Position Envelope used to set the wander rate. State without `position_wander_rate` gets `legacy_wander_rate(position_envelope)` and Position Envelope 0, on both paths.
- Host automation can't be migrated; call it out as BREAKING in the proposal.

## Git and PRs

- **Branch:** `dev` only (see "Branch: USE `dev`, ALWAYS" at the top). PR state lives on GitHub, not in this file.
- Commit messages: imperative subject naming the finding, e.g. "Skip redundant mode retunes (H12)", and a body saying what changed and why. Add measurements when relevant. Don't put model identifiers in commits, code or PRs.
- Don't create a PR unless asked. There's no PR template: use Summary / Behaviour changes / Testing sections and say plainly what wasn't tested (Windows build, pluginval, clap-validator, DAW smoke tests haven't been run in these sessions).

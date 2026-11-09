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

## Factory presets are placeholders

The 50 factory presets (`presets/factory/`, seeded by `src/bin/seed_presets.rs`) are irrelevant for now: the maintainer will regenerate them all when the project is finished.

- Don't spend effort keeping them bit-identical, re-levelling them, or measuring how a change affects them.
- Don't treat "a factory preset changes" as a risk or a reason to avoid a change.
- Keep them loading and passing the existing tests (`preset_levels_are_sane`, `tests/preset_roundtrip.rs`). If a schema change adds a field, regenerate them with `cargo run --bin seed_presets` so they stay valid.
- **User presets and saved DAW projects still matter: keep their migrations** (see "Migrations" below).

## Current state (2026-11-09)

The work follows the code review in `FABLE-CODE-REVIEW-2026-07-07.md`. Each fixed finding there is struck through, with a `Status: FIXED` note naming its OpenSpec change.

| Findings | State |
|---|---|
| C1–C6 (critical) | Fixed, archived in `openspec/changes/archive/` |
| H1–H10 | Fixed. All H changes archived 2026-11-08 |
| H11 (prune unsoundable modes), H12 (skip redundant retunes) | Fixed. H12 bounds re-verified: bent 1.019×, dynamics 1.535× (median of 5) |
| H13 `h13-render-presets-through-post-chain` | Fixed. Also fixed `reset()`'s unbiased drive start |
| H14 `h14-wrap-low-kit-notes` | Fixed |
| Medium M1–M20 | 7 OpenSpec changes, applied in the order under "Execution order for the M changes" below. **Done:** `m03-m20-start-at-host-rate-and-report-latency` (also restarts four exciters' per-note state), `m13-sustain-pedal-and-panic`, `m07-m08-interpolate-echo-and-tidy-spread`, `m06-m09-smooth-starvation-and-oversample-clipper`. **Next:** `m01-m04-m05-clarify-parameters`. M16 fixed with H10; M2 and M19 already resolved (marked by the last change) |
| Rust, Damage, Heat (reported by the maintainer, not in the review) | Fixed by `fix-rust-damage-heat-response`: Rust and Damage no longer silence notes or lose the fundamental (worst −3.2 dB across their ranges); Heat darkens spectrally and warbles. Softening the exciter for Heat was dropped (maintainer, 2026-11-09) |
| Backlog | `docs/BACKLOG.md` (P0–P4): Windows CI, DAW smoke automation, GUI tests, benchmark baselines |

**Archived:** `h01`–`h14`, `resolve-exciter-tradeoffs` and `stabilize-mallet-and-grinding-contact` (2026-11-08), and `m03-m20-start-at-host-rate-and-report-latency`, `m13-sustain-pedal-and-panic`, `m07-m08-interpolate-echo-and-tidy-spread`, `m06-m09-smooth-starvation-and-oversample-clipper` and `fix-rust-damage-heat-response` (2026-11-09), are in `openspec/changes/archive/`, and their delta specs are synced into `openspec/specs/` (25 capabilities; `midi-control`, `output-clipper` and `material-transforms` new). The active changes are the remaining M changes and the two feature changes; finished ones stay active until archived. A new change syncs its specs on archive as usual (`/opsx:archive`).

### Execution order for the M changes (and two feature changes)

Apply them one at a time, in this order, each closed out (tests, docs, review status, this file) before the next. The order puts cheap host fixes first and the BREAKING change late, and keeps changes that touch the same code apart.

| # | Change | Findings | Notes |
|---|---|---|---|
| 1 ✅ | `m03-m20-start-at-host-rate-and-report-latency` | M3, M20 | Latency at `initialize`; voices rated before they play. No sound change at 48 kHz. Stop and ask if an exciter turns out to depend on the rate at trigger time. |
| 2 ✅ | `m13-sustain-pedal-and-panic` | M13 | CC64, CC120 (5 ms panic fade in the output path), CC121, CC123; new stealing tier. |
| 3 ✅ | `m07-m08-interpolate-echo-and-tidy-spread` | M7, M8 | Echo sound changes; the spread stays bit-identical (maintainer's choice). |
| 4 ✅ | `m06-m09-smooth-starvation-and-oversample-clipper` | M6, M9 | Half-band IIR clipper (maintainer's choice). Bias Starvation also rescaled to act across its range. The clipper clips at the ceiling ÷ 1.3 when oversampling (headroom for the hard clamp) and delays 4–5 samples, unreported. Alias −51 dB Normal, −76 dB Render; CPU within noise (Normal 1.031×, Render 0.992×, median of 5). |
| 5 | `m01-m04-m05-clarify-parameters` | M1, M4, M5 | Onset Level with preset and host migrations; unique names; Output in dB, +40 dB kept. **BREAKING** for Output automation. Regenerate the factory presets. |
| 6 | `add-asymmetric-diode-stage` | (maintainer request) | The clipper as a feature: tanh knee set by Diode Softness, new Diode Asymmetry (default 0.25, even harmonics, 5 Hz DC blocker), the same curve and headroom in Eco, the brickwall panel's red dot as a diode meter. Older presets and projects migrate to Softness 0 / Asymmetry 0. **BREAKING** for Softness automation. Its `output-clipper` delta modifies the spec m06 created (archived). Stop and ask if the headroom must exceed 1.3 or Render is more than 20 % slower. |
| 7 | `add-parameter-tooltips` | (maintainer request) | GUI only: ⓘ badge (painted 1 px circle and "i" in `MUTED_GREY`) right of each label, hover on the badge only, 1–2 sentences; selector help is a removable trial. After m01 and the diode stage so texts use the final names and controls. |
| 8 | `m12-per-note-expression` | M12 | Voice-ID addressing, PolyTuning, Choke. Channel MPE out of scope. |
| 9 | `m10-m18-tidy-dead-code-and-docs` | M10, M11, M14, M15, M17, M18 | Sound-neutral cleanup (prove with render hashes); also marks M2 and M19, already resolved. |

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

# Design

## Context

Today (`.github/workflows/ci.yml`):
- one `quality-and-build` job on `ubuntu-latest`: fmt, clippy, musl tests, gnu GUI tests, renderer smoke;
- a `bundle` matrix, also on Linux: Linux bundles and Windows bundles cross-compiled with mingw (`x86_64-pc-windows-gnu`);
- a `prerelease` job that replaces the `nightly` release on pushes to `main`.

It runs on `main`, `master` and pull requests only; the maintainer works on `dev`. Nothing runs on Windows or macOS, nothing loads the bundles, and `Swatinem/rust-cache` caches `target/` while `.cargo/config.toml` builds into `../corrosion-target`, so every run builds from scratch.

Maintainer's choices (2026-11-10): the nightly publishes **only from `main`**; Windows builds and tests on a **Windows runner with MSVC**; both **clap-validator and pluginval**; **macOS added** (universal bundles).

## Goals / Non-Goals

**Goals:** every check of the local close-out on Linux, the portable ones on Windows and macOS; bundles built natively on each platform; both validators; a nightly that always reflects the last fully green `main`.

**Non-Goals:**
- Code signing beyond ad-hoc on macOS, notarization, or Windows Authenticode: they need paid certificates and secrets. The release notes explain the macOS quarantine workaround.
- A Linux arm64 build, or musl bundles.
- DAW smoke automation (stays in the backlog).
- Benchmarks as a gate: they are compiled, not run (this container's noise is ±10–15 %; shared runners are worse).

## Decisions

### 1. Jobs and their order
```
checks (linux | windows | macos) ──► bundle (linux | windows | macos) ──► nightly (main only)
         every lane of that OS            build, package, validate          replace release
```
- `checks` is a matrix over the three runners, `fail-fast: false` so one platform's failure doesn't hide another's. Steps that apply to only some platforms are gated with `if: runner.os == …`.
- `bundle` depends on all of `checks`: a red platform blocks every bundle, so a release never mixes a tested and an untested platform.
- Validation runs inside each `bundle` job, right after packaging, on the same runner, so a validator failure fails that platform.
- `nightly` depends on all of `bundle` and runs only when `github.ref == 'refs/heads/main'` and the event is `push` or `workflow_dispatch`.
- *Alternative:* a separate validation job downloading the artifacts. Rejected: an extra runner start per platform for no gain.

### 2. Triggers and concurrency
- `on: push: branches [dev, main]`, `pull_request`, `workflow_dispatch`.
- Workflow-level `concurrency: { group: ci-${{ github.ref }}, cancel-in-progress: true }` for `dev` and pull requests; on `main`, `cancel-in-progress` is false (`${{ github.ref != 'refs/heads/main' }}`), so a publishing run is never killed half-way.
- The `nightly` job has its own `concurrency: nightly` group (no cancel), so two `main` runs publish one after the other, newest last.

### 3. Targets per platform
| Runner | Checks target | Bundle |
|---|---|---|
| `ubuntu-latest` | musl for the CI lane (as today, matches CLAUDE.md), gnu for GUI lane, renderer and `no_alloc` | gnu `.so` |
| `windows-latest` | `x86_64-pc-windows-msvc` everywhere | MSVC `.dll` |
| `macos-latest` (arm64) | `aarch64-apple-darwin` | `aarch64` + `x86_64-apple-darwin`, joined with `lipo` |

`.cargo/config.toml` sets the default target to `x86_64-pc-windows-gnu` for the maintainer's machine, so every job sets `CARGO_BUILD_TARGET` explicitly. Windows cross-builds from Linux (mingw) are dropped: the shipped Windows build is now MSVC, and the maintainer's local gnu build stays available through `CORROSION_WINDOWS_TARGET`.

### 4. Real-time safety tests per platform
`tests/no_alloc.rs` needs nih-plug's `assert_process_allocs`, which `Cargo.toml` enables only off Windows (the checking allocator hung nih-plug's test setup on Windows; `docs/BACKLOG.md` history). It runs on Linux and macOS, in debug as the file requires. On Windows it is not run; that stays recorded. **To try during implementation:** whether it now works on MSVC; if it does, enable it, otherwise keep the exclusion with the reason.

### 5. Platform-dependent results
Some tests compare against values computed with the platform's math library:
- `mode_banks_match_golden_fixture` (hashes; CLAUDE.md: must match on gnu and musl);
- `generated_bank_equals_the_files_on_disk` (factory presets regenerated in code with seeded draws and stored levels, compared byte-for-byte);
- render hashes are already kept out of CI for this reason.

The first Windows and macOS runs show whether they hold. If one fails because `libm` differs (not a real bug), **stop and ask the maintainer** with the numbers; the options are a tolerance, a per-platform skip with the reason, or making the computation platform-independent. They are not loosened silently.

### 6. Bundling script
`scripts/bundle.sh <linux|windows|macos> [profile]`, bash (Git Bash on the Windows runner):
- builds `--lib` for the platform's target(s) and finds the library under `${CARGO_TARGET_DIR:-../corrosion-target}`;
- **Linux:** `Corrosion.vst3/Contents/x86_64-linux/Corrosion.so`, `Corrosion.clap` (the `.so`);
- **Windows:** `Corrosion.vst3/Contents/x86_64-win/Corrosion.vst3`, `Corrosion.clap` (the `.dll`);
- **macOS:** builds both architectures and `lipo -create`s them; `Corrosion.vst3/Contents/{MacOS/Corrosion, Info.plist, PkgInfo}` and `Corrosion.clap/Contents/{MacOS/Corrosion, Info.plist, PkgInfo}` (a CLAP is a bundle on macOS); then `codesign --force --deep --sign -` on each bundle (arm64 refuses unsigned code);
- output in `target/bundled/<platform>/`.
- `bundle.sh` and `bundle-win.sh` stay as wrappers (`linux`, and a gnu `windows` variant for the maintainer's host via `CORROSION_WINDOWS_TARGET`), so existing habits and docs keep working.
  - *As built (maintainer, 2026-11-10):* both wrappers moved to `scripts/archive/`; the docs and scripts call `scripts/bundle.sh <platform>` directly. On the maintainer's host (no MSVC) the Windows build is `CORROSION_WINDOWS_TARGET=x86_64-pc-windows-gnu scripts/bundle.sh windows`.
- `scripts/check_bundle.sh <platform> <dir>` checks the layout (paths, file types, `lipo -archs` on macOS, `codesign --verify`) and runs in CI after bundling; locally it runs on Linux.

### 7. Validators
- **clap-validator:** the release binary for each OS, pinned to a version and downloaded from its GitHub releases; `clap-validator validate <Corrosion.clap>` (on macOS the bundle path).
- **pluginval:** the release binary for each OS, pinned; `pluginval --strictness-level 5 --skip-gui-tests --validate <Corrosion.vst3>` (level 5 is what `scripts/validate-plugins.sh` already uses). On Linux it runs under `xvfb-run` (JUCE wants a display even without GUI tests).
- If a validator reports a real problem in the plugin, it's fixed in this change if small; otherwise **stop and ask** with the report (fix first, or lower the level / skip one test with the reason, temporarily).

### 8. Cache
`CARGO_TARGET_DIR: ${{ github.workspace }}/target` in every job, so `Swatinem/rust-cache` caches the actual build. Keys per job and platform (`checks-<os>`, `bundle-<os>`).

### 9. Nightly release
Same mechanism as today, extended to six zips: `gh release delete nightly --yes --cleanup-tag`, then `gh release create nightly … --prerelease --target $GITHUB_SHA`. Notes list the commit, where each format goes on each OS, that the bundles passed clap-validator and pluginval, that they are not signed for distribution, and the macOS first-run step (`xattr -dr com.apple.quarantine <bundle>`). The delete-then-create gap is a few seconds without a nightly; accepted (as today).

## Risks / Trade-offs

- **Minutes and time:** three platforms × two jobs; macOS runners are the slowest. The cache (decision 8) and cancelling stale runs (decision 2) keep it bounded. First runs will be long (cold cache).
- **Platform-dependent tests** (decision 5) may fail on the first Windows or macOS run.
- **Validators may find real issues** (decision 7) that block green until fixed.
- **Untestable locally:** Windows and macOS jobs are verified only by pushing to `dev` and reading the run; the nightly publish only by a run on `main` (after the maintainer merges, or a manual run on `main`).
- **macOS without notarization:** Gatekeeper quarantines downloaded bundles; documented, not solved.

## Measurements

Validator results per platform are in "Validators green" below; the platform-dependent tests are in "First runs on Windows and macOS".

### Run times (2026-11-14)

Cached: run #284 on `dev` (25a5e9a), every job green, about 10 min end to end (`checks` start to `bundle` end). Cold: run #100 (attempt 2) on `main`, the first run with no cache (the cache step took 1 s). Windows and macOS failed their tests in that run, so it gives a complete cold time for Linux only. Bundle jobs have never run cold here: their first run already had a cache.

| Job | Cold (#100) | Cached (#284) | Longest steps (cached) |
|---|---|---|---|
| checks (linux) | 8 min 12 s | 5 min 30 s | CI-lane tests 2:06, GUI lane 2:02 |
| checks (windows) | — (tests failed at 3:58) | 8 min 14 s | CI-lane tests 3:31, GUI lane 3:21 |
| checks (macos) | — (tests failed at 3:39) | 5 min 14 s | CI-lane tests 2:19, GUI lane 2:20 |
| bundle (linux) | — | 53 s | build 11 s, validators 2 s |
| bundle (windows) | — | 1 min 31 s | build 32 s, validators 3 s |
| bundle (macos) | — | 1 min 22 s | build 41 s (universal), validators 4 s |
| web | — | 2 min 46 s | build 41 s, browser checks 40 s |
| guide | — | 23 s | build 6 s |

Clippy took 27–61 s cold and 5–14 s cached. The tests dominate, and their time is running, not compiling: `every_sound_shaping_control_changes_its_note` and `every_factory_preset_meets_the_quality_floor` each run over a minute in the GUI lane. The nightly's first run on `main` took 11 min (#108).

### `no_alloc` on Windows (2026-11-14)

Design decision 4 said to try it. Cargo.toml still leaves nih-plug's `assert_process_allocs` off on Windows; a CI step turns it on for `no_alloc` alone (`--features nih-plug/assert_process_allocs`).

- **Trial** (run #290, `continue-on-error`, 10-minute timeout): 12 of 12 passed on MSVC in 0.78 s, compiled in 39 s. No hang. On its own this proved nothing: a missing checker passes too.
- **Sentinel:** new test `the_checker_is_armed` runs a deliberately allocating test (`allocation_aborts_under_the_checker`, ignored) as a child of the test binary and requires it to abort. Locally it is green in debug and red in release, where the checker compiles out.
- **A wrong turn, recorded:** runs #292 and #294 failed on Windows. I first read that as "the checker isn't installed on MSVC". The failure was in the *workspace* test step, which also runs `tests/no_alloc.rs`, but without the feature, so the checker is absent there by design. The feature step never ran. On Windows the sentinel now applies only when `CORROSION_EXPECT_ALLOC_CHECKER` is set, which only the no_alloc step does; off Windows it always applies.
- **Result** (run #296, `3cd53c7`): the Windows no_alloc step passed in 33 s with the sentinel required. The checker is installed and catches allocations on MSVC, and nothing hangs.
- **As built:** the step is permanent on Windows (no `continue-on-error`; the timeout stays). All three platforms now run the real-time tests, each with a sentinel proving the checker is armed.

### First runs on Windows and macOS (2026-11-10)

Runs #74–#100 never started: the organization's Actions billing blocked
them ("recent account payments have failed or your spending limit needs to
be increased"), not the workflow. Once billing was fixed, the first run on
`main` (#100, attempt 2) gave these results:

| Test | Linux | Windows (MSVC) | macOS (aarch64) |
|---|---|---|---|
| `dsp::envelopes::tests::loop_output_matches_golden` | pass | hash differs | hash differs |
| `dsp::resonators::core::tests::mode_banks_match_golden_fixture` | pass | pass | hash differs (first case: Plate scale 0.5) |
| `every_factory_preset_meets_the_quality_floor` | pass | `felt_mallet/tank` at −8.0 dB (±6) | pass |

Everything else passed on all three platforms (437 tests).

**Golden hashes (maintainer's choice: Linux only).** Both tests hash output
whose values come from the platform's libm (`powf`, trigonometry, `exp`).
They now run on Linux only, where gnu and musl agree. Elsewhere they are
ignored, with the reason in `#[cfg_attr(not(target_os = "linux"), ignore = …)]`.
The level, character and decay tests still run on every platform.

**`felt_mallet/tank` (maintainer's choice: investigate, then re-draw).** A
temporary diagnostic, run on the Windows runner, printed the level with each
parameter reset to the base preset:

- Parameters that do nothing for this preset (Rust, Width, Chaos Depth)
  gave exactly the platform's own level: 0.00 dB on Linux, −8.0 dB on
  Windows. The render itself is consistent on each platform.
- Other parameters flipped the two platforms between the same two states.
  Resetting Compression Curve gave −6.2 dB on Linux and +0.1 dB on Windows.
  Resetting Violence gave −8.9 dB and −0.6 dB.

So the preset is bistable, and rounding decides which state it falls into.
On Linux, changing Size, Damage or Pressure by 0.01 % moved it 4–8 dB. The
engine is not at fault: the first seed happened to land on a knife edge.

**As built:** the seeder also rejects an experimental preset whose floor
level moves more than `MAX_LEVEL_INSTABILITY_DB` (2 dB) when Size, Damage,
Drive or Pressure moves by +0.01 % or −0.1 % (`level_instability_db`), and
re-draws it.

Bank ranking on Linux:

| Preset | Spread |
|---|---|
| `felt_mallet/tank` | 8.06 dB |
| `tension_snap/chain` | 3.75 dB |
| next highest (`tension_rise/sheet_metal`) | 1.65 dB |
| `hard_mallet/pipe` (reference) | 0.03 dB |
| every other preset | ≤ 1.65 dB |

Re-drawn to try 1:

| Preset | Output | Level before |
|---|---|---|
| `felt_mallet/tank` | 1.0 → 2.49 | — |
| `tension_snap/chain` | 31.22 → 1.72 | −3.85 dB |

No other preset changed. Covered by the tests
`level_stability_rejects_a_knife_edge_preset`,
`level_stability_accepts_a_steady_preset` and
`the_felt_mallet_tank_in_the_bank_is_stable`, plus the ignored
`print_experimental_level_instability`. The temporary diagnostic and its CI
step are removed.

### First validator run (2026-11-10, run #102)

pluginval (strictness 5, no GUI tests) passed on Linux. clap-validator 0.3.2
ran 21 tests on each platform: 17 passed on Linux and macOS, 16 on Windows,
and the rest were skipped (preset discovery is not implemented). One test
failed on all three platforms, `param-conversions`, on Filter Cutoff:
`0.1171646878594357 → '294.2767' → 0.11716467887163162 → '294.27667'`.
The text a parameter shows has to read back to the same text.

**Cause:** parameters without a formatter showed every digit of the `f32`,
and the last digit moves when the value goes from normalized to plain and
back. `display_float_param` used six decimals, which is also more than an
`f32` holds above 1.

**Test:** `params::tests::parameter_text_round_trips` checks every parameter
over a grid, the reported value and 20 000 pseudo-random values. Before the
fix it failed on 14 parameters: Size, the six envelope times, Time Scale, Tip
Stiffness, Plate Aspect, Chain Link Mass, Filter Cutoff, plus Pressure,
which showed `-0.0 dB` against `+0.0 dB`.

**Fix:**
- Every parameter without its own formatter (74 constructors, including
  `display_float_param`) shows 5 significant digits, with trailing zeros
  dropped (`significant_text`). For example 1100.0438 shows as 1100 and
  0.50075006 as 0.50075.
- Pressure rounds before formatting, so a value just under 0 dB shows
  `+0.0 dB`.

Only the displayed text changes; no parameter's value or sound changes.

### Validators green (2026-11-10, run #104)

Run #104 (merge of `main` into `dev`, same tree as the param-conversions fix)
passed every job: checks and bundles on Linux, Windows and macOS.

| Platform | pluginval v1.0.4 (VST3, strictness 5, no GUI) | clap-validator 0.3.2 (CLAP) |
|---|---|---|
| Linux | passed | passed (step green; count not read) |
| Windows | passed | 17 passed, 0 failed, 4 skipped |
| macOS | passed | passed (step green; count not read) |

The four skipped clap-validator tests are the preset-discovery ones: the
plugin implements no preset-discovery factory. Validation takes 2–3 s per
platform.

### First nightly (2026-11-10, run #108 on `main`)

The merge of the CI fixes into `main` (6af5fa2) ran every job green, and
`nightly` replaced the rolling pre-release: "Nightly (6af5fa2)", marked
pre-release, with the six zips (VST3 and CLAP for Linux x86_64, Windows
x86_64 and macOS universal; 2.2–5.1 MB each) and the generated notes
(install folders per OS, the macOS quarantine step). The run took 11 min
(19:01–19:12 UTC).

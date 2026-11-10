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

(to be filled during implementation: run times per job cold and cached, validator results per platform, platform-dependent test results, `no_alloc` on Windows)

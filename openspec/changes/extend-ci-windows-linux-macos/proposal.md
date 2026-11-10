# Proposal

## Why

The CI runs only on Linux, on pushes to `main` and on pull requests. Windows is cross-compiled and never tested or loaded, macOS isn't built at all, the bundles are never validated, and the work happens on `dev`, where no CI runs. The maintainer wants an extensive CI on Linux, Windows and macOS that bundles the plugin and keeps one nightly release, overwritten by every passing run on `main`.

## What Changes

- **Triggers:** the CI runs on every push to `dev` and `main`, on pull requests and by hand. A newer push to the same branch cancels the run in progress.
- **Quality gates on three platforms:**
  - **Linux** (`ubuntu-latest`): fmt, clippy `-D warnings`, the CI-lane tests (musl), the GUI lane (gnu), the real-time safety tests (`no_alloc`, debug), `cargo doc` with warnings denied, the benchmarks compiled (`--no-run`), and the offline renderer smoke test with its WAV check.
  - **Windows** (`windows-latest`, `x86_64-pc-windows-msvc`, the maintainer's choice): clippy, the CI-lane tests, the GUI lane and the renderer smoke test.
  - **macOS** (`macos-latest`, Apple Silicon): clippy, the CI-lane tests, the GUI lane, `no_alloc` and the renderer smoke test.
- **Bundles for all three:** VST3 and CLAP for Linux (x86_64), Windows (x86_64, MSVC) and macOS (universal: arm64 + x86_64, ad-hoc signed, not notarized), built on their own runners by one bundling script.
- **Validation:** `clap-validator` and `pluginval` run against the built bundles on each platform; a validator failure fails the run.
- **Nightly release:** after every fully green run on `main` (or a manual run on `main`), the `nightly` pre-release is replaced by that commit's six zips. Pull requests and `dev` never publish. A failing run leaves the previous nightly in place.
- **Fixes found on the way:** the CI never cached its build (the target directory sits outside the workspace, `../corrosion-target`); `scripts/validate-plugins.sh` looked for the CLAP in the wrong place.

## Capabilities

### New Capabilities
- `continuous-integration`: when the CI runs, what it checks on each platform, the bundles it produces, the validators, and the nightly release.

### Modified Capabilities
(none)

## Impact

- `.github/workflows/ci.yml` (rewritten: `checks`, `bundle` and `nightly` jobs, platform matrices).
- `scripts/bundle.sh` (new, per-platform bundle layouts, honours `CARGO_TARGET_DIR`); `bundle.sh` and `bundle-win.sh` become thin wrappers so local use keeps working.
- `scripts/validate-plugins.sh` (fixed CLAP path, platform-aware), `scripts/verify-local.sh`.
- Possibly tests whose results depend on the platform's math library (the golden mode-bank hashes, the factory files): if they fail on MSVC or macOS the fix is a decision for the maintainer (see design).
- README (downloads, macOS quarantine note), `docs/ARCHITECTURE.md` §10, CLAUDE.md, `docs/BACKLOG.md` (the Windows lane item).
- No plugin code or sound changes are intended.

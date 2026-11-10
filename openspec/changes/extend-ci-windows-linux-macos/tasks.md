# Tasks

## 1. Bundling (local first)

- [x] 1.1 `scripts/check_bundle.sh <platform> <dir>`: checks each format's layout, file type, macOS architectures and signature; run it red against today's Linux output (paths under `target/bundled/linux/` don't exist yet)
- [x] 1.2 `scripts/bundle.sh <linux|windows|macos> [profile]` with the three layouts, `CARGO_TARGET_DIR`, `lipo` and ad-hoc `codesign` on macOS; `bundle.sh` / `bundle-win.sh` as wrappers; `check_bundle.sh linux` green locally
- [x] 1.3 Fix `scripts/validate-plugins.sh` (CLAP path, platform argument); `scripts/verify-local.sh` uses the new script

## 2. Workflow

- [x] 2.1 Triggers (`dev`, `main`, pull requests, manual) and concurrency; `CARGO_TARGET_DIR` and cache keys; lint the workflow with `actionlint`
- [x] 2.2 `checks` matrix (Linux, Windows MSVC, macOS) with every lane of decision 1; push to `dev` and read the run
- [x] 2.3 Resolve what the first Windows and macOS runs show (platform-dependent tests: stop and ask if one needs a tolerance or a skip); try `no_alloc` on Windows and record it
- [x] 2.4 `bundle` matrix: build with `scripts/bundle.sh`, `check_bundle.sh`, zip, upload; macOS universal
- [x] 2.5 Validators: pinned clap-validator and pluginval per platform; resolve or escalate what they report
- [x] 2.6 `nightly` job: main-only, after every bundle job, own concurrency group, six zips, release notes

## 3. Docs and close-out

- [ ] 3.1 Measurements in `design.md` (run times cold and cached, validator results, platform-dependent tests, `no_alloc` on Windows)
- [ ] 3.2 README (nightly downloads per OS, macOS quarantine step), `docs/ARCHITECTURE.md` §10, `docs/BACKLOG.md` (Windows lane done), CLAUDE.md
- [ ] 3.3 Local close-out lanes; CLAUDE.md "Current state"; commit and push to `dev`; the nightly is verified on the first run on `main` (maintainer's merge or a manual run)

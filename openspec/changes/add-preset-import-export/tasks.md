## 1. Tests first

- [x] 1.1 Write tests for `import_preset_files` against temp dirs:
  - a web-app download imports and parses;
  - a name clash gets "(2)", case-insensitively;
  - a bad file is skipped and counted;
  - nothing is written outside the target;
  - the last preset is returned.

  See them fail (the function doesn't exist yet).
- [x] 1.2 Write tests for `export_preset`:
  - the extension is added when missing;
  - the output is byte-identical to Save's;
  - export then import restores every parameter but UI Scale.

## 2. File logic

- [x] 2.1 Implement `import_preset_files` and `export_preset` in `src/presets/mod.rs` (migrate, then parse, then free name, then write). Make 1.1 and 1.2 pass.

## 3. Editor

- [x] 3.1 Add `rfd` (`xdg-portal`, no GTK) behind the GUI feature. Confirm `cargo build --no-default-features` doesn't build it, and record the Linux bundle size before and after.
- [x] 3.2 Add the Import… and Export… buttons next to Open folder:
  - a dialog worker with a one-slot result polled per frame (the main-thread API on macOS);
  - buttons disabled while a dialog is open;
  - messages for imported / skipped / failed / no dialog;
  - a silent cancel.
- [x] 3.3 Add editor tests for the message and enablement logic: no user folder disables Import, a dialog error shows the fallback, a cancel shows nothing.
- [x] 3.4 Check in REAPER on Linux (`docs/REAPER_TESTING.md`): the buttons, the fallback message without a portal, and Export through a portal if one can be run. Take screenshots.

## 4. Docs and close-out

- [x] 4.1 Wiki (EN + ES): rewrite the presets transfer section around Import and Export, keep the manual path as the fallback, update Troubleshooting and the editor page. Run `export_guide_data` if any control text changed.
- [x] 4.2 `docs/ARCHITECTURE.md` (presets, GUI dependencies) and the README's feature list.
- [x] 4.3 Run `cargo fmt --check`, clippy (`-D warnings`, CI lane and `--lib`, with no new warnings), the CI and GUI test lanes, no_alloc, `npm run check -- --complete`, and `openspec validate add-preset-import-export`. Record numbers in `design.md` `## Measurements`.
- [x] 4.4 Update "Current state" in `CLAUDE.md`; commit and push to `dev`.

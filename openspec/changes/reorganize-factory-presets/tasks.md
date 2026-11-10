# Tasks

## 1. Tests first

- [x] 1.1 Replace the factory-file fixtures in the 24 failing test runs (14 CI, 10 GUI, 9 shared) with in-test fixtures, each with a comment on what it needs; the benchmark and `render_presets` get a fixture or a path. Check each still tests its original claim; the full lanes go green with the bank still empty
- [x] 1.2 Bank tests over `presets::factory` (fail until it exists): five folders × 12, unique names, the experimental grid covers 16 × 9 exactly once, regeneration is byte-identical, parameters vary, and the generated presets equal the files on disk
- [x] 1.3 Quality-floor test (all presets) and character tests (curated: bass low, pitched, chords harmonic/sustaining/clean, drones sustain, percussion decays), with the measurement helpers; verify each fails on a deliberately wrong preset
- [ ] 1.4 User-preset tests: recursive listing with folders, save into a subfolder, the folder selector lists only user folders, Open folder builds the right command per platform (the command is built by a pure function and run separately), a disabled button when there is no folder
- [ ] 1.5 Browser tests (GUI lane): factory submenus in order, Experimental nested by exciter, User subfolders, arrows wrap within the folder

## 2. Generator and bank

- [x] 2.1 `presets::factory` module: layout, slugs and names; the seeder becomes a writer over it, with recursive output folders
- [x] 2.2 Experimental generator: per-pair seed, playable ranges, every slot/character/shared/post parameter varied, play mode by family
- [x] 2.3 Levelling and quality floor in the seeder (target loudness constant, re-draw up to 8 seeds, curated failures stop the run)
- [x] 2.4 Curated presets: 12 bass, 12 lead, 12 chords, 12 drones, 12 percussion, designed by hand with intent comments
- [x] 2.5 Regenerate `presets/factory/` (204 files) with `cargo run --bin seed_presets`

## 3. Editor and user presets

- [ ] 3.1 Recursive factory loader with folder paths; menu-button browser with nested submenus; folder-scoped arrows
- [ ] 3.2 Recursive user listing; Save folder selector restricted to user folders; saving into subfolders
- [ ] 3.3 Open folder button (create, then the platform command; errors shown; disabled without a folder)

## 4. Docs and close-out

- [ ] 4.1 Measurements in `design.md`: target loudness, per-preset table (level, centroid, decay, partial deviation, flatness, pass), re-drawn seeds, bank-test CI time
- [ ] 4.2 REAPER check at 75 %: browse every folder, the arrows, save into a subfolder, Open folder; screenshot
- [ ] 4.3 CLAUDE.md: replace the "factory presets are placeholders" rules (the levels matter again; `preset_levels_are_sane` passes on the new bank); README preset section; `docs/ARCHITECTURE.md`
- [ ] 4.4 `cargo fmt --check`, clippy `-D warnings`, tests on gnu and musl, GUI lane, no_alloc, `cargo doc`; update CLAUDE.md "Current state"; commit and push to `dev`

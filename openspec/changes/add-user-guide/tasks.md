# Tasks

## 1. Move the help and control tables (no behaviour change)

- [x] 1.1 Move `help_for` and the control tables from `src/gui/editor.rs` to a feature-free `src/gui/controls.rs`; the editor imports them. GUI lane (`cargo test --lib`) and CI lane green, help coverage tests unchanged (as built: also `FIXED_CONTROLS`, the selector option lists and `selector_items`, guarded by `fixed_controls_match_the_editor`; see design.md)

## 2. Guide data (tests first)

- [x] 2.1 Test first: `guide_data_equals_the_files_on_disk` (CI lane) and a test that every parameter of `param_map()` is in the export; run red (no files) (as built: plus `guide_data_lists_every_model_and_mode`; all three red against an empty export)
- [x] 2.2 `export_guide_data` binary: `parameters.json` and `models.json` (Decision 3); run it, tests green; remove the temporary `allow(dead_code)` at the top of `src/gui/controls.rs`
- [x] 2.3 Mutation checks: change one parameter's default and one model name and see the drift test fail, naming them (as built: also a deleted file; see design.md)

## 3. Guide site skeleton

- [x] 3.1 `web/guide/` Starlight project, exact pins and lockfile, `base: '/wiki'`, `outDir: '../dist/wiki'`, locales EN (root) and ES, Pagefind, `starlight-links-validator`
- [x] 3.2 Dark-only theme from `web/theme/tokens.css` (`guide.css` mapping, empty `ThemeSelect`, `data-theme="dark"`), header links to `/` and `/play/`, `wiki/404.html`
- [x] 3.3 `check.mjs` first (twins and heading sequence, coverage via `covers:`, ES IDs, clip references, `--complete` stub rule); run red against an empty content folder
- [x] 3.4 Every sitemap page in EN and ES with title, headings, `covers:` front matter and `<Stub />` body; translated sidebar labels; `npm run check` green
- [x] 3.5 `ParamReference` component and the six reference pages per panel (EN and ES)
- [x] 3.6 `parameters.es.json`: Spanish help for every parameter ID (translation of the ⓘ texts, maintainer to review in `write-user-guide-content`)
- [x] 3.7 Mutation checks: delete one ES page, add a heading to one EN page, remove one ES ID, break one link; each fails with a clear message

## 4. Audio clips (tests first)

- [x] 4.1 Test first: `guide_clip_plan_covers_every_parameter` and `guide_clip_manifest_is_complete_and_audible` (CI lane); run red (as built: the manifest test is `#[ignore]`d until 4.3 renders the clips, so `dev` stays green; 4.3 removes the ignore)
- [x] 4.2 `src/guide/clip_plan.rs`: base patch, per-parameter entries, exclusions with reasons, comparison sets, preset, kit and demo lists. Render candidate base patches and comparison sets, and choose them with the maintainer by ear before the full render; record in design.md (as built: base patch D, Hand Strike on Plate; exciters compared on Tank, objects with Felt Mallet; demos provisional)
- [x] 4.3 `render_guide_clips` binary and `scripts/build-guide-clips.sh` (`opusenc --bitrate 64`, manifest); generate all clips; tests green
- [ ] 4.4 `<Clips>` component (shared lazy `<audio>`, translated labels, keyboard access); wired into `ParamReference`, the exciter and object pages and the kit page
- [ ] 4.5 Home page audio demos (`web/site/`) from the generated demo clips
- [ ] 4.6 Mutation checks: a planned parameter removed, a clip file deleted, the plan changed without regenerating; each test fails

## 5. Build, CI, docs

- [ ] 5.1 `web/scripts/build.sh` builds the guide into `dist/wiki/`; `cloudflare-build.sh` unchanged or extended if Node needs setup; full local build and `site.cjs` against it
- [ ] 5.2 `ci.yml` `guide` job (Node 22: `npm ci`, `npm run check`, `npm run build`)
- [ ] 5.3 Docs: `docs/ARCHITECTURE.md` (guide layout, exporter, clip generator, checks); CLAUDE.md rules (regenerate guide data after a parameter change, clips after an engine-wide sound change, like the preset bank); README link to the guide
- [ ] 5.4 Record in design.md `## Measurements`: clip count and total size, generator run time, guide build time, page count
- [ ] 5.5 Close-out lanes (fmt, clippy `-D warnings`, CI test lane, GUI lane, guide check and build); commit and push to `dev`; update CLAUDE.md Current state

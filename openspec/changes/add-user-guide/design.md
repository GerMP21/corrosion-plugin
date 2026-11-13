# Design

## Context

- `add-site-shell` gives the site its layout: one Worker with static assets, `dist/` assembled by `web/scripts/build.sh`, and shared tokens in `web/theme/tokens.css` checked against `src/gui/palette.rs`. This change depends on it.
- The ⓘ help texts are `help_for(label)` in `src/gui/editor.rs`, keyed by display label. The model-dependent slot labels are in `ExciterControlSpec` and `ResonatorControlSpec` tables (`id`, `label`, `min`, `max`) in the same file. `editor.rs` is compiled only with `gui` or `web-gui`, so the CI lane (`--no-default-features`) can't reach them.
- The plugin has 149 `#[id]` parameters. `corrosion::offline::render_preset` renders a preset through the full output path (H13). `presets::factory::TARGET_RMS` is 0.13.
- The factory bank's pattern already solves "data generated from code, committed, drift-tested": `seed_presets` writes the files, and `generated_bank_equals_the_files_on_disk` fails on drift. The guide data and clips follow it.
- The Cloudflare build image has Node (it runs `npx wrangler`). The container has Node 22.

## Goals / Non-Goals

**Goals:**
- Whatever can come from the code (parameters, ranges, defaults, help texts, model lists, clips) comes from the code, with a CI test against drift.
- A writer of the content change only writes prose. Pages, routes, components, data and clips already exist.
- Readers can hear each control, not just read about it.

**Non-Goals:**
- Translating the plugin's UI or ⓘ texts (they stay English).
- Writing the prose (`write-user-guide-content`).
- A light theme, comments, versioned docs, analytics.
- Asserting clip hashes: renders aren't bit-portable across libm, so clips are checked by plan fingerprint and level, not by bytes.

## Decisions

### 1. Astro Starlight, under `/wiki`, English as the root locale
`web/guide/` is a Starlight project with:
- `base: '/wiki'`;
- `outDir: '../dist/wiki'`;
- `locales: { root: { label: 'English', lang: 'en' }, es: { label: 'Español', lang: 'es' } }`.

English content lives in `src/content/docs/` and Spanish in `src/content/docs/es/`, giving `/wiki/<page>/` and `/wiki/es/<page>/`. Starlight provides the language picker, Pagefind search (indexed per language at build), MDX, a sidebar with translated labels, and a fallback notice for untranslated pages (which the twin check makes moot).

Versions are pinned exactly in `package.json` (no ranges) with a committed `package-lock.json`, installed with `npm ci`. Node 22 in CI.

*Alternatives:*
- mdBook (Rust-native): weaker i18n, no MDX components for clips and tables.
- Docusaurus: React and heavier, with the same features.
- Hand-written HTML: no search, no i18n, no link checks.

### 2. Dark-only theme from the shared tokens
- `customCss` imports `web/theme/tokens.css` and a `guide.css` that maps Starlight's variables to the tokens (`--sl-color-bg: var(--bg)`, `--sl-color-accent: var(--accent)`, …).
- The `ThemeSelect` component is overridden with an empty one, and `Head` sets `data-theme="dark"` before paint.
- Starlight's light palette is never reached.
- The header links back to `/` and `/play/`.
- If Vite refuses the import from outside the project root, the build copies `tokens.css` into `web/guide/src/styles/` first (generated, git-ignored).

### 3. Guide data exported from the plugin
- **Move the tables.** The help table and the control tables move from `editor.rs` to a feature-free `src/gui/controls.rs`: `help_for`, `ExciterControlSpec`, `ResonatorControlSpec`, the per-model tables and the labels of fixed controls. The editor imports them. This is a move only, so the GUI lane's help coverage tests keep passing unchanged.
- **`export_guide_data` writes `web/guide/src/data/parameters.json`.** For each parameter, from `CorrosionParams::param_map()` and the control tables:
  - `id`, `name`, `panel` and `group` (the editor section it sits in);
  - `kind` (float, int, bool, enum);
  - `min`, `max` and `default` as display strings (the parameter's own `value_to_string`);
  - `steps` for enums (every value's display string);
  - `unit`;
  - `slots` for model-dependent parameters: `[{ model, label, min, max }]`;
  - `help`: the ⓘ text for its label, or per slot.
  
  Parameters with no editor control (internal or migration-only) are marked `hidden: true`. They are still listed, so the reference is complete.
- **It also writes `models.json`:** exciters, objects, Space modes, quality modes, note modes, limiter modes and randomizer modes (display names and indices), plus the factory folders with their preset names and the drum kits.
- **Drift test.** `guide_data_equals_the_files_on_disk` (CI lane) runs the exporter in memory and compares it with the committed files, naming the first differing parameter. The format is pretty-printed, key-sorted JSON, so diffs are readable.
- **Spanish.** `parameters.es.json` (hand-written) maps `id → { help, slots?: { model → help } }`. The names stay English. The guide check verifies that the ID sets match.

### 4. Reference pages render from the data
The reference has one page per panel (Header, Macros, Exciter, Resonator, Processing, Kit), as thin MDX twins:
```mdx
---
title: Exciter parameters
covers: { panels: [exciter] }
---
<ParamReference panel="exciter" />
```
`ParamReference` reads the JSON and the page's locale, and renders one card per parameter:
- name and ID;
- range and default;
- per-model labels;
- the help text (EN from `parameters.json`, ES from `parameters.es.json`);
- its `<Clips>` row.

Narrative pages link to the cards by anchor (`/wiki/reference/exciter/#rust`). Search indexes the rendered cards, so "óxido" finds Rust on the Spanish reference.

### 5. Coverage through front matter
Every page declares what it covers in front matter, for example `covers: { exciters: [Bow] }`, `covers: { modes: { space: [Spring] } }` or `covers: { features: [kit-editor] }`. The check reads `models.json` and a short list of features: `kit-editor`, `user-presets`, `midi`, `expression`, `ui-scale`, `web-app`, `randomizer`, `macros`. It fails for anything not covered in both languages.

### 6. One check script: `npm run check`
`web/guide/scripts/check.mjs` runs in CI and before the build. It verifies:
1. Every page has a twin in the other language, with the same heading sequence by level.
2. Coverage (Decision 5).
3. The ID sets of `parameters.json` and `parameters.es.json` are equal.
4. Every clip a page or component references is in `clips.json`, and its file exists.
5. Placeholder pages are allowed in this change. With `--complete`, any `<Stub />` fails; `write-user-guide-content` turns that flag on in CI.

Internal links are checked at build by the `starlight-links-validator` plugin.

### 7. Clip generator: `render_guide_clips`
- **Plan, as code.** `src/guide/clip_plan.rs` (feature-free) has:
  - the base patch `guide_base_patch()`: the default patch with Space off and the limiter at its default, to be confirmed by ear with the maintainer;
  - per-parameter entries (`Triplet`, `VelocityPair`, `LongNote`, `Tempo { bpm }`, or `Excluded { reason }`), with optional `quiet: [Min]` declarations that carry a reason;
  - the comparison sets: exciters on Plate, objects with Hard Mallet, both on C3 at velocity 100 (the shared object and exciter are chosen by ear at implementation and recorded here);
  - representative presets per exciter and object, the drum kits, and the home-page demos.
  
  The plan's fingerprint is a hash of its `Debug` form.
- **Render.** Each clip renders at 48 kHz in Render quality through `offline::render_preset`: 3 s, note-off at 2 s.
- **Level.** Each set gets one gain, so its reference clip (the middle value, or the set's first) sits at `TARGET_RMS`. The gain is then lowered until every clip's peak is ≤ −1 dBFS.
- **Encode.** The binary writes WAVs to a temp dir. `scripts/build-guide-clips.sh` then runs it and encodes with `opusenc --bitrate 64` into `web/guide/public/clips/<group>/<name>.opus`, and writes `clips.json` (source, value, RMS and peak before encoding, plan fingerprint, generator commit).
- **Tests (CI lane, no audio decoding).** `guide_clip_plan_covers_every_parameter` (every parameter planned or excluded), and `guide_clip_manifest_is_complete_and_audible`. The second checks that every planned clip is in the manifest and on disk, that its RMS is > −60 dBFS unless declared quiet, and that the fingerprint matches the plan.
- **No asserted hashes.** Clips are rendered with the gnu toolchain, and hashes aren't portable.

*Alternative:* rendering clips during the Cloudflare build. That needs a native release build of the crate plus an Opus encoder in the build image, adds minutes and moving parts to every deploy, and can produce different bytes per build. Committing them follows the preset bank's pattern.

### 8. `<Clips>` component
`<Clips param="rust" />` (or `set="exciters" item="Bow"`) renders small labelled buttons: Min, Mid and Max, or the item name. Each plays its file through one shared `<audio preload="none">`, so pages with many clips load nothing until clicked. Labels are translated, and the component is keyboard-accessible.

### 9. Build and CI
- `web/scripts/build.sh` runs `npm ci && npm run check && npm run build` in `web/guide/` after the app, so the guide lands in `dist/wiki/` next to the home page and `play/`.
- The guide ships `wiki/404.html`.
- `ci.yml` gets a `guide` job (setup-node 22, `npm ci`, `npm run check`, `npm run build`) that doesn't need Rust. The Rust drift and clip tests run in the existing test lane.

## Risks / Trade-offs

- [Repository growth from clips]
  - Estimate: about 140 parameters × 3, plus about 25 comparison clips, 25 preset clips, 10 kit clips and 5 demos ≈ 470 clips × 3 s × 64 kbps ≈ 11 MB, rewritten on each sound change.
  - Mitigation: regenerate only for intended sound changes (like the golden fixture); 48 kbps if size matters (≈ 8.5 MB).
  - Git LFS is avoided because Workers Builds would need LFS support.
  - Recorded in Measurements with the real size.
- [Clips drift from the engine silently] → The fingerprint catches plan changes but not engine changes. CLAUDE.md gets a rule: a change that moves timbre engine-wide (the same trigger as reseeding presets) regenerates the clips, and the close-out says so.
- [Default patch's long decay] (backlog item) → The base patch is chosen for clarity, not inherited blindly. It is confirmed by ear with the maintainer before the first full render.
- [Starlight major-version changes] → Exact pins and a lockfile. Upgrades are deliberate.
- [Moving the control tables churns `editor.rs`] → A move-only commit first, with the GUI lane green before any new code.

## Migration Plan

No user-facing migration. The deploy adds `/wiki/`. Rollback is reverting the merge, which removes `dist/wiki/`; the home page's Guide link then 404s until it is restored.

## Open Questions

- The exact base patch and the shared object and exciter for the comparison sets: chosen by ear at implementation (task 4.2) and recorded in Decision 7's As built note. This doesn't change the specs or tasks.

## As built

### Task 1.1: control tables out of the editor (2026-11-13)

- **Moved to `src/gui/controls.rs`** (no egui; built in every configuration):
  - `ExciterControlId` and `ResonatorControlId`;
  - the control and panel specs, and the 16 exciter and 9 object tables;
  - `exciter_panel` and `resonator_panel`;
  - `exciter_param_ref` and `resonator_param_ref`;
  - `help_for` and the trial `selector_help`.
  
  `editor.rs` imports them, and nothing else changed in it.
- **Added for the exporter:**
  - `FIXED_CONTROLS`: every control that isn't a model slot, in drawing order, with parameter ID, label as drawn, `Panel`, group (the section heading) and `Shown`. `Shown` is `Always`, an exciter family for the AD, MSEG and ADSR envelopes, or a Space mode for its controls. A parameter on several panels (`env_attack`, `env_release`) has one entry per panel.
  - The selectors' option lists (`UI_SCALE_ITEMS`, `QUALITY_MODE_ITEMS`, `PLAY_MODE_ITEMS`, `LIMITER_MODE_ITEMS`, `LOOP_MODE_ITEMS`, `LOOP_STAGE_ITEMS`, `SPACE_MODE_ITEMS`) and `selector_items(id)`, also used by the editor's combos.
  - Coverage: all 149 parameters are covered, 74 in `FIXED_CONTROLS` and 75 in the slot tables.
- **Guard test `fixed_controls_match_the_editor`** (GUI lane). It renders every exciter, object and Space mode, and checks three things:
  - every drawn non-slot control has an entry shown in that configuration;
  - every entry shown there is drawn;
  - the drawn help is `help_for(label)`, or for a selector, `selector_help(label, option)` for one of its options.
  
  Mutation: relabelling Rust as "Rusty" fails it on all 27 configurations.
- **Temporary `allow(dead_code)`** without the GUI features, until the exporter (2.2) uses the tables.
- **Lanes:** fmt; CI-lane clippy `-D warnings`; CI test lane 516 passed; GUI lane 511 passed (one new test); GUI-lane clippy the same 27 warnings as before; web workspace clippy (native and wasm) passes.

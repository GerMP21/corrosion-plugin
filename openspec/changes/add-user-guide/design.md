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

### Tasks 2.1–2.3: guide data exporter (2026-11-13)

- **Module `src/guide/`** (feature-free, `corrosion::guide`): `parameters_json()`, `models_json()` and `guide_files()`. The binary `export_guide_data` writes them to `web/guide/src/data/`. Pretty-printed, keys sorted (`serde_json` without `preserve_order`), trailing newline.
- **`parameters.json`** is `{ "parameters": [...] }`, 149 entries in the editor's order: panel by panel, fixed controls in drawing order, a panel's model slots right after its first section (Exciter: after Model Type, Pressure, Speed, Roughness; Resonator: after Material). Per entry: `id`, `name`, `panel`, `group`, `kind`, `min`, `max`, `default` and `unit`, all display strings from the parameter's own formatter. Then one of:
  - `controls` (fixed controls): `label`, `group`, `shown` (`always`, `family:hit|friction|specialty`, `space:factory|spring|echo`) and `help`. `env_attack` and `env_release` have two (AD and ADSR panels).
  - `slots` (model controls, `group: "Model controls"`): `model`, `group` (the model's panel title), `label`, `min`, `max` (the slot's slider range, formatted) and `help`.
  
  **Differs from Decision 3:** selectors have `options` (`value`, `name`, and the trial `selector_help` text per option) instead of `min`, `max` and `steps`, because their host min and max are value indices (UI Scale's "max" was 145 %, while the menu ends at 150 %). When every option shares one text (UI Scale, loop Start and End), it goes on the control's `help` instead. No parameter is `hidden`: all 149 have an editor control. Kinds: 139 float, 10 int.
- **`models.json`**: `exciters` (value, name, family; editor order), `objects`, `space_modes`, `quality_modes`, `note_modes`, `limiter_modes`, `randomizer_modes`, and `factory`: `folders` (21: the five curated, `drums`, and 16 `experimental/<exciter>`, each with preset names and slugs in browser order) and `drum_kits` (name, slug, pad count).
- **Sizes:** `parameters.json` 88 KB, `models.json` 28 KB.
- **Tests (CI lane):** `guide_data_equals_the_files_on_disk` lists every stale or missing file and the first difference in each (a parameter ID, or a top-level key); `guide_data_lists_every_host_parameter` (the export's IDs equal `param_map()`'s, no duplicates); `guide_data_lists_every_model_and_mode` (16 exciters, 9 objects, the mode counts, every factory preset and kit). All three ran red against an empty export.
- **Mutations:** Rust's default 0 → 0.5 fails with "parameter `rust` differs"; Plate → "Plates" fails naming parameter `object` in `parameters.json` and `factory` in `models.json` (the first differing key; `objects` differs too); deleting `models.json` fails with "is missing". Each restored and green.
- The temporary `allow(dead_code)` in `controls.rs` is gone; the exporter uses the tables in every configuration.
- **Lanes:** fmt; CI-lane clippy `-D warnings` (exit 0; the `include_dir` unused-dependency note is cargo's and was already there); CI test lane 519 passed; GUI lane 514 passed; GUI-lane clippy the same 27 warnings; web workspace clippy (native and wasm) passes.

### Tasks 3.1–3.4 and 3.6: guide skeleton (2026-11-13)

- **Versions** (exact pins, `package-lock.json`): `astro` 7.4.4, `@astrojs/starlight` 0.42.7, `starlight-links-validator` 0.27.0, `js-yaml` 4.3.2 (for `check.mjs`). The newest releases (astro 7.4.12, Starlight 0.43.0) came out the same day, so the pins are the latest at least two weeks old. Astro 7.4.4 needs **Node ≥ 22.23.4** (`engines`); CI and the Cloudflare build must provide it (task 5.1/5.2).
- **Config:** `base: '/wiki'`, `outDir: '../dist/wiki'`, `trailingSlash: 'always'`, locales root `en` and `es`, Pagefind, `starlightLinksValidator({ errorOnLocalLinks: true })`. `customCss` imports `../theme/tokens.css` directly: Vite accepted the path outside the project root, so the copy fallback of Decision 2 isn't needed.
- **Dark-only:** `ThemeProvider` is overridden with one that always sets `data-theme="dark"` before paint, and `ThemeSelect` with an empty component. Checked with Playwright under `colorScheme: 'light'`: the theme is dark, the body is CHARCOAL (`rgb(13, 13, 13)`), no switch is shown. `guide.css` maps Starlight's variables to the tokens and gives `h2` the editor's caps with tracking.
- **Header:** `SocialIcons` is replaced by `SiteLinks` (Home → `/`, Play → `/play/`). The components' UI strings are in Starlight's `i18n` collection (`src/content/i18n/{en,es}.json`).
- **404:** `src/content/docs/404.md` (splash, links to both languages' guide home) builds to `wiki/404.html`. Astro warns that `/404` from `[...slug]` conflicts with the 404 route; it's harmless (the 404 route renders that entry). Without the file Starlight warns instead that the entry is missing.
- **Sidebar** (`sidebar.mjs`): the agreed sitemap, groups with Spanish labels, each group's pages autogenerated and ordered by `sidebar.order`; group overview pages are labelled Overview / Resumen. Starlight 0.39+ needs `items: [{ autogenerate }]` inside a labelled group.
- **Pages:** 74 per language (148, plus the 404), each with `title`, `covers:` where it applies, `<Stub />` and the section headings. Exciter and object pages have the seven model sections (What it is, How it sounds, Controls, Pairings, Tips, Factory presets, Listen / Qué es, Cómo suena, Controles, Combinaciones, Consejos, Presets de fábrica, Escúchalo). Coverage: the 16 exciter and 9 object pages; Space modes on `output-chain/space`; limiter modes on `output-chain/diode-limiter`; note modes on `note-modes/{tonal,drone,kit}`; quality modes on `quality`; randomizer modes and feature on `randomizer`; the seven top-level factory folders (`bass`, `lead`, `chords`, `drones`, `percussion`, `drums`, `experimental`) on `presets/factory-bank`; the features on their pages; the six reference panels on `reference/<panel>`. The limiter modes are covered too, beyond the spec's list, because `models.json` has them.
- **`check.mjs`** (`npm run check`, `--complete`, and `--docs=<dir>` for testing): twins and heading levels, coverage (unknown `covers` entries also fail), Spanish IDs and texts (`help`, and `slots.<model>` / `options.<name>` wherever the English has a text), clip references against `public/clips/clips.json`, and `<Stub />` with `--complete`. Against an empty content folder it failed with 65 problems; it passes on the skeleton (149 pages, 149 parameters).
- **`parameters.es.json`:** 149 entries, translated from the 189 distinct English ⓘ texts (`help`, per-model `slots`, per-option `options`). First pass, for the maintainer's review in `write-user-guide-content`.
- **Build:** 150 pages in about 3.5 s (12.8 s on one later run), all internal links valid.

### Tasks 3.5 and 3.7: reference pages and the checks' mutations (2026-11-13)

- **`ParamReference`** renders one card per parameter of a panel, grouped under the editor's section headings, in editor order. Each card shows the name (an anchor to the card), the ID, the help text (EN from `parameters.json`, ES from `parameters.es.json`), range or options, default, unit, when it's shown (envelope family or Space mode), each option's help, and a per-model table for slots (model, label, range, help). Card ids are the parameter IDs (`/wiki/reference/resonator/#rust`). Labels are in the `i18n` collection.
- **Kit page:** kit pads aren't host parameters, so `reference/kit` renders Kit mode's facts from **`models.json`'s new `kit` block** (exported from `crate::kit`): the drum zone (MIDI 36–59), the largest tune (±48 st), the pad level, and the lists of parameters stored in each pad (116) and shared by every pad (33, `NON_PAD_PARAMS`), each linked to its card. This adds `kit_json()` to the exporter (not in Decision 3).
- **Search scenario:** on `/wiki/es/`, searching "óxido" first found only the Macros reference (the CORROSION text says "más óxido"), because the first-pass Spanish Rust text said "Corroe". It now reads "Cubre el objeto de óxido: …", and the search returns `/wiki/es/reference/resonator/` too (Playwright). The card headings come from the component, so Starlight's per-page table of contents lists only the overview; search and anchors reach the cards.
- **Width:** at 375 px the reference page doesn't scroll sideways (`scrollWidth` 375).
- **Mutations (3.7):**
  - Deleting `es/exciters/bow.mdx` fails with "src/content/docs/exciters/bow.mdx has no ES twin (expected …)" and "exciter `Bow` has no page in: ES".
  - Adding `## Extra` to `objects/pipe.mdx` fails, naming both files and listing both heading sequences.
  - Removing `heat` from `parameters.es.json` fails with "no entry for `heat`".
  - A link to `/wiki/exciters/bowed/` in the glossary fails the build (exit 1), with starlight-links-validator naming the file and the link.
  
  Each was restored, and the check and the build pass again.

### Tasks 4.1 and 4.2 (in progress): clip plan and generator (2026-11-13)

- **`src/guide/clip_plan.rs`** (feature-free): constants (48 kHz, C3 = MIDI 48, velocity 100, 3 s with note-off at 2 s, `TARGET_RMS`, −1 dBFS ceiling, −60 dBFS silence), `guide_base_patch()` (default patch, Space Off, Render quality; provisional), `with_values()` (sets parameters by ID through the preset loader, as kit pads do), `parameter_plans()`, `plan()`, `fingerprint()` (FNV-1a 64 over every clip's reference, file, set, notes, length, tempo and preset JSON; stable across Rust versions, unlike `DefaultHasher`) and `render_clip()`.
- **Parameter plan:** every one of the 149 parameters. Excluded with reasons: `ui_scale`, `quality_mode`, `play_mode`, `limiter_mode`, `exciter` and `object` (heard in the comparison sets), `space_mode` (each mode is heard on its own controls' clips). `VelocityPair` (a soft note at velocity 40, then a hard one at 120, in each clip): `velocity_to_peak`, `velocity_to_level`, `velocity_to_time`. `LongNote` (5 s): `position_wander`, `position_wander_rate`. `Tempo { bpm: 120 }`: `sync_rate`. All others `Triplet`. Output is a Triplet, so its set gain keeps the minimum quieter (the spec's scenario).
- **Setup derived from the control tables:** a model slot selects the first model (editor order) that shows it and uses that slot's slider range for min, mid and max; the envelope panels select a family's exciter (Hit: Hand Strike, Friction: Bow, Specialty: Pneumatic Jet); Space controls turn their mode on with Amount 0.5; `space_amount` uses Factory; `sync_rate` and `delay_time` use Echo; the loop stages use loop Forward on Bow. Fixed controls use the host range's ends and normalized middle.
- **Sets:** exciters on `COMPARISON_OBJECT` and objects with `COMPARISON_EXCITER` (provisional: Plate, Hard Mallet); a representative preset per exciter and per object (the first curated preset using it, else its experimental grid entry); each drum kit's pads in turn, a quarter second apart; five home demos (the first preset of each curated folder, a four-note phrase; provisional). 491 clips in all with the current plan (a scratch render took 13.4 s on 4 cores and 558 MB of float WAVs).
- **Offline renderer:** `render_preset_at_tempo` / `PresetRenderer::render_at_tempo` pass a host tempo to `process_block` (the plain `render` passes `None`, as before), for the Echo Sync clips.
- **`render_guide_clips`** renders the plan (or `--only <prefix>`) over all cores, levels each set and writes 32-bit float WAVs and `clips.json`. `--candidates` writes the listening set instead.
- **Tests:** `guide_clip_plan_covers_every_parameter` failed listing every parameter against an empty plan, and passes. `guide_clip_manifest_is_complete_and_audible` fails ("clips.json is missing") and is `#[ignore]`d until 4.3 so `dev` stays green.
- **Candidates for the maintainer** (103 clips, rendered in 2.8 s on 4 cores): base patches A (default: Hand Strike on Pipe), B (Hard Mallet on Plate), C (Felt Mallet on Plate) and D (Hand Strike on Plate), each with Rust 0/5, Strike Pos 0.05/0.5 and Cutoff 300 Hz/20 kHz; exciters on Plate, Pipe and Tank; objects with Hard Mallet, Felt Mallet and Hand Strike. RMS levels across a probe barely move for some (Cutoff 300 Hz moves B by 0.1 dB), which says nothing about timbre; the choice is by ear.
- **Near-silent minimums in the scratch render** (to settle before 4.3, either declared `quiet` with a reason or reported as a bug): `param:output:min` (−∞: Output's minimum mutes, expected), `param:coil_proximity:min` (−∞: Electromagnetic Hum with the coil at 0 couples nothing), `param:internal_rattle:min` (−62 dBFS: Metal Chain with no rattle is almost silent, which is unexpected and goes to the maintainer). Next lowest: Air Pressure min −55.8, Cutoff min −55.6 dBFS.
- **Base patch chosen (maintainer, 2026-11-13): candidate D, Hand Strike on Plate** (the default patch with the object set to Plate, Space Off, Render quality). `guide_base_patch()` now sets it. The comparison object, comparison exciter and home demos are still open.
- **Comparison sets chosen (maintainer, 2026-11-13):** the exciters on **Tank**, the objects with **Felt Mallet** (`COMPARISON_OBJECT`, `COMPARISON_EXCITER`). The home demos stay provisional (the first preset of each curated folder, a four-note phrase); the maintainer hasn't picked them.

### Task 4.3: clips rendered (2026-11-13)

- `scripts/build-guide-clips.sh` runs `render_guide_clips` (release, host target), encodes every WAV with `opusenc --quiet --bitrate 64` into `web/guide/public/clips/<group>/<name>.opus`, and copies `clips.json`. It replaces the folder, so a removed clip doesn't linger.
- **491 clips:** 426 parameter clips (142 parameters × 3), 16 exciters, 9 objects, 16 exciter presets, 9 object presets, 10 kits and 5 demos. Render 13.2 s, whole script 44 s on 4 cores.
- **Size: 16.1 MB** (15 MB of it parameter clips), over the design's estimate of about 11 MB (the proposal said 10–15 MB). The velocity-pair, long-note and tempo clips are 4–5 s, and Opus at 64 kbps came out a little above its nominal rate. 48 kbps would bring it to about 12 MB; left at 64 kbps unless the maintainer prefers smaller.
- **Levels:** every non-quiet clip is above −60 dBFS RMS; the loudest peak is −1.0 dBFS (the ceiling).
- **Quiet, declared with reasons:** `output` min (mutes), `coil_proximity` min (the hum doesn't reach the object), `air_pressure` min (Pneumatic Jet with no air; −61.3 dBFS, caught by the audibility test on the new base), `internal_rattle` min (Metal Chain with no rattle, −62 dBFS; **asked of the maintainer whether it's intended**).
- `guide_clip_manifest_is_complete_and_audible` is no longer ignored and passes; CI test lane 521 passed.

### Task 4.4: `<Clips>` (2026-11-13)

- `<Clips param="id" />` (Min, Mid, Max), `<Clips set="…" item="…" />` (one item) and `<Clips set="…" />` (every item, labelled by item name). It reads `public/clips/clips.json` at build. Every row on a page shares one `Audio` with `preload = 'none'`; a button toggles its clip, and `aria-pressed` and a play/stop icon show the state.
- Labels are translated (Min/Mid/Max, Mín/Medio/Máx) or come from the manifest's source, so they follow the plan: "On Tank" / "Sobre Tank", "With Felt Mallet" / "Con Felt Mallet", "Preset: <name>".
- Wired into every reference card (142 parameters have clips; excluded ones show none), each exciter and object page's Listen section (its comparison clip and its preset), the Exciters and Objects overview pages (the whole comparison set) and the Kit page (the ten kits).
- Checked with Playwright: on the Spanish Resonator reference, 84 buttons and no clip requests before a click; clicking Rust's Max requests `/wiki/clips/params/rust/max.opus` (200) and presses the button; Enter on a focused button plays it; the Spanish Bow page shows "Sobre Tank" and "Preset: Bowed Cable Sub". No console errors.

### Task 4.5: home demos (2026-11-13)

- A "Hear it" section on the home page (`web/site/index.html`) with five buttons, Bass, Lead, Chords, Drones and Percussion, playing `/wiki/clips/demos/<folder>.opus` through one shared `Audio` (`preload = 'none'`), with the same toggle and `aria-pressed` as `<Clips>`. The labels are the folder names, which are stable; which preset plays is up to the clip plan (`HOME_DEMOS`).
- **Demos chosen (maintainer, 2026-11-13):** Bass · Low Sheet Bow from C2, Lead · Driven Pipe Lead from C4, Chords · Glass Cable Choir from C5, Drones · Spring Bow Drone from C3, Percussion · Cog Gong Hit from C3 (Cog Tick since the follow-up); each the same four-note phrase (root, fifth, octave, fifth), as `HOME_DEMOS` in `clip_plan.rs`, checked by `home_demos_cover_each_curated_folder_once`. Only the five demo clips changed; the other 486 render the same and were kept.
- **Follow-up (maintainer, 2026-11-13):** the Chords demo plays major triads, the Drones demo one held note (3.5 s), as `DemoPhrase`; every clip runs at least `RELEASE_TAIL` = 2 s past its last note-off (`every_clip_leaves_time_for_the_release`; it was 1 s for most parameter clips, 0.5 s for the long notes); all clips re-encoded at **48 kbps** (`opusenc --bitrate 48`). 491 clips, 14.9 MB (from 16.1 MB), render 20.7 s.
- **Tails cut at silence (maintainer, 2026-11-13):** the fixed 2 s tail left Cog Gong Hit's ring cut off. Every clip now renders `RELEASE_TAIL` = 8 s past its last note-off and `trim_to_silence` cuts it after its last sample within 60 dB of its peak, with a 50 ms fade (`trimming_cuts_where_the_tail_falls_silent`, red with the trim disabled). Percussion demo now **Cog Tick** (maintainer). Clip lengths 0.1–11.5 s, median 3.0 s; **59 clips reach the 8 s cap** without falling 60 dB (the exciters on Tank, the long-ringing object parameters, most representative presets and kits, the Bass, Lead, Chords and Drones demos). Size **19.5 MB**; render 203.5 s. From the decoded clips, cutting at −40 dB instead would save about 10 % and leave about 50 clips at the cap.
- `site.cjs` passes against the local site (no third-party scripts, 375 px without horizontal scroll). Playwright: clicking Drones fetches the clip (200) and presses the button, at 1280 and 375 px.

### Task 4.6: the clip tests' mutations (2026-11-13)

- Dropping `rust` from `parameter_plans()` fails the coverage test ("parameters without a clip plan or exclusion: [\"rust\"]") and the manifest test (the plan's fingerprint changed).
- Deleting `params/heat/max.opus` fails the manifest test: "param:heat:max has no file (params/heat/max.opus)".
- Changing `NOTE_OFF` from 2.0 to 1.5 without regenerating fails it: "the clip plan changed since the clips were rendered: run `scripts/build-guide-clips.sh`".
- The silent-clip path was exercised for real: with base patch D, Air Pressure's minimum fell to −61.3 dBFS and the test named it until it was declared quiet (task 4.3).

Each mutation was restored, and the guide tests pass.

### Tasks 5.1 and 5.2: build and CI (2026-11-13)

- `web/scripts/build.sh` runs `npm ci && npm run check && npm run build` in `web/guide/` after the app, so the guide lands in `dist/wiki/`. The `add-site-shell` placeholder (`web/site/wiki/index.html`) is gone: the guide replaces it.
- **Fixed on the way:** the script ended with `ls -lR dist | head -40`, which under `set -o pipefail` exits 141 (SIGPIPE) once `dist` lists more than 40 lines, as it does with the guide. It now prints `du -sh dist/* | sort -h`.
- `cloudflare-build.sh` installs the official Node 22.23.4 build into `web/.tools` when the image's Node is older than Astro 7.4.4 requires (comparator checked against 22.23.4, 22.23.3, 22.12.0, 23.1.0, 20.19.0 and 24.0.0; the download and extraction checked by hand). CI's `web` job runs the same script, so it covers this path whenever the runner's Node is older.
- **Local full build:** `bash web/scripts/build.sh` passes (10.5 s with Rust cached; the guide's part is about 6 s). `dist/` has 833 files; `dist/wiki/` is 26 MB, `dist/play/` 5.5 MB; no file over 5 MB (Workers limit 25 MiB per file, 20,000 files).
- `site.cjs` gains guide checks: `/wiki/` and `/wiki/es/` return 200 with the guide's titles, are dark and have search; an unknown `/wiki/` path returns 404 with the guide's page. `site.cjs`, `smoke.cjs` and `features.cjs` pass against the full build.
- **`ci.yml` `guide` job:** `actions/setup-node@v4` with Node 22.23.4 and the npm cache keyed on `web/guide/package-lock.json`, then `npm ci`, `npm run check`, `npm run build` (the build fails on a broken internal link). No Rust; the drift and clip tests run in `checks`.

## Measurements

| What | Value |
|---|---|
| Guide pages | 74 per language (148) plus the 404; Astro builds 150 routes |
| Guide build (`npm run build`, 150 pages, Pagefind, link validation) | 4.1 s (3.4–4.3 s across runs); 4.9 s wall |
| `npm run check` | under 1 s; 149 pages, 149 parameters |
| Guide data | `parameters.json` 88 KB, `models.json` 31 KB (with the `kit` block), `parameters.es.json` 47 KB |
| Clips | 491 (426 parameter clips over 142 parameters, 16 exciters on Tank, 9 objects with Felt Mallet, 16 + 9 representative presets, 10 kits, 5 demos) |
| Clip size | 19.5 MB of Opus at 48 kbps, tails cut at silence (was 16.1 MB at 64 kbps with fixed 3 s clips; design estimate about 11 MB) |
| Clip render | 13.2 s for 491 clips on 4 cores (release); `build-guide-clips.sh` 44 s with encoding |
| Full site build (`web/scripts/build.sh`, Rust cached) | 10.5 s; `dist/` 833 files, `dist/wiki/` 26 MB |

### Task 5.5: close-out (2026-11-13)

- fmt clean; CI-lane clippy `-D warnings` clean; CI test lane 521 passed; GUI lane 516 passed (30 ignored); GUI-lane clippy the same 27 warnings as before.
- `cargo doc --no-deps --no-default-features` had one new warning, an unresolved intra-doc link to `guide_files` in `src/guide/mod.rs` (since task 2.2); fixed, 0 warnings.
- Guide check and build pass; `site.cjs`, `smoke.cjs` and `features.cjs` pass against the full local build.
- **Not tested:** the Cloudflare deploy itself (the Node install path was run by hand, not in Workers Builds); the guide's look on Safari and Firefox (Chromium only).
- **Open for the maintainer:** the review of `parameters.es.json` in `write-user-guide-content`.
- **Moved to `docs/BACKLOG.md` (maintainer, 2026-11-13):** Metal Chain almost silent at Internal Rattle 0 (−62 dBFS); the clip stays declared quiet.

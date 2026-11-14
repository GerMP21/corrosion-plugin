# Design

## Context

- After `add-user-guide`, every page exists in English and Spanish with its headings, its `covers:` front matter and a `<Stub />` body.
- The parameter reference, `<Clips>`, `parameters.json`, `parameters.es.json` (first-pass translations of the ⓘ texts), `models.json` and the clip generator are in place.
- `check.mjs` already supports `--complete`.

## Goals / Non-Goals

**Goals:**
- A reader who has never used a physical-modelling synth can go from install to their own patch with the guide alone.
- An experienced reader can find every control's exact behaviour in two clicks: from search or the sidebar, to the reference card.
- Prose stays correct as the plugin evolves, because numbers come from data and pages have a fixed shape.

**Non-Goals:**
- Changing the plugin, tooltips or presets. Problems found while writing are reported to the maintainer as their own changes.
- Video. Possible later; screenshots and clips cover v1.

## Decisions

### 1. Voice and style
- Second person, short sentences, and the plain style the repository's docs already use.
- Each page opens with one sentence on what the thing does to the sound, then a "Try it" step that a reader can do in the web app.
- No jargon without a glossary link the first time it appears on a page.
- English is written first, page by page. The Spanish twin is then written as a translation of meaning, not of words, using the plugin's English control names in `code` style.

### 2. Fixed shapes, enforced by the check
- **Model pages:** the seven sections of the spec. `check.mjs` gains a rule: an exciter or object page must have those headings in order (translated headings are mapped by a small table), and its Controls section must link every control the model shows. The list comes from `parameters.json` slots for that model plus the common controls.
- **Panel pages** (exciter controls, resonator, output chain): one `###` per control, in editor order. Each has a "what you hear" paragraph, an interaction note where one exists, and the control's `<Clips>`.

### 3. Numbers from data
A `<Data>` component renders values from the generated JSON:
- `<Data count="factoryPresets" />`;
- `<Data param="cutoff" field="range" />`.

Prose uses it wherever it would otherwise type a count, range or default. The check warns about bare numbers next to known parameter names. It is a heuristic, so it only warns and does not fail.

### 4. Screenshots from the web app
`web/scripts/screenshots.cjs` (Playwright, deviceScaleFactor 2) opens the local web app at a fixed width and captures:
- the whole editor;
- each panel (by known fractions of the canvas, like `features.cjs`);
- the kit bar, the preset browser open, and the Randomizer menu.

Annotation callouts (numbered markers) are drawn by an MDX component over the image. The image itself has no text, so one image serves both languages. Screenshots are regenerated when the editor changes, and the script is rerunnable.

### 5. Recipes
8–12 recipes, each a page section:
- the starting factory preset;
- the steps (setting, value as displayed);
- the before and after clips.

The clip plan gains a `Recipe { preset, changes }` entry, so the after clip is rendered from exactly the listed changes, and the manifest test covers recipes too. Proposed recipes, for the maintainer to trim:
- metallic bass;
- industrial drone;
- a kit from scratch;
- a bowed lead;
- an impact sound effect;
- a rusted bell;
- a glitchy percussion loop;
- a pad with a Spring tail;
- turning a preset into a kit pad;
- taming a loud friction release.

### 6. Review workflow for Spanish
- Pages go in batches by section, in sidebar order. Each batch is one commit with EN and ES, and the maintainer reviews the Spanish in the PR or in chat.
- `design.md` keeps a table: page → EN written, ES written, ES approved (date).
- The final PR to main is opened only when every row is approved (spec: reviewed before publishing).
- Corrections to `parameters.es.json` from review land in the same batches.

### 7. Accuracy pass
Before each batch is offered for review, every behavioural claim is checked against the code, a test or a render. Claims that needed a render are listed in the batch's notes. Any mismatch between a tooltip and the behaviour goes to the maintainer as a question, with the numbers.

### As built: task 1.1 (2026-11-14)
- **Sections:** `check.mjs` rule 6 asks every exciter and object page (one model in `covers:`, not the folder's index) for the seven `##` sections in order. EN: What it is, How it sounds, Controls, Pairings, Tips, Presets to try, Listen; ES: Qué es, Cómo suena, Controles, Combinaciones, Consejos, Presets para probar, Escúchalo. The stubs said "Factory presets" / "Presets de fábrica", which the spec doesn't; renamed. Run against the stubs before the rename, the rule failed all 50 model pages.
- **Which controls a model shows** (`src/lib/model-controls.mjs`, shared by the check and the component): its slots, its panel's shared controls except the model selector, and for an exciter its family's envelope (AD for hit, MSEG for friction, ADSR for specialty). Bow: 23 controls.
- **Links:** checked only once a page is written (no `<Stub />`), so placeholder pages still pass until `--complete`. A link is `reference/<panel>/#<id>` in the Controls section. As an alternative, `<ModelControls exciter="Bow" />` (or `object=`) renders the whole list from the data, grouped as the editor groups it; the check accepts it, since it can't miss a control.
- **Listen:** must have `<Clips set="exciters|objects" item="<model>" />` and the `-presets` set.
- **Tests:** `scripts/check.test.mjs` (`npm test`, new CI step "Guide check tests"), 9 cases on copies of the real pages: shape, Spanish headings, the spec's Bow Pressure scenario, a complete page, `<ModelControls>`, Hand Strike's AD envelope, an object page, a missing clip. Six were red before the rule (the other three expect no error).

### As built: task 1.2 (2026-11-14)
- **`<Data>`** (`src/components/Data.astro`, values in `src/lib/data-values.mjs`): `count=` factoryPresets, drumKits, exciters, objects, parameters; `param=<id> field=` name, unit, min, max, range (default field), default, options. Units come from the data (only Output, dB, and Wander Rate, Hz, carry one today; Filter Cutoff's range renders as 20–20000 without Hz). An unknown count, parameter or field, or a range asked of a selector, throws, so the build fails instead of printing nothing.
- **factoryPresets** counts every preset in the bank's folders: 214, the drum kits included (they're the `drums` folder). A first version added the kits again (224); the test now compares against the files in `presets/factory/`.
- **Bare-number warning** (check rule 7): a number in the same sentence as a parameter's name (four letters or more), outside code, links and `<Data>`, prints `warning:` and doesn't fail. None on the current pages.
- **Tests:** `scripts/data.test.mjs` (4 cases, red before the module existed) and 2 more in `check.test.mjs` (warns without failing; `<Data>` doesn't warn), all under `npm test`. A build with a temporary `<Data>` line rendered "214 presets", "20–20000" and "Off, Factory, Spring, Echo".

## Risks / Trade-offs

- [Volume: about 70 pages × 2 languages] → Batches by section, so review keeps pace with writing. The strict check is turned on only in the last task, so earlier batches can merge to `dev` with placeholders left.
- [Prose drifts after future plugin changes] → Numbers come from data, model pages are shape-checked, and CLAUDE.md adds: a change that alters a user-visible behaviour updates its guide page (EN and ES) in the same change.
- [Screenshots go stale when the editor changes] → The script is rerunnable. CLAUDE.md lists "regenerate guide screenshots" with the editor-change close-out.

## Open Questions

- Which recipes stay (Decision 5's list is a proposal). This doesn't change the structure or the checks.

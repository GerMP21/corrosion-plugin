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
- *As built (maintainer, 2026-11-14):* the voice is now the `guide-writing-style` spec: warm and conversational, explaining why things sound as they do, with practical tips and step-by-step walkthroughs, in the spirit of the best synth manuals (u-he's, by Howard Scarr). It replaces the plain style first planned here. The pages written before it (batches 2.1–2.4) are restyled in task 2.11.
- Second person and short sentences.
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

### As built: task 1.3 (2026-11-14)
- **`web/scripts/screenshots.cjs`** captures 10 WebP images (320 KB in all) into `web/guide/src/assets/screenshots/` from the local web app: `editor`, `header`, `macros`, `exciter`, `resonator`, `processing`, `keyboard`, `preset-browser` (the bank's folder menu open), `randomizer` (its mode menu open) and `kit` (the Foundry Kit loaded through `open_preset_file`: the kit bar and the zone). Regions are CSS pixels of a 1280 px editor, measured on the default patch. Needs ImageMagick's `convert`. A run takes about 3 minutes (software WebGL at 2560 px).
- **Not deviceScaleFactor 2**, as the design said: at a device pixel ratio of 1.5 or 2 the web editor's canvas comes out 939 or 705 CSS px tall instead of 1408, with a 1280-pixel backing store. The bottom of the editor is cut off and it draws blurry. The script uses a 3200 px window at ratio 1 instead (a 2560 px editor; the layout scales with the width). **Reported to the maintainer**: it affects HiDPI screens (`add-web-app`), not this change.
- **Also found:** at startup the preset bank shows "Bass / Bowed Cable Sub" (the first entry of its list, `selected_factory_preset: 0`) while the default patch, Hand Strike on Pipe, is what plays. It's in the plugin's editor too. Reported to the maintainer; the screenshots show it as it is.
- **`<Screenshot src={…} alt="…" callouts={[[x, y], …]} />`** (`src/components/Screenshot.astro`): the image through `astro:assets` (1280 and 2560 px WebP), numbered callouts at fractions of the image, explained by an ordered list in the page. A callout outside the image fails the build. Checked in a build and in the browser with a temporary use on the editor page.

### As built: task 1.4 (2026-11-14)
- **`Recipe { slug, folder, preset, root, phrase, changes }`** and `RECIPES` in `src/guide/clip_plan.rs`. `recipe_clips` renders `set:recipes:<slug>:before` (the preset as it ships) and `:after` (`with_values(preset, changes)`, exactly the listed changes), with the home demos' phrases, in one set so both share a gain. `<Clips set="recipes" item="<slug>" />` plays them, labelled Before / After (Antes / Después).
- **`check_recipe`** refuses an unknown preset, an unknown parameter, a value outside its range and a recipe that changes nothing; `every_recipe_in_the_plan_is_valid` runs it on `RECIPES`. The manifest test covers recipe clips like every other planned clip (file, audibility, fingerprint).
- **`RECIPES` is empty** until the maintainer picks the list (Open Questions; task 2.8), so the fingerprint and the clips don't change yet. The tests use a sample recipe (Low Sheet Bow with Rust 2 and the cutoff at 2 kHz). The three tests failed to compile before the code existed.

## Review

Every page, EN and ES (Decision 6). Approved means the maintainer approved the Spanish.

| Page | EN written | ES written | ES approved |
|---|---|---|---|
| `index` (guide home) | 2026-11-14 | 2026-11-14 | |
| `getting-started/what-is-corrosion` | 2026-11-14 | 2026-11-14 | |
| `getting-started/installation` | 2026-11-14 | 2026-11-14 | |
| `getting-started/first-note` | 2026-11-14 | 2026-11-14 | |
| `getting-started/web-app` | 2026-11-14 | 2026-11-14 | |
| `getting-started/tour` | 2026-11-14 | 2026-11-14 | |
| `how-it-sounds` | 2026-11-14 | 2026-11-14 | |
| `editor` | 2026-11-14 | 2026-11-14 | |
| `exciters/index` | 2026-11-14 | 2026-11-14 | |
| `exciters/common-controls` | 2026-11-14 | 2026-11-14 | |
| `exciters/envelopes` | 2026-11-14 | 2026-11-14 | |
| `exciters/gesture-modifiers` | 2026-11-14 | 2026-11-14 | |
| `exciters/hand-strike` | 2026-11-14 | 2026-11-14 | |
| `exciters/felt-mallet` | 2026-11-14 | 2026-11-14 | |
| `exciters/hard-mallet` | 2026-11-14 | 2026-11-14 | |
| `exciters/drumstick` | 2026-11-14 | 2026-11-14 | |
| `exciters/wire-brush` | 2026-11-14 | 2026-11-14 | |
| `exciters/metal-pipe` | 2026-11-14 | 2026-11-14 | |
| `exciters/metal-chain` | 2026-11-14 | 2026-11-14 | |
| `exciters/bow` | 2026-11-14 | 2026-11-14 | |
| `exciters/stiff-point-scrape` | 2026-11-14 | 2026-11-14 | |
| `exciters/heavy-grinding` | 2026-11-14 | 2026-11-14 | |
| `exciters/corrugated-drag` | 2026-11-14 | 2026-11-14 | |
| `exciters/tension-rise` | 2026-11-14 | 2026-11-14 | |
| `exciters/pneumatic-jet` | 2026-11-14 | 2026-11-14 | |
| `exciters/electromagnetic-hum` | 2026-11-14 | 2026-11-14 | |
| `exciters/tension-snap` | 2026-11-14 | 2026-11-14 | |
| `exciters/particle-rain` | 2026-11-14 | 2026-11-14 | |
| `objects/index` | 2026-11-14 | 2026-11-14 | |
| `objects/geometry` | 2026-11-14 | 2026-11-14 | |
| `objects/environmental-exposure` | 2026-11-14 | 2026-11-14 | |
| `objects/mechanical-linkage` | 2026-11-14 | 2026-11-14 | |
| `output-chain/index` | 2026-11-14 | 2026-11-14 | |
| `output-chain/filter-bank` | 2026-11-14 | 2026-11-14 | |
| `output-chain/overdrive` | 2026-11-14 | 2026-11-14 | |
| `output-chain/dispersion` | 2026-11-14 | 2026-11-14 | |
| `output-chain/space` | 2026-11-14 | 2026-11-14 | |
| `output-chain/diode-limiter` | 2026-11-14 | 2026-11-14 | |
| `output-chain/output-width` | 2026-11-14 | 2026-11-14 | |
| `macros` | 2026-11-14 | 2026-11-14 | |
| `randomizer` | 2026-11-14 | 2026-11-14 | |
| `note-modes/index` | 2026-11-14 | 2026-11-14 | |
| `note-modes/tonal` | 2026-11-14 | 2026-11-14 | |
| `note-modes/drone` | 2026-11-14 | 2026-11-14 | |
| `note-modes/kit` | 2026-11-14 | 2026-11-14 | |
| `presets/index` | 2026-11-14 | 2026-11-14 | |
| `presets/factory-bank` | 2026-11-14 | 2026-11-14 | |
| `presets/browse-and-save` | 2026-11-14 | 2026-11-14 | |
| `presets/files` | 2026-11-14 | 2026-11-14 | |
| `presets/daw-projects` | 2026-11-14 | 2026-11-14 | |
| `midi/index` | 2026-11-14 | 2026-11-14 | |
| `midi/computer-keyboard` | 2026-11-14 | 2026-11-14 | |
| `midi/expression` | 2026-11-14 | 2026-11-14 | |
| `quality` | 2026-11-14 | 2026-11-14 | |
| `ui-scale` | 2026-11-14 | 2026-11-14 | |
| `recipes` | 2026-11-14 | 2026-11-14 | |
| `troubleshooting` | 2026-11-14 | 2026-11-14 | |
| `glossary` | 2026-11-14 | 2026-11-14 | |
| `reference/index` | 2026-11-14 | 2026-11-14 | |
| `objects/pipe` | 2026-11-14 | 2026-11-14 | |
| `objects/plate` | 2026-11-14 | 2026-11-14 | |
| `objects/tank` | 2026-11-14 | 2026-11-14 | |
| `objects/chain` | 2026-11-14 | 2026-11-14 | |
| `objects/i-beam` | 2026-11-14 | 2026-11-14 | |
| `objects/taut-cable` | 2026-11-14 | 2026-11-14 | |
| `objects/coil-spring` | 2026-11-14 | 2026-11-14 | |
| `objects/sheet-metal` | 2026-11-14 | 2026-11-14 | |
| `objects/industrial-cog` | 2026-11-14 | 2026-11-14 | |

### Batch 2.1 notes: Getting started (2026-11-14)
- **Sources checked:** install folders, zip names and the macOS quarantine step against `README.md` and `ci.yml`'s package step; the default sound, labels (MODEL TYPE, MATERIAL, NOTE MODE, the header's MODE) and help texts against `parameters.json`; the piano strip and computer keys against `src/gui/editor.rs` (C3–C6, `A`…`K` = C4–C5); the web app's bar against `web/www/index.html`; counts through `<Data>` (16 exciters, 9 objects, 214 presets, 10 kits).
- **Rendered:** pitch against size, Hand Strike at C4, Render quality: Pipe 533 / 268 / 134 Hz and Taut Cable 528 / 264 / 132 Hz at size 0.5 / 1 / 2. Raising the size lowers the pitch on every object.
- **Reported to the maintainer, not changed here:**
  - **Cable Tension's ⓘ text said "Higher sounds higher"**, but it is the cable's size and raising it lowers the pitch (above). Fixed 2026-11-14 in its own commit: the control is labelled **Cable Length**, its help says longer cables sound lower, and the tour says lengthening it drops the pitch. The parameter ID (`object_size`'s slot) and saved values are unchanged, so no migration.
  - The web editor's canvas at device pixel ratios above 1 (task 1.3 note): in `docs/BACKLOG.md`.
  - The preset bank's startup label (task 1.3 note): in `docs/BACKLOG.md`.
  - The README listed UI Scale as 50/75/100/125/150 %; the selector has every 5 % from 50 to 150 %. Fixed in the README.
- **Review:** the maintainer delegated the Spanish review and the open choices to Claude (2026-11-14, "Lo dejo a tu autoridad"). The ES approved column stays empty until the maintainer approves; the final PR to main waits for it.
- **Check change:** the bare-number heuristic skips the name "Corrosion" (the product's name is also the macro's).

### Batch 2.2 notes: How it sounds and The editor (2026-11-14)
- **Signal flow** drawn by a new `<Flow>` component (boxes and arrows, labels passed by the page so each language writes its own, a two-way arrow for the exciter–object coupling, an `aria-label` for screen readers). No image to translate.
- **Sources checked:** the chain order against `docs/ARCHITECTURE.md` §1 and §9 and `Corrosion::begin_block` (voices → mix DC blocker → Legacy Drive → Filter Bank → Overdrive Unit → BODY → spread → Space → diode → Output → limiter); a note-off releases only the exciter (`Voice::note_off`); voice stealing (`VoiceManager::steal_index`: quietest tail, then pedal-held, then quietest held note past 100 ms); the macros are neutral at 0.5 and reach exciter mass, object damping, brightness, rust, damage, drive and filter (`resolve_macro_bias`); Speed and Roughness are inert on Hand Strike (ARCHITECTURE §6); the envelope per family (AD, MSEG, ADSR; `controls.rs`); knobs drag vertically, faders jump to the click (`knob_drag_value`, `industrial_fader`); the header scrolls with the panels in the plugin and only the strip is pinned (`render_window_body`); the kit zone is 36–59, C2–B3 (`KIT_ZONE`).
- **Screenshots:** the whole editor and each column carry numbered callouts, placed and checked in the built page (EN and ES). The header has none: at the page's width its labels are too small for callouts not to cover them, so its list names the labels instead.

### Batch 2.3 findings: exciter controls (2026-11-14, resolved by `fix-inert-controls`)
- **Envelope help texts fixed** in their own commit: Global Time speeds up (it said slows), Vel To Peak reads no velocity, Ping-Pong equals Forward 3 → 4, a strike's Release starts at its peak, Env Amount 0 silences; the six Gesture Modifiers act only on the friction MSEG (backlog item for the panel).
- **Measured** with `print_slot_effects` (ignored test in `src/guide/clip_plan/tests.rs`): one note, C3 at velocity 100, each slot at its minimum and maximum on its clip patch (the exciter on Plate, Render quality); RMS, ring length to −40 dB, and the mean change of the first second's octave-band levels beyond the level change ("bands").
- **No measurable effect** (level within 0.3 dB, same ring, bands ≤ 0.25 dB): Slip Curve, Chatter Pitch, Chatter Damping, Chain Mass (0.00 dB), Flesh Stiffness 0.04, Core Hardness 0.03, Grind Pressure 0.06, Link Count 0.06, Restitution Bounciness 0.07, Micro Bounce Limit 0.16, Compression Curve 0.22, Tip Stiffness 0.25. Stiff Point Scrape appears to slip every sample, so its spring (Chatter Pitch, Chatter Damping) never shapes the output (`stiff_point.rs`); the others look normalised away by the drive level and governor. Not yet confirmed per control.
- **Help says the opposite of the measurement:** Mute Decay (help: lower mutes sooner; 0.85 → 0.999 gives −8.1 dB and ring 2.23 → 1.46 s, the hand rests longer and mutes more), Flesh Damping (help: softens the attack; min → max +4.3 dB, brighter), Rattle Color (help: brighter; −3.9 dB and darker overall, it raises a high-pass on the rattle), Stick Mass and Pipe Mass (help: darker; brighter), Voltage Sag (help: wavers; it adds 3rd and 5th harmonics, no wavering).
- **Maintainer's decision (2026-11-14): fix the engine first.** Proposed as `fix-inert-controls`; batch 2.3 waits for it. Meanwhile the How it sounds and editor pages say the Gesture Modifiers act only on the friction exciters.

### Batch 2.3 notes: Exciters (2026-11-14)

The 20 pages are written in EN and ES. Each model page lists its controls with `<ModelControls>`, then gives one line per dedicated control.

**Sources checked.**
- Families and envelopes: `models.json` and `controls.rs`. The struck AD rises over Attack, then falls over Release at once, without waiting for note-off (`OneShotEnvelopePhase`).
- MSEG stages and loop rules: `dsp/envelopes`. Only stages 1–4 loop. Forward jumps to Start on reaching End, so End itself doesn't play. Ping-Pong is Forward 3 → 4.
- Speed and Roughness, which controls each scales and where they are inert: ARCHITECTURE §6 "Global exciter controls". Pressure is smoothed over about 10 ms.
- Drumstick re-strikes: the first flight is 40 ms × Restitution.
- Tension Snap: one snap per note, and velocity scales the snap force.
- Preset lists: from the bank files, by exciter. Kit counts per exciter: from the pads' `exciter` values.

**Measured, re-run after `fix-inert-controls`.** `print_slot_effects`: one C3 note, minimum → maximum of each control, on Plate.
- Every direction stated on the pages matches.
- Three help texts said the opposite, and were fixed in their own commit:
  - Hand Mass: +5.9 dB, centroid 305 → 532 Hz.
  - Pipe Mass: +4.2 dB, 66 → 109 Hz.
  - Chatter Pitch: −4.3 dB, 388 → 196 Hz.
- Where a help text's direction rests on a small centroid change, the page leaves the colour out and keeps to what was measured: Nozzle Width +6.1 dB, Particle Mass +10.5 dB, Exciter Mass, Pipe Pitch and Pipe Ring Decay.

**Release snap, measured through the whole chain** (render, voice-mix DC blocker included):

| Patch | MSEG Release | Half second after letting go, against the held note |
|---|---|---|
| Corrugated Drag on Plate | 0.1 s (default) | +14.6 dB |
| Corrugated Drag on Plate | 0.5 s | 0.0 dB |
| Bow on Taut Cable | 0.1 s | +0.7 dB |
| Stiff Point Scrape on Plate | any | none |

The pages give the Corrugated Drag numbers and advise a Release of half a second or more.

**Surprises documented where readers meet them:**
- the release snap: envelopes, Bow, Stiff Point Scrape and Corrugated Drag;
- Env Amount 0: gesture modifiers and Bow;
- the gesture modifiers acting only on friction: gesture modifiers.

Troubleshooting (2.9) links back to these.

**Glossary links** wait for batch 2.9, since the glossary is still a stub. No glossary anchors are linked yet.

### Batch 2.4 notes: Objects (2026-11-14)

The 13 pages are written in EN and ES. Each object page lists its controls with `<ModelControls>`, then gives one line per slot.

**Sources checked.**
- Tuning: the sizes that put each object on the key, and the offsets at size 1, come from the bank's measured constants and the tuning notes at the top of `curated.rs`. Chain and Industrial Cog have none, so the pages say to tune them by ear. The bare-number warnings on the size values are these constants; they aren't in the data.
- The Pipe's fixed mode (800–1030 Hz, set by its diameter) and BODY at 0 for chords: also from those notes.
- Live versus note-on: `Voice::update_live_controls` takes damping, brightness and the six Mechanical Linkage controls, so these act on held notes. Every other object and exposure control is read at note-on (ARCHITECTURE §5's transform chain).
- The order of the exposure transforms, rust → thickness → heat → sludge → damage: ARCHITECTURE §5.
- Strike position: `c_n = sin((n+1)πP)` (§7). In the middle every second mode is on a node; the two halves mirror each other; Anchor holds `c_0` up.
- Pos Env moves the point by up to half the object: `+ position_envelope · envelope · 0.5`.
- Coupling blends feed-forward with position-weighted force.

**Measured** (`print_slot_effects`, from the 2.3 run):
- Rust halves the ring on a plate (2.31 → 1.17 s).
- Sludge takes it under half (0.99 s, centroid 525 → 387 Hz).
- Plate Aspect at its maximum is −27 dB and much brighter.
- Tank Volume and Cavity Mix both lower the centroid.
- Link Mass sounds lower.
- Rust and Damage keeping the fundamental: from `fix-rust-damage-heat-response`.

### Batch 2.5 notes: Output chain (2026-11-14)
- The first batch written in the `guide-writing-style` voice: why each stage behaves as it does, a tips list per control group, and a five-step walkthrough on the index page from **Plate Strike Lead**.
- **Sources checked:**
  - **Chain order:** `Corrosion::begin_block`.
  - **Filter:** four-pole ladder, 24 dB/octave; cutoff and drive smoothed over about 20 ms; Tolerance a random walk up to 30 %.
  - **Bias Starve:** the attractor-region starvation, through a 5 ms follower.
  - **BODY:** fixed resonances near 190 and 390 Hz, mixed at BODY/5; Chassis Vol only retunes it.
  - **Space:** low partials at 98–144 Hz in every mode; Delay Time `0.02 + t·0.78` s.
  - **Diode:** knee at `limit·(1 − 0.8·softness)`, giving −4.4 dB at the 0.5 default and −14 dB at 1 (`oversampled_clipper.rs`).
  - **Limiter:** the Hard default (`limiter_mode_param(0)`).
  - **Meter:** brightens over 0.1–6 dB of reduction.
  - **Output:** 0 dB at 57.5 % of the travel; −20..+6 dB over 18.9 %.
  - **Labels and help:** every control's label and help from `parameters.json`.
- `space_mode` has no clip set, so the Mode section has no clip; each Space mode's controls have their clips.
- The bare-number warnings left are walkthrough values ("about 1.5", "about 2") and Delay Time's display range in milliseconds, which the data holds only as 0–1.

### Task 2.11 notes: restyle to `guide-writing-style` (2026-11-14)
Every page written before the style spec was revised in both languages (paragraph-level edits, so the earlier accuracy fixes stay):
- **Getting started, How it sounds, The editor:**
  - warmer openings and why physical modelling behaves as it does;
  - a "fix it at the source" rule of thumb;
  - "If Corrosion doesn't show up" (Install) and "If you hear nothing" (First note).
- **Exciters:**
  - each model page opens with what the exciter is like to play and why, and gains a tip;
  - the overview gains "Choosing an exciter";
  - common controls, envelopes and gesture modifiers gain tips.
- **Objects:** the same treatment: a new lead and a tip per model page, "Choosing an object", and tips on the panel pages.
- **Claims checked while restyling:**
  - **Cog beating scales with pitch:** its pairs are `f(1 ± ε)` (`industrial_cog.rs`).
  - **Taut Cable's pitch dips, then climbs back** (not the reverse; caught in the draft).
  - **Bow:** described by its friction curve, not as stick-slip, which the backlog holds.
  - **Coil Spring:** its dispersion stated without a direction.
  - **MSEG loops:** not tempo-synced, so no tip claims they are.

### Batch 2.6 notes: Macros, Randomizer, note modes (2026-11-14)
- **Macros** (`resolve_macro_bias`):
  - **MASS:** exciter mass ×(1 ± 0.5) and size ×(1 ± 0.25). At the rails that is −3.9 / +5.0 semitones; the page warns that MASS moves the pitch.
  - **CORROSION:** damping ±0.2, brightness ∓0.2 and rust ±1.
  - **VIOLENCE:** damage ±2, Legacy Drive ±0.5, Drive Amt and Chaos ±0.2.
  - **BRIGHTNESS:** object brightness ±0.3 and cutoff ×2^(±1), i.e. ±1 octave. The code comment says ±2 octaves, but `2^(b·2)` with b = ±0.5 is ±1. The cutoff clamps at 20 kHz, so above the middle the filter part does nothing on an open filter.
  - **When each part acts:** live for damping, brightness, the drives and the cutoff (`apply_post_chain_params`, the per-block live controls); at note-on for size, rust and damage.
- **Randomizer** (`randomizer/mod.rs`, `randomizer_assignments`):
  - It sets only seven parameters: size, rust, damage, Legacy Drive, width, BODY and OUTPUT. The exciter, object and envelopes never change.
  - Per-mode ranges as in `*_ranges`. OUTPUT is a linear gain: 0.5–1.0 is −6..0 dB, and Object and Damage fix it at 0.7 (−3.1 dB).
  - Object and Damage set BODY to 0 and width to 0.5.
  - Safety redraws and the final clamp: `randomize`.
  - The guide's earlier "Safe draws within safe limits" holds.
- **Drone:** forces the MSEG Forward loop over stages 3–4 (`note_setup`), so only friction exciters change, and the note pulses at the Decay's rate (as the envelopes page says).
- **Kit:**
  - A pad keeps every parameter except the post chain, master, quality, sync and UI ones (`NON_PAD_PARAMS`).
  - Pads don't follow the knobs (`pad_voice`).
  - Tune is a semitone drag value in ±48 (`MAX_TUNE`).
  - The level can't be edited: Assign keeps a pad's level, and a new pad starts at 0 dB.
  - Assign's tune comes from the last note auditioned outside Kit mode.
  - Computer keys play 36–48; the zone is 36–59; the strip spans 36–84.
  - The GM layout of the 12 pads is from `drums.rs`.
- **Wording to watch:** "a friction note can settle and fade while held" in Tonal. The MSEG's Sustain stage holds the force; the fading is the object's response, and how much depends on the preset. The page doesn't claim a mechanism.

### Batch 2.7 notes: presets, MIDI, quality, UI scale (2026-11-14)
- **Presets:**
  - The user folder comes from `user_preset_dir`. The factory bank is built in (`include_dir!` in `editor.rs`).
  - A preset holds everything but `ui_scale` (`PRESET_SKIPPED_PARAMS`), quality mode included. All 214 factory files store Normal, so loading one resets MODE to Normal; the quality page says so.
  - Save, overwrite confirmation, user subfolders and Open folder follow the `user-presets` spec.
  - Migrations on both paths (`migrate_preset_json`, `filter_state`): diode stage, Onset Level, Wander Rate, old Kit mode.
  - The BREAKING automation notes cover Output (dB), Diode Softness and Note Mode → Kit.
- **MIDI:**
  - Pitch bend is ±2 semitones (`PITCH_BEND_RANGE_SEMITONES`).
  - Pressure (the larger of channel and poly) and CC1 scale the voice gain by `1 + 0.5·p + 0.3·m` (about +3.5 dB and +2 dB).
  - CC64, CC120, CC121 and CC123 are as in ARCHITECTURE §4; other CCs are ignored (README).
  - Computer keys play at velocity 100 (`KEY_VELOCITY`), from 60 in Tonal and Drone and from 36 in Kit, matched by letter.
- **Expression:**
  - Notes are addressed by voice ID, else by key and channel.
  - PolyTuning goes on top of the channel bend; Choke is a 2 ms fade; channel MPE is out of scope.
  - No host is named as supporting per-note editing: none was checked in these sessions.
- **Quality:**
  - Mode counts ×0.5 / 1 / 1.5 / 2 (`quality_mode_count_scale`); oversampling 1× / 4× / 8× / 16×.
  - The two switching times and the unreported delay of 4–5 samples (ARCHITECTURE §9).
- **UI scale:** 50–150 % in 5 % steps, with 810 × 630 at the 75 % default.

### Batch 2.8 notes: Recipes (2026-11-14)
- **As built: ten recipes in `RECIPES`, applying the proposed list (standing instruction: take the recommended option).** Two proposals were replaced: "a kit from scratch" and "turning a preset into a kit pad". A pad isn't a parameter change, so the clip plan can't render them; the Kit page's Assign walkthrough covers both. Their places went to:
  - a dying-battery lead;
  - a distant snare.
- **"Taming a loud friction release":**
  - On Chain Drag Drone, the release has no snap at all (the before and after peaks after note-off were identical), so the recipe uses **Experimental → Corrugated Drag · Plate** instead.
  - That preset ships with a 0.419 s Release and still snaps; it needs about 2 s. The page says so, while Envelopes and Troubleshooting keep "half a second" for the base patch, where it was measured.
- **"Metallic bass":** the first draft (Felt Softness 0.3, Core Hardness 4) came out 4 dB quieter and no brighter, so it was replaced by Hard Mallet. The page says what was measured: about 4 dB louder and a much longer tail, and no claim of brightness.
- **Measurements:** `print_recipe_measurements` (ignored), raw renders, before → after.

  | Recipe | RMS dB | Last 0.5 s dB | Peak after note-off vs held, dB | 20 ms level s.d., dB |
  |---|---|---|---|---|
  | metallic-bass | −9.4 → −5.3 | −20.5 → −7.5 | | 4.1 → 1.2 |
  | industrial-drone | −17.2 → −18.2 | | | 0.4 → 0.6 |
  | singing-bowed-lead | −12.9 → −13.7 | | | 1.1 → 1.4 |
  | impact | −14.5 → −6.3 | −45.6 → −24.7 | | |
  | rusted-bell | −15.8 → −16.9 | | | |
  | glitch-percussion | −15.7 → −16.1 | | | 13.4 → 14.5 |
  | spring-pad | −25.9 → −26.2 | | | |
  | dying-battery-lead | −21.4 → −22.9 | | | 2.2 → 3.5 |
  | distant-hit | −12.2 → −11.9 | −174.6 → −86.6 (hall tail) | | |
  | gentle-release | | | +6.5 → +1.1 | |

- **What the numbers don't show:**
  - The brightness proxy (first-difference RMS over RMS) follows the fundamental's level, so no recipe text claims "brighter" from it.
  - Several recipes are changes of character, not of level: the Spring tail, the echo and the wandering bow. The page describes those, not numbers.
  - The industrial drone's and glitch percussion's movement is small by these metrics. Their text stays modest: "keeps the texture from sitting perfectly still", "each tick a little different".
- **Clips:**
  - Regenerated: 511 (491 + 20), 16 MB; clips.json gains the recipes and the new fingerprint.
  - The 491 existing `.opus` files were restored from git. Their audio is unchanged (same plan, same engine), but opusenc writes a random stream serial, so a regeneration rewrites every file.
- **Listening:** the maintainer should listen to the recipe clips. They were checked by measurement only.

### Batch 2.9 notes: Troubleshooting and Glossary (2026-11-14)
- **Troubleshooting** covers the five documented surprises the spec lists:
  - the release snap, under "A loud release": +14.6 dB measured on Corrugated Drag on Plate, with the 2 s case from the recipe;
  - Env Amount 0, under "No sound";
  - the BREAKING automation changes and old Kit presets loading as Tonal, under a new "Old projects sound different";
  - the preview-address browser presets, under "Missing presets".

  Each also stays on the page where readers meet it.
- **Glossary:** 28 terms, each a `###` heading so it has an anchor. The Spanish list is sorted by its Spanish terms.
- **Glossary links**, on the first mention of the terms the earlier pages explain least:
  - modes: What is Corrosion and Objects;
  - partials: How it sounds;
  - aliasing: Quality;
  - knee: Diode and limiter.

  Pages written after the style spec explain their terms in place, so they link less.

### Task 2.10 notes: `parameters.es.json` (2026-11-14)
- **Self-review against `guide-writing-style`; the maintainer's review is still pending.** Corrections:
  - "la velocity" → "la velocidad de pulsación";
  - "el fundamental" → "la fundamental";
  - the diode's "Codo" → "Rodilla", the term the pages use;
  - "arenilla" → "aspereza" for the diode;
  - "clipper" → "recortador";
  - the Bow model description and Rosin Grip no longer say "stick-slip".
- **Found in the English texts** (plugin tooltips; fixed 2026-11-14 at the maintainer's request):
  - The Bow model description says "Smooth stick-slip bow friction … from the documented bow model". "Documented bow model" means nothing to a player.
  - Rosin Grip's help says "a stronger, rougher stick-slip". The bow isn't real stick-slip yet (`docs/BACKLOG.md`).
  - Corrugated Drag's description, "explicit spacing, depth, and moving contact mass", reads like code.
  - **Fixed:** Bow is now "A rosined bow that grips and drags the object, for sustained, singing tones. Shaped by pressure, speed, grip and slip."; Corrugated Drag "A tool dragged across ridged metal: ticks that blur into a buzz as it speeds up. Shaped by ridge spacing, depth and the tool's weight."; Rosin Grip "… for a stronger, rougher bowed tone." Guide data re-exported; the three Spanish entries follow them. Text only, no sound change.

## Risks / Trade-offs

- [Volume: about 70 pages × 2 languages] → Batches by section, so review keeps pace with writing. The strict check is turned on only in the last task, so earlier batches can merge to `dev` with placeholders left.
- [Prose drifts after future plugin changes] → Numbers come from data, model pages are shape-checked, and CLAUDE.md adds: a change that alters a user-visible behaviour updates its guide page (EN and ES) in the same change.
- [Screenshots go stale when the editor changes] → The script is rerunnable. CLAUDE.md lists "regenerate guide screenshots" with the editor-change close-out.

## Open Questions

- Which recipes stay (Decision 5's list is a proposal). This doesn't change the structure or the checks.

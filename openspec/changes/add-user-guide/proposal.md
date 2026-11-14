# Proposal

## Why

Corrosion has 149 parameters, 16 exciters, 9 objects, three note modes and a kit editor. Today the only documentation is the README and the one- or two-sentence ⓘ tooltips. The maintainer wants an exhaustive but friendly user guide at `corrosion-vst.org/wiki/`, in English by default with a Spanish version, in the plugin's colours, and with audio examples of what each control does. It has to stay true to the plugin as the plugin changes, so the parts that can come from the code must come from the code, and tests must say when the guide falls behind.

This change builds the guide's machinery and skeleton. The prose is written in `write-user-guide-content`.

## What Changes

- **Guide site** built with Astro Starlight in `web/guide/`, served at `/wiki/` (English) and `/wiki/es/` (Spanish), with a language switcher on every page and full-text search (Pagefind, per language). It is dark-only, styled from the shared tokens of `add-site-shell`, and built into `dist/wiki/` by the same build and deploy as the rest of the site.
- **Page skeleton:** every page of the agreed sitemap exists in both languages, with its title, its headings and a visible "being written" placeholder. The sitemap covers:
  - Getting started; How it sounds; The editor;
  - Exciters (16 pages) and Objects (9 pages);
  - Output chain; Macros; Randomizer; Note modes;
  - Presets; MIDI and expression; Quality and CPU; UI scale;
  - Recipes; Troubleshooting; Glossary; Parameter reference.
- **Parameter reference generated from the plugin.** A Rust exporter writes `parameters.json` from the live parameter set and the editor's control tables:
  - every parameter's ID, display name, panel, range, default, unit and steps;
  - its per-model labels for model-dependent slots;
  - its ⓘ help text.
  
  It also writes `models.json` (exciters, objects, Space modes, quality modes, note modes, randomizer modes, factory folders). The reference pages render from these files. Spanish explanations live in a hand-written `parameters.es.json` keyed by ID.
- **Audio clips generated from the plugin.** A Rust binary renders, through the plugin's own output path:
  - per-parameter clips at minimum, middle and maximum, on C3 at velocity 100 over a fixed base patch, 2–4 s each;
  - every exciter on one object, and every object with one exciter;
  - a representative factory preset per exciter and object;
  - the factory drum kits;
  - a few demo clips for the home page.
  
  Clips are normalized per set to `TARGET_RMS`, encoded as OGG Opus, and listed in a manifest with their measured levels.
- **Coverage tests** that fail when the guide falls behind:
  - the generated data differs from the plugin;
  - a parameter has no clip plan or exclusion reason;
  - a planned clip is missing or silent;
  - a model has no page;
  - an English page has no Spanish twin, or their headings differ;
  - an internal link is broken;
  - a parameter has no Spanish text.
- **Help text moves out of the GUI-only module.** `help_for` and the editor's control tables move to a feature-free module, so the exporter and its tests run in the CI lane. The editor behaves the same.
- **CI** gets a `guide` job (Node: check and build). Cloudflare's build script also builds the guide.

## Capabilities

### New Capabilities
- `user-guide`: the guide site. Covers its paths and languages, theme, search, the skeleton covering every feature, the reference generated from the plugin, the Spanish twins, and the coverage checks that keep it in step with the plugin.
- `guide-audio-clips`: the audio examples. Covers what is rendered and how (render path, note, velocity, base patch, length), normalization, format, the manifest, the clip plan covering every parameter, and the completeness and audibility checks.

### Modified Capabilities
None. The ⓘ tooltips (`editor-controls`) keep their texts and behaviour; only the module holding them moves.

## Impact

- New `web/guide/` (Astro Starlight project, `package.json` and lock with exact versions, content, components, data, clips).
- New Rust binaries `export_guide_data` and `render_guide_clips` in `src/bin/`, and a feature-free module for the help texts and control tables (moved out of `src/gui/editor.rs`).
- `web/scripts/build.sh` and `cloudflare-build.sh` build the guide (Node is in Cloudflare's build image).
- `.github/workflows/ci.yml` gets a `guide` job; the Rust drift and coverage tests run in the existing CI lane.
- Repository size grows by the committed clips (estimated 10–15 MB of Opus, see design).
- A dev tool for regenerating clips: `opusenc` (opus-tools). It is not needed by CI or the deploy.
- CLAUDE.md: regenerate clips and guide data after a sound or parameter change, as with the preset bank.

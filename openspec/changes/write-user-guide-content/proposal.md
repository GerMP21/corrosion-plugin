# Proposal

## Why

`add-user-guide` builds the guide's machinery: pages in both languages, the generated parameter reference and the audio clips. Every narrative page is still a placeholder. This change writes the guide itself: exhaustive, detailed and friendly, in English, with a Spanish version the maintainer reviews.

## What Changes

- **Every placeholder page written, in English.** That covers:
  - Getting started (install per OS, first note, the web app, a 10-minute tour);
  - How it sounds (signal flow);
  - The editor panel by panel;
  - one page per exciter (16) and per object (9);
  - exciter controls (common, force envelope and MSEG, gesture modifiers);
  - resonator controls (geometry, Environmental Exposure, Mechanical Linkage);
  - the output chain (filter, overdrive, dispersion and BODY, Space and Echo, diode and limiter, output and width);
  - Macros, the Randomizer, the note modes (Tonal, Drone, Kit and the kit editor);
  - presets (factory bank, user presets, files, old presets and projects);
  - MIDI and expression, quality modes and CPU, UI scale;
  - recipes, troubleshooting and the glossary.
- **A fixed shape for model pages.** Each exciter and object page has the same sections:
  - what it is, physically;
  - how it sounds;
  - its controls (linking the reference cards);
  - how it pairs with the other side;
  - tips;
  - the factory presets that show it;
  - its clips.
- **The Spanish version**, written by me and reviewed by the maintainer, page by page. It keeps the plugin's English control names.
- **Annotated screenshots** of the editor and each panel, taken from the web app with Playwright (true colours), in one shared set used by both languages.
- **Recipes** (8–12) built from real factory presets with exact settings, each with before and after clips.
- **Accuracy pass.** Every number and behaviour the prose states is checked against the code, the generated data or a render. Known behaviours are documented:
  - releasing Bow, Stiff Point Scrape or Corrugated Drag snaps the object free;
  - Env Amount 0 silences friction exciters;
  - the BREAKING changes for Output and Softness automation;
  - old Kit presets load as Tonal;
  - presets saved at the preview address don't move to the new site.
- **The checks become strict.** CI runs the guide check with `--complete`, so a placeholder fails.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `user-guide`: adds requirements on the content. No placeholders remain. Model pages follow a fixed section shape. Statements match the plugin. Spanish pages are reviewed by the maintainer before they are published. Recipes are reproducible from factory presets. (`user-guide` is created by `add-user-guide`, which must be archived first; these are written as ADDED requirements so they append to it.)

## Impact

- Content only: `web/guide/src/content/docs/**` (EN and ES), screenshots under `web/guide/src/assets/`, recipe clips through the existing generator (new plan entries), and `parameters.es.json` corrections from review.
- `ci.yml`: the guide check runs with `--complete`.
- `web/scripts/` gets a screenshot script (Playwright against the local web app).
- No plugin, engine or sound change. If writing the guide uncovers a bug or a misleading tooltip, it is reported to the maintainer and handled in its own change, not fixed silently here.

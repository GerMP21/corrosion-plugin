# Tasks

## 1. Tooling for content

- [ ] 1.1 Test first: `check.mjs` rules for model-page shape (seven headings, EN/ES mapping) and control links; run red against the stub pages
- [ ] 1.2 `<Data>` component (counts, ranges, defaults from the generated JSON) and the bare-number warning
- [ ] 1.3 `web/scripts/screenshots.cjs` and the callout component; capture the editor and every panel
- [ ] 1.4 `Recipe` entries in the clip plan; the manifest test covers them

## 2. Writing, in batches (EN, then ES, then the accuracy pass, then maintainer review)

- [ ] 2.1 Getting started: what Corrosion is, install (Windows, macOS, Linux; VST3, CLAP), first note, the web app, 10-minute tour
- [ ] 2.2 How it sounds (signal-flow diagram) and The editor, panel by panel (annotated screenshots)
- [ ] 2.3 Exciters: overview and comparison, common controls, force envelope and MSEG, gesture modifiers, and the 16 exciter pages
- [ ] 2.4 Objects: overview and comparison, geometry and tuning, Environmental Exposure, Mechanical Linkage, and the 9 object pages
- [ ] 2.5 Output chain: Filter Bank, Overdrive Unit, Dispersion and BODY, Space and Echo, diode and limiter, Output and Width
- [ ] 2.6 Macros, Randomizer, note modes (Tonal, Drone, Kit and the kit editor, factory kits)
- [ ] 2.7 Presets (factory bank, user presets, files, old presets and projects, BREAKING automation notes), MIDI and expression, quality and CPU, UI scale
- [ ] 2.8 Recipes (the list agreed with the maintainer) with before and after clips
- [ ] 2.9 Troubleshooting (including the documented surprises) and Glossary
- [ ] 2.10 `parameters.es.json` review corrections

## 3. Close-out

- [ ] 3.1 Review table in design.md: every page EN written, ES written, ES approved
- [ ] 3.2 Turn on `--complete` in CI; the guide check and build pass
- [ ] 3.3 CLAUDE.md rules: user-visible behaviour changes update their guide pages (EN and ES); editor changes regenerate screenshots
- [ ] 3.4 Close-out lanes; commit and push to `dev`; update CLAUDE.md Current state

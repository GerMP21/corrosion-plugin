# user-guide Specification

## Purpose
An exhaustive, friendly user guide for Corrosion at `corrosion-vst.org/wiki/`, in English and Spanish, that stays in step with the plugin.

## Requirements

### Requirement: Guide paths and languages
The guide SHALL be served in English at `/wiki/` and in Spanish at `/wiki/es/`, with the same pages at mirrored paths (`/wiki/<page>/` and `/wiki/es/<page>/`). Every page SHALL have a language switcher that goes to the same page in the other language. Only the guide is translated; control names stay as the plugin shows them (English) in both languages.

#### Scenario: Switch language
- **WHEN** a reader on `/wiki/exciters/bow/` picks Español
- **THEN** the browser goes to `/wiki/es/exciters/bow/`

#### Scenario: Control names stay in English
- **WHEN** a Spanish page mentions a control
- **THEN** it uses the label the plugin shows (for example "Bias Starve"), with the explanation in Spanish

### Requirement: Theme and search
The guide SHALL be dark-only and use the shared site tokens (the plugin palette). It SHALL offer full-text search over the pages of the current language, working offline from the built site with no external service.

#### Scenario: Search in Spanish
- **WHEN** a reader on a Spanish page searches for "óxido"
- **THEN** the results are Spanish pages, including the Rust parameter's entry

#### Scenario: No light mode
- **WHEN** the reader's system prefers a light colour scheme
- **THEN** the guide still renders in the dark theme, with no theme switch shown

### Requirement: Every feature has a place
The guide SHALL have a page or section for every user-facing feature of the plugin:
- each of the 16 exciters and 9 objects;
- every parameter;
- each Space, quality, note and randomizer mode;
- the factory bank and its folders;
- user presets;
- the kit editor;
- MIDI input and expression;
- the UI scale;
- the web app.

The models and modes SHALL come from the plugin (generated `models.json`), and a check SHALL fail when one has no page in either language.

#### Scenario: New exciter without a page
- **WHEN** the plugin gains a 17th exciter and the guide data is regenerated, but no page declares that it covers it
- **THEN** the coverage check fails, naming the exciter and the missing language(s)

### Requirement: Parameter reference generated from the plugin
The guide SHALL include a reference of every host parameter:
- ID, display name and panel;
- range, default and unit or steps;
- per-model labels for model-dependent slots;
- the ⓘ help text.

It SHALL be rendered from data exported from the plugin. A test in the CI lane SHALL fail when the committed data differs from what the plugin exports. Each parameter SHALL have a Spanish explanation, and a check SHALL fail when one is missing or refers to an ID that no longer exists.

#### Scenario: Parameter range changes
- **WHEN** a parameter's range or default changes in `src/params.rs` and the guide data is not regenerated
- **THEN** the drift test fails, naming the parameter

#### Scenario: Missing Spanish entry
- **WHEN** a parameter is added and regenerated, but `parameters.es.json` has no entry for its ID
- **THEN** the guide check fails, naming the ID

### Requirement: Spanish twins
Every English page SHALL have a Spanish page at the mirrored path, and the reverse. The two SHALL have the same section structure: the same number of headings at each level, in the same order. A check SHALL fail otherwise.

#### Scenario: Section added in English only
- **WHEN** an English page gains a heading and its Spanish twin does not
- **THEN** the guide check fails, naming both files

### Requirement: The guide builds and deploys with the site
The guide SHALL be built into `dist/wiki/` by the site's build script, deployed with the rest of the site, and checked in CI: the coverage checks, internal link validation and a full build. A broken internal link SHALL fail the build.

#### Scenario: Broken link
- **WHEN** a page links to `/wiki/exciters/bowed/`, which doesn't exist
- **THEN** the guide build fails, naming the page and the link

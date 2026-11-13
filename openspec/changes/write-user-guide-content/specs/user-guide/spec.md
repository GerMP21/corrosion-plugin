# Spec Delta

## Purpose

The user guide's content is complete, accurate, consistently shaped and reviewed in both languages.

## ADDED Requirements

### Requirement: No placeholder pages
No page of the guide SHALL be a placeholder in either language. The guide check SHALL run in CI in its strict mode, which fails on any placeholder.

#### Scenario: Placeholder left behind
- **WHEN** a page still contains the placeholder component
- **THEN** the strict guide check fails in CI, naming the page

### Requirement: Model pages share one shape
Each exciter page and each object page SHALL have these sections, in this order:
1. What it is.
2. How it sounds.
3. Controls.
4. Pairings.
5. Tips.
6. Presets to try.
7. Listen.

The Controls section SHALL link to the reference card of every control that the model shows. The Listen section SHALL include the model's comparison clip and its representative preset clip.

#### Scenario: A control without a link
- **WHEN** the Bow page's Controls section omits Bow Pressure
- **THEN** the guide check fails, naming the page and the control

### Requirement: Statements match the plugin
Ranges, defaults, labels, counts and mode names stated in the prose SHALL agree with the plugin. Where the prose gives a number that the generated data holds, it SHALL be rendered from the data rather than typed in.

#### Scenario: Count from data
- **WHEN** a page says how many factory presets there are
- **THEN** the number comes from `models.json`, so it updates when the bank changes

### Requirement: Surprising behaviours are documented
Known behaviours that surprise users SHALL be documented both where readers meet them and in Troubleshooting:
- the release snap of friction exciters;
- Env Amount 0 silencing friction exciters;
- the BREAKING automation changes;
- old Kit presets loading as Tonal;
- browser presets left at the preview address.

#### Scenario: Release snap
- **WHEN** a reader looks up why a Bow note gets louder on release
- **THEN** the Bow page and Troubleshooting both explain the release snap

### Requirement: Spanish reviewed before publishing
Every Spanish page SHALL be reviewed and approved by the maintainer before it is merged to the main branch. Control names SHALL stay as the plugin shows them. The review state SHALL be recorded per page in the change's notes.

#### Scenario: Unreviewed page
- **WHEN** a Spanish page has not been approved by the maintainer
- **THEN** it is not merged to main

### Requirement: Recipes are reproducible
Each recipe SHALL start from a named factory preset and list every setting it changes, with values as the plugin displays them. It SHALL include a before clip and an after clip, rendered by the clip generator from the same settings.

#### Scenario: Following a recipe
- **WHEN** a reader loads the named preset and applies the listed settings
- **THEN** the result sounds like the recipe's after clip, since the clip was rendered from those settings

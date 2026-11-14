# Spec Delta

## ADDED Requirements

### Requirement: The Wiki's name
Every place a reader sees the guide's name SHALL call it "Corrosion Wiki" in English and "Wiki de Corrosion" in Spanish. That covers:
- the site title;
- the Wiki home page;
- the site home page's navigation and links;
- the site's 404 page;
- the web app's navigation;
- the Wiki pages' own text.

The path stays `/wiki/`.

#### Scenario: Site navigation
- **WHEN** a visitor opens `https://corrosion-vst.org/` or `/play/`
- **THEN** the navigation link to `/wiki/` reads "Wiki", and no visible text calls it "Guide"

#### Scenario: Browser title
- **WHEN** a reader opens any English Wiki page
- **THEN** the page title ends with "Corrosion Wiki", and on a Spanish page with "Wiki de Corrosion"

### Requirement: Header links keep their casing
The links in the Wiki header SHALL be shown with the casing of their text, as on the rest of the site: "Home" and "Play" in English, "Inicio" and "Tocar" in Spanish.

#### Scenario: English header
- **WHEN** a reader opens an English Wiki page
- **THEN** the header links read "Home" and "Play", not "HOME" and "PLAY"

### Requirement: Clips of one control set expectations
The Wiki home page and the parameter reference SHALL each carry a short note, in both languages. The note SHALL say:
- that clips of one control alone are rendered from one plain patch and can sound alike;
- that Corrosion's character comes from combining controls;
- where to hear combinations: the recipes and the factory presets.

#### Scenario: Reference page
- **WHEN** a reader opens `/wiki/reference/` or `/wiki/es/reference/`
- **THEN** the note is shown before the first parameter card, with links to the recipes and the presets

### Requirement: Moving presets between the web app and the plugin
The Wiki SHALL explain, in both languages, how to move a preset between the web app and the plugin, with the means each one has:
- **From the web app to the plugin:** download the `.corrosion-preset` file, copy it into the folder the plugin's Open folder opens, and find it in the browser.
- **From the plugin to the web app:** open a saved preset's file with the web app's file opener.

Troubleshooting SHALL cover a copied preset that doesn't appear.

#### Scenario: Web preset into the plugin
- **WHEN** a reader follows the steps with a preset downloaded from the web app
- **THEN** the preset appears in the plugin's browser under the user presets and loads

### Requirement: The Wiki follows the plugin
A change that alters how the plugin works or sounds SHALL update the Wiki pages that describe that behaviour, in English and Spanish, in the same change. The project instructions (`CLAUDE.md`) SHALL state this as a mandatory rule.

#### Scenario: A behaviour change
- **WHEN** a change makes a control act differently
- **THEN** the same change updates every Wiki page that describes that control, in both languages

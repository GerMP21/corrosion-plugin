# Spec Delta

## Purpose

The public site at `corrosion-vst.org` introduces Corrosion, serves the browser synth and the user guide from one deploy, and shares the plugin's look.

## ADDED Requirements

### Requirement: Paths of the site
The site SHALL serve the home page at `/`, the web app at `/play/` and the user guide at `/wiki/`, all from one Cloudflare Worker's static assets deployed from the repository's main branch. A request for `/play` or `/wiki` without the trailing slash SHALL redirect to the path with it. An unknown path SHALL return a 404 page in the site's theme that links to the home page.

#### Scenario: The app at its new path
- **WHEN** a visitor opens `https://corrosion-vst.org/play/`, starts audio and plays a note
- **THEN** the web app loads with all its assets and the note sounds, as it did at the old root

#### Scenario: Missing trailing slash
- **WHEN** a visitor opens `https://corrosion-vst.org/play`
- **THEN** the response redirects to `/play/`

#### Scenario: Unknown path
- **WHEN** a visitor opens a path the site doesn't have
- **THEN** the response is a 404 with a themed page linking to `/`

### Requirement: Preview host redirect
Every request to `preview.corrosion-vst.org` SHALL be answered with a permanent (301) redirect to `https://corrosion-vst.org/play/`. The rule SHALL be documented in the repository with the exact dashboard settings, because it is not stored in `wrangler.toml`.

#### Scenario: Old link
- **WHEN** a visitor opens `https://preview.corrosion-vst.org/` or any path under it
- **THEN** the response is a 301 to `https://corrosion-vst.org/play/`

### Requirement: Home page
The home page SHALL say in one or two sentences what Corrosion is, and SHALL link prominently to the web app (`/play/`), the downloads (the repository's GitHub releases) and the guide (`/wiki/`). It SHALL also link the source repository and state the GPLv3 license and the build commit, and it SHALL be usable at phone width without horizontal scrolling.

#### Scenario: Calls to action
- **WHEN** a visitor opens `/`
- **THEN** the links to `/play/`, the GitHub releases and `/wiki/` are visible without scrolling on a 1280 × 720 window, and each resolves

#### Scenario: Phone width
- **WHEN** the home page is shown 375 px wide
- **THEN** the page has no horizontal scroll and the three calls to action are reachable

### Requirement: No third-party scripts or tracking cookies
The site's pages SHALL load no third-party script other than Cloudflare Web Analytics' cookie-free beacon, which the host injects (maintainer's choice), and SHALL set no tracking cookie.

#### Scenario: Scripts on the home page
- **WHEN** the live home page is loaded in a browser
- **THEN** every script comes from the site's own origin, except the Cloudflare Web Analytics beacon

### Requirement: Shared dark theme
The home page, the web app's page chrome and the guide SHALL use one shared set of design tokens whose colours are the plugin editor's palette. The site SHALL be dark-only. The token file's colours SHALL equal the plugin palette (`src/gui/palette.rs`, which the editor draws with), and a test SHALL fail when either side changes alone.

#### Scenario: Palette drift
- **WHEN** a colour in `src/gui/palette.rs` (for example `SAFETY_YELLOW`) changes and `web/theme/tokens.css` does not
- **THEN** the palette test fails, naming the colour

#### Scenario: Same look across sections
- **WHEN** a visitor moves between `/`, `/play/` and `/wiki/`
- **THEN** each page uses the same background, text and accent colours from the tokens

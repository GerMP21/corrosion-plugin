# Proposal

## Why

The web app is the whole site today: `preview.corrosion-vst.org` opens straight into the synth, with nothing that says what Corrosion is, where to download it, or how to use it. The maintainer now owns `corrosion-vst.org` and wants a real site there: a home page, the synth at `/play/`, and an exhaustive user guide at `/wiki/` (built in `add-user-guide`). This change builds the shell those pieces live in.

## What Changes

- **Home page** at `https://corrosion-vst.org/`. It says what Corrosion is and has three calls to action: Play in the browser (`/play/`), Download (the GitHub releases), and Read the guide (`/wiki/`). It also has a short feature tour (exciters × objects, kits, presets) and the GPLv3, source and build notices. It loads no third-party scripts and has no analytics.
- **The web app moves from `/` to `/play/`.** The app and its assets are unchanged; only the path changes. `/play` without a trailing slash redirects to `/play/`.
- **One Worker serves everything.** The `corrosion-vst` Worker's static assets hold the home page at the root, the app under `play/` and (from `add-user-guide`) the guide under `wiki/`. `wrangler.toml` adds `corrosion-vst.org` as a custom domain.
- **`preview.corrosion-vst.org` redirects** to `https://corrosion-vst.org/play/` with a 301, through a Cloudflare Redirect Rule set in the dashboard and documented in the repo. **BREAKING for browser storage:** sessions and user presets saved at `preview.` belong to that origin and are not carried over (the maintainer accepted this).
- **Shared theme.** One CSS file of design tokens holds the plugin's palette (CHARCOAL … THUMB_BORDER) as custom properties, plus type and spacing. The home page and the app's page chrome use it now, and the guide uses it in the next change. The site is dark-only. A test keeps the tokens equal to the editor's colour constants.
- **Build:** `web/scripts/build.sh` assembles `dist/` (home at the root, the app under `play/`). The smoke and feature scripts point at `/play/`, and a new home-page check is added.
- **Not in this change:** the guide's content and generator (`add-user-guide`, `write-user-guide-content`), and audio demos on the home page (added by `add-user-guide` once its clip generator exists).

## Capabilities

### New Capabilities
- `site-shell`: the public site's layout and shared look. Covers which path serves what (home, app, guide), the preview-host redirect, the home page's content and links, the shared dark theme and its equality with the plugin palette, and one deploy for the whole site.

### Modified Capabilities
None. The `web-app` capability (from `add-web-app`, not yet archived) states no URL path, so moving the app to `/play/` changes no requirement of it.

## Impact

- `web/www/` moves to `web/play/` (or the build copies it under `dist/play/`); new `web/home/` and `web/theme/tokens.css`.
- `web/scripts/build.sh`, `web/scripts/smoke.cjs`, `web/scripts/features.cjs`, `web/wrangler.toml`, `web/www/_headers` (moves to the dist root and covers `/play/*`).
- Cloudflare: a new custom domain `corrosion-vst.org` on the Worker, and a Redirect Rule for `preview.` (dashboard, done by the maintainer).
- `src/gui/palette.rs` (new, feature-free) holds the palette; `editor.rs` builds its colours from it. A Rust test reads `web/theme/tokens.css` (palette equality).
- Docs: README links, `docs/ARCHITECTURE.md` web section, `add-web-app` design note on the move.
- No change to the plugin, the engine or the sound.

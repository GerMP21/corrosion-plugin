# Tasks

## 1. Tests first

- [ ] 1.0 Move the palette to a feature-free `src/gui/palette.rs` (`[u8; 3]` constants, same names) and build the editor's `Color32` constants from it; GUI lane green, editor unchanged
- [ ] 1.1 `site_theme_matches_editor_palette` (CI lane): parse `web/theme/tokens.css`, compare both ways with `palette.rs`; run it red (file missing) and against one wrong colour
- [ ] 1.2 `web/scripts/site.cjs`: home links, `/play` → `/play/`, themed 404, no horizontal scroll at 375 px; run it red against the current `dist/`
- [ ] 1.3 Point `smoke.cjs` and `features.cjs` at `<base>/play/`; confirm they fail on the current layout

## 2. Theme and layout

- [ ] 2.1 `web/theme/tokens.css`: the 13 palette colours, semantic aliases, font stack, spacing scale; 1.1 goes green
- [ ] 2.2 `web/scripts/build.sh`: `site/*` to the dist root, `www/*` and the wasm builds to `dist/play/`, `theme/` to `dist/theme/`, build commit stamped in both pages
- [ ] 2.3 `wrangler.toml`: custom domain `corrosion-vst.org` (keep `preview.`), `html_handling = "auto-trailing-slash"`, `not_found_handling = "404-page"`
- [ ] 2.4 Move `_headers` to `web/site/` covering the whole site (wasm, js, css, html)

## 3. Pages

- [ ] 3.1 `web/site/index.html` and `home.css`: hero, the three calls to action, feature tour, requirements, footer; tokens only, no third-party scripts
- [ ] 3.2 Editor screenshot from the local web app with Playwright (1280 px wide, deviceScaleFactor 2), stored under `web/site/assets/`
- [ ] 3.3 `web/site/404.html`, themed, linking home
- [ ] 3.4 `web/www/index.html`: colours from the tokens; Home and Guide links in the header (Guide links `/wiki/`, which 404s until `add-user-guide` lands; acceptable or hidden behind a build flag, decide when implementing and note it here)
- [ ] 3.5 Run `site.cjs`, `smoke.cjs` and `features.cjs` against a local `wrangler dev` (or a static server reproducing the trailing-slash and 404 rules); all green

## 4. Deploy

- [ ] 4.1 Docs: README links to `https://corrosion-vst.org` and `/play/`; `docs/ARCHITECTURE.md` web section (layout, domains, Redirect Rule settings, tokens test); `add-web-app` design note on the move
- [ ] 4.2 Close-out lanes: fmt, clippy `-D warnings`, CI test lane, the web scripts; commit and push to `dev`
- [ ] 4.3 After the maintainer merges: `site.cjs` and `smoke.cjs` against `https://corrosion-vst.org/`; the maintainer adds the Redirect Rule; `curl -sI` on preview shows the 301; record both in design.md `## Measurements`
- [ ] 4.4 Update CLAUDE.md Current state

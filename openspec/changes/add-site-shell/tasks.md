# Tasks

## 1. Tests first

- [x] 1.0 Move the palette to a feature-free `src/gui/palette.rs` (`[u8; 3]` constants, same names) and build the editor's `Color32` constants from it; GUI lane green, editor unchanged
- [x] 1.1 `site_theme_matches_editor_palette` (CI lane): parse `web/theme/tokens.css`, compare both ways with `palette.rs`; run it red (file missing) and against one wrong colour
- [x] 1.2 `web/scripts/site.cjs`: home links, `/play` → `/play/`, themed 404, no horizontal scroll at 375 px; run it red against the current `dist/`
- [x] 1.3 Point `smoke.cjs` and `features.cjs` at `<base>/play/`; confirm they fail on the current layout

## 2. Theme and layout

- [x] 2.1 `web/theme/tokens.css`: the 13 palette colours, semantic aliases, font stack, spacing scale; 1.1 goes green
- [x] 2.2 `web/scripts/build.sh`: `site/*` to the dist root, `www/*` and the wasm builds to `dist/play/`, `theme/` to `dist/theme/`, build commit stamped in both pages
- [x] 2.3 `wrangler.toml`: custom domain `corrosion-vst.org` (as built: the only one; `preview.` moves to a placeholder DNS record), `html_handling = "auto-trailing-slash"`, `not_found_handling = "404-page"`
- [x] 2.4 Move `_headers` to `web/site/` covering the whole site (wasm, js, css, html)

## 3. Pages

- [x] 3.1 `web/site/index.html` and `home.css`: hero, the three calls to action, feature tour, requirements, footer; tokens only, no third-party scripts
- [x] 3.2 Editor screenshot from the local web app with Playwright (1280 px wide editor, deviceScaleFactor 2), stored under `web/site/assets/` (as built: `editor.webp`, 1600 px, 33 KB; `web/scripts/home-screenshot.cjs`)
- [x] 3.3 `web/site/404.html`, themed, linking home
- [x] 3.4 `web/www/index.html`: colours from the tokens; Home and Guide links in the header (Guide links `/wiki/`; as built: a themed placeholder at `web/site/wiki/index.html` answers there until `add-user-guide` replaces it, so every link resolves)
- [x] 3.5 Run `site.cjs`, `smoke.cjs` and `features.cjs` against a local `wrangler dev` (or a static server reproducing the trailing-slash and 404 rules); all green

## 4. Deploy

- [x] 4.1 Docs: README links to `https://corrosion-vst.org` and `/play/`; `docs/ARCHITECTURE.md` web section (layout, domains, Redirect Rule settings, tokens test); `add-web-app` design note on the move
- [x] 4.2 Close-out lanes: fmt, clippy `-D warnings`, CI test lane, the web scripts; commit and push to `dev`
- [ ] 4.3 After the maintainer merges: `site.cjs` and `smoke.cjs` against `https://corrosion-vst.org/`; the maintainer removes `preview.` from the Worker's domains, adds the placeholder record `AAAA preview 100::` (proxied) and the Redirect Rule; `curl -sI` on preview shows the 301; record both in design.md `## Measurements`
- [ ] 4.4 Update CLAUDE.md Current state

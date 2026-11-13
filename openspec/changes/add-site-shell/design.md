# Design

## Context

- `web/scripts/build.sh` writes `web/dist/` by copying `web/www/*` and the two wasm builds to the root. Workers Builds runs `bash scripts/cloudflare-build.sh` (root directory `web`) on pushes to `main`, then `npx wrangler deploy`.
- `wrangler.toml` serves `./dist` as static assets on the custom domain `preview.corrosion-vst.org`.
- The app already loads its assets by relative path: `fetch("corrosion_web_engine.wasm")`, `addModule("engine-worklet.js")` and `./corrosion_web_editor.js`. Moving the whole folder under `play/` therefore needs no path edits inside the app.
- The palette lives as `Color32` constants in `src/gui/editor.rs`, listed in the `add-web-app` notes: CHARCOAL #0D0D0D, DARK_STEEL #1A1A1A, CONCRETE #333333, SAFETY_YELLOW #FACC15, RUST_ORANGE #C2410C, DANGER_RED #B91C1C, OFF_WHITE #D4D4D4, MUTED_GREY #737373, STEEL_LIGHT #555555, RECESSED_BG #111111, FADER_TRACK_BG #000000, THUMB_BG #444444 and THUMB_BORDER #666666.

## Goals / Non-Goals

**Goals:**
- One deploy, one Worker, no server code: static assets only.
- The app's files move without being edited, so the move cannot change the app.
- The guide (next change) drops into `dist/wiki/` without touching this change's files.

**Non-Goals:**
- A site generator for the home page. The home page is one hand-written HTML page; the guide gets its own generator in `add-user-guide`.
- Moving browser storage from `preview.` to the new origin (accepted loss, see proposal).
- A `www.` host. It can be added later as another Redirect Rule without changing this design.

## Decisions

### 1. Source layout: `web/site/` holds everything that is not the app
```
web/
├── www/              the app's static page (unchanged), built into dist/play/
├── site/
│   ├── index.html    home page
│   ├── 404.html      themed not-found page
│   ├── home.css
│   ├── _headers      cache rules for the whole site
│   └── assets/       screenshot, icons
├── theme/tokens.css  shared design tokens (palette, type, spacing)
└── dist/             build output: site/* at the root, www/* + wasm in play/,
                      theme/ at /theme/, guide in wiki/ (next change)
```
`web/www/` keeps its name, so the `add-web-app` history and scripts still make sense. The build decides where things land. *Alternative:* renaming `www/` to `play/`. It reads better, but it touches every reference for no runtime gain.

### 2. Trailing slashes and 404 through Workers static-asset settings
`[assets]` gets `html_handling = "auto-trailing-slash"` (the default, set explicitly) and `not_found_handling = "404-page"`. With these, `/play` redirects to `/play/`, and unknown paths serve the nearest `404.html`. The guide can ship its own `wiki/404.html`. *Alternative:* a Worker script with routing. It isn't needed, and it would add code to maintain and requests to bill.

### 3. Domains
`routes` lists `corrosion-vst.org` as a custom domain. It keeps `preview.corrosion-vst.org` too, so its DNS record stays proxied by Cloudflare, which the Redirect Rule needs to run.

The Redirect Rule is set by the maintainer in the dashboard (Rules → Redirect Rules):
- Hostname equals `preview.corrosion-vst.org`.
- Static URL `https://corrosion-vst.org/play/`.
- Status 301, without preserving the query string.

`docs/ARCHITECTURE.md` records these settings. The rule runs before the Worker, so preview requests never reach the assets.

### 4. Theme tokens: one CSS file, checked against the Rust constants
`web/theme/tokens.css` defines `--charcoal`, `--dark-steel`, … (one per constant, same names in kebab case), and some semantic aliases:
- `--bg: var(--charcoal)`;
- `--surface: var(--dark-steel)`;
- `--text: var(--off-white)`;
- `--accent: var(--safety-yellow)`;
- `--warn: var(--rust-orange)`;
- `--danger: var(--danger-red)`.

It also holds the font stack and a spacing scale. Every page links `/theme/tokens.css`.

`src/gui/editor.rs` is compiled only with the `gui` or `web-gui` feature, so the CI lane (`--no-default-features`) can't see its constants. The palette therefore moves to a feature-free `src/gui/palette.rs`, as `pub const NAME: [u8; 3]` with the same names. The editor builds its `Color32` constants from it (`Color32::from_rgb(p[0], p[1], p[2])`, still `const`), so nothing on screen changes.

A Rust test (`site_theme_matches_editor_palette`, in `palette.rs`, in the CI lane) parses the file's `--name: #RRGGBB;` lines and compares them both ways with a list of the editor constants. A colour missing from the file, one with a different value, or one in the file with no constant all fail, and the failure names the colour.

*Alternative:* generate the CSS from Rust. That adds a generator to run for 13 lines; the equality test catches drift just as well.

### 5. The app's page chrome uses the tokens
The colours in `web/www/index.html` (header, footer, GitHub link) are replaced by `var(--…)` from the tokens. The canvas content is egui and already uses the constants. The header gains a "Home" link and a "Guide" link next to the GitHub icon.

### 6. Home page content, plainly
Hero (name, one sentence, editor screenshot from the web app at 1280 px), three buttons (Play in the browser, Download, Read the guide), and a tour of four cards:
- 16 exciters × 9 objects;
- Kit mode with per-key pads;
- 214 factory presets;
- runs in the browser.

It ends with requirements (VST3 and CLAP; Windows, macOS, Linux) and a footer (source, GPLv3, build commit, replaced at build time like the app's `#build`). There are no web fonts from third parties: a system font stack, or a self-hosted font under `/theme/` if the maintainer wants one. The screenshot comes from Playwright against the local web app, so its colours are real (unlike the REAPER `srgb: false` build).

### 7. Checks
- `web/scripts/smoke.cjs` and `features.cjs` take the base URL and append `play/`.
- A new `web/scripts/site.cjs` (Playwright, run locally and against the live site) checks:
  - `/` shows the three links, and each returns 200 (the releases link is checked for presence only, offline);
  - `/play` redirects to `/play/`;
  - an unknown path returns 404 with the themed page;
  - at 375 px wide, the page doesn't scroll sideways.
- The preview redirect is checked against the live site with `curl -sI` and recorded in design.md. It can't be checked locally, since the rule lives in Cloudflare.

## Risks / Trade-offs

- [Visitors lose their preview-origin session and presets] → Accepted by the maintainer. The home page and the app's header mention the new address. No migration code is written.
- [Wrangler's custom-domain route needs the zone `corrosion-vst.org` on the account] → The zone is already there (preview works). If the apex domain is missing, the deploy fails visibly and nothing changes for preview.
- [A cached old `index.html` at preview] → The Redirect Rule answers before the cache, and `_headers` already sets `max-age=3600, must-revalidate`.
- [Workers Builds deploys on merge, and the Redirect Rule is manual] → The tasks order them: merge and deploy first (both hosts serve the site), then the maintainer adds the rule, then the live check.

## Migration Plan

1. Merge to `main`. Workers Builds deploys, and both `corrosion-vst.org` and `preview.` now serve the new layout (preview's root shows the home page for a few minutes).
2. The maintainer adds the Redirect Rule. Check `curl -sI https://preview.corrosion-vst.org/` → `301 location: https://corrosion-vst.org/play/`.
3. Rollback: revert the merge. The previous deploy serves the app at the root again. Disable the Redirect Rule if it was added.

## As built

- **`/wiki/` placeholder.** The home page's guide link and the app's Guide link would 404 until `add-user-guide`. Instead, `web/site/wiki/index.html` (themed, "The guide is being written", pointing at the ⓘ help) answers there, so every call to action resolves (spec: home page links resolve). `add-user-guide` replaces it.
- **`routes` was inside `[assets]`.** In the `add-web-app` config, wrangler ignored it ("Unexpected fields found in assets field: routes"). The preview domain worked only because it had been added in the dashboard. `routes` is now top-level, listing both domains.
- **Local and CI server.** `python3 -m http.server` returns its own 404 page, not `404.html`. `web/scripts/serve.cjs` (no dependencies) follows the two Workers rules: directory → 307 to the trailing slash, nearest `404.html`. CI's browser step runs `site.cjs`, `smoke.cjs` and `features.cjs` against it. Locally, `wrangler dev` (4.156.0, with `--compatibility-date 2026-10-30` because its runtime predates the config's date) gives the same answers: `/play` → 307 `/play/`.
- **Screenshot.** `web/scripts/home-screenshot.cjs` captures the whole editor from the local app (1600 px window, so the editor is 1280 px wide, at deviceScaleFactor 2). It is converted to `web/site/assets/editor.webp` (1600 × 881, 33 KB), and CSS fades its cut-off bottom edge.
- **Extras.** A palette favicon (`web/site/favicon.svg`) on every page, `Referrer-Policy` in `_headers`. The app's footer says presets are kept "in this browser only, for this address".
- **App colours changed slightly.** The app's page chrome used colours outside the palette (`#121212` background, `#e8b400` accent, `#e6e1d8` text). It now uses the tokens: CHARCOAL, SAFETY_YELLOW, OFF_WHITE.

## Measurements

- `site.cjs`: all checks pass locally (`wrangler dev` and `serve.cjs`). Before the change, against the old `dist`: 11 of 22 checks failed (no calls to action, `/play` 404, unthemed 404, `#121212` background).
- `smoke.cjs` at `/play/`: strip RMS 0.31, key 0.35, next preset 0.51, no console errors. `features.cjs`: all pass.
- Palette test: green; with `--safety-yellow` changed to `#FACC16` it fails with `SAFETY_YELLOW: palette #FACC15, tokens.css #FACC16`.
- Lanes: fmt clean; clippy `-D warnings` (CI lane) clean; CI test lane 516 passed, 0 failed; GUI lane 510 passed, 0 failed (30 ignored). GUI-lane clippy shows 27 warnings, the same count as before the change.
- **Domains (maintainer, 2026-11-13).** The Worker's only domain is `corrosion-vst.org`. `preview.` was removed from `routes` (wrangler would re-add it on each deploy otherwise). It stays reachable for the redirect through a proxied placeholder record, `AAAA preview 100::`, which replaces Decision 3's "keep preview as a Worker domain". The Redirect Rule is unchanged: 301 to `/play/`.
- **Cloudflare Web Analytics kept (maintainer, 2026-11-13).** On the live site, Cloudflare injects its cookie-free beacon (`static.cloudflareinsights.com/beacon.min.js`) into the HTML it serves to browsers; it is not in the repository and `curl` doesn't see it. The spec and `site.cjs` allow that one script and nothing else.

## Live checks (2026-11-13, after #27 and #28 and the dashboard settings)

- `curl -sI`:
  - `https://corrosion-vst.org/` → 200 (home, build 3783b63);
  - `/play` → 307 `/play/`;
  - `/play/` → 200;
  - `/wiki/` → 200 (placeholder);
  - `https://preview.corrosion-vst.org/` and `/algo?x=1` → 301 `https://corrosion-vst.org/play/` (query string dropped).
- `site.cjs` against `https://corrosion-vst.org/` with `CHECK_EXTERNAL=1`: the only failure was the injected analytics beacon, now allowed (rerun below).
- `features.cjs`: passes.
- `smoke.cjs`: failed once (the wheel didn't scroll the page, `scrolled: 0`), then passed twice in a row (strip RMS 0.31, key 0.31). It looks like a timing flake on the live network. If it recurs, `smoke.cjs` should wait for the canvas height to settle before the wheel step.

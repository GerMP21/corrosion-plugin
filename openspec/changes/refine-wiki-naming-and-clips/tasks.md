## 1. Tests first

- [x] 1.1 Add the caption rule to `web/guide/scripts/check.mjs`: every clip group has a non-empty caption, and no two groups on a page share one unless they play the same files. Build the current site and confirm the rule fails, naming the envelope page.
- [x] 1.2 Add a value rule to the same check: every parameter button's accessible name contains its value, and choice parameters show an option name. Confirm it fails on the current build (Loop shows no value).

## 2. Clip captions and values

- [x] 2.1 `Clips.astro`: caption cell from `parameters.json` labels (parameter rows) and new i18n keys (set rows), optional `caption` prop override. Layout per design Decision 3, phone width included.
- [x] 2.2 Button titles and accessible names with the value: the manifest `source`, mapped to option names for choice parameters.
- [x] 2.3 Add the EN and ES i18n keys for the captions and the value label.
- [x] 2.4 `exciters/envelopes.mdx` (EN + ES): add the Attack and Release rows to the ADSR section.
- [x] 2.5 Run the check from 1.1 and 1.2 and confirm it passes. Screenshot the envelope page at desktop and phone width to check the layout.

## 3. Name, header and note

- [x] 3.1 `SiteLinks.astro`: drop the uppercase transform; check the header in both languages.
- [x] 3.2 Rename to "Corrosion Wiki" / "Wiki de Corrosion":
  - the Starlight title and the Wiki home page;
  - `web/site/index.html` and `404.html` (nav, call to action, footnote);
  - `web/www/index.html` (nav);
  - the pages' mentions of the guide, reworded in the writing style (EN + ES).
- [x] 3.3 Add the combinations note to the Wiki home and the reference page (EN + ES), with links to the recipes and the presets.

## 4. Presets between the web app and the plugin

- [x] 4.1 Write the transfer section on the user presets page (EN + ES): per-OS folder, copy, find it in the browser; the reverse through **Open preset…**; no Import button yet. Check the folder paths against `user_preset_dir_for`.
- [x] 4.2 Add a Troubleshooting entry (EN + ES) for a copied preset that doesn't appear: wrong extension, wrong folder, a file that doesn't parse (the browser's skipped count).

## 5. Rule, docs and close-out

- [x] 5.1 `CLAUDE.md`: a mandatory rule near the top that every change to how the plugin works or sounds updates the Wiki (EN + ES) in the same change. Rename "guide" to "Wiki" where it names the reader-facing site.
- [x] 5.2 `README.md` and `docs/ARCHITECTURE.md`: use the Wiki name where they point readers to it.
- [x] 5.3 Run `npm run check -- --complete` and the guide build (`web/guide`), `cargo test --workspace --no-default-features` (guide data drift tests), and `openspec validate refine-wiki-naming-and-clips`.
- [x] 5.4 Update "Current state" in `CLAUDE.md`; commit and push to `dev`.

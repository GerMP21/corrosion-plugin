# Proposal

## Why

The maintainer's first read of the published guide found rough edges:
- **Clip rows don't say what they play.** On the envelope page, three rows of Min / Mid / Max follow each other with nothing to tell Onset from Attack or Release. The specialty ADSR section has no Attack or Release rows at all.
- **The header shouts.** The header links read HOME and PLAY, while the rest of the site writes Home and Play.
- **The name doesn't match the path.** The guide should be called the Corrosion Wiki, as its `/wiki/` path already suggests.
- **Single-parameter clips set the wrong expectation.** Heard one control at a time, many clips sound alike; nothing tells the reader that the character comes from combinations.
- **Preset files have no documented way back.** The web app downloads presets, but the guide doesn't say how a downloaded file gets into the plugin.

There is also a process gap: nothing makes a change to the plugin update the Wiki in the same change.

## What Changes

- **Every clip row gets a caption** saying what it plays:
  - a parameter row shows the control's label as the editor shows it;
  - each Min / Mid / Max button shows, on hover and to screen readers, the value it plays, formatted as the plugin displays it;
  - set rows say what they compare (an exciter on the comparison object, an object with the comparison exciter, a factory preset, a recipe's before and after, all kits).
- **The specialty ADSR section** gets its Attack and Release rows.
- **A short note on the Wiki home and the parameter reference** says that clips of one control alone can sound alike, and that the character comes from combining controls. It points to recipes and presets. English and Spanish.
- **Header links** keep the text's own casing: Home / Play, Inicio / Tocar.
- **Rename the guide** to "Corrosion Wiki" ("Wiki de Corrosion" in Spanish) everywhere a reader sees it:
  - the Starlight site title and the home page title;
  - the site home's navigation, call to action and footnote, and the 404 page;
  - the web app's navigation;
  - the pages' own mentions of "this guide" or "the guide".
- **The Wiki documents moving presets between the web app and the plugin:** download a `.corrosion-preset` file in the web app, then copy it into the folder the plugin's **Open folder** opens (and the reverse). The page is in English and Spanish, with a Troubleshooting entry.
- **CLAUDE.md:** a mandatory rule, near the top, that every change to how the plugin works or sounds updates the Wiki (English and Spanish) in the same change.
- **Out of scope:**
  - Import and Export buttons in the plugin, a separate change (`add-preset-import-export`).
  - Renaming Legacy Drive.
  - Any change to the sound or to the clips' audio.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

These capabilities are still deltas of unarchived changes (`add-user-guide`, `write-user-guide-content`). This change adds requirements to them, so archive it after those.
- `user-guide`:
  - the Wiki's name;
  - the header link casing;
  - the combinations note;
  - documented preset transfer;
  - the rule that the Wiki follows every behaviour change.
- `guide-audio-clips`: every clip row is captioned and every parameter button states its value.

## Impact

- **Wiki:**
  - `web/guide/src/components/Clips.astro` (captions, value titles) and `SiteLinks.astro` (casing);
  - `web/guide/src/content/i18n/{en,es}.json`;
  - `web/guide/astro.config.mjs` (title);
  - pages: `index`, `reference`, `exciters/envelopes`, preset and troubleshooting pages, mentions of "guide" (EN + ES);
  - `web/guide/scripts/check.mjs` (a test that every clip row has a caption).
- **Site and web app:** `web/site/index.html`, `web/site/404.html`, `web/www/index.html`.
- **Repo docs:** `CLAUDE.md`, `README.md` and `docs/ARCHITECTURE.md`, where they name the guide.
- **No Rust code, clip audio, parameters or presets change.** The clip manifest already holds each value's display text, so no clips are regenerated.

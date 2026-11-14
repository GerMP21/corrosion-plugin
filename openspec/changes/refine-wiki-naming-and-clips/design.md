# Design

## Context

`Clips.astro` renders a row of buttons from `public/clips/clips.json`.
- **Parameter rows** (`<Clips param="…">`, 112 uses) label their buttons only Min / Mid / Max.
- **Set rows** (`exciters`, `objects`, `exciter-presets`, `object-presets`, `kits`, `recipes`) label their buttons with the compared item or preset, and have no caption either.
- **Values:** each manifest entry's `source` already holds the value as the plugin formats it (`param:output:min` → `output = -inf dB`). Choice parameters come out as their index (`loop_mode = 2`), and times without a unit (`mseg_onset = 0.001`), because the plugin itself formats them that way.
- **Labels:** `parameters.json` holds each parameter's editor labels (`controls[].label`), its host `name`, its `unit` and, for choices, its `options`.

The header links come from `SiteLinks.astro`. The text is already "Home" / "Play"; the CSS sets `text-transform: uppercase`.

The capabilities this change adds to (`user-guide`, `guide-audio-clips`) exist only as deltas of `add-user-guide` and `write-user-guide-content`, which are done but not archived.

## Goals / Non-Goals

**Goals:**
- Every clip row has a caption, and every parameter button states its value.
- The Wiki name is used everywhere a reader sees it.
- The header casing matches the site.
- The combinations note.
- Preset transfer is documented.
- The CLAUDE.md rule.

**Non-Goals:**
- Any change to clip audio, the clip plan or the plugin's Rust code. The fingerprint stays the same and no clips are regenerated.
- Plugin Import / Export buttons (`add-preset-import-export`).
- Renaming Legacy Drive or any parameter.

## Decisions

1. **Captions are computed in `Clips.astro`, not written in each page.**
   - **Parameter rows:** the first editor label of the parameter in `parameters.json`, else its host `name`. Labels stay in English on both languages (the `user-guide` rule on control names).
   - **Set rows:** captions come from new i18n keys.
     - `guide.clips.caption.exciter` → "{name}, on {object}";
     - `guide.clips.caption.object` → "{name}, with {exciter}";
     - `guide.clips.caption.preset` → "Factory preset";
     - `guide.clips.caption.recipe` → "Before and after";
     - `guide.clips.caption.exciters` / `objects` / `kits` for whole sets.
   - **Compared item:** the comparison object or exciter is read from the manifest's `source` ("Bow on Tank"), as the button labels already do.
   - **Alternative rejected:** a `caption` prop on every `<Clips>` (260 hand edits that would drift).
   - **Override:** an optional `caption` prop still overrides the computed text, for a page that needs it.

2. **Values come from the manifest, mapped through `parameters.json`.**
   - **Value text:** the text after ` = ` in `source`.
   - **Choice parameters:** a parameter with `options` maps the index to the option's name.
   - **Units:** the text is used as is, which is how the plugin shows it, so a time reads "0.001" exactly like the plugin's fader.
   - **Where it appears:** in the button's `title` and in its accessible name ("Play Onset, Min: 0.001"). The visible label stays Min / Mid / Max, which keeps rows compact.
   - **Alternative rejected:** a new `display` field in the manifest. It changes the clip plan's fingerprint and forces a 3-minute clip rebuild for no audible change.

3. **Row layout.** A caption cell of fixed minimum width before the buttons, in the existing muted caps style of `.clip`, so the stacked rows line up like a table. On a phone, the caption wraps above the buttons.

4. **The check enforces captions.** `scripts/check.mjs` already walks the built pages. It gains a rule: every `.clips` group has a non-empty `.clips-caption`, and no two groups on a page share a caption unless they play the same files. The rule is written first and run against the current build, where it must fail.

5. **The ADSR section reuses the struck rows.** `env_attack` and `env_release` are the same parameters for struck and specialty exciters. The ADSR section repeats `<Clips param="env_attack" />` and `env_release`, which play the same files. Decision 4's "same files" exception allows the shared caption.

6. **Naming.**
   - **English:**
     - Starlight `title` → "Corrosion Wiki";
     - the Wiki home's title → "Corrosion Wiki";
     - site nav and 404 "Guide" → "Wiki";
     - "Read the guide" → "Read the Wiki";
     - the home footnote → "in the Wiki";
     - the web app nav → "Wiki".
   - **Spanish:** "Wiki de Corrosion".
   - **Prose:** pages that say "this guide" / "esta guía" (about 17 places) are reworded in the guide-writing-style voice, not search-and-replaced.
   - **Code names stay:** the code names (`web/guide`, "guide data", `guide_data_equals_the_files_on_disk`) keep their names. They are not reader-facing, and renaming them would churn tests and CI for nothing.

7. **Header casing:** remove `text-transform: uppercase` from `.site-links a`, and keep the letter spacing at normal.

8. **Preset transfer page.** A section on the existing presets page (`presets/` user presets page, EN + ES), plus a Troubleshooting entry. The section gives:
   - the folder each OS uses (from `user_preset_dir`);
   - that the browser lists `.corrosion-preset` files from that folder and its subfolders;
   - the reverse path through the web app's **Open preset…** button.

   It says plainly that the plugin has no Import button yet.

## Risks / Trade-offs

- **Caption text for set rows depends on the manifest's `source` wording.** Mitigation: the check from Decision 4 catches empty or duplicated captions. A unit test in `check.mjs` parses one example of each `source` form.
- **Raw values for choices and unitless times:** the tooltip shows what the plugin shows, which for times has no unit. This is accepted and matches the plugin. Giving times a unit is a plugin change, out of scope.
- **The archive order matters** (after `add-user-guide` and `write-user-guide-content`), as the proposal notes.

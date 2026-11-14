# Proposal

## Why

The web app opens and downloads `.corrosion-preset` files, but the plugin can only save into its own user folder and open that folder in the file manager. Moving a sound out of the plugin, or into it, means finding a hidden per-user folder by hand. The maintainer noticed it from the web app's Download button: a preset downloaded there has no obvious way into the plugin, and a plugin preset has no obvious way out.

## What Changes

- **Import… button** in the plugin's preset browser:
  - it opens the system file dialog and accepts one or more `.corrosion-preset` files;
  - each file is checked (migrated and parsed as a preset load would), then copied into the folder chosen in the browser's folder selector;
  - a name clash gets a numbered file name, never an overwrite;
  - the last imported preset loads;
  - files that don't parse are skipped and reported, with no crash and no effect on audio.
- **Export… button:**
  - it opens the system save dialog, named after the current preset;
  - it writes the current sound as a `.corrosion-preset` file anywhere the user picks, in the same format Save writes and the web app opens.
- **No dialog available:** when the platform has no file dialog (for example a Linux desktop without a portal), both buttons report it and point to Open folder. Nothing hangs.
- **Wiki:** the presets pages and Troubleshooting (English and Spanish) describe Import and Export, replacing the manual copy steps written by `refine-wiki-naming-and-clips`.
- **New dependency:** a native file dialog crate (`rfd`), on the GUI build only. The `--no-default-features` (CI) build is unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `user-presets`: adds importing preset files into a user folder, and exporting the current sound to a chosen file.

## Impact

- **Code:**
  - `src/presets/mod.rs` gets the import and export file logic, as pure, testable functions over paths;
  - `src/gui/editor.rs` gets the two buttons, the dialog call and the status messages;
  - `Cargo.toml` gets `rfd` behind the GUI feature.
- **Platforms:**
  - the Windows and macOS dialogs are native;
  - Linux uses the XDG desktop portal (no GTK link), and falls back to a message when no portal answers.
- **Bundles and validators:** the CI bundles and pluginval and clap-validator must stay green, and the Linux bundle must not gain a GTK dependency.
- **No sound change.** No parameter, preset format or migration changes. The web app is unchanged.
- **Order:** after `refine-wiki-naming-and-clips`, whose Wiki section this replaces.

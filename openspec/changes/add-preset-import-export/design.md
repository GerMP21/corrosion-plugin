# Design

## Context

- **Plugin presets:** user presets live in `user_preset_dir()` (`%APPDATA%\Corrosion\Presets`, `~/Library/Application Support/Corrosion/Presets`, or `$XDG_DATA_HOME/corrosion/presets`), with subfolders.
- **The browser:** it lists them, Save writes into a selected user folder (overwrite asks first), and Open folder runs `explorer`, `open` or `xdg-open`.
- **Format:** files are `.corrosion-preset` JSON. They go through `migrate_preset_json` before parsing, so old files load.
- **The web app:** it uses the same format (`browser.rs` keeps them in page storage) and already has Open preset… and Download preset.
- **Editor environment:** the editor is egui on baseview, inside a host's GUI thread.

## Goals / Non-Goals

**Goals:**
- Import and Export in the plugin with the system file dialogs.
- File logic separated from the dialog, so it can be tested.
- Failures are reported, never block.

**Non-Goals:**
- Drag-and-drop onto the editor (baseview's drop support differs by platform; possible later).
- Changing the web app's buttons.
- Bank or folder import (one or more files only).
- Changes to the preset format.

## Decisions

1. **File logic in `presets`.** Two functions, tested with temp dirs:
   - **`import_preset_files(paths, target_dir) -> ImportReport`:** reads, migrates and parses each file, picks a free name (`Name`, `Name (2)`, …, matched case-insensitively as `same_place` does) and writes it. It returns the imported paths, the last preset and the skip count.
   - **`export_preset(preset, path)`:** writes the same JSON as Save, adding the extension if the user left it out.
   - **Target folder:** the folder selector's current choice, the same one Save uses, so the factory bank is impossible by construction.

2. **Dialogs with `rfd`, behind the GUI feature.**
   - **Windows and macOS:** native dialogs.
   - **Linux:** the `xdg-portal` backend, not GTK, so the bundle gains no GTK link and no init conflict with the host.
   - **Alternatives rejected:**
     - `native-dialog`: on Linux it shells out to zenity or kdialog, which may be missing.
     - a home-made egui file picker: a lot of UI, and it looks foreign.

3. **Threading.**
   - **Where dialogs run:** on Windows and macOS the dialog must run on the GUI thread (AppKit requires the main thread), modally, as hosts expect from plugin dialogs. On Linux the portal call is asynchronous.
   - **One path for all three:** the dialog future runs on a short-lived thread, and the editor polls its result each frame through a one-slot channel. On macOS the dialog is opened with `rfd`'s main-thread API instead.
   - **While waiting:** the buttons are disabled, and audio never waits on any of this.

4. **No-dialog fallback.** If the portal call errors or returns no handle within the call, the editor shows "No file dialog available: use Open folder and copy the file there". A user cancel is silent.

5. **Wiki.** The section written by `refine-wiki-naming-and-clips` (manual copy) is rewritten around Import and Export, EN + ES. The manual path stays as the fallback paragraph.

## Risks / Trade-offs

- **Hosts and modal dialogs.** Some hosts dislike a plugin opening a modal dialog. Mitigation: check in REAPER on Linux, following `docs/REAPER_TESTING.md` (in this container the portal is absent, so it exercises the fallback message). Windows and macOS are checked by the maintainer. DAW behaviour on those platforms is not tested in CI, and the close-out says so.
- **Dependency weight.** `rfd` with `xdg-portal` pulls in `ashpd`/`zbus`. The Linux bundle size is measured before and after and recorded in `## Measurements`. CI's no-default-features lanes don't build it.
- **Linux portals vary.** Without `xdg-desktop-portal`, Import falls back to the message. That's acceptable, since Open folder still works.

## As built

- **File logic (Decision 1).**
  - **`import_preset_files(paths, dir, folder)`:** checks each file with `Preset::from_json_str` (migrations included) and copies its bytes unchanged, so migrations keep running at each load, like every other user preset. The name is `sanitize_file_name(preset.name)`, numbered `Name (2)`, `(3)`, … against the folder's file names compared in lowercase. Files that can't be read or parsed count as skipped. A folder that would leave the user folder is refused (`user_subfolder`).
  - **`export_preset(name, params, path)`:** writes `Preset::from_params(name).save`, byte-identical to Save, and adds `.corrosion-preset` when the path lacks it.
- **Dialogs (Decisions 2–4).** `src/gui/file_dialog.rs` (`gui` feature only):
  - **Windows and Linux:** `rfd::FileDialog` (sync) runs on a named thread and answers through a one-slot channel, which the editor polls each frame, asking for a repaint every 100 ms while a dialog is open.
  - **macOS:** it opens in place, modal.
  - **Thread safety:** the receiver sits in a `Mutex`, because the editor state must be `Sync`.
  - **Linux backend:** `rfd` 0.18 with only `xdg-portal`. That backend falls back to zenity, so there is no GTK link.
- **Telling a missing dialog from a cancel (Decision 4, refined).** On Linux, when neither the portal nor zenity answers, `rfd` only logs and returns `None`, the same as a cancel. `classify` treats an empty answer under `UNSEEN_DIALOG` (250 ms) as "no dialog", because no person cancels that fast. A dialog thread that dies without answering also counts as "no dialog".
- **Editor.**
  - **Buttons:** Import… and Export… sit right of Open folder, both disabled while a dialog is open. Import is also disabled without a user folder. Each has a tooltip.
  - **Import:** goes into the Save row's folder. It selects the last imported preset (by name and folder, the last match) and loads it through `apply_preset`.
  - **Messages:**
    - `Imported "X"`, `Imported N presets`, plus `, K file(s) skipped`;
    - `That file isn't a preset` / `None of those files is a preset` (errors);
    - `No file dialog on this system: …` pointing to Open folder (errors);
    - a cancel shows nothing.
  - **Export:** proposes the typed Save name, else the selected preset's (sanitized). The preset's name is the stem of the file the user chose.
- **Known limit: zenity fallback.** Zenity (the GTK4 build in Ubuntu 24.04) ignores `--filename`, with a bare name or an absolute path alike, so Export's proposed name is empty there. The portal and the Windows and macOS dialogs take it.

## Measurements

**Plugin library size**, Linux x86_64 gnu, release, `libcorrosion.so`, before = `dd9e6cd`, after = this change:

| | before | after | change |
|---|---|---|---|
| unstripped | 8,204,904 B | 8,408,352 B | +203 KB (+2.5 %) |
| stripped | 6,903,400 B | 7,051,064 B | +148 KB (+2.1 %) |
| `NEEDED` libraries | xcb, GL, X11-xcb, X11, gcc_s, m, c, ld-linux | same | none added (no GTK, no D-Bus library) |

`cargo tree --no-default-features` and the `web/` workspace contain no `rfd`.

**Checked in REAPER (Linux, 75 %)**, with the `srgb: false` screenshot build:
- **Layout:** Open folder / Import… / Export… fit on one row of the PRESET BANK box.
- **No dialog:** with no portal and no zenity, Import showed "No file dialog on this system: copy preset files into Open folder instead" at once, and the editor kept drawing.
- **With zenity installed:**
  - Export opened the save dialog with the "Corrosion preset" filter and wrote `Night Shift.corrosion-preset` (name "Night Shift"), reporting `Exported "Night Shift" to …`.
  - Import picked a factory preset file, copied it into the user folder, selected "User / Tank Boom Bass" and loaded it (Felt Mallet on Tank, cutoff 900), reporting `Imported "Tank Boom Bass"`.
- **Not checked:** the XDG portal path (no portal in the container), Windows and macOS dialogs, and other hosts.

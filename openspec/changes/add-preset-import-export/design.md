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

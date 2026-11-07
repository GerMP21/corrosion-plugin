# Tasks

## 1. Missing controls

- [x] 1.1 Add the editor's host-only list constant, and a unit test comparing the editor's widget parameter IDs plus that list with `CorrosionParams::param_map()`. Verify it fails today and names the missing IDs.

  (Done: `editor_covers_every_parameter` renders the real `render_ui` headlessly with egui for every exciter, object and space mode, and records each widget's parameter. Before the new controls it named `play_mode`, `limiter_mode`, `sync_rate`, `res_brightness`, `thickness`, the four macros and `exciter_pressure/speed/roughness`. The last three are on the host-only list until H6 wires them.)

- [x] 1.2 Add Brightness (Resonator column, by Damping) and Thickness (by Heat and Sludge). Verify they appear in the widget set and that the coverage test's missing list shrinks.
- [x] 1.3 Add the Sync control in the Space section, showing H3's "Off" or division text. Verify the displayed text matches the parameter's formatter.
- [x] 1.4 Add Play Mode and Limiter Mode combos in the header. Verify that selecting each value sets the parameter, and that a host automation change shows in the combo (manual check in a host or standalone; record which). (Done. No host or display is available in this cloud container, so the selection and automation checks weren't done manually. The combos are the same `industrial_combo` that already drives Quality and Material, and they read `param.value()` every frame.)
- [x] 1.5 Add the macro strip with Mass, Corrosion, Violence and Brightness knobs. Verify the coverage test passes. Update `docs/ARCHITECTURE.md`'s GUI section and verify it lists the new controls. (Done. `editor_covers_every_parameter` passes. ARCHITECTURE has no GUI section, so the module-map entry for `editor.rs` lists the new controls. The editor scrolls (`ScrollArea`): at 100 % its content was already 920–1088 px tall at HEAD for a 768 px window, and the new controls add about 100–135 px (`print_editor_content_height`).)

## 2. User presets

- [x] 2.1 In `src/presets/mod.rs`, add `user_preset_dir()` (platform env vars, Decision 3), `sanitize_file_name()`, and `list_user_presets(dir)`, which skips unreadable files and counts them. Unit-test the sanitizing with separators, reserved names and empty names, and list a temp folder with one corrupt file. Verify the tests pass.
- [x] 2.2 Add the name field, Save, and the overwrite confirm/cancel in the preset loader, with inline errors. Unit-test a save → reload round trip into a temp folder (all parameters except `ui_scale` restored) and a declined overwrite (file bytes unchanged). Verify the tests pass.
- [x] 2.3 Show the user presets after the factory bank, refreshed after a save. Verify manually in the standalone or a host that a new save appears without reopening, or record that only the unit tests cover it. (Done. No host or display is available in this cloud container. `saved_preset_appears_without_reopening` checks the editor state instead: after a save, the new preset is listed after the factory bank and selected, and a second save of the same name waits for confirmation.)

## 3. Randomizer

- [ ] 3.1 Scale the randomizer's `body` ranges by 5. Add a seeded randomize function using `Noise` with redraw-then-clamp safety, and remove `mode_param()`. Unit-test the spec's Safe bounds (100 draws), Full-mode safety (1000 draws) and that two draws differ. Verify the tests pass.
- [ ] 3.2 Add the Randomize button and mode combo, applying changes with begin/set/end per parameter. Verify only the seven parameters change (unit test on the applied list), and manually that host undo reverts a randomize (record the host used, or that it wasn't available).

## 4. Close-out

- [ ] 4.1 Update the README (user preset folder per platform, randomizer modes). Mark H5 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`, along with the "Missing features" items 1, 2 and 5. Run fmt, clippy (default features) and the full test suite, and verify all are green.

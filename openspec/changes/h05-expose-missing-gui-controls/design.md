# Design

## Context

- **Editor:** `src/gui/editor.rs` (about 3 500 lines, egui via nih_plug_egui) draws exciter, resonator and processing columns, a header with the preset loader, and a footer. Controls use `industrial_knob`, `industrial_fader` and `industrial_combo` against `ParamSetter`.
- **Missing controls:** Brightness, Thickness, Sync, Play Mode, Limiter Mode and the macros have no widget.
- **Presets:** the preset loader lists the embedded factory bank (`include_dir`). `Preset::save`/`load` already read and write JSON files. The Save button is `add_enabled(false, …)`.
- **Randomizer:** `src/randomizer/mod.rs` provides mode ranges, `randomize_params(mode, t_values)`, `SafetyConstraints` and a `mode_param()` that nothing registers. Its ranges predate C5: `body` is 0–1, while the BODY parameter is now 0–5.
- **Load path:** loading uses H4's generic `apply_preset`.

## Goals / Non-Goals

**Goals:**
- Every audible parameter is reachable.
- User presets work on Windows, macOS and Linux.
- The randomizer is usable and safe.

**Non-Goals:**
- Re-laying out the editor.
- Preset folders, tags or search.
- Importing or exporting presets through a file dialog.
- Randomizing beyond the module's seven parameters.

## Decisions

1. **Control placement.**
   - Brightness goes next to Damping, and Thickness with Heat and Sludge, in their existing columns.
   - Sync goes in the Space section, shown with Echo.
   - Play Mode and Limiter Mode are combos in the header, by Quality.
   - The four macros are a horizontal knob strip above the footer. They are global performance controls, so they sit apart from the per-section ones.
2. **Host-only list.** A constant in the editor lists the parameters deliberately left host-only, each with a reason (for example `ui_scale`, driven by the scale menu). A unit test compares the editor's widget parameter set plus that list with the parameter map, so the next added parameter fails the test until placed.
3. **User preset folder from standard environment variables, without a new dependency.**
   - Windows: `%APPDATA%\Corrosion\Presets`.
   - macOS: `$HOME/Library/Application Support/Corrosion/Presets`.
   - Linux: `$XDG_DATA_HOME/corrosion/presets`, falling back to `$HOME/.local/share/corrosion/presets`.
   - The folder is created on first save.
   - *Alternative:* the `dirs` crate. It's simple, but it's a new dependency for three paths.
4. **Saving.**
   - An inline name field and a Save button in the preset loader.
   - File names are the typed name with path separators and characters invalid on any platform replaced by `_`, plus `.corrosion-preset`. The display name stays as typed.
   - Existing names trigger an inline confirm/cancel.
   - All file I/O runs on the GUI thread, never on the audio thread. Errors show inline and are kept until the next action.
5. **Listing.**
   - The list is the factory entries, a separator, then the user files sorted by display name.
   - The user list is re-read on editor open and after every save.
   - Unreadable files are skipped and counted in a small notice.
6. **Randomizer integration.**
   - A Randomize button with a mode combo next to it. The mode is editor state, not a host parameter: it shapes the next click, not the sound, so `mode_param()` is removed.
   - Randomness comes from `dsp::utils::noise::Noise`, seeded from the system time when the editor opens.
   - Draws that fail `SafetyConstraints::is_safe_patch` are redrawn, up to 16 times, then clamped with `clamp_to_safe_ranges`.
   - The `body` ranges are scaled by 5, so Safe gives BODY 0–2.5 (the spec scenario's bound).
   - Each changed parameter is applied with begin/set/end through `ParamSetter`.

## Risks / Trade-offs

- **[The editor gets busier]** → The macros get their own strip, and the other new controls go where their siblings already are.
- **[Sandboxed hosts (macOS AU and App Store hosts) can deny writes to the folder]** → The save error is shown. A configurable folder is a later follow-up.
- **[Randomizer output depends on the system time]** → Intended for a creative tool. Tests seed the source explicitly.

## Migration Plan

Factory presets are unchanged. User presets are new files, so there's nothing to migrate. Rollback means removing the controls; the files on disk stay readable by any later version through the existing schema migration.

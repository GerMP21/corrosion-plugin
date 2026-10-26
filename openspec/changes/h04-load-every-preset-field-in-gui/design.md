# Design

## Context

- **Editor load:** `apply_preset(params, setter, preset)` converts the preset with `Preset::into_params()`, then calls `setter.set_parameter` for a hand-written list of about 140 of the plugin's 146 parameter IDs. Missing: `play_mode`, `limiter_mode` and `macro_*`.
- **Host restore:** nih-plug persists every parameter by ID through the `Params` derive.
- **The parameter map:** `Params::param_map()` returns `(id, ParamPtr, group)` for every parameter.
- **Setting a parameter from the GUI** has to go through `ParamSetter`, wrapped in the begin/set/end gesture, so the host records it.

## Goals / Non-Goals

**Goals:**
- No hand-maintained list.
- A test that fails when a parameter isn't applied.

**Non-Goals:**
- User preset saving and the GUI controls (H5).
- The preset schema.

## Decisions

1. **Walk the parameter maps by ID.**
   - Build the loaded `CorrosionParams` with `into_params()` and pair each live parameter with the loaded parameter of the same ID.
   - Set it through the setter's raw-context begin/set-normalized/end on the live `ParamPtr`.
   - *Alternative:* keep the list and add the six. It forgets the next parameter added, which is exactly how this bug happened.
2. **Separate the "what" from the "how".**
   - A pure function `preset_parameter_values(preset) -> Vec<(id, normalized)>` lists what a load sets. Unit tests compare it with the parameter map and with the host path, without an editor or host.
   - `apply_preset` just feeds that list to the setter.
3. **Exclude `ui_scale` by ID.**
   - It's a window preference. Today loading a preset can resize the editor.
   - The exclusion list is one constant, so a future editor-only parameter goes there visibly.

## Risks / Trade-offs

- **[More host automation events per load: every parameter, not about 140]** → Six more events per preset load. Negligible.
- **[A parameter whose preset field is missing]** → `into_params` already fills every field (older schemas are migrated), so every ID has a value. The "every parameter" test pins it.

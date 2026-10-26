# Tasks

## 1. Generic apply

- [ ] 1.1 Add `preset_parameter_values(preset)` (id → normalized value, from `into_params().param_map()`, excluding `ui_scale`), with unit tests:
  - the Kit/lookahead/macros preset from the spec;
  - an "every parameter differs" preset, checking the IDs are exactly the parameter map minus `ui_scale`;
  - agreement with the host path (`Preset::into_params` normalized values).

  Verify they pass. Also check that the old hand-written list misses the six IDs, recorded in the test's comment.
- [ ] 1.2 Rewrite `apply_preset` to apply that list through the setter's raw context (begin, set normalized, end per parameter). Verify `cargo build` with the GUI feature, and the editor tests (`cargo test --lib gui`).
- [ ] 1.3 Load a factory preset with Drone mode in the standalone or a host, if one is available, and verify play mode changes. Otherwise record that this check is covered by the unit tests only.

## 2. Close-out

- [ ] 2.1 Mark H4 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`, noting the `ui_scale` exclusion. Run fmt and clippy (default features and `--no-default-features`), and verify both are clean.

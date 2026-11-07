# Tasks

## 1. Generic apply

- [x] 1.1 Add `preset_parameter_values(preset)` (id → normalized value, from `into_params().param_map()`, excluding `ui_scale`), with unit tests:
  - the Kit/lookahead/macros preset from the spec;
  - an "every parameter differs" preset, checking the IDs are exactly the parameter map minus `ui_scale`;
  - agreement with the host path (`Preset::into_params` normalized values).

  Verify they pass. Also check that the old hand-written list misses the six IDs, recorded in the test's comment.

  (Done: `Preset::parameter_values` in `src/presets/mod.rs`, with tests in `tests/preset_loading.rs`. Comparing the old `apply_preset` list with `src/params.rs` shows it skips exactly `play_mode`, `limiter_mode` and the four `macro_*`. Values are normalized against the live parameter, because `into_params` builds some parameters with other ranges, Size 0.05–10 among them.)

- [x] 1.2 Rewrite `apply_preset` to apply that list through the setter's raw context (begin, set normalized, end per parameter). Verify `cargo build` with the GUI feature, and the editor tests (`cargo test --lib gui`).
- [x] 1.3 Load a factory preset with Drone mode in the standalone or a host, if one is available, and verify play mode changes. Otherwise record that this check is covered by the unit tests only. (Done: no host or display is available in this cloud container, so this is covered by the unit tests only. `tests/preset_loading.rs` checks play mode, limiter mode and the macros, and the editor's `apply_preset` is a thin loop over the same list.)

## 2. Close-out

- [x] 2.1 Mark H4 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`, noting the `ui_scale` exclusion. Run fmt and clippy (default features and `--no-default-features`), and verify both are clean. (Done. This container's stable toolchain is 1.97, which doesn't know the `semicolon_in_expressions_from_non_local_macros` lint allowed in `src/lib.rs`, so `--no-default-features` clippy runs with `-A unknown_lints`, as the project's 1.97 gate does. The default-feature clippy warnings are all in existing editor code.)

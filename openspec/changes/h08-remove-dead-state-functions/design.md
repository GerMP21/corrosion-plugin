# Design

## Context

- **Host state:** nih-plug saves and restores plugin state from the `#[derive(Params)]` struct, by parameter ID. Plugin-specific extra state would go in `#[persist]` fields; Corrosion has none for presets.
- **The dead pair:** `get_state` serializes a `Preset` from the live parameters. `load_state` parses one and swaps `self.params` for a new `Arc`.
- **Callers:** there are none, in `src/`, `tests/` or `benches/`.

## Goals / Non-Goals

**Goals:** remove code that can only do harm if ever called.

**Non-Goals:**
- Changing host state persistence.
- The preset parameter-metadata asymmetry (finding M15), which only mattered for this path.

## Decisions

1. **Delete, don't rewrite through setters.**
   - The host path already restores every parameter, and the editor loads presets through `ParamSetter` (H4).
   - A third route would only duplicate those.
   - *Alternative:* rewrite `load_state` to set values through the existing parameters. That adds code with no caller.

## Risks / Trade-offs

- **[An external crate depends on these methods]** → The crate is a plugin, not a library dependency, and nothing in the repository uses them. If a use appears later, it should go through `Params`.

# Proposal

## Why

`Corrosion::get_state` and `Corrosion::load_state` (`src/lib.rs`) are public, but nothing calls them: nih-plug persists parameters through the `Params` derive. `load_state` is also dangerous. It replaces `self.params` with a new `Arc`, while the host and the editor keep the `Arc` that `params()` handed out. If anything ever called it, the audio thread would then read a parameter set that neither the host nor the GUI can see or change (fable review 2026-07-07, finding **H8**).

## What Changes

- **Delete `get_state` and `load_state`.** The host's state save and restore, through nih-plug, is unchanged.
- **Keep the preset schema migration.** It lives in `Preset::from_json_str`, which presets still use.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
<!-- none: removes unreachable code; no behavior changes (`skip_specs: true`) -->

## Impact

- **Code:** `src/lib.rs` (two methods and their doc comments).
- **API:** two public methods are removed. Nothing in the repository calls them; `grep -rn "get_state\|load_state"` shows only their definitions.
- **Docs:** any doc comment or architecture note that mentions them, and the review's H8 status.

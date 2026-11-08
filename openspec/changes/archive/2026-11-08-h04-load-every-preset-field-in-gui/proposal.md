# Proposal

## Why

The editor loads a preset by setting each parameter from a hand-written list (`apply_preset` in `src/gui/editor.rs`). The list misses `play_mode`, `limiter_mode` and the four macros. A preset that stores Kit or Drone mode, the lookahead limiter, or non-neutral macros therefore loads only partly from the GUI: those six parameters keep whatever was live. The host's own state path restores them, so the same preset sounds different depending on how it was loaded (fable review 2026-07-07, finding **H4**).

## What Changes

- **One generic loop.** The editor applies a preset by walking the plugin's parameter map and setting every parameter the preset stores, by its ID, instead of a hand-written list. No parameter can be forgotten, including ones added later (H5, H6, H9 add some).
- **UI Scale is an editor setting, not part of a sound.** It is excluded on purpose, so loading a preset no longer resizes the window.
- **A test ties the editor's load path to the parameter map.** A newly added parameter can't silently be skipped.

## Capabilities

### New Capabilities
- `preset-loading`: what loading a preset sets, and that every load path agrees.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:** `src/gui/editor.rs` (`apply_preset`, plus a pure helper that lists the parameter values a preset sets); possibly a small accessor in `src/presets/mod.rs`.
- **Tests:** a unit test of the helper, which needs no GUI.
- **Docs:** the review's H4 status.

# Proposal

## Why

In Kit play mode, `note_to_kit_object` maps a MIDI note to one of the nine objects in 10-note slots starting at MIDI 24. Its doc comment promises the mapping "wraps so the player can still trigger every family at extreme registers". That's true above the block, but `.max(0)` clamps every note below 24 into slot 0 (Pipe). The bottom two octaves of the keyboard all play Pipe (fable review 2026-07-07, finding **H14**).

## What Changes

- **Notes below MIDI 24 wrap the same way notes above the block do.** Slot = `floor((note − 24) / 10) mod 9`, so MIDI 14–23 plays the ninth object, 4–13 the eighth, and 0–3 the seventh.
- **Notes from 24 up map exactly as today.**
- **A test pins both directions,** and the doc comment and README describe the wrap.

## Capabilities

### New Capabilities
- `kit-mode`: how Kit play mode maps MIDI notes to objects.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:** `src/lib.rs` (`note_to_kit_object`, one line, plus tests).
- **Sound:** Kit-mode notes below MIDI 24 change object. Factory presets that use Kit mode play whatever notes the user plays, and no preset file changes.
- **Docs:** the README's Kit-mode line, and the review's H14 status.

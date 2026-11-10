# Proposal

## Why

To hear a patch today, the maintainer needs a MIDI keyboard or a MIDI clip in the host. Sound design and listening checks, such as judging the diode knee, go faster if the editor can play notes itself. That means clicking keys on screen, or playing the middle octave on the computer keyboard.

## What Changes

- A piano keyboard strip at the very bottom of the editor, below every panel. It spans C3–C6 and can be played with the mouse: click to play, drag for a glissando, release to stop. Velocity comes from where the key is pressed, soft at the top and loud at the bottom.
- The computer keyboard plays the middle octave, C4 to C5, on the physical keys `A W S E D F T G Y H U J K` (white keys on the home row, black keys above). Keys are matched by position, so AZERTY and other layouts use the same physical keys. Each mapped on-screen key shows its letter.
- **Computer keys play only while the plugin editor is open and has keyboard focus.** They are ignored when the editor window is unfocused, while a text field such as the preset name is being edited, and for key auto-repeat. When the editor loses focus or closes, every note it is holding is released.
- Notes from the editor reach the engine through a preallocated, lock-free queue that the audio thread drains at the start of each block. They then follow the same path as host MIDI notes (play mode, sustain pedal, voice stealing, kit mode), on channel 0 and without a voice ID. Nothing on the audio path allocates.
- Keys light up while their note is held, whether the note comes from the editor or from the host's MIDI.
- The editor becomes taller by the height of the strip; the layout above it does not change.

No parameter, preset or host-state changes; no migration. Not BREAKING.

## Capabilities

### New Capabilities
- `editor-keyboard`: the editor's built-in keyboard. Covers the on-screen keys, the computer-key mapping and when it is active (editor open and focused), and how editor notes reach the voices in real time without stuck notes.

### Modified Capabilities
(none: the editor-controls requirements cover parameter controls, and the keyboard is not a parameter)

## Impact

- `src/gui/editor.rs`: the strip's layout and painting, mouse and key handling, focus tracking, a taller base editor height, and updates to the size tests (`editor_content_fits_the_window_width`, `print_editor_content_height`).
- New module, e.g. `src/gui_notes.rs` (no GUI dependency, so the CI lane can test it): the SPSC note queue and the held-note mask.
- `src/lib.rs`: `Corrosion` owns an `Arc` of the shared note state, passes it to `gui::create_editor` (like `DiodeMeter`), drains it in `process()` before host events, and publishes the held-key mask after each block.
- `tests/no_alloc.rs`: draining editor notes must not allocate.
- README (how to play from the editor, and the host setting some hosts need to pass keystrokes to plugins) and `docs/ARCHITECTURE.md`.
- No new dependencies.

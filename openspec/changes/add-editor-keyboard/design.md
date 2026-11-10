# Design

## Context

- The editor is egui 0.31 on baseview (`egui-baseview`), laid out in `render_ui` (`src/gui/editor.rs`). The base size is 1080×768, and the default UI scale is 75 %.
- The GUI and the audio thread already share state through an `Arc`: `Arc<dsp::DiodeMeter>`, passed to `gui::create_editor`. There is no queue crate among the dependencies.
- Host notes go through `handle_note_event(plugin, NoteEvent)` (`src/lib.rs`) inside `process_block`, which follows the host's event timing. Since M12, notes are addressed by `NoteAddress { voice_id, channel }`.
- baseview turns window focus changes into `egui::Event::WindowFocused(bool)` and sets `RawInput::focused`. Key events carry `physical_key: Option<Key>` and `repeat`.
- Hosts decide whether a plugin window gets keystrokes. REAPER, for one, forwards them only when the plugin window is focused, and may keep some for its own shortcuts.

## Goals / Non-Goals

**Goals:**
- Play any note from C3 to C6 with the mouse, and C4–C5 from the computer keyboard, with no stuck notes.
- Computer keys are active only while the editor is open and focused, and never while typing in a text field.
- Editor notes behave like host notes (play mode, sustain, stealing, kit mode).
- No allocation or locking on the audio thread; CI-testable logic outside the GUI feature.

**Non-Goals:**
- Octave shift or velocity keys on the computer keyboard (possible follow-up).
- Sending editor notes to the host's MIDI output; recording them in the host.
- Pitch-bend or mod wheels on the strip.
- Fighting hosts that keep keystrokes for themselves. That is documented, not worked around.

## Decisions

### 1. Strip at the bottom, the editor grows
The strip spans the full width under the bottom panel row, with a base height of about 64 px (48 px at 75 %), measured at implementation. `BASE_EDITOR_HEIGHT` grows by that amount, and the layout above it stays as it is.
- *Alternative:* shrink the panels to make room. Rejected: it changes a layout the maintainer has already checked by eye.
- The size tests (`editor_content_fits_the_window_width`, `print_editor_content_height`) are updated. The saved editor size grows with the new base height, the same way it does for the UI scale.

### 2. Range C3–C6, mapped octave C4–C5
37 keys (48–84) fit the width with white keys about 49 px wide at base scale. The mapped octave is the "middle octave" the maintainer asked for, plus its top C, on the usual DAW tracker layout: `A W S E D F T G Y H U J K`. The mapped keys show their letter in `MUTED_GREY`.

### 3. Physical keys, no repeats, fixed velocity
`Event::Key` is matched on `physical_key` (falling back to `key` when the backend gives no physical key), so the layout under AZERTY or Dvorak stays positional. `repeat == true` is ignored. Key velocity is fixed at 100/127. The mouse velocity maps the press height from 0.25 at the top of the key to 1.0 at the bottom.

### 4. Focus gate
A key event plays only when all three hold:
- the editor is open (the event reaches `render_ui` at all);
- the window is focused (the last `WindowFocused`, else `RawInput::focused`);
- `ctx.wants_keyboard_input()` is false.

On `WindowFocused(false)`, every note the editor holds is released (keys and mouse). On close, a `Drop` on the editor's note sender releases them and sets a `release_all` flag in the shared state, which the audio thread honours even if no GUI frame runs again. Releasing a key that was released by the focus loss sends nothing, because the GUI tracks what it holds.
- *Alternative:* listen to keys globally. Rejected: the maintainer asked for "only while the plugin is open and in focus", and a global hook would need OS-specific code.

### 5. Lock-free SPSC queue with a GUI-side backlog
A new module `src/gui_notes.rs` (no GUI dependency) holds `EditorNotes`:
- A fixed ring of 256 `AtomicU32` slots, each one event packed as on/off, note and velocity, with head and tail `AtomicUsize` indices. It is written by one GUI thread and read by one audio thread.
- `release_all: AtomicBool`.
- `held: [AtomicU64; 2]`, the 128-bit held-note mask that the audio thread publishes.

The GUI side wraps the producer in a `NoteSender` with a `VecDeque` backlog. The GUI may allocate, so when the ring is full, events wait there and are retried each frame. That way nothing is lost and order is kept. The audio side `drain(|event| ...)` reads without allocating.
- *Alternative:* a held-state bitmask diffed per block. Rejected: a press and release within one block would be lost.
- *Alternative:* a queue crate. Rejected: a few dozen lines of atomics don't justify a dependency.

### 6. Drained at block start, through `handle_note_event`
In `process()`, before `process_block`:
- if `release_all` is set, a note-off goes out for each note the editor had on;
- otherwise each drained event becomes a `NoteEvent::NoteOn` or `NoteOff` with `timing: 0`, `channel: 0`, `voice_id: None`, passed to `handle_note_event`.

The audio thread keeps its own 128-bit "editor has it on" mask, so the release-all only touches editor notes. Host notes on the same key aren't cut, beyond what a channel-0 note-off already does today.

### 7. Highlight from the voices
After each block, the audio thread ORs the keys of the held voices into `held` and stores it (relaxed). The strip reads it each frame and also asks egui for a repaint while any bit is set, so the highlight follows the release. The GUI's own pressed keys are drawn pressed at once, without waiting for the audio round trip.

## Risks / Trade-offs

- **Host keystroke capture.** Some hosts never pass keys to plugin windows, or keep them for shortcuts (for example REAPER's "send all keyboard input to plugin" option). Then only the mouse works. *Mitigation:* the README names the setting, and the mouse path does not depend on it.
- **Focus edge cases on X11 and Windows.** baseview starts with `focused: Some(true)`, before any focus event. Key events only arrive at a focused window, though, so the default does not let unfocused keys through. *Mitigation:* a test for the gate's logic, and a manual check in REAPER.
- **Hidden editor that is not closed.** Some hosts hide the window rather than destroying it. The focus loss still releases the notes.
- **Latency.** Editor notes start at the next block, one block of delay. That is fine for auditioning.
- **Taller window.** A taller window may not fit small screens at 100 %. The UI scale already covers that.

## Measurements

(to be filled during implementation: strip height, editor height at 75 %, no_alloc result, REAPER check)

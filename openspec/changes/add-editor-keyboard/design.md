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

> *As built (maintainer, 2026-11-10):* egui-baseview always sends `physical_key: None`, and its `key` comes from the layout's virtual key, so keys match by letter: on AZERTY the row is the same letters, not the same positions. The code still prefers `physical_key`, so it becomes positional if the backend ever reports it. Patching egui-baseview was the rejected alternative (a fork to maintain).
`Event::Key` is matched on `physical_key` (falling back to `key` when the backend gives no physical key), so the layout under AZERTY or Dvorak stays positional. `repeat == true` is ignored. Key velocity is fixed at 100/127. The mouse velocity maps the press height from 0.25 at the top of the key to 1.0 at the bottom.

### 4. Focus gate

> *As built (maintainer, 2026-11-10):* baseview emits `Focused`/`Unfocused` only on macOS, so on Windows and X11 focus loss never reaches the editor, and a key held while focus moves away stuck its note in REAPER. The gate also requires the pointer over the editor (`PointerState::has_pointer`). Its leaving (`PointerGone`, which baseview reports on every platform) releases the strip's notes like a focus loss.
> - On that release, the mapped keys are also removed from egui's `keys_down`. egui never clears that set, and it marks a press of a key it still thinks is down as a repeat, so after a lost key-up the next press would be ignored.
> - Consequence: a key still held when the pointer comes back plays again on its next auto-repeat.
> - Rejected alternative: forking baseview to report focus on Windows and X11.
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

- **Editor size.** The panels already scrolled before this change: their content is 1115–1330 points tall at 100 % in a 768-point window (`print_editor_content_height`, all exciter × object combinations). A strip at the end of the content would sit out of view.
  - *As built:* the strip is pinned under the panels' scroll view, outside it (`render_window_body`). The window grows by the strip (64 points plus an 8-point gap): 1080×840 at 100 %, 810×630 at 75 %, 540×420 at 50 %.
  - `height_pct` keeps the old 768-point layout height (`BASE_LAYOUT_HEIGHT`), so nothing above the strip moves.
  - The footer now claims its own space, so the content height grew 93 points: 72 for the strip, the rest the footer.
- **Closing the editor (*As built*, replaces decision 4's `Drop`).** nih-plug-egui keeps the editor state between openings, and the handle's `Drop` only clears `EguiState::open`, so a `Drop` on the sender never fires on close. Instead, the audio thread watches `editor_state.is_open()` once per block. On the open → closed transition it releases every note the editor started (`EditorNoteState::observe_editor_open`). The strip clears its holders on reopen (the editor's build callback).
- **Tests.**
  - 18 `gui_notes` unit tests and 5 engine tests (CI lane), 11 strip tests (GUI lane), and `editor_note_drain_is_alloc_free`.
  - Mutations, each caught: dropping the repeat filter (caught after adding the focus-loss repeat test), dropping the focus gate, dropping the text-field gate, no release on focus loss, no release-all on close, white keys before black in the hit test, flat velocity, and an allocation in `drain` (aborts the no_alloc test).
  - An editor note-on renders bit-identically to a host note-on on channel 0 (five 256-sample blocks).
- **REAPER (Linux, X11, 75 %).**
  - The strip shows at the bottom with its letters.
  - With the editor focused, holding A lights C4, and releasing it turns it off.
  - With REAPER's main window focused, S does nothing.
- **Found in REAPER: focus loss is never reported.** baseview emits `Focused`/`Unfocused` only on macOS; on X11 and Windows it never does, and egui-baseview never fills `physical_key`.
  - Holding D and moving focus to REAPER's main window leaves C4–D stuck: the key-up goes to the other window. Pressing and releasing D again frees it.
  - Typing on AZERTY follows the letters, not the positions.
  - The maintainer chose the pointer gate and letter matching (decisions 3 and 4, *As built*). With the pointer gate: 2 more strip tests, both red without the gate.
- **REAPER again, with the pointer gate (Linux, X11, 75 %).**
  - Holding D lights it. Moving the pointer out, focusing the main window and releasing D there releases the note. Back over the plugin, pressing D plays and releasing stops it. The stuck note is gone.
  - Found and fixed on the way:
    - egui's `keys_down` kept D after the lost key-up, so the next press came in as a repeat and was ignored. The strip now forgets the mapped keys on release.
    - X11 auto-repeat arrives as key-up/key-down pairs that baseview doesn't flag. Half a second of a held D gave 6 press/release pairs, a retrigger at the repeat rate. `key_actions` coalesces a same-frame pair into one repeat; the log then showed one press, repeats, one release.
  - A focused panel control does not block the keys: in this egui version only the preset-name field takes focus on click (probed on the SCALE, PLAY, LIMITER and MODE combos and the Save button: no focus).
  - Keys typed while REAPER's main window had focus went to REAPER's shortcuts (D opened its split dialog). That is the host's behaviour, not the plugin's.
  - Capture: the whole editor at 75 % with F♯4 held by the mouse.
- **Strip tests.** 14 in the GUI lane (pointer gate, pointer leave, X11 pairs), each red against its mutation. In all: 18 `gui_notes` tests, 5 engine tests, 14 strip tests and `editor_note_drain_is_alloc_free`.

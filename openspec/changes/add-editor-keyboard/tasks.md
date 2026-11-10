# Tasks

## 1. Tests first

- [x] 1.1 `gui_notes` unit tests (CI lane): push/drain order, a full ring with a backlog loses nothing and keeps order, `release_all` releases only editor-held notes, the held mask round-trips; verify they fail (or don't compile) before the module exists
- [x] 1.2 Key-mapping tests: physical A W S E D F T G Y H U J K → 60..72; unmapped keys send nothing; repeats are ignored (one on, one off)
- [x] 1.3 Focus-gate tests: unfocused window sends nothing; `wants_keyboard_input` (preset name field focused, typing "aws") sends nothing; `WindowFocused(false)` releases a key-held and a mouse-held note, and a later key-up sends nothing; dropping the sender sets `release_all`
- [x] 1.4 Strip tests (GUI lane): bottom-most element at 75 %; 37 keys from 48 to 84 with black keys on C♯ D♯ F♯ G♯ A♯; click/release, drag 60→62, velocity bottom > top, black over white at the overlap
- [x] 1.5 Engine tests: an editor note-on 60 activates a voice like a host note on channel 0; on/off/on in one block leaves only 62 held; host note-on 64 appears in the published mask and leaves it after note-off
- [x] 1.6 `tests/no_alloc.rs`: draining editor note-ons and note-offs (and a `release_all`) does not allocate

## 2. Shared note state

- [x] 2.1 `src/gui_notes.rs`: `EditorNotes` (256-slot `AtomicU32` ring, `release_all`, `held` mask), packing helpers, `drain`
- [x] 2.2 `NoteSender` (GUI side): `VecDeque` backlog retried each frame, tracks what it holds, `release_all_held()`, `Drop` sets `release_all`

## 3. Engine

- [x] 3.1 `Corrosion` owns `Arc<EditorNotes>`; `editor()` passes it to `gui::create_editor`
- [x] 3.2 `process()`: honour `release_all`, drain events into `handle_note_event` (timing 0, channel 0, no voice ID) before host events; keep the audio-side editor-on mask
- [x] 3.3 Publish the held-voice mask after each block

## 4. Editor

- [x] 4.1 Grow `BASE_EDITOR_HEIGHT` by the strip; lay out and paint the strip (white/black keys, letters on 60..72, pressed and held highlights), scaling with the UI
- [x] 4.2 Mouse: press, drag glissando, release, velocity by height, black-key precedence
- [x] 4.3 Computer keys: physical-key mapping (letter fallback; *As built*), repeat filter, focus gate plus the pointer over the editor, release on focus loss and pointer leave
- [x] 4.4 Request repaints while any key is held or lit
- [x] 4.5 Update the editor size tests and `print_editor_content_height`

## 5. Docs and close-out

- [x] 5.1 Record measurements in `design.md` (strip and editor height at 75 %, no_alloc, anything that differed: *As built* notes)
- [x] 5.2 Check in REAPER at 75 %: mouse plays, keys play only while the editor is focused, nothing sticks when focus leaves or the window closes; screenshot
- [x] 5.3 README (how to play from the editor; the host keyboard-forwarding setting) and `docs/ARCHITECTURE.md` (the note queue and where it is drained)
- [x] 5.4 `cargo fmt --check`, clippy `-D warnings` (CI lane; no new GUI-lane warnings), full tests on gnu and musl, GUI lane, no_alloc, `cargo doc`
- [x] 5.5 Update CLAUDE.md "Current state"; commit and push to `dev`

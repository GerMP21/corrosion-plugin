# editor-keyboard Specification

## Purpose
Lets the user play and audition the instrument from the editor itself, with the mouse on an on-screen keyboard or with the computer keyboard for the middle octave, without a MIDI controller or host clip.

## Requirements

### Requirement: On-screen keyboard strip
The editor SHALL draw a piano keyboard strip along its bottom edge, below every other panel and spanning the editor's width, covering MIDI notes 36 (C2) to 84 (C6). The strip SHALL scale with the UI scale, and the editor content including the strip SHALL fit the window at every supported UI scale.

#### Scenario: The strip is the bottom-most element
- **WHEN** the editor is laid out at the default UI scale
- **THEN** the keyboard strip's top edge is at or below the bottom edge of every other panel, and its bottom edge is inside the window

#### Scenario: The strip covers C3 to C6
- **WHEN** the keyboard strip is laid out
- **THEN** it has one key for every MIDI note from 48 to 84 inclusive (the range before kits, still covered)

#### Scenario: The strip covers C2 to C6
- **WHEN** the keyboard strip is laid out
- **THEN** it has one key for every MIDI note from 36 to 84 inclusive, and black keys sit on the notes whose pitch class is C♯, D♯, F♯, G♯ or A♯

### Requirement: Mouse playing
Pressing the primary mouse button on a key SHALL start that key's note, and releasing the button SHALL stop it. Dragging across keys with the button held SHALL stop the previous key's note and start the new key's note. The note-on velocity SHALL rise from the top of the key to its bottom. A black key SHALL take precedence over the white keys under it.

#### Scenario: Click and release
- **WHEN** the user presses the mouse on the key for note 60 and then releases it
- **THEN** the editor sends a note-on for 60 followed by a note-off for 60

#### Scenario: Drag across keys
- **WHEN** the user presses on note 60 and drags onto note 62 without releasing
- **THEN** the editor sends note-off 60 and then note-on 62, and releasing sends note-off 62

#### Scenario: Velocity follows the press position
- **WHEN** the same key is pressed once near its top and once near its bottom
- **THEN** the press near the bottom has the higher velocity

#### Scenario: Black key over white key
- **WHEN** the user presses where the black key for note 61 overlaps the white key for note 60
- **THEN** the note started is 61

### Requirement: Computer keys play the middle octave
While the editor plays from the computer keyboard, the keys A, W, S, E, D, F, T, G, Y, H, U, J and K SHALL play MIDI notes 60 to 72 in that order, or MIDI notes 36 to 48 while Note Mode is Kit. Keys SHALL be matched by the physical key when the window system reports one, and otherwise by the key's letter in the active layout. Pressing a mapped key SHALL send a note-on at a fixed velocity, releasing it SHALL send a note-off, and auto-repeat key events SHALL be ignored. Each mapped on-screen key SHALL show its key letter.

#### Scenario: The home row plays C4 to C5
- **WHEN** the keys A, W, S, E, D, F, T, G, Y, H, U, J, K are pressed in turn
- **THEN** the editor sends note-ons for 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72

#### Scenario: In Kit mode the home row plays the kit zone
- **WHEN** Note Mode is Kit and the keys A, W, S, …, K are pressed in turn
- **THEN** the editor sends note-ons for 36 to 48, and the letters are drawn on those keys

#### Scenario: Auto-repeat does not retrigger
- **WHEN** a mapped key is held and the window system sends repeat events for it
- **THEN** exactly one note-on is sent, followed by one note-off when the key is released

#### Scenario: Auto-repeat as key-up and key-down pairs
- **WHEN** a mapped key is held and the window system reports its auto-repeat as a key-up immediately followed by a key-down of the same key in one frame, without marking it as a repeat
- **THEN** the pair is treated as a repeat: no note-off and no new note-on is sent

#### Scenario: Unmapped keys do nothing
- **WHEN** a key outside the mapping is pressed
- **THEN** no note event is sent

### Requirement: Computer keys only while the editor is open and focused
Computer keys SHALL play notes only while the plugin editor is open, its window has keyboard focus, and the pointer is over the editor; and not while a text field in the editor (such as the preset name) has keyboard input. When the editor window loses focus, the pointer leaves the editor, or the editor closes, every note held by the computer keys or the mouse SHALL be released.

#### Scenario: Unfocused editor ignores keys
- **WHEN** the editor window does not have focus and a mapped key event arrives
- **THEN** no note event is sent

#### Scenario: Typing a preset name does not play
- **WHEN** the preset name field has keyboard focus and the user types "aws"
- **THEN** no note event is sent

#### Scenario: Keys need the pointer over the editor
- **WHEN** the editor window has focus but the pointer is outside the editor, and a mapped key is pressed
- **THEN** no note event is sent, and once the pointer is back over the editor the same key plays

#### Scenario: The pointer leaving releases held notes
- **WHEN** a mapped key is sounding and the pointer leaves the editor, and the key-up goes to another window
- **THEN** a note-off is sent when the pointer leaves, and pressing the key again after the pointer returns plays the note

#### Scenario: Losing focus releases held notes
- **WHEN** a mapped key and a mouse-held key are both sounding and the editor window loses focus
- **THEN** note-offs are sent for both notes, and releasing the key afterwards sends nothing more

#### Scenario: Closing the editor releases held notes
- **WHEN** the editor closes while it is holding notes
- **THEN** the audio thread receives note-offs for every one of them

### Requirement: Editor notes play like host notes
The audio thread SHALL handle each editor note event exactly as a host note on channel 0 with no voice ID, at the block's first sample and before the block's host events, so play mode, sustain pedal, voice stealing and kit mode apply.

#### Scenario: An editor note plays a voice
- **WHEN** the editor sends note-on 60 and the next block is processed
- **THEN** a voice is active for note 60, exactly as when the host sends note-on 60 on channel 0

### Requirement: Editor notes cross threads without loss or allocation
Notes from the editor SHALL pass from the GUI thread to the audio thread through a preallocated, lock-free single-producer, single-consumer queue that the audio thread drains at the start of each block. Events SHALL keep their order. When the queue is full the editor SHALL hold the remaining events and send them on a later frame, so none is lost. Draining the queue SHALL NOT allocate.

#### Scenario: Order is kept
- **WHEN** the editor sends note-on 60, note-off 60 and note-on 62 before one block
- **THEN** the block handles them in that order and only note 62 is held afterwards

#### Scenario: A full queue loses nothing
- **WHEN** the editor sends more events than the queue holds before the audio thread drains it
- **THEN** every event is eventually handled once, in order

#### Scenario: Draining does not allocate
- **WHEN** the audio thread drains editor note-ons and note-offs under the allocation checker
- **THEN** no allocation occurs

### Requirement: Held notes light up
Each on-screen key SHALL be drawn highlighted while its note is held, whether the note comes from the editor or from host MIDI. The audio thread SHALL publish the held notes lock-free after each block.

#### Scenario: Host MIDI lights a key
- **WHEN** the host sends note-on 64 and the block is processed
- **THEN** the published held-note state includes 64, and after the note-off it does not

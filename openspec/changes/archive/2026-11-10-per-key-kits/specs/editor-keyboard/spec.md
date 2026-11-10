# Spec Delta

## MODIFIED Requirements

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

### Requirement: Computer keys play the middle octave
While the editor plays from the computer keyboard, the keys A, W, S, E, D, F, T, G, Y, H, U, J and K SHALL play MIDI notes 60 to 72 in that order, or MIDI notes 36 to 48 while Play Mode is Kit. Keys SHALL be matched by the physical key when the window system reports one, and otherwise by the key's letter in the active layout. Pressing a mapped key SHALL send a note-on at a fixed velocity, releasing it SHALL send a note-off, and auto-repeat key events SHALL be ignored. Each mapped on-screen key SHALL show its key letter.

#### Scenario: The home row plays C4 to C5
- **WHEN** the keys A, W, S, E, D, F, T, G, Y, H, U, J, K are pressed in turn
- **THEN** the editor sends note-ons for 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72

#### Scenario: In Kit mode the home row plays the kit zone
- **WHEN** Play Mode is Kit and the keys A, W, S, …, K are pressed in turn
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

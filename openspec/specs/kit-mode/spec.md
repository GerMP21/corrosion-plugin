# kit-mode Specification

## Purpose
Defines Kit play mode: each key plays its own stored pad (a whole voice-level sound with a tune and a level), assigned from the editor's kit bar, saved with presets and projects, through a post chain shared by the kit (`per-key-kits`).

## Requirements

### Requirement: Each key plays its own pad
In Kit mode, a note on a key that has a pad SHALL start a voice from the pad's stored sound: its exciter, its object and every voice-level parameter. It SHALL play at the pad's pitch, the key's pitch plus the pad's tune in semitones, whatever the live parameters are. A note on a key without a pad SHALL produce no sound. The post chain SHALL stay shared by all voices.

#### Scenario: A pad plays its own sound
- **WHEN** key 36 holds a pad stored from a Felt Mallet on Tank and key 38 a pad stored from a Drumstick on Sheet Metal, and the live parameters select Bow on Pipe
- **THEN** note 36 renders exactly like a Felt Mallet on Tank note with the pad's parameters at its pitch, and note 38 like the Drumstick on Sheet Metal one

#### Scenario: Keys without a pad are silent
- **WHEN** Kit mode is on and a note arrives on a key without a pad
- **THEN** no voice starts

#### Scenario: Pads keep their pitch
- **WHEN** a pad on key 42 has a tune of +24 semitones
- **THEN** note 42 plays at the pitch of MIDI 66

#### Scenario: Live controls do not change pad voices
- **WHEN** a pad voice is sounding and the user moves a live voice control (for example Brightness or Damping)
- **THEN** the pad voice keeps its stored values

### Requirement: Pads are assigned from the editor
In Kit mode the editor SHALL show a kit bar with the selected key, Assign, Clear and Tune. Clicking a key on the strip SHALL select it, as well as play it. Assign SHALL store the editor's current sound as the selected key's pad. Its tune SHALL be the difference between the note the sound was last auditioned on in the editor and the key, or 0 if none. Clear SHALL remove the pad. Selecting a key that has a pad SHALL load the pad's sound into the editor's parameters.

#### Scenario: Assign the current sound
- **WHEN** the user plays the current sound on MIDI 72, selects key 42 and presses Assign
- **THEN** key 42 holds a pad with the current sound and a tune of +30, and note 42 sounds at the pitch the sound was auditioned at

#### Scenario: Clear a pad
- **WHEN** the user selects a key with a pad and presses Clear
- **THEN** the key has no pad and is silent in Kit mode

#### Scenario: Selecting a pad loads it
- **WHEN** the user selects a key whose pad was stored from Drumstick on Sheet Metal
- **THEN** the editor's parameters show the pad's sound, and Assign without changes leaves the pad as it was

### Requirement: The kit zone
In Kit mode the strip SHALL grey out the keys outside MIDI 36–59, and they SHALL neither play nor be selectable from the editor, until the Full keyboard toggle is on. With the toggle on, every key on the strip SHALL play and be selectable. Host MIDI SHALL play any key that has a pad, regardless of the toggle.

#### Scenario: Keys outside the zone are locked
- **WHEN** Kit mode is on, the Full keyboard toggle is off, and the user clicks MIDI 60 on the strip
- **THEN** the key is drawn greyed out, no note is sent and the selection does not change

#### Scenario: The toggle unlocks the keyboard
- **WHEN** the user turns Full keyboard on and clicks MIDI 60
- **THEN** MIDI 60 plays and becomes the selected key

### Requirement: Kits are saved
A kit SHALL be saved with a preset (an optional `kit` field, absent when there are no pads) and in the plugin's host state, and loading either SHALL restore every pad exactly. Loading a preset without a `kit` field SHALL leave no pads.

#### Scenario: Preset round trip
- **WHEN** a kit with pads on keys 36, 38 and 42 is saved as a preset and loaded back
- **THEN** the three pads are restored with identical sounds and tunes, and no other key has a pad

#### Scenario: Project round trip
- **WHEN** the host saves and restores the plugin state with a kit
- **THEN** the kit is restored exactly

### Requirement: Old Kit state loads as Tonal
A preset file or host state with Play Mode Kit and no kit SHALL load with Play Mode Tonal and its other values unchanged.

#### Scenario: Old Kit preset
- **WHEN** a preset saved before this change with Play Mode Kit is loaded
- **THEN** Play Mode is Tonal and every other parameter keeps its value

#### Scenario: Old Kit project
- **WHEN** the host restores state saved before this change with Play Mode Kit
- **THEN** Play Mode is Tonal

### Requirement: Pads are real-time safe
Starting a voice from a pad, and assigning or clearing pads while notes play, SHALL NOT allocate, free or lock on the audio thread.

#### Scenario: Pad note-ons in a checked zone
- **WHEN** the audio thread starts voices from pads, under the allocation checker, while the editor assigns a pad
- **THEN** no allocation occurs on the audio thread

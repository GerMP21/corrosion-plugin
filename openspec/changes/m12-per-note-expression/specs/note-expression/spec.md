# Spec Delta

## Purpose

Defines how the plugin addresses individual notes from the host: voice IDs on note events, per-note tuning, per-note pressure and chokes.

## ADDED Requirements

### Requirement: Notes are addressed by voice ID
When a note event carries a voice ID, the plugin SHALL apply it to the voice started with that ID; when it carries none, or no voice has that ID, it SHALL apply it to the matching key and channel.

#### Scenario: Overlapping notes on one key
- **WHEN** two notes on the same key start with voice IDs 1 and 2, and a note-off arrives for ID 2
- **THEN** the voice with ID 2 is released and the voice with ID 1 keeps sounding

#### Scenario: Plain MIDI host
- **WHEN** notes arrive without voice IDs
- **THEN** note-offs release voices as before: the oldest held voice on that key

### Requirement: Per-note tuning retunes one note
A per-note tuning event SHALL shift only the addressed note's pitch by the given number of semitones, on top of the channel pitch bend.

#### Scenario: One note of a chord bent
- **WHEN** a three-note chord is held and a tuning of +1 semitone arrives for one of its voice IDs
- **THEN** that note's modes rise by one semitone and the other two notes are unchanged

#### Scenario: Tuning plus channel bend
- **WHEN** a note has a per-note tuning of +1 semitone and the channel bend is +2 semitones
- **THEN** the note sounds 3 semitones above its base pitch

### Requirement: Per-note pressure follows voice IDs
A per-note pressure event SHALL reach only the addressed voice.

#### Scenario: Pressure on one of two same-key notes
- **WHEN** two notes on one key have voice IDs 1 and 2 and pressure arrives for ID 1
- **THEN** only the voice with ID 1 responds

### Requirement: A choke stops a note quickly without a click
A choke event SHALL fade the addressed voice, or every voice on the given key and channel when no voice ID matches, to silence within 2 ms, and the voice SHALL then be free.

#### Scenario: Choked ringing note
- **WHEN** a ringing Pipe note is choked
- **THEN** its output reaches zero within 2 ms with no sample-to-sample step larger than the largest step in the 2 ms before the choke, and the voice is idle afterwards

### Requirement: Real-time safe note expression
Handling voice-addressed note-offs, tuning, pressure and chokes SHALL NOT allocate or free heap memory on the audio thread.

#### Scenario: Expression events in a checked zone
- **WHEN** inside an allocation-checking zone, notes with voice IDs receive tuning, pressure, note-offs and chokes
- **THEN** no heap allocation or deallocation is recorded

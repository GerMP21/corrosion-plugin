# Spec Delta

## Purpose

Defines how Kit play mode assigns MIDI notes to resonator objects across the whole keyboard.

## ADDED Requirements

### Requirement: Notes map to objects in repeating 10-note slots
In Kit mode, MIDI note `n` SHALL play object `floor((n − 24) / 10) mod 9`, with the floor taken toward negative infinity, so the nine objects repeat in 10-note slots across the whole MIDI range, both above and below MIDI 24.

#### Scenario: Inside the main block
- **WHEN** MIDI notes 24, 33, 34 and 113 are played in Kit mode
- **THEN** they play objects 0, 0, 1 and 8 respectively

#### Scenario: Above the block
- **WHEN** MIDI notes 114 and 127 are played in Kit mode
- **THEN** they play objects 0 and 1

#### Scenario: Below the block
- **WHEN** MIDI notes 23, 14, 13, 4, 3 and 0 are played in Kit mode
- **THEN** they play objects 8, 8, 7, 7, 6 and 6 respectively

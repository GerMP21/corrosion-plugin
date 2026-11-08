# Spec Delta

## Purpose

Defines where an exciter strikes the resonator: the base strike position, how wander moves it, and how the note's force envelope moves it during the note.

## ADDED Requirements

### Requirement: The force envelope moves the strike position
With Position Envelope above 0, the effective strike position SHALL be `strike position + Position Envelope × force envelope × 0.5`, clamped to 0..1, so it moves with the note's force envelope.

#### Scenario: Envelope moves the position
- **WHEN** a Bow note is played with Strike Position 0.2, Position Envelope 1 and Wander 0, and the force envelope rises from 0 to 1
- **THEN** the effective strike position rises from 0.2 to 0.7

#### Scenario: Audible effect without wander
- **WHEN** the same note is rendered with Wander 0 at Position Envelope 0 and at Position Envelope 1
- **THEN** the two outputs differ

#### Scenario: Zero is neutral
- **WHEN** a note is rendered at Position Envelope 0
- **THEN** its output is identical to the output before this requirement existed, at the same Wander and Wander Rate

### Requirement: Wander has its own rate
The position wander's rate SHALL be set by Wander Rate (0.1–5 Hz), independent of Position Envelope.

#### Scenario: Rate follows Wander Rate only
- **WHEN** Wander is 0.3 and Wander Rate is 2 Hz, with Position Envelope at 0 and then at 1
- **THEN** the wander component of the strike position has a 2 Hz period in both cases

### Requirement: Older presets keep their wander
Loading a preset, or host state saved in a DAW project, that predates Wander Rate SHALL give a wander rate equal to the one it produced before.

#### Scenario: Migrated rate
- **WHEN** a preset without Wander Rate, storing Position Envelope 0.5, is loaded
- **THEN** Wander Rate is 2.55 Hz (0.1 + 0.5 × 4.9) and Position Envelope is 0

#### Scenario: Migrated host state
- **WHEN** a DAW project saved before Wander Rate existed, storing Position Envelope 0.5, is reopened
- **THEN** Wander Rate is 2.55 Hz and Position Envelope is 0

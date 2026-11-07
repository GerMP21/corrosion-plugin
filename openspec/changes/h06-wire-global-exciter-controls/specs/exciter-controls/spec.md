# Spec Delta

## Purpose

Defines the global exciter controls (Pressure, Speed, Roughness): what each does to every exciter, where they are neutral, and the limits they keep to.

## ADDED Requirements

### Requirement: Defaults are neutral
At Pressure 0.5, Speed 0.5 and Roughness 0.3, every exciter SHALL produce exactly the output it produced before these controls were connected.

#### Scenario: Default controls unchanged
- **WHEN** each exciter (1–16) plays Pipe C4 at velocity 100 for one second held and one released at the default Pressure, Speed and Roughness
- **THEN** the output is bit-identical to the same note rendered with the controls ignored

### Requirement: Pressure scales the exciter force
Pressure SHALL scale every exciter's force by a factor that rises monotonically from −12 dB at 0 to +12 dB at 1, and SHALL take effect on held notes.

#### Scenario: Quieter and louder
- **WHEN** a HandStrike and a Bow note on Pipe C4 are each played at Pressure 0.25, 0.5 and 0.75
- **THEN** each exciter's pre-clamp peak rises from 0.25 to 0.5 to 0.75, or the higher setting is already held back by the drive governor

#### Scenario: Automated during a held note
- **WHEN** Pressure is moved from 0.5 to 0.1 while a Bow note is held
- **THEN** the note's level falls within 50 ms of the change

### Requirement: Speed and Roughness shape the exciters they apply to
Speed SHALL scale each exciter's motion-speed parameters, and Roughness SHALL scale each exciter's grit and randomness parameters, at note-on. Neither SHALL push a dedicated parameter outside its own allowed range.

#### Scenario: Speed changes a friction exciter
- **WHEN** a Bow note on Pipe C4 is played at Speed 0.25 and at 0.75
- **THEN** the two outputs differ

#### Scenario: Roughness changes a noisy exciter
- **WHEN** a HeavyGrinding note on Pipe C4 is played with the same seed at Roughness 0 and at 1
- **THEN** the two outputs differ

#### Scenario: Extremes stay in range
- **WHEN** any exciter's note starts with Speed and Roughness each at 0 and at 1
- **THEN** every scaled dedicated parameter lies within that parameter's allowed range

### Requirement: No setting overdrives a voice
At any Pressure, Speed and Roughness setting, a voice's signal before its safety clamp SHALL stay within ±1.0.

#### Scenario: Maximum settings
- **WHEN** each exciter plays every object at C2, C4 and C6, velocity 127, with Pressure, Speed and Roughness all at 1
- **THEN** no pre-clamp sample exceeds ±1.0

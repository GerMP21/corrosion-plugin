# Spec Delta

## Purpose

Defines how the multi-stage envelope used by friction-family exciters sequences its stages and loops, how velocity scales its level, and guarantees that every host-reachable configuration is safe to run on the audio thread.

## ADDED Requirements

### Requirement: Zero stage level is silence
When the envelope's current stage level is 0, its output SHALL be exactly 0 for every velocity and every `velocity_to_level` setting.

#### Scenario: Low velocity at zero level
- **WHEN** an envelope with sustain level 0 and `velocity_to_level` 1 is triggered at velocity 0 and held past its decay
- **THEN** every output sample during sustain is 0.0

#### Scenario: Velocity-to-level off
- **WHEN** `velocity_to_level` is 0 and the envelope is held at a sustain level of 0.5
- **THEN** the sustain output is 0.5 (no added offset), for any velocity

### Requirement: Velocity scales level multiplicatively
The output SHALL equal `level · ((1 − velocity_to_level) + velocity_to_level · velocity_scale) · env_amount`, where `velocity_scale = 0.5 + velocity / 2` for normalized velocity in [0, 1].

#### Scenario: Half-scale at velocity zero
- **WHEN** `velocity_to_level` is 1, the velocity is 0, and the current level is 1.0
- **THEN** the output is 0.5 · env_amount

#### Scenario: Full velocity gives the stage level
- **WHEN** the velocity is 1.0
- **THEN** the output equals level · env_amount exactly, for any `velocity_to_level` and `env_amount`

### Requirement: Release ends without a step
The envelope SHALL reach Idle from Release without a discontinuity: the last Release sample and the first Idle sample differ by no more than the largest single-sample change during that Release.

#### Scenario: Release end at low velocity
- **WHEN** a held envelope at velocity 0.3 is released and processed until it goes Idle
- **THEN** the step from the last Release sample to the first Idle sample (0.0) is no larger than the largest sample-to-sample change within the Release stage

### Requirement: Release starts from the current level
Release SHALL ramp from the level the envelope had when the gate closed, so releasing before Sustain is reached does not jump.

#### Scenario: Release during Attack or Decay
- **WHEN** the gate closes during Attack or Decay, with the sustain level well below the current level
- **THEN** the first Release sample differs from the last pre-release sample by no more than the largest single-sample change before the release

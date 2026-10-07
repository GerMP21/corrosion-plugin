# Spec Delta

## Purpose

Guarantees that a note, its tail and the post-chain body sound the same whichever sample rate the host runs at, so a session doesn't change character when its sample rate changes.

## ADDED Requirements

### Requirement: Notes sound the same at every sample rate
For every exciter, a note SHALL render at 44.1 and 96 kHz with the same level as at 48 kHz.

#### Scenario: Every exciter on a Pipe
- **WHEN** each exciter (1–16) plays Pipe C4 at velocity 100 with default controls for one second, then releases for one second, at 44.1, 48 and 96 kHz
- **THEN** the voice output's RMS at 44.1 and 96 kHz is within 1.5 dB of its 48 kHz RMS

#### Scenario: Attack timing of struck exciters
- **WHEN** the HandStrike, FeltMallet, HardMallet and Drumstick exciters play Pipe C4 at 44.1, 48 and 96 kHz
- **THEN** the time from note-on to the output's peak is within 10 % of the 48 kHz time

### Requirement: Tails end at the same time at every sample rate
A released voice SHALL stop sounding after the same amount of time at every sample rate.

#### Scenario: Struck Pipe tail
- **WHEN** a HandStrike Pipe C4 note is released after 0.1 s, at 44.1, 48 and 96 kHz
- **THEN** the time until the voice stops sounding is within 10 % of the 48 kHz time

### Requirement: The body sounds the same at every sample rate
The post-chain body resonance SHALL have the same level and ring time at every sample rate.

#### Scenario: Body impulse response
- **WHEN** the FEM body at default material and chassis volume is excited by the same 1 ms pulse at 44.1, 48 and 96 kHz
- **THEN** its peak is within 1 dB, and its time to fall 40 dB within 10 %, of the 48 kHz response

### Requirement: 48 kHz output is unchanged
At 48 kHz every affected stage SHALL produce output bit-identical to before.

#### Scenario: Existing 48 kHz tests
- **WHEN** the existing 48 kHz test suite runs, including golden fixtures and the factory-preset level checks
- **THEN** it passes unchanged

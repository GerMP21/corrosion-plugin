# Spec Delta

## Purpose

Defines how strongly the modal resonator responds to excitation, so that a note's loudness depends on the sound being played and not on incidental settings such as the host sample rate.

## ADDED Requirements

### Requirement: Resonator level does not depend on the sample rate
The same force applied over the same time SHALL produce the same resonator output level at every supported sample rate (44.1, 48, 88.2, 96 and 192 kHz).

#### Scenario: Short strike
- **WHEN** a 1 ms unit force pulse excites each object's resonator at C4 with default controls, at 44.1, 48, 96 and 192 kHz
- **THEN** each object's peak output at every rate is within 0.5 dB of its 48 kHz peak

#### Scenario: Sustained excitation
- **WHEN** a sine force at the resonator's lowest mode frequency drives it for one second at 44.1, 48, 96 and 192 kHz
- **THEN** the steady-state output RMS at every rate is within 0.5 dB of the 48 kHz RMS

### Requirement: 48 kHz output is unchanged
At 48 kHz the resonator output SHALL be bit-identical to the output before this requirement existed.

#### Scenario: Golden mode banks and renders
- **WHEN** the existing 48 kHz tests run, including the golden mode-bank fixture and the factory-preset level check
- **THEN** they pass unchanged

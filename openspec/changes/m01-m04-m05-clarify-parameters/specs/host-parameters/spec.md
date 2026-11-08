# Spec Delta

## Purpose

Defines how the plugin's parameters present themselves to the host: names that can't be confused, units and value display, and controls whose taper puts their useful range within reach.

## ADDED Requirements

### Requirement: Parameter names are unique
Every parameter the plugin exposes to the host SHALL have a display name that no other parameter shares.

#### Scenario: Host automation list
- **WHEN** the display names of all exposed parameters are listed
- **THEN** no name appears twice

### Requirement: Output is shown and entered in decibels
The Output parameter SHALL display its value in decibels and SHALL accept typed values in decibels, showing minus infinity at zero gain.

#### Scenario: Display
- **WHEN** Output is at gain 1, 0.5 and 0
- **THEN** it displays as about 0 dB, about −6 dB and −inf dB respectively

#### Scenario: Typed value
- **WHEN** the user types "-6" into Output
- **THEN** Output is set to a gain of about 0.501

### Requirement: Output's taper reaches the useful range
The Output control SHALL place 0 dB between 50 % and 65 % of its travel and SHALL spread −20 to +6 dB over at least 15 % of its travel, while keeping its maximum at +40 dB.

#### Scenario: Knob positions
- **WHEN** the normalized positions of −20 dB, 0 dB, +6 dB and the maximum are computed
- **THEN** 0 dB lies between 0.50 and 0.65, the −20 to +6 dB span covers at least 0.15, and the maximum is +40 dB at position 1

### Requirement: Stored Output values keep their level
A preset or saved project SHALL play at the same output gain after the taper change as before it.

#### Scenario: Old project
- **WHEN** a project saved with Output at gain 2.0 is restored
- **THEN** Output's gain is 2.0 (about +6 dB)

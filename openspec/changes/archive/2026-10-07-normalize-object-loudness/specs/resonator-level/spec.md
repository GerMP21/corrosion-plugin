# Spec Delta

## ADDED Requirements

### Requirement: Loudness does not depend on pitch
A struck note's resonator level SHALL stay within 3 dB of the same object's C4 level from C2 to C6.

#### Scenario: Keyboard sweep
- **WHEN** each object's resonator is excited by the same one-sample unit force impulse at 48 kHz at C2, C4 and C6 with default controls
- **THEN** every peak is within 3 dB of that object's C4 peak

### Requirement: Objects are comparably loud
All objects SHALL produce comparable strike levels for the same excitation.

#### Scenario: Object sweep
- **WHEN** every object's resonator is excited by the same one-sample unit force impulse at 48 kHz at C4 with default controls
- **THEN** every object's peak is within 6 dB of every other object's

### Requirement: Quality mode and damage do not change loudness
The number of modes in a bank (set by quality mode and damage) SHALL NOT change its strike level by more than 2 dB.

#### Scenario: Eco versus Render
- **WHEN** an object is excited by the same one-sample unit impulse at 48 kHz at C4 with the Eco and the Render mode-count scale
- **THEN** the two peaks are within 2 dB

#### Scenario: Damage on versus off
- **WHEN** an object is excited by the same one-sample unit impulse at 48 kHz at C4 with damage 0 and damage 1
- **THEN** the two peaks are within 2 dB

### Requirement: Default notes do not overdrive the voice
With default controls at full velocity, a note SHALL stay within full scale before the voice's safety clamp.

#### Scenario: Struck, bowed and specialty notes across the keyboard
- **WHEN** every object plays C2, C4 and C6 at velocity 127 with default controls, using the HandStrike, Bow and PneumaticJet exciters, for one second held plus one second released
- **THEN** the voice's signal before its clamp never exceeds ±1.0

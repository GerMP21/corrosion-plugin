# resonator-level Specification

## Purpose
Defines how strongly the modal resonator responds to excitation, so that a note's loudness depends on the sound being played and not on incidental settings such as the host sample rate.

## Requirements

### Requirement: Resonator level does not depend on the sample rate
The same force applied over the same time SHALL produce the same resonator output level at every supported sample rate (44.1, 48, 88.2, 96 and 192 kHz).

#### Scenario: Short strike
- **WHEN** a 1 ms unit force pulse excites each object's resonator at C4 with default controls, at 44.1, 48, 96 and 192 kHz
- **THEN** each object's peak output at every rate is within 0.5 dB of its 48 kHz peak

#### Scenario: Sustained excitation
- **WHEN** a sine force at the resonator's lowest mode frequency drives it for one second at 44.1, 48 and 96 kHz
- **THEN** the output RMS over the last half second at every rate is within 0.5 dB of the 48 kHz RMS

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
With default controls at full velocity, a note SHALL stay within full scale before the voice's safety clamp, for every exciter.

#### Scenario: Struck, bowed and specialty notes across the keyboard
- **WHEN** every object plays C2, C4 and C6 at velocity 127 with default controls, using each of the 16 exciters, for one second held plus one second released
- **THEN** the voice's signal before its clamp never exceeds ±1.0

### Requirement: High modes keep their level at every sample rate
A mode's ring amplitude SHALL be that of the oscillator it models, so modes near the top of the audible band have the same level at every sample rate and none is amplified by its closeness to Nyquist.

#### Scenario: A 15 kHz mode
- **WHEN** a bank with a single 15 kHz mode is struck by the same one-sample impulse, scaled to equal force × time, at 44.1, 48 and 96 kHz
- **THEN** its peak at 44.1 and 96 kHz is within 1 dB of its peak at 48 kHz

### Requirement: Modes fade out at the top of the audible band
Modes SHALL fade out smoothly over the top of the audible band rather than switching off at a fixed frequency, so retuning a mode across it does not click.

#### Scenario: Sweeping a mode through the top of the band
- **WHEN** a ringing single-mode bank is pitch-bent so its mode sweeps from 16 kHz to 22 kHz over 100 ms at 48 kHz
- **THEN** the output's sample-to-sample change never exceeds twice its largest change during the first 10 ms of the sweep

# Spec Delta

## MODIFIED Requirements

### Requirement: Default notes do not overdrive the voice
With default controls at full velocity, a note SHALL stay within full scale before the voice's safety clamp, for every exciter.

#### Scenario: Struck, bowed and specialty notes across the keyboard
- **WHEN** every object plays C2, C4 and C6 at velocity 127 with default controls, using each of the 16 exciters, for one second held plus one second released
- **THEN** the voice's signal before its clamp never exceeds ±1.0

## ADDED Requirements

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

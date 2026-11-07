# Spec Delta

## ADDED Requirements

### Requirement: Space timing does not depend on the sample rate
Each Space mode's delays and decay times SHALL be defined in time, so the same settings give the same tail at every supported sample rate (44.1, 48, 88.2, 96 and 192 kHz).

#### Scenario: Factory reverb decay
- **WHEN** an impulse is fed through Space = Factory at default settings at 44.1, 48, 96 and 192 kHz
- **THEN** the time for the tail to fall 30 dB is within 5 % of the 48 kHz time at every rate

#### Scenario: Echo delay
- **WHEN** an impulse is fed through Space = Echo with Delay Time 1 (0.8 s) and no modulation at 44.1, 48, 96 and 192 kHz
- **THEN** the first echo arrives 0.8 s after the impulse, within one sample, at every rate

#### Scenario: Spring delay
- **WHEN** an impulse is fed through Space = Spring at tank size 1 at 48 and 192 kHz
- **THEN** the first echo arrives at the same time, within one sample, at both rates

### Requirement: No Space control silently clamps
Every position of the Space section's size, tank and delay controls SHALL produce the delay it specifies, at every supported sample rate.

#### Scenario: Full Delay Time range
- **WHEN** Delay Time is swept from 0 to 1 at 96 kHz with no modulation
- **THEN** the measured echo delay rises monotonically from 0.02 s to 0.8 s

#### Scenario: Factory combs stay distinct
- **WHEN** the Factory reverb's size is set to 0, 0.25, 0.5, 0.75 and 1 at 48 kHz
- **THEN** its four comb delays are all different at every size

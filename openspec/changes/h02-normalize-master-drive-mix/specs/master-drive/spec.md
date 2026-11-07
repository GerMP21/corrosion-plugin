# Spec Delta

## Purpose

Defines how the master Drive control maps to the balance between the dry and the shaped signal, so raising Drive adds saturation without phase-inverting the dry signal or adding level by mixing.

## ADDED Requirements

### Requirement: Dry and shaped signals crossfade
For Drive between 0 and 1, the master drive's output SHALL be a crossfade between the dry and the shaped signal: weight `1 − drive` on the dry and `drive` on the shaped signal. From Drive 1 up, the output SHALL be the shaped signal alone.

#### Scenario: Halfway
- **WHEN** a sample is processed at Drive 0.5
- **THEN** the output equals half the dry sample plus half the shaped sample at Drive 0.5, within 1e-6

#### Scenario: No dry above 1
- **WHEN** a sample is processed at Drive 1, 2.5 and 5
- **THEN** each output equals the shaped sample at that drive, with no dry component

### Requirement: Drive never inverts the signal
The master drive's output SHALL have the same sign as its input for every Drive setting.

#### Scenario: Full knob sweep
- **WHEN** samples of ±0.01, ±0.3 and ±1.0 are processed at Drive 0, 0.5, 1, 2, 3, 4 and 5
- **THEN** every output has its input's sign (or is zero for zero input)

### Requirement: Higher drive saturates more
Above Drive 1, raising Drive SHALL increase saturation, not only level.

#### Scenario: Harmonic content rises
- **WHEN** a 0.25-amplitude 1 kHz sine is processed at Drive 1, 2 and 5 at 48 kHz
- **THEN** the share of output power outside the 1 kHz component increases from each setting to the next

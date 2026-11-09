# Spec Delta

## Purpose

Defines the output clipper's curve as a sound-shaping control: how Diode Softness and Diode Asymmetry shape it, that it adds no DC, that it sounds the same in every quality mode, how the editor shows it working, and how older presets and projects map onto it.

## ADDED Requirements

### Requirement: Diode Softness sets the knee's width
Diode Softness SHALL set how far under the curve's limit the clipping starts, from a hard knee at 0 to a knee starting 14 dB under the limit at 1, and the curve's slope SHALL change gradually through the knee at every Softness above 0.

#### Scenario: Hard knee at 0
- **WHEN** Diode Softness and Diode Asymmetry are 0 and a sine at 0.9 of the curve's limit runs through the clipper
- **THEN** it comes out within 0.1 dB of its input level

#### Scenario: Wide knee at 1
- **WHEN** Diode Softness is 1, Diode Asymmetry is 0, and a sine at 0.6 of the curve's limit runs through the clipper
- **THEN** it comes out at least 0.2 dB quieter than its input, while with Diode Softness 0 it comes out within 0.1 dB of its input

#### Scenario: Softer knees add fewer high harmonics
- **WHEN** a 1 kHz sine at twice the curve's limit runs through the clipper in Render with Diode Asymmetry 0 and Diode Softness 0, 0.5 and 1
- **THEN** the energy in the 7th and higher harmonics, relative to the fundamental, falls from each Softness to the next

### Requirement: Diode Asymmetry limits the negative half earlier
Diode Asymmetry SHALL lower the negative half's limit relative to the positive half's, to 6 dB under it at 1, so the curve adds even harmonics. At 0 the curve SHALL be symmetric.

#### Scenario: Limits per polarity
- **WHEN** a 200 Hz sine at four times the curve's limit runs through the clipper with Diode Softness 0 and Diode Asymmetry 0, 0.25 and 1
- **THEN** the output's peak-to-peak span is the positive limit times 1 + 2^(−Asymmetry), within 2 %

#### Scenario: Even harmonics follow asymmetry
- **WHEN** a 200 Hz sine at twice the curve's limit runs through the clipper with Diode Asymmetry 0, 0.25 and 1
- **THEN** at 0 the 2nd harmonic is at least 60 dB under the fundamental, and it rises from 0 to 0.25 to 1

### Requirement: The diode stage adds no DC
While Diode Asymmetry is above 0, the stage SHALL remove the DC its curve creates. At Diode Asymmetry 0 it SHALL add no filtering.

#### Scenario: Asymmetric clipping is centred
- **WHEN** a 200 Hz sine at twice the curve's limit runs for two seconds with Diode Asymmetry 1
- **THEN** the mean of the last second of output is under 0.001 of the ceiling

#### Scenario: Symmetric curve is unfiltered
- **WHEN** Diode Asymmetry is 0
- **THEN** the output is bit-identical to the curve without the DC removal

### Requirement: Peaks stay under the ceiling
At the default Diode Softness and at any Diode Asymmetry, the stage's output, after DC removal, SHALL stay at or under the Analog Ceiling for the hard-driven test tone in every quality mode, so the output limiter doesn't clip it again.

#### Scenario: Hard-driven tone at default Softness
- **WHEN** a 5 kHz sine at twice the Analog Ceiling runs through the stage with Diode Softness 0.5 and Diode Asymmetry 0, 0.25 and 1, in each quality mode
- **THEN** no output sample exceeds the Analog Ceiling

### Requirement: Quality modes share the diode's character
The curve, its headroom and its DC removal SHALL be the same in every quality mode; quality modes SHALL only change how much the clipping aliases.

#### Scenario: Eco sounds like Render
- **WHEN** a 200 Hz sine at twice the Analog Ceiling runs through the stage at the default Diode Softness and Diode Asymmetry in Eco and in Render
- **THEN** the output levels agree within 0.1 dB, and the 2nd and 3rd harmonics within 0.5 dB

### Requirement: Moving the diode controls doesn't click
Changes to Diode Softness and Diode Asymmetry SHALL reach the curve gradually, including the DC removal starting or stopping.

#### Scenario: Asymmetry jumps while playing
- **WHEN** a 200 Hz sine at twice the Analog Ceiling plays and Diode Asymmetry jumps from 0 to 1, and later back to 0, between two blocks
- **THEN** the largest sample-to-sample step of the output is at most 1.5 times the largest step with Diode Asymmetry held at 1

### Requirement: The limiter panel's dot shows the diode working
The red dot in the brickwall limiter panel SHALL show how much the diode stage is reducing the level: at rest under 0.1 dB of reduction, brightening with the reduction to full brightness at 6 dB, rising at once and falling back over about 300 ms.

#### Scenario: Idle
- **WHEN** the signal stays under the knee
- **THEN** the dot is drawn at its resting colour

#### Scenario: Hard clipping
- **WHEN** the stage reduces the level by 6 dB or more
- **THEN** the next editor frame draws the dot at full brightness

#### Scenario: Release
- **WHEN** the reduction stops after the dot reached full brightness
- **THEN** the dot is back at its resting colour within one second, dimming steadily on the way

### Requirement: Older presets and projects keep the current sound
User presets and saved host projects from before Diode Asymmetry existed SHALL load with Diode Softness 0 and Diode Asymmetry 0, the setting closest to the clipper they were made with. New instances SHALL start at Diode Softness 0.5 and Diode Asymmetry 0.25.

#### Scenario: Old preset file
- **WHEN** a user preset without a Diode Asymmetry value is loaded, whatever its Diode Softness
- **THEN** Diode Softness is 0 and Diode Asymmetry is 0

#### Scenario: Old host project
- **WHEN** a host restores plugin state saved without Diode Asymmetry
- **THEN** Diode Softness is 0 and Diode Asymmetry is 0

#### Scenario: Current state is untouched
- **WHEN** a preset or project that has a Diode Asymmetry value is loaded
- **THEN** its Diode Softness and Diode Asymmetry are restored as stored

#### Scenario: New instance
- **WHEN** the plugin is created without restored state
- **THEN** Diode Softness is 0.5 and Diode Asymmetry is 0.25

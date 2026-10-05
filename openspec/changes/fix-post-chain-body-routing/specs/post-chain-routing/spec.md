# Spec Delta

## Purpose

Defines how the post chain routes the voice engine's stereo output through filter, drive, body, spread and space, what the body controls do, and what the quality modes are allowed to change.

## ADDED Requirements

### Requirement: Filtered signal reaches the output directly
The filtered and driven signal SHALL reach the post-chain output without passing through the body resonator. The body resonance is added in parallel.

#### Scenario: Body amount zero
- **WHEN** BODY is 0 and a broadband stereo signal passes through the post chain with space off
- **THEN** the output matches the same chain with the body stage removed, within 1e-6 per sample

#### Scenario: Cutoff shapes the broadband output
- **WHEN** white noise is processed with BODY at its factory value and the filter cutoff is moved from 20 kHz to 1 kHz
- **THEN** the output energy above 4 kHz drops by at least 20 dB

### Requirement: Stereo image survives the post chain
Filter and drive SHALL process left and right independently, so a signal present on only one input channel stays predominantly on that channel.

#### Scenario: Hard-left input
- **WHEN** a tone is fed to the left input only, with BODY 0, spread width 0 (spread bypassed) and space off
- **THEN** the right output's RMS is at least 20 dB below the left output's RMS

### Requirement: BODY sets the body wet amount
BODY (0–5) SHALL set the body resonance's mix level linearly from 0 % to 100 %, with no dependence on Chassis Volume.

#### Scenario: Body level tracks BODY
- **WHEN** BODY is raised from 1.0 to 4.0 with all else fixed
- **THEN** the body resonance's contribution to the output rises by a factor of 4 (±5 %)

#### Scenario: Chassis Volume does not override BODY
- **WHEN** Chassis Volume is swept from 0 to 1 with BODY at 0
- **THEN** the output is unchanged (no body resonance at any Chassis Volume)

### Requirement: Chassis Volume sets body size only
Chassis Volume SHALL change the body's resonant frequencies, and SHALL NOT apply any output gain of its own to the body.

#### Scenario: Larger chassis resonates lower
- **WHEN** Chassis Volume is raised from 0.2 to 0.8 with BODY above 0
- **THEN** the body's lowest resonant peak moves to a lower frequency

#### Scenario: No volume-dependent gain stage
- **WHEN** the body resonator is driven by the same input at Chassis Volume 0.2 and 0.8, and each output is compared with a resonator retuned to the same frequencies but with no volume gain
- **THEN** each output matches its retuned-only counterpart within 1e-6 per sample

### Requirement: Quality modes trade fidelity, not effects
Every quality mode SHALL run the same stages (filter, drive, body, spread, space, clipper). Modes MAY differ only in oversampling and modal density.

#### Scenario: Eco keeps the reverb
- **WHEN** a preset with Space = Factory reverb is rendered at Eco and at Normal quality
- **THEN** both renders contain a reverb tail; the Eco render's tail energy (after the dry signal stops) is within 3 dB of Normal's

#### Scenario: Eco keeps the body
- **WHEN** BODY is above 0 and the same input is processed at Eco and at Normal
- **THEN** both outputs contain the body resonance; their difference is limited to clipper oversampling effects

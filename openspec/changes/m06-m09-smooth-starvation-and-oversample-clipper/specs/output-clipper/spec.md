# Spec Delta

## Purpose

Defines what the post chain's final clipper does to the signal: transparent below its ceiling, band-limited clipping above it, oversampling set by the quality mode, and no latency for the host to compensate.

## ADDED Requirements

### Requirement: The clipper is transparent below its ceiling
In every quality mode, a signal that stays below the clipper's ceiling SHALL pass with its level unchanged across the audible band.

#### Scenario: Sines below the ceiling
- **WHEN** sines at amplitude 0.5 and 100 Hz, 1 kHz, 10 kHz and 18 kHz are fed through the clipper at 48 kHz in each quality mode
- **THEN** each comes out within 0.1 dB of its input level

#### Scenario: Eco is plain clipping
- **WHEN** the quality mode is Eco
- **THEN** each output sample is the diode curve applied to the input sample, with no filtering

### Requirement: Clipping above the ceiling is band-limited
When the clipper clips, the harmonics it creates above half the sample rate SHALL NOT fold back into the audible band beyond the stated bounds, and higher quality modes SHALL fold back less.

#### Scenario: Hard-driven sine
- **WHEN** a 5 kHz sine at twice the ceiling is fed through the clipper alone at 48 kHz
- **THEN** the energy outside the sine's harmonics is at least 40 dB below the harmonic energy in Normal and at least 60 dB below it in Render

#### Scenario: Quality ordering
- **WHEN** the hard-driven sine runs in Normal, High and Render
- **THEN** the fold-back energy decreases from Normal to High to Render

### Requirement: The clipper adds no latency to compensate
The clipper SHALL delay the signal by less than one sample at low frequencies in every quality mode, so the plugin reports no latency for it.

#### Scenario: Delay at 1 kHz
- **WHEN** a 1 kHz sine below the ceiling is fed through the clipper at 48 kHz in Render
- **THEN** the output lags the input by less than one sample

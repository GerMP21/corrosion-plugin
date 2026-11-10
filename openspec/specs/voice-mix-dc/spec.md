# voice-mix-dc Specification

## Purpose
Defines that the voices' mix, as it enters the output path (master drive, post chain, output), carries no DC, keeps its audible low end, and behaves the same at every sample rate.

## Requirements

### Requirement: The voice mix carries no DC
The voices' stereo mix SHALL reach the master drive without a DC offset, whatever the exciter's force.

#### Scenario: Held bow
- **WHEN** a Bow note on a Plate is held for two seconds in Drone with BODY 0, Space off, no drive and Diode Asymmetry 0
- **THEN** the mean of the plugin's output over the second half of the held note is under 2 % of its RMS

#### Scenario: Felt mallet strike
- **WHEN** a felt mallet strikes a Plate with BODY 0, Space off, no drive and Diode Asymmetry 0
- **THEN** the largest 20 ms mean of the output over the first 400 ms is at least 12 dB lower than without the blocker

### Requirement: The audible low end is kept
Removing the DC SHALL leave the audible band's level unchanged within 0.2 dB from 30 Hz up.

#### Scenario: Low sine
- **WHEN** a 30 Hz sine and a 100 Hz sine pass through the voice mix's DC blocker at 48 kHz
- **THEN** each comes out within 0.2 dB of its input level

#### Scenario: A note without an offset
- **WHEN** a hand strike on a Pipe, an exciter with no offset, is played with default settings
- **THEN** the output's RMS is within 0.1 dB of the output without the blocker

### Requirement: The same at every sample rate
The DC blocker's cutoff SHALL be the same in hertz at every host sample rate.

#### Scenario: Other rates
- **WHEN** the low-sine and held-bow scenarios run at 44.1 kHz and 96 kHz
- **THEN** they meet the same bounds

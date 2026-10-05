# space-effects Specification

## Purpose
Defines how the post chain's Space section (Factory reverb, Spring reverb, Echo) treats the stereo signal, starting with per-channel independence and delay timing of the spring reverb.

## Requirements

### Requirement: Spring reverb channels are independent
In Spring mode, each output channel's reverb tail SHALL be produced only from that channel's input.

#### Scenario: Left-only input
- **WHEN** an impulse is fed to the Space section's left input only, with Space = Spring and space amount 1
- **THEN** the right channel's wet output stays exactly 0.0 for the whole tail

### Requirement: Spring echo arrives at the configured delay
The first spring echo SHALL arrive one full configured delay length after the input, on the same channel.

#### Scenario: Impulse echo timing
- **WHEN** an impulse is fed to both Space-section inputs with a spring delay length of N samples
- **THEN** each channel's first non-zero wet sample occurs N samples after the impulse

### Requirement: Mono input gives a mono tail
With identical left and right inputs, the spring reverb SHALL produce identical left and right tails.

#### Scenario: Mono input
- **WHEN** the same signal is fed to both inputs in Spring mode
- **THEN** the left and right wet outputs are equal sample for sample

### Requirement: Spring reset clears both channels
Resetting the spring reverb SHALL clear all state in both channels, so silence in gives silence out.

#### Scenario: Reset after a loud tail
- **WHEN** both channels have a ringing tail, the post chain is reset, and silence is processed
- **THEN** every subsequent wet sample is exactly 0.0

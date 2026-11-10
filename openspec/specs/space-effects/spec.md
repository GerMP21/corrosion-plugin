# space-effects Specification

## Purpose
Defines how the post chain's Space section (Factory reverb, Spring reverb, Echo) treats the stereo signal: per-channel independence and delay timing of the spring reverb, timing that doesn't depend on the host sample rate, controls that never silently clamp, and an echo that stays on the beat when synced.

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

### Requirement: A synced echo stays on the beat
When tempo sync picks a division longer than the echo's 0.8 s maximum, the echo SHALL play that division halved as many times as it takes to fit, so it stays on the beat instead of stopping at an off-tempo 0.8 s.

#### Scenario: Long division at 120 BPM
- **WHEN** Sync is 1/1 (4 beats, 2 s) and the host tempo is 120 BPM
- **THEN** the echo delay is 0.5 s (one beat)

#### Scenario: Divisions that fit are unchanged
- **WHEN** Sync is 1/8 at 120 BPM
- **THEN** the echo delay is 0.25 s

### Requirement: Echo modulation glides
As Machinery Movement modulates the echo's delay, the echo SHALL read between samples, so a smooth input stays smooth in the repeats: no repeated or skipped samples.

#### Scenario: Sine through a moving delay
- **WHEN** a 1 kHz sine at amplitude 0.5 is fed through Space = Echo with Machinery Movement 1, Delay Time 0.25 and High Frequency Damping 0, at 48 kHz
- **THEN** the largest sample-to-sample step of the wet signal over two modulation periods is at most 1.2 times the largest step of a 1 kHz sine at the same amplitude

#### Scenario: No modulation stays exact
- **WHEN** Machinery Movement is 0
- **THEN** the echo's first repeat of an impulse is a single sample at the configured delay, as before

#### Scenario: Changing Machinery Movement
- **WHEN** Machinery Movement changes while a 1 kHz sine plays through the echo (0 to 0.01 at Delay Time 1, 0 to 1 at Delay Time 0.25, and 0.2 to 0.8 at 44.1 kHz)
- **THEN** in the 100 ms after the change, no sample-to-sample step of the wet signal exceeds 1.2 times the larger of the step before the change and the step of a steady echo at the new setting

### Requirement: High Frequency Damping darkens the repeats
High Frequency Damping SHALL low-pass the echo's repeats, more strongly at higher settings and more with each repeat, and SHALL NOT change the level of their low frequencies.

#### Scenario: Highs fade, lows stay
- **WHEN** 200 Hz and 8 kHz sines are each fed through Space = Echo with no modulation, at High Frequency Damping 0, 0.5 and 1
- **THEN** at 0 the first repeats of both are within 0.5 dB of each other; at 1 the 8 kHz repeat is at least 12 dB below the 200 Hz repeat; and the 200 Hz repeat's level is within 0.5 dB across all three settings

#### Scenario: Each repeat is darker
- **WHEN** an 8 kHz sine burst is fed through Space = Echo with High Frequency Damping 1
- **THEN** each of the first three repeats has a lower 8 kHz-to-200 Hz level ratio than the one before, measured against a 200 Hz burst

#### Scenario: Same tone at every rate
- **WHEN** the 8 kHz scenario runs at 44.1, 48 and 96 kHz with High Frequency Damping 1
- **THEN** the first repeat's 8 kHz level is within 1 dB of its 48 kHz level at each rate

# Spec Delta

## ADDED Requirements

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

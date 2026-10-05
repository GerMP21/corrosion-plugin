# Spec Delta

## Purpose

Guarantees that the post-chain low-pass ladder filter stays stable and maps its cutoff parameter to the intended frequency at every host-reachable cutoff and supported sample rate.

## ADDED Requirements

### Requirement: Filter is stable across the full cutoff range
The post-chain filter SHALL produce bounded, finite output with no sustained DC offset for every cutoff value the host can set (20 Hz–20 kHz) at every supported sample rate (44.1, 48, 88.2, 96, 192 kHz).

#### Scenario: Default cutoff at 48 kHz
- **WHEN** the filter runs at 48 kHz with cutoff 20 kHz and zero resonance and receives a 0.1-amplitude 1 kHz sine for one second followed by one second of silence
- **THEN** every output sample is finite and within ±1.0, and the mean of the final 100 ms of output is within ±1e-3 of zero

#### Scenario: Cutoff swept through a quarter of the sample rate
- **WHEN** the filter runs at 44.1 kHz and 48 kHz with cutoff set in turn to 0.24·fs, 0.25·fs, 0.26·fs, 0.375·fs and 20 kHz
- **THEN** no output sample is NaN or infinite and the output decays to within ±1e-3 of zero within one second after input stops

#### Scenario: Cutoff above Nyquist is clamped
- **WHEN** the requested cutoff is at or above half the sample rate (for example 20 kHz at a 32 kHz sample rate)
- **THEN** the filter operates at a cutoff strictly below Nyquist and remains stable

### Requirement: Cutoff parameter maps to the displayed frequency
The filter's single-stage −3 dB point SHALL be within 10 % of the cutoff value set by the host for cutoffs up to 0.4·fs.

#### Scenario: 1 kHz cutoff at 48 kHz
- **WHEN** the cutoff is set to 1 kHz at 48 kHz
- **THEN** the measured per-stage −3 dB frequency lies between 900 Hz and 1100 Hz

#### Scenario: Higher cutoff produces more high-frequency energy
- **WHEN** the same broadband input is filtered with cutoffs of 2 kHz and 8 kHz
- **THEN** the 8 kHz output contains more energy above 4 kHz than the 2 kHz output

### Requirement: Filter recovers from reset
After a reset the filter SHALL hold no residual state, so silent input produces silent output.

#### Scenario: Reset after loud input at high cutoff
- **WHEN** the filter processes full-scale input at cutoff 20 kHz, is reset, and then processes silence
- **THEN** every output sample after the reset is exactly 0.0

# Spec Delta

## Purpose

Guarantees that the post-chain low-pass ladder filter stays stable and maps its cutoff parameter to the intended frequency at every host-reachable cutoff and supported sample rate.

## ADDED Requirements

### Requirement: Filter is stable across the full cutoff range
The post-chain filter SHALL produce bounded, finite output with no sustained DC offset for every cutoff value the host can set (20 Hz–20 kHz) at every supported sample rate (44.1, 48, 88.2, 96, 192 kHz).

#### Scenario: Default cutoff at 48 kHz
- **WHEN** the filter runs at 48 kHz with cutoff 20 kHz and zero resonance and receives a 0.1-amplitude 1 kHz sine for one second followed by one second of silence
- **THEN** every output sample is finite and within ±1.0, and every sample of the final 100 ms of output is within ±1e-3 of zero

#### Scenario: Cutoff swept through a quarter of the sample rate
- **WHEN** the filter runs at 44.1 kHz and 48 kHz with cutoff set in turn to 0.24·fs, 0.25·fs, 0.26·fs, 0.375·fs and 20 kHz
- **THEN** no output sample is NaN or infinite and every sample of the final 100 ms of a one-second silence is within ±1e-3 of zero

#### Scenario: Cutoff above Nyquist is clamped
- **WHEN** the requested cutoff is at or above half the sample rate (for example 20 kHz at a 32 kHz sample rate)
- **THEN** the filter operates at a cutoff strictly below Nyquist (its prewarp coefficient is finite and positive) and remains stable

### Requirement: Cutoff parameter maps to the displayed frequency
The filter's single-stage −3 dB point SHALL be within 10 % of the cutoff value set by the host for cutoffs up to 0.4·fs.

#### Scenario: 1 kHz cutoff at 48 kHz
- **WHEN** the cutoff is set to 1 kHz at 48 kHz
- **THEN** the measured per-stage −3 dB frequency lies between 900 Hz and 1100 Hz

#### Scenario: Higher cutoff produces more high-frequency energy
- **WHEN** the same white-noise input is filtered with cutoffs of 2 kHz and 8 kHz
- **THEN** the 8 kHz output contains more than four times the energy above 4 kHz of the 2 kHz output

### Requirement: Passband has unity gain
With zero resonance, a tone well below the cutoff SHALL pass through the filter at its input level, within 3 %.

#### Scenario: Low tone at the default cutoff
- **WHEN** a 0.1-amplitude 100 Hz sine is filtered at 48 kHz with cutoff 20 kHz and zero resonance
- **THEN** the steady-state output RMS is between 0.97 and 1.01 times the input RMS

### Requirement: Maximum resonance stays bounded
At resonance 1.0 the filter SHALL produce finite output within ±1.0 for every cutoff and supported sample rate.

#### Scenario: Resonance 1.0 across cutoffs
- **WHEN** a 0.5-amplitude 1 kHz sine burst is filtered with resonance 1.0 at cutoffs 100 Hz, 1 kHz, 5 kHz, fs/4 and 20 kHz, at 44.1, 48 and 96 kHz
- **THEN** every output sample is finite and within ±1.0, including the self-oscillating tail after the burst

### Requirement: Self-oscillation only from 90 % resonance
Below resonance 0.9 the filter's output SHALL decay to silence after the input stops, at every cutoff and supported sample rate. From resonance 0.9 up the filter SHALL self-oscillate, sustaining a bounded tone after the input stops.

#### Scenario: Moderate resonance at the default cutoff decays
- **WHEN** a 0.5-amplitude 1 kHz sine is filtered for one second with resonance 0.35, 0.6 or 0.85 at cutoffs 200 Hz, 1 kHz, 5 kHz, fs/4 and 20 kHz, at 44.1, 48 and 96 kHz, followed by one second of silence
- **THEN** every sample of the final 100 ms is within ±1e-3 of zero

#### Scenario: High resonance self-oscillates
- **WHEN** the same input is filtered with resonance 0.95 or 1.0 at the same cutoffs and sample rates
- **THEN** the final 100 ms of the silent section still reaches above 0.01 in magnitude

### Requirement: Filter recovers from reset
After a reset the filter SHALL hold no residual state, so silent input produces silent output, at any component tolerance.

#### Scenario: Reset after loud input at high cutoff
- **WHEN** the filter processes full-scale input at cutoff 20 kHz, is reset, and then processes silence
- **THEN** every output sample after the reset is exactly 0.0

#### Scenario: Component tolerance does not create output from silence
- **WHEN** the filter runs with component tolerance 1.0, is reset after a signal, and then processes silence
- **THEN** every output sample is exactly 0.0

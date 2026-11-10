# Spec Delta

## MODIFIED Requirements

### Requirement: The clipper is transparent below its threshold
In every quality mode, a signal that stays below the clipper's threshold SHALL pass with its level unchanged across the audible band. The threshold is where the diode curve's knee starts: it depends on Diode Softness and Diode Asymmetry, and it is the same in every quality mode, under the Analog Ceiling by the headroom that keeps the down-sampling filters' overshoot under the output limiter.

#### Scenario: Sines below the ceiling
- **WHEN** sines at amplitude 0.3 and 100 Hz, 1 kHz, 10 kHz and 18 kHz are fed through the clipper at 48 kHz in each quality mode, at the default Diode Softness and Diode Asymmetry
- **THEN** each comes out within 0.1 dB of its input level

#### Scenario: Eco is plain clipping
- **WHEN** the quality mode is Eco and Diode Asymmetry is 0
- **THEN** each output sample is the diode curve applied to the input sample, with no filtering

# Spec Delta

## ADDED Requirements

### Requirement: Bent and dynamic notes cost about the same as plain notes
Holding a pitch bend SHALL NOT make a voice cost more than 10 % more CPU than the same voice unbent. An object with per-sample pitch dynamics SHALL cost no more than 1.5 times the same voice with its dynamics held still.

#### Scenario: Sixteen bent voices
- **WHEN** 16 Render-quality voices are processed for 4 096 samples with a held +2-semitone pitch bend, and again with no bend
- **THEN** the bent run takes at most 1.1 times as long as the unbent run, taken as the median over 5 benchmark runs of the ratio within each run (`scripts/bench_ratios.py`)

#### Scenario: Sixteen dynamic voices
- **WHEN** 16 TautCable voices with tension drop are processed for 4 096 samples while ringing, and again with tension drop 0
- **THEN** the dynamic run takes at most 1.5 times as long as the static run, taken as the median over 5 benchmark runs of the ratio within each run (`scripts/bench_ratios.py`)

### Requirement: Skipping retunes does not change the pitch heard
Each mode's frequency SHALL stay within 0.05 cents of the frequency its object's dynamics and pitch bend call for at every sample.

#### Scenario: Held bend is exact
- **WHEN** a note is held with a constant +2-semitone bend for one second
- **THEN** the output is bit-identical to rebuilding every mode's coefficients every sample

#### Scenario: Moving dynamics stay in tune
- **WHEN** a ringing TautCable note's pitch factor changes every sample
- **THEN** at every sample each mode's frequency is within 0.05 cents of `base frequency × current factor`

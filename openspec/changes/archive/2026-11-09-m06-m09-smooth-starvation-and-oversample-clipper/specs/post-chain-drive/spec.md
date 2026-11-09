# Spec Delta

## ADDED Requirements

### Requirement: Bias Starvation changes the gain smoothly
When Bias Starvation is above 0, the drive's starvation SHALL move its gain between full and starved gradually, without single-sample steps, at every supported sample rate. With Bias Starvation at 0 the drive SHALL behave as without starvation.

#### Scenario: No clicks from starvation
- **WHEN** a 100 Hz sine at amplitude 0.5 runs through the post-chain drive with Drive Amount 2.5, chaos 0, and Bias Starvation 0.1, 0.25, 0.35, 0.5 and 1 in turn, for two seconds each at 48 kHz
- **THEN** at each value the largest sample-to-sample step of the output is at most 1.5 times the largest step with Bias Starvation 0

#### Scenario: Starvation off is unchanged
- **WHEN** Bias Starvation is 0
- **THEN** the drive's output is bit-identical to the drive without the starvation stage

#### Scenario: Same ramp at every rate
- **WHEN** the no-clicks scenario runs at 44.1 and 96 kHz
- **THEN** the bound holds at each value and rate

### Requirement: Bias Starvation acts across its whole range
At every Bias Starvation value above 0, the drive SHALL spend part of the time starved and part at full gain, and the starved share of time SHALL rise with the value.

#### Scenario: Starvation keeps switching
- **WHEN** the drive runs for sixty seconds at 48 kHz with Bias Starvation 0.1, 0.25, 0.5, 0.75 and 1 in turn
- **THEN** at each value the gate is starved for between 1 % and 99 % of the samples after the first second

#### Scenario: More starvation starves more often
- **WHEN** the same runs are compared
- **THEN** the starved share increases from each value to the next

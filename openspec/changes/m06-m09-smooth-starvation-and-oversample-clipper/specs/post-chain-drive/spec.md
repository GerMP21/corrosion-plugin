# Spec Delta

## ADDED Requirements

### Requirement: Bias Starvation changes the gain smoothly
When Bias Starvation is above 0, the drive's starvation SHALL move its gain between full and starved gradually, without single-sample steps, at every supported sample rate. With Bias Starvation at 0 the drive SHALL behave as without starvation.

#### Scenario: No clicks from starvation
- **WHEN** a 100 Hz sine at amplitude 0.5 runs through the post-chain drive with Drive Amount 2.5 and Bias Starvation 1 for two seconds at 48 kHz
- **THEN** the largest sample-to-sample step of the output is at most 1.5 times the largest step with Bias Starvation 0

#### Scenario: Starvation off is unchanged
- **WHEN** Bias Starvation is 0
- **THEN** the drive's output is bit-identical to the drive without the starvation stage

#### Scenario: Same ramp at every rate
- **WHEN** the no-clicks scenario runs at 44.1 and 96 kHz
- **THEN** the bound holds at each rate

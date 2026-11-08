# post-chain-drive Specification

## Purpose
Defines how the post-chain drive's amount control maps to the balance between its saturated and dry signals, so the whole control range is audible and the drive never adds level by mixing.

## Requirements

### Requirement: The whole amount range is audible
The post-chain drive's wet share SHALL rise monotonically with Drive Amount (including the Violence macro's offset) across the parameter's full 0–5 range, reaching fully wet at 5.

#### Scenario: Amounts above 1 differ
- **WHEN** the same signal is processed with Drive Amount 1, 3 and 5 and no macro offset
- **THEN** the three outputs differ from each other, and the wet share is 0.2, 0.6 and 1.0 respectively

#### Scenario: Violence pushes past 1
- **WHEN** Drive Amount is 1 and the Violence macro adds its maximum offset
- **THEN** the wet share is greater than at Drive Amount 1 with no offset

### Requirement: Drive mixing is a crossfade
The post-chain drive's output SHALL be a crossfade between its dry input and its saturated signal: weight `1 − w` on the dry input and `w` on the saturated signal, where `w` is the wet share, so the weights always sum to 1.

#### Scenario: Linear region
- **WHEN** a signal small enough to stay in the saturator's linear region is processed, with chaos and starvation at 0, at Drive Amount 0, 2.5 and 5
- **THEN** the output equals the input scaled by `(1 − w) + w·g` within 1e-6, where `w` is the wet share (0, 0.5 and 1) and `g = 1 + amount/2` is the drive's input gain: ×1, ×1.625 and ×3.5 respectively

#### Scenario: Fully wet has no dry
- **WHEN** a signal is processed at Drive Amount 5
- **THEN** the output contains no dry component: it equals the saturated signal alone

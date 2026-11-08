# output-limiting Specification

## Purpose
Defines the plugin's output ceiling: the largest sample magnitude the plugin emits, set by Analog Ceiling and respected by both limiter modes.

## Requirements

### Requirement: Output never exceeds the Analog Ceiling
In both Hard and Lookahead limiter modes, every output sample's magnitude SHALL be at most the Analog Ceiling, whatever the output gain.

#### Scenario: Low ceiling, Hard mode
- **WHEN** Analog Ceiling is 0.5, the limiter is Hard, output gain is 4, and a full-velocity note plays
- **THEN** no output sample exceeds ±0.5

#### Scenario: Both modes agree
- **WHEN** the same loud note is rendered with Analog Ceiling 0.7 in Hard mode and in Lookahead mode
- **THEN** both outputs peak at or below 0.7, and both reach at least 0.65

### Requirement: The default ceiling is unchanged
At the default Analog Ceiling, Hard mode SHALL clamp exactly where it did before this requirement existed (0.9661).

#### Scenario: Factory preset output
- **WHEN** a factory preset is rendered in Hard mode at its stored Analog Ceiling of 0.9661
- **THEN** the output is bit-identical to the render with a fixed 0.9661 clamp

# Spec Delta

## Purpose

Sets how loud exciters play relative to each other and guarantees that an exciter's contact with the resonator stays stable whatever its drive.

## ADDED Requirements

### Requirement: Mallet and grinder play at a struck-exciter level
With default controls, the HardMallet and HeavyGrinding exciters SHALL play at a level comparable to the other struck exciters, not near silence.

#### Scenario: Median level across objects and the keyboard
- **WHEN** HardMallet, HeavyGrinding, Drumstick and WireBrush each play every object at C2, C4 and C6, velocity 127, default controls, for one second held plus one second released, at 48 kHz
- **THEN** HardMallet's and HeavyGrinding's median (over the 27 notes) peak before the voice's safety clamp is within 6 dB of the mean of Drumstick's and WireBrush's medians
- **AND** no note of either exceeds ±0.9 before the clamp

### Requirement: Mallet and grinder contact stays stable at any drive
The HardMallet and HeavyGrinding contact with the resonator SHALL NOT diverge, whatever the drive.

#### Scenario: Far above the default drive
- **WHEN** HardMallet and HeavyGrinding each play every object at C2, C4 and C6, velocity 127, default controls, with their drive 16× (24 dB) above default, at 48 kHz
- **THEN** the resonator output stays finite and below 100 at every sample

### Requirement: The hard mallet leaves after its strike
A hard-mallet note SHALL stop exciting the resonator once the mallet has bounced off, instead of staying pressed against it.

#### Scenario: Held hard-mallet note
- **WHEN** HardMallet plays Pipe C4 at velocity 100 and the note is held for two seconds
- **THEN** the exciter applies no force after 0.5 s

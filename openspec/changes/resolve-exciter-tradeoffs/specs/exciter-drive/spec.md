# Spec Delta

## ADDED Requirements

### Requirement: Exciters play at a common level
With default controls, each exciter's typical note SHALL land near one common level, so no exciter is much quieter or louder than the rest.

#### Scenario: Median level per exciter
- **WHEN** each exciter (1–16) plays every object at C2, C4 and C6, velocity 127, default controls, for one second held plus one second released, at 48 kHz
- **THEN** each exciter's median (over the 27 notes) peak before the voice's safety clamp is within 6 dB of the median of all 16 exciters' medians

### Requirement: No note overdrives its voice
A voice SHALL keep its signal within full scale before its safety clamp, whatever the object, pitch, exciter or velocity.

#### Scenario: Every exciter across the keyboard
- **WHEN** each exciter (1–16) plays every object at C2, C4 and C6, velocity 127, default controls, for one second held plus one second released, at 48 kHz
- **THEN** the voice's signal before its clamp never exceeds ±1.0

#### Scenario: Excess drive is reduced, not clipped
- **WHEN** an exciter's drive is set 12 dB above its default on the object and note where it is loudest
- **THEN** the voice's signal before its clamp still never exceeds ±1.0, and the note's level is within 3 dB of the same note at default drive

### Requirement: Contact stays stable at any drive
An exciter whose force depends on the resonator's position or motion SHALL NOT diverge, whatever the drive.

#### Scenario: Contact exciters at high drive without the governor
- **WHEN** each exciter plays every object at C2, C4 and C6, velocity 127, with its drive 12 dB above default and the voice's overdrive protection turned off, at 48 kHz
- **THEN** the resonator output stays finite and below 100 at every sample

### Requirement: Strikes come to rest
A struck exciter SHALL stop exciting the resonator once its strike has finished, instead of buzzing against it for as long as the note is held.

#### Scenario: Held drumstick and hard-mallet notes
- **WHEN** the Drumstick or HardMallet exciter plays Pipe C4 at velocity 100 and the note is held for two seconds
- **THEN** the exciter applies no force after 0.5 s

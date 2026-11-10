# exciter-drive Specification

## Purpose
Sets how loud exciters play relative to each other, keeps every note within its voice's headroom, guarantees that an exciter's contact with the resonator stays stable whatever its drive, and makes struck exciters come to rest after their strike.

## Requirements

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
- **THEN** the voice's signal before its clamp still never exceeds ±1.0, and the note's peak level is within 3 dB of the same note's peak at default drive

  (The peak, not the RMS: a governor that acts above a threshold can only pull a driven note down to about that threshold, and capping a strike's force lengthens its contact, so a note quieter than the threshold at default drive ends up louder on average while its peak stays bounded.)

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

### Requirement: The hard mallet plays at a struck-exciter level
With default controls, the HardMallet exciter SHALL play at a level comparable to the other struck exciters, not near silence, and neither HardMallet nor HeavyGrinding SHALL push a default note near full scale.

#### Scenario: Median level across objects and the keyboard
- **WHEN** HardMallet, HeavyGrinding, Drumstick and WireBrush each play every object at C2, C4 and C6, velocity 127, default controls, for one second held plus one second released, at 48 kHz
- **THEN** HardMallet's median (over the 27 notes) peak before the voice's safety clamp is within 6 dB of the mean of Drumstick's and WireBrush's medians
- **AND** no note of HardMallet or HeavyGrinding exceeds ±0.9 before the clamp

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

### Requirement: Harder playing never silences the pneumatic jet
The Pneumatic Jet exciter SHALL keep sounding as its air pressure grows, whether from the Air Pressure control, Turbulence Chaos or the note's velocity: its saturation SHALL hold its peak instead of falling to silence.

#### Scenario: Across Air Pressure, Turbulence Chaos and velocity
- **WHEN** the Pneumatic Jet plays every object at C4, held for 1 s at 48 kHz, with Air Pressure 0.1, 1.1 and 2.1 and Turbulence Chaos 0, 0.6 and 2, at velocities 64 and 127
- **THEN** every velocity-127 note is above −120 dB RMS and no more than 6 dB below the velocity-64 note with the same settings

#### Scenario: Notes below the knee are unchanged
- **WHEN** the Pneumatic Jet plays every object at C4 with default controls at velocity 100
- **THEN** the output is bit-identical to the output before the change

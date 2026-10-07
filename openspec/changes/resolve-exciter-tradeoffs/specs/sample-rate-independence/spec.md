# Spec Delta

## MODIFIED Requirements

### Requirement: Notes sound the same at every sample rate
For every exciter, a note SHALL render at 44.1 and 96 kHz with the same level as at 48 kHz, with no exciter exempted and no part of the note excluded.

#### Scenario: Every exciter on a Pipe
- **WHEN** each exciter (1–16) plays Pipe C4 at velocity 100 with default controls for one second, then releases for one second, at 44.1, 48 and 96 kHz
- **THEN** the voice output's RMS at 44.1 and 96 kHz is within 1.5 dB of its 48 kHz RMS, for every exciter

#### Scenario: Attack timing of struck exciters
- **WHEN** the HandStrike, FeltMallet, HardMallet and Drumstick exciters play Pipe C4 at 44.1, 48 and 96 kHz
- **THEN** the time from note-on to the output's peak over the whole note is within 10 % of the 48 kHz time

# Spec Delta

## MODIFIED Requirements

### Requirement: Notes sound the same at every sample rate
For every exciter, a note's expected level SHALL be the same at 44.1 and 96 kHz as at 48 kHz, with no exciter exempted and no part of the note excluded. The expected level is the note's mean power over velocities and note seeds.

#### Scenario: Every exciter on a Pipe
- **WHEN** each exciter (1–16) plays Pipe C4 with default controls for one second, then releases for one second, at 44.1, 48 and 96 kHz, at velocities 40, 70, 100 and 127, each with 32 note seeds
- **THEN** the voice output's mean power over those 128 notes at 44.1 and 96 kHz is within 1.5 dB of its 48 kHz mean power, for every exciter

  (A single note can't resolve 1.5 dB for a noise-driven exciter: its one-second level varies by several dB from the seed alone. A stick-slip exciter can also lock into a different regime at one velocity on one grid. The averaged level is the note's expected level.)

#### Scenario: Attack timing of struck exciters
- **WHEN** the HandStrike, FeltMallet, HardMallet and Drumstick exciters play Pipe C4 at 44.1, 48 and 96 kHz
- **THEN** the time from note-on to the output's peak over the whole note is within 10 % of the 48 kHz time

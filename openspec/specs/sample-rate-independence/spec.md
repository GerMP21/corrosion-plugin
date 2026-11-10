# sample-rate-independence Specification

## Purpose
Guarantees that a note, its tail and the post-chain body sound the same whichever sample rate the host runs at, so a session doesn't change character when its sample rate changes.

## Requirements

### Requirement: Notes sound the same at every sample rate
For every exciter, a note's expected level SHALL be the same at 44.1 and 96 kHz as at 48 kHz, with no exciter exempted and no part of the note excluded. The expected level is the note's mean power over velocities and note seeds.

#### Scenario: Every exciter on a Pipe
- **WHEN** each exciter (1–16) plays Pipe C4 with default controls for one second, then releases for one second, at 44.1, 48 and 96 kHz, at velocities 40, 70, 100 and 127, each with 32 note seeds
- **THEN** the voice output's mean power over those 128 notes at 44.1 and 96 kHz is within 1.5 dB of its 48 kHz mean power, for every exciter

  (A single note can't resolve 1.5 dB for a noise-driven exciter: its one-second level varies by several dB from the seed alone. A stick-slip exciter can also lock into a different regime at one velocity on one grid. The averaged level is the note's expected level.)

#### Scenario: Attack timing of struck exciters
- **WHEN** the HandStrike, FeltMallet, HardMallet and Drumstick exciters play Pipe C4 at 44.1, 48 and 96 kHz
- **THEN** the time from note-on to the output's peak over the whole note is within 10 % of the 48 kHz time

### Requirement: Tails end at the same time at every sample rate
A released voice SHALL stop sounding after the same amount of time at every sample rate.

#### Scenario: Struck Pipe tail
- **WHEN** a HandStrike Pipe C4 note is released after 0.1 s, at 44.1, 48 and 96 kHz
- **THEN** the time until the voice stops sounding is within 10 % of the 48 kHz time

### Requirement: The body sounds the same at every sample rate
The post-chain body resonance SHALL have the same level and ring time at every sample rate.

#### Scenario: Body impulse response
- **WHEN** the FEM body at default material and chassis volume is excited by the same 1 ms pulse at 44.1, 48 and 96 kHz
- **THEN** its peak is within 1 dB, and its time to fall 40 dB within 10 %, of the 48 kHz response

### Requirement: A note's start does not depend on voice history
At any supported sample rate, a note SHALL render the same samples whether its voice has played before at that rate or has never played, and a pitch bend received before the note's first sample SHALL take effect at the host's rate.

#### Scenario: First note at 96 kHz
- **WHEN** a plugin initialized at 96 kHz plays Pipe C4 with each of the 16 exciters on voices that have never played, and the same notes are played on voices that have already played at 96 kHz
- **THEN** the two renders are bit-identical for every exciter

#### Scenario: Bend before the first sample
- **WHEN** at 44.1 kHz a held +2-semitone bend arrives in the same block as a note-on, before the note's first sample
- **THEN** every mode of the note's bank is tuned for 44.1 kHz from its first sample (its pole angle matches its bent frequency at 44.1 kHz)

# Spec Delta

## ADDED Requirements

### Requirement: A note's start does not depend on voice history
At any supported sample rate, a note SHALL render the same samples whether its voice has played before at that rate or has never played, and a pitch bend received before the note's first sample SHALL take effect at the host's rate.

#### Scenario: First note at 96 kHz
- **WHEN** a plugin initialized at 96 kHz plays Pipe C4 with each of the 16 exciters on voices that have never played, and the same notes are played on voices that have already played at 96 kHz
- **THEN** the two renders are bit-identical for every exciter

#### Scenario: Bend before the first sample
- **WHEN** at 44.1 kHz a held +2-semitone bend arrives in the same block as a note-on, before the note's first sample
- **THEN** every mode of the note's bank is tuned for 44.1 kHz from its first sample (its pole angle matches its bent frequency at 44.1 kHz)

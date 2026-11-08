# Spec Delta

## Purpose

Defines which modes a resonator bank may contain, so that every mode the engine spends time on can contribute to the sound.

## ADDED Requirements

### Requirement: Banks contain only modes that can sound
After note-on, a resonator bank SHALL contain no mode whose frequency is at or above 80 kHz (the top of the audible band at the lowest pitch a note can be tuned to), and no mode whose decay time constant is under 0.1 ms.

#### Scenario: Small, high Pipe
- **WHEN** a Pipe note at C6 with Size at its minimum is started at Render quality
- **THEN** every mode in the bank is below 80 kHz and has a decay time constant of at least 0.1 ms

#### Scenario: Every object and quality
- **WHEN** each object plays C2, C4 and C6 at every quality mode and damage 0 and 1
- **THEN** no bank contains a mode at or above 80 kHz or with a decay time constant under 0.1 ms

### Requirement: Pruning does not change how loud notes are
Removing unsoundable modes SHALL keep each note's level within 0.5 dB of its level with the full bank.

#### Scenario: Struck notes across objects
- **WHEN** each object plays C2, C4 and C6 with a HandStrike at velocity 100, once with the pruned bank and once with the unpruned bank
- **THEN** the RMS levels of each pair are within 0.5 dB

### Requirement: Retuned notes keep every mode they can reach
A mode kept by the bank SHALL stay in the bank for the whole note, so bending or retuning a note down never removes modes that come into the audible band.

#### Scenario: Bend down two octaves
- **WHEN** a held note is retuned down 24 semitones
- **THEN** the bank holds the same modes as at note-on

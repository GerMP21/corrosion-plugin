# Spec Delta

## Purpose

Defines how many notes the instrument can sound at once, how a new note is given a voice, and which sounding voice gives way when every voice is busy.

## ADDED Requirements

### Requirement: Sixteen voices of polyphony
The instrument SHALL sound up to 16 notes (held or tailing) at once without stealing.

#### Scenario: Sixteen simultaneous notes
- **WHEN** 16 different notes are started and held
- **THEN** all 16 are sounding and each produces audible output

#### Scenario: Seventeenth note steals
- **WHEN** a seventeenth note starts while 16 voices are sounding
- **THEN** the new note sounds and exactly 16 voices are sounding afterwards

### Requirement: Free voices are used before stealing
A new note SHALL take a voice that is not sounding whenever one exists, and SHALL steal only when all 16 voices are sounding.

#### Scenario: Free slot preferred over a released tail
- **WHEN** some voices are tailing after release and at least one voice is silent
- **THEN** the new note takes a silent voice and every tail keeps sounding

### Requirement: Released tails are stolen before held notes
When the pool is full and at least one voice is tailing after its note was released, the stolen voice SHALL be a tailing voice: the quietest one, with the oldest losing ties.

#### Scenario: Tail gives way to a new note
- **WHEN** 15 notes are held, one note has been released and is still tailing, and a new note starts
- **THEN** the tailing voice is stolen and all 15 held notes keep sounding

### Requirement: Newly started notes are protected from stealing
When the pool is full and every voice is held, a held note started less than 100 ms ago SHALL NOT be stolen while any held note at least 100 ms old exists. Among the held notes that may be stolen, the quietest SHALL be stolen, with the oldest losing ties. If every held note is younger than 100 ms, the oldest SHALL be stolen.

#### Scenario: Rapid notes over a full pool
- **WHEN** 16 notes have been held for at least 100 ms and 16 further notes start in quick succession (one every 64 samples)
- **THEN** every new note is still sounding after the last one starts, and only the original 16 notes were stolen

#### Scenario: Quiet young note survives
- **WHEN** 16 notes are held, one of them started under 100 ms ago with a lower peak than all the others, and a new note starts
- **THEN** the young note keeps sounding and an older held note is stolen

#### Scenario: Every held note is young
- **WHEN** all 16 held notes started under 100 ms ago and a new note starts
- **THEN** the oldest held note is stolen

#### Scenario: Protection is measured in time
- **WHEN** at 44.1 kHz and at 96 kHz, a full pool holds loud old notes and one quiet held note, and a new note starts when the quiet note is 90 ms old, and again (from the same setup) when it is 110 ms old
- **THEN** the quiet note survives at 90 ms and is stolen at 110 ms, at both sample rates

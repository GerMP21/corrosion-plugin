# realtime-safety Specification

## Purpose
Defines what the plugin may do on the host's audio thread so it never blocks, aborts, or glitches the host: no heap allocation or deallocation during processing, note handling, voice management, or reset.

## Requirements

### Requirement: Note events do not allocate
Handling note-on and note-off events on the audio thread SHALL NOT allocate or free heap memory, for every object, play mode, quality mode, damage amount and character parameter value.

#### Scenario: Note-on for every object and quality mode
- **WHEN** inside an allocation-checking zone, a note-on is sent for each of the 9 objects at each of the 4 quality modes, with damage 0 and damage 1
- **THEN** no heap allocation or deallocation is recorded

#### Scenario: Note-off and tail
- **WHEN** inside an allocation-checking zone, held notes are released and processed until their voices go idle
- **THEN** no heap allocation or deallocation is recorded

### Requirement: Live expression does not allocate
Pitch bend, channel and polyphonic pressure, mod wheel and live parameter automation applied to sounding voices SHALL NOT allocate or free heap memory.

#### Scenario: Pitch bend on a dynamic object
- **WHEN** inside an allocation-checking zone, a TautCable note is held while pitch bend sweeps across its full range
- **THEN** no heap allocation or deallocation is recorded

### Requirement: Reset does not allocate
Resetting the plugin's processing state SHALL NOT allocate or free heap memory, and SHALL leave the plugin silent and ready for new notes.

#### Scenario: Reset with active voices
- **WHEN** inside an allocation-checking zone, the plugin is reset while voices are sounding and then processes silence
- **THEN** no heap allocation or deallocation is recorded and the output is silent

### Requirement: Removing allocations does not change the sound
The modal bank produced at note-on SHALL contain the same modes, in the same order, with the same frequency, decay and gain as before this change.

#### Scenario: Mode bank equivalence
- **WHEN** a note-on is performed for each object at each quality mode with damage values 0, 0.5 and 1
- **THEN** each resulting mode bank matches the bank produced by the previous implementation within 1e-6 relative error per value

### Requirement: Stealing a voice does not allocate
Starting a note when every voice in the pool is sounding SHALL NOT allocate or free heap memory.

#### Scenario: Notes beyond a full pool steal voices
- **WHEN** inside an allocation-checking zone, every voice in the pool is playing and further note-ons arrive with a different object than the voices they steal
- **THEN** each note-on steals a voice and no heap allocation or deallocation is recorded

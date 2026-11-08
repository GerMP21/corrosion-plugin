# realtime-safety Specification

## Purpose
Defines what the plugin may do on the host's audio thread so it never blocks, aborts, or glitches the host: no heap allocation or deallocation during processing, note handling, voice management, or reset, and no per-sample work that makes bent or dynamic notes cost far more than plain ones.

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

### Requirement: Bent and dynamic notes cost about the same as plain notes
Holding a pitch bend SHALL NOT make a voice cost more than 10 % more CPU than the same voice unbent. An object with per-sample pitch dynamics SHALL cost no more than 1.6 times the same voice with its dynamics held still. (Even with retuning free, a dynamic note costs about 1.1 times its static twin, because it sounds different and its exciter and governor do different work.)

#### Scenario: Sixteen bent voices
- **WHEN** 16 Render-quality voices are processed for 4 096 samples with a held +2-semitone pitch bend, and again with no bend
- **THEN** the bent run takes at most 1.1 times as long as the unbent run, taken as the median over 5 benchmark runs of the ratio within each run (`scripts/bench_ratios.py`)

#### Scenario: Sixteen dynamic voices
- **WHEN** 16 TautCable voices with tension drop are processed for 4 096 samples while ringing, and again with tension drop 0
- **THEN** the dynamic run takes at most 1.6 times as long as the static run, taken as the median over 5 benchmark runs of the ratio within each run (`scripts/bench_ratios.py`)

### Requirement: Skipping retunes does not change the pitch heard
Each mode's frequency SHALL stay within 0.05 cents of the frequency its object's dynamics and pitch bend call for at every sample.

#### Scenario: Held bend is exact
- **WHEN** a note is held with a constant +2-semitone bend for one second
- **THEN** the output is bit-identical to rebuilding every mode's coefficients every sample

#### Scenario: Moving dynamics stay in tune
- **WHEN** a ringing TautCable note's pitch factor changes every sample
- **THEN** at every sample each mode's frequency is within 0.05 cents of `base frequency × current factor`

# midi-control Specification

## Purpose
Defines how the plugin responds to MIDI controllers and channel-mode messages: the sustain pedal, All Notes Off, All Sound Off and Reset All Controllers.

## Requirements

### Requirement: The sustain pedal holds released notes
While the sustain pedal (CC64) is down, at a value of 64 or more, a note whose key is released SHALL keep sounding as if its key were held. When the pedal lifts, every note whose key is up SHALL be released.

#### Scenario: Chord held by the pedal
- **WHEN** the pedal goes down, three notes are played and their keys released, and one second later the pedal lifts
- **THEN** the three notes sound at their held level until the pedal lifts, then enter their release

#### Scenario: Key still down when the pedal lifts
- **WHEN** a note's key is still held when the pedal lifts
- **THEN** that note keeps sounding until its key is released

#### Scenario: Pedal up without notes
- **WHEN** the pedal goes down and up with no notes playing
- **THEN** nothing changes in the output

### Requirement: Re-striking a sustained note replaces it
When a note that is sounding only because of the pedal is played again, the earlier instance SHALL be released, so repeated notes under the pedal never hold more than one pedal-sustained voice per note.

#### Scenario: Repeated note under the pedal
- **WHEN** with the pedal down, the same note is played and released ten times
- **THEN** at most one voice for that note is held by the pedal at any moment

### Requirement: All Notes Off releases every held key
All Notes Off (CC123) SHALL release every note whose key is held, exactly as releasing each key would, so notes held by a down pedal keep sounding until it lifts.

#### Scenario: Stuck notes
- **WHEN** four notes are held without the pedal and All Notes Off arrives
- **THEN** all four enter their release in that sample

#### Scenario: Pedal down
- **WHEN** four notes are held, the pedal is down and All Notes Off arrives
- **THEN** the four notes keep sounding until the pedal lifts

### Requirement: All Sound Off silences everything without a click
All Sound Off (CC120) SHALL bring the plugin's output to silence within 10 ms, including voice tails and Space tails, by fading rather than cutting, and the plugin SHALL play normally afterwards.

#### Scenario: Panic with a reverb tail
- **WHEN** notes ring through Space = Factory and All Sound Off arrives
- **THEN** the output reaches exactly zero within 10 ms, no sample-to-sample step during the fade exceeds the largest step in the 10 ms before it, and the output stays zero until a new note

#### Scenario: Playing after a panic
- **WHEN** a note is played 50 ms after All Sound Off
- **THEN** it renders the same samples as the same note on a freshly reset plugin

### Requirement: Reset All Controllers returns controls to rest
Reset All Controllers (CC121) SHALL lift the sustain pedal and return mod wheel, pitch bend and channel pressure to their rest values.

#### Scenario: Controllers left set
- **WHEN** the pedal is down, the mod wheel is at 1, pitch bend is at +2 semitones and Reset All Controllers arrives
- **THEN** pedal-held notes are released, and the next note renders as with the mod wheel at 0, no bend and no pressure

### Requirement: Real-time safe controller handling
Handling any of these controller messages SHALL NOT allocate or free heap memory on the audio thread.

#### Scenario: Pedal and panic in a checked zone
- **WHEN** inside an allocation-checking zone, the pedal goes down and up around held notes, and All Notes Off, All Sound Off and Reset All Controllers arrive
- **THEN** no heap allocation or deallocation is recorded

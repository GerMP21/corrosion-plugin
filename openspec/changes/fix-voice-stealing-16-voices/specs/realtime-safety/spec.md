# Spec Delta

## REMOVED Requirements

### Requirement: Voice stealing does not allocate
**Reason**: Its only scenario names a fixed pool size ("eight voices are playing and a ninth note-on arrives"), which is wrong once the pool grows to 16 voices.
**Migration**: Replaced by "Stealing a voice does not allocate" below, which states the same guarantee for a full pool of any size.

## ADDED Requirements

### Requirement: Stealing a voice does not allocate
Starting a note when every voice in the pool is sounding SHALL NOT allocate or free heap memory.

#### Scenario: Notes beyond a full pool steal voices
- **WHEN** inside an allocation-checking zone, every voice in the pool is playing and further note-ons arrive with a different object than the voices they steal
- **THEN** each note-on steals a voice and no heap allocation or deallocation is recorded

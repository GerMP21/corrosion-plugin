# Spec Delta

## ADDED Requirements

### Requirement: Pedal-held notes are stolen before held keys
When every voice is sounding, a note sounding only because the sustain pedal is down SHALL be stolen before any note whose key is still held, and after any released tail.

#### Scenario: Full pool under the pedal
- **WHEN** with the pedal down, 16 notes are played, the keys of the first 8 are released, the last 8 stay held, and a 17th note arrives
- **THEN** the voice taken is one of the 8 pedal-held notes, not one of the held keys

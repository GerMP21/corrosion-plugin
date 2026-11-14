## ADDED Requirements

### Requirement: Every sound-shaping control changes its note
Every host parameter that shapes the sound SHALL, moved from its minimum to its maximum on a patch where it applies (its exciter or object selected, its effect switched on, its stage engaged), change one note's level by at least 1 dB, its ring length by at least 10 %, or its octave-band spectrum by at least 1 dB beyond the level change. A parameter that can't SHALL be listed with its reason (for example UI Scale, Width on a mono measure, Quality Mode).

#### Scenario: A dedicated exciter control acts
- **WHEN** Chatter Pitch is moved from its minimum to its maximum on a Stiff Point Scrape note on Plate, C3 at velocity 100, held 2 s
- **THEN** the note's octave-band spectrum changes by at least 1 dB beyond its level change

#### Scenario: A force control acts through the governor
- **WHEN** Chain Mass is moved from its minimum to its maximum on a Metal Chain note on Plate, C3 at velocity 100
- **THEN** the note's level, ring or spectrum changes by at least the floor

#### Scenario: A control is disconnected later
- **WHEN** a change makes a listed control's minimum and maximum render the same note
- **THEN** the control-response test fails, naming the control and its measurement

### Requirement: Help texts point the way the control sounds
Each control's help text SHALL describe the direction of its measured effect: when it says higher is louder, brighter, longer, darker or shorter, raising the control SHALL do that on its measured note.

#### Scenario: Mute Decay
- **WHEN** Mute Decay is raised from its minimum to its maximum on a Hand Strike note on Plate
- **THEN** the note changes the way Mute Decay's help text says it does

## ADDED Requirements

### Requirement: Harder playing never silences the pneumatic jet
The Pneumatic Jet exciter SHALL keep sounding as its air pressure grows, whether from the Air Pressure control, Turbulence Chaos or the note's velocity: its saturation SHALL hold its peak instead of falling to silence.

#### Scenario: Across Air Pressure, Turbulence Chaos and velocity
- **WHEN** the Pneumatic Jet plays every object at C4, held for 1 s at 48 kHz, with Air Pressure 0.1, 1.1 and 2.1 and Turbulence Chaos 0, 0.6 and 2, at velocities 64 and 127
- **THEN** every velocity-127 note is above −120 dB RMS and no more than 6 dB below the velocity-64 note with the same settings

#### Scenario: Notes below the knee are unchanged
- **WHEN** the Pneumatic Jet plays every object at C4 with default controls at velocity 100
- **THEN** the output is bit-identical to the output before the change

# mseg-envelope Specification

## Purpose
Defines how the multi-stage envelope used by friction-family exciters sequences its stages and loops, how velocity scales its level, how Release starts and ends, and guarantees that every host-reachable configuration is safe to run on the audio thread.

## Requirements

### Requirement: Every loop configuration is real-time safe
The envelope SHALL produce one finite output value in [0, 1] per sample using a bounded amount of work for every combination of loop mode (Off, Forward, PingPong), loop start stage (0–5) and loop end stage (0–5), with the gate held or released.

#### Scenario: Previously crashing configuration
- **WHEN** loop mode is Forward, loop start stage is 4 (Sustain), loop end stage is 4, and a note is held for 2 seconds
- **THEN** the envelope returns 2 seconds of finite samples without stack overflow or hang

#### Scenario: Exhaustive parameter sweep
- **WHEN** every combination of loop mode 0–2, start 0–5 and end 0–5 is run for 1 second held plus 1 second released at 48 kHz
- **THEN** every combination completes and every sample is finite and in [0, 1]

### Requirement: Loop region is defined by start and end stages
With looping enabled and the gate held, reaching the loop end stage SHALL restart the loop start stage. Stages are numbered 0 Onset, 1 Attack, 2 Hold, 3 Decay, 4 Sustain, 5 Release. A start of 0 is treated as 1 and a start or end of 5 as 4, since Onset and Release are never looped.

#### Scenario: Default loop Decay to Sustain
- **WHEN** loop mode is Forward with start 3 and end 4 and the gate is held
- **THEN** the envelope repeats the Decay stage each time it reaches Sustain, producing output identical to the behavior before this change

#### Scenario: Custom loop end
- **WHEN** loop mode is Forward with start 1 (Attack) and end 3 (Decay) and the gate is held
- **THEN** on reaching Decay the envelope restarts Attack, and it never enters Decay or Sustain while the gate stays held

### Requirement: Degenerate loop regions hold instead of looping
When the loop start stage is greater than or equal to the effective loop end stage, the envelope SHALL behave as if looping were off.

#### Scenario: Start equals end
- **WHEN** loop mode is Forward with start 4 and end 4 and the gate is held
- **THEN** the envelope runs Onset through Decay once and then holds the sustain level

#### Scenario: Start after end
- **WHEN** loop mode is Forward with start 3 and end 1 and the gate is held
- **THEN** the envelope output is identical to the same settings with loop mode Off

### Requirement: Releasing the gate exits the loop
Releasing the gate SHALL move the envelope to the Release stage from any looping stage, and the envelope SHALL reach Idle after the release time.

#### Scenario: Release during loop
- **WHEN** a Forward 3 → 4 loop is running and the gate is released
- **THEN** the envelope enters Release, stops looping, and becomes idle within the release time plus one buffer

### Requirement: Zero stage level is silence
When the envelope's current stage level is 0, its output SHALL be exactly 0 for every velocity and every `velocity_to_level` setting.

#### Scenario: Low velocity at zero level
- **WHEN** an envelope with sustain level 0 and `velocity_to_level` 1 is triggered at velocity 0 and held past its decay
- **THEN** every output sample during sustain is 0.0

#### Scenario: Velocity-to-level off
- **WHEN** `velocity_to_level` is 0 and the envelope is held at a sustain level of 0.5
- **THEN** the sustain output is 0.5 (no added offset), for any velocity

### Requirement: Velocity scales level multiplicatively
The output SHALL equal `level · ((1 − velocity_to_level) + velocity_to_level · velocity_scale) · env_amount`, where `velocity_scale = 0.5 + velocity / 2` for normalized velocity in [0, 1].

#### Scenario: Half-scale at velocity zero
- **WHEN** `velocity_to_level` is 1, the velocity is 0, and the current level is 1.0
- **THEN** the output is 0.5 · env_amount

#### Scenario: Full velocity gives the stage level
- **WHEN** the velocity is 1.0
- **THEN** the output equals level · env_amount exactly, for any `velocity_to_level` and `env_amount`

### Requirement: Release ends without a step
The envelope SHALL reach Idle from Release without a discontinuity: the last Release sample and the first Idle sample differ by no more than the largest single-sample change during that Release.

#### Scenario: Release end at low velocity
- **WHEN** a held envelope at velocity 0.3 is released and processed until it goes Idle
- **THEN** the step from the last Release sample to the first Idle sample (0.0) is no larger than the largest sample-to-sample change within the Release stage

### Requirement: Release starts from the current level
Release SHALL ramp from the level the envelope had when the gate closed, so releasing before Sustain is reached does not jump.

#### Scenario: Release during Attack or Decay
- **WHEN** the gate closes during Attack or Decay, with the sustain level well below the current level
- **THEN** the first Release sample differs from the last pre-release sample by no more than the largest single-sample change before the release

### Requirement: Onset time and onset level are separate controls
The friction envelope's Onset stage SHALL take its duration from Onset Time and its level from Onset Level, so changing one never changes the other.

#### Scenario: Longer onset, same level
- **WHEN** Onset Time is changed from 0.01 s to 0.5 s with Onset Level fixed at 0.2
- **THEN** the envelope starts at the same level, 0.2, and only the Onset stage's duration changes

#### Scenario: Default patch unchanged
- **WHEN** a friction note is played with default controls
- **THEN** its output is bit-identical to the same note before Onset Level existed

### Requirement: Older state keeps its onset level
A user preset or saved project that has no Onset Level SHALL load with Onset Level equal to its Onset Time value clamped to 0–1, through both the preset loader and the host's state restore, so it sounds as it did.

#### Scenario: Old preset with a long onset
- **WHEN** a preset saved before Onset Level existed, with Onset 0.4, is loaded
- **THEN** Onset Time is 0.4 s and Onset Level is 0.4

#### Scenario: Old project restored by the host
- **WHEN** a saved project whose parameter state has `mseg_onset` 0.4 and no `mseg_onset_level` is restored
- **THEN** Onset Level is 0.4 and a friction note renders as it did before the change

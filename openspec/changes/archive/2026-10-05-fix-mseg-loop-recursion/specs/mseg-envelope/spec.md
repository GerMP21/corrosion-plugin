# Spec Delta

## Purpose

Defines how the multi-stage envelope used by friction-family exciters sequences its stages and loops, and guarantees that every host-reachable loop configuration is safe to run on the audio thread.

## ADDED Requirements

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

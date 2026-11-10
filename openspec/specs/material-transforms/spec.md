# material-transforms Specification

## Purpose
Defines how the Rust, Damage and Heat controls reshape a note across their whole ranges: each must stay audible and keep its fundamental, and each must change the sound in the way its name describes.

## Requirements

### Requirement: Rust darkens without silencing
Across Rust's whole range (0–5), every object SHALL keep its fundamental mode and stay within 6 dB of its Rust 0 level, while higher Rust shortens the high modes more than the low ones.

#### Scenario: Rust sweep
- **WHEN** each object plays C4 with HandStrike at velocity 100 for 2 s at Rust 0, 1, 2, 3, 4 and 5
- **THEN** every render's lowest mode is within a semitone of the Rust 0 lowest mode, every RMS is within 6 dB of the Rust 0 RMS, and no render is silent

#### Scenario: Rust eats the highs first
- **WHEN** the same note is rendered at Rust 0 and Rust 5
- **THEN** the ratio of the top mode's decay to the fundamental's decay is lower at Rust 5

### Requirement: Damage keeps the note
Across Damage's whole range (0–10), every object SHALL keep its fundamental within a semitone of the undamaged note and stay within 6 dB of its Damage 0 level, while damage splits modes into detuned pairs.

#### Scenario: Damage sweep
- **WHEN** each object plays C4 with HandStrike at velocity 100 for 2 s at Damage 0, 1, 2, 4, 6, 8 and 10
- **THEN** every render's lowest mode is within a semitone of the Damage 0 lowest mode, every RMS is within 6 dB of the Damage 0 RMS, and no render is silent

#### Scenario: Damage splits modes
- **WHEN** an object plays a note at Damage 0 and at Damage 5
- **THEN** the damaged bank has more modes than the undamaged one

### Requirement: Heat lowers, darkens and warbles
Heat SHALL lower the pitch, tilt the spectrum darker (high modes lose more level than low), and make the pitch wander slowly over time, while staying within 6 dB of the Heat 0 level.

#### Scenario: Darker
- **WHEN** each object plays C4 at Heat 0 and Heat 1
- **THEN** at Heat 1 the top mode's gain relative to the fundamental's is at least 6 dB lower, and the RMS is within 6 dB of Heat 0

#### Scenario: Warble
- **WHEN** a held note plays at Heat 1 for 3 s
- **THEN** the fundamental's frequency varies over time by at least 4 cents peak to peak, and at Heat 0 it doesn't vary

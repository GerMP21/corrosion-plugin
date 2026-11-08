# Spec Delta

## Purpose

Defines the editor's randomizer: what each mode changes, the bounds it keeps to, and how its changes reach the host.

## ADDED Requirements

### Requirement: Randomize within the mode's ranges
Pressing Randomize SHALL set Size, Rust, Damage, Drive, Width, BODY and Output to values inside the selected mode's ranges (Safe, Object, Damage or Full), and leave every other parameter unchanged.

#### Scenario: Safe mode bounds
- **WHEN** Randomize is pressed 100 times in Safe mode
- **THEN** every result has Size 0.5–1.5, Rust 0–0.5, Damage 0–0.5, Drive 0–0.5, Width 0.3–0.8, BODY 0–2.5 and Output 0.5–1.0, and every other parameter keeps its value

#### Scenario: Results vary
- **WHEN** Randomize is pressed twice in Full mode
- **THEN** the two results differ in at least one randomized parameter

### Requirement: Randomized patches are safe
Every randomized patch SHALL satisfy the randomizer's safety constraints: audible output, no extreme damage combined with extreme drive, and Size within 0.1–3.0.

#### Scenario: Full mode never produces an unsafe patch
- **WHEN** Randomize is pressed 1000 times in Full mode
- **THEN** every result passes the safety constraints

### Requirement: Randomizing is undoable in the host
Each parameter the randomizer changes SHALL be reported to the host as a complete parameter gesture, so the host's undo and automation recording treat it like a user edit.

#### Scenario: Host sees the change
- **WHEN** Randomize is pressed while the host records automation
- **THEN** the host receives a begin, a set and an end gesture for each changed parameter

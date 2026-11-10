# Spec Delta

## ADDED Requirements

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

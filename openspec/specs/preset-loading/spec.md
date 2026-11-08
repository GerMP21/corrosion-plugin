# preset-loading Specification

## Purpose
Defines what loading a preset changes, so a preset sounds the same whether the editor or the host's state restore loads it.

## Requirements

### Requirement: Loading a preset sets every sound parameter
Loading a preset from the editor SHALL set every plugin parameter to the preset's value, except UI Scale, which is an editor preference.

#### Scenario: Non-default play mode, limiter and macros
- **WHEN** a preset storing Kit play mode, the lookahead limiter and non-neutral values for all four macros is loaded from the editor while every parameter holds a different value
- **THEN** play mode, limiter mode and the four macros take the preset's values

#### Scenario: Every parameter
- **WHEN** a preset whose every parameter differs from the live values is loaded from the editor
- **THEN** every parameter except UI Scale equals the preset's value, and UI Scale is unchanged

### Requirement: Editor and host loads agree
A preset loaded from the editor SHALL leave every sound parameter at the same value as the host's state restore of the same preset.

#### Scenario: Same preset, two paths
- **WHEN** the same preset is applied through the editor's load path and through the host's state restore
- **THEN** every parameter except UI Scale has the same normalized value on both paths

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

### Requirement: Preset-built parameters display like the plugin's own
Every parameter built from a preset SHALL have the same name, range and value display as the plugin's own parameter with the same ID.

#### Scenario: Every parameter
- **WHEN** a factory preset is turned into parameters and compared with the plugin's default parameters, ID by ID, at the preset's values
- **THEN** each pair has the same name, maps the same plain value to the same normalized value, and displays it with the same text

#### Scenario: UI Scale and Filter Cutoff
- **WHEN** a preset with UI Scale 2 and Filter Cutoff 1 000 Hz is turned into parameters
- **THEN** UI Scale displays as "100%" and Filter Cutoff displays the same text as the plugin's own Filter Cutoff at 1 000 Hz

### Requirement: An out-of-range exciter loads as the default exciter
A preset whose exciter value is outside 1–16 SHALL load with the default exciter, HandStrike, and every value from 1 to 16 SHALL load as its own exciter.

#### Scenario: Exciter 0 and 17
- **WHEN** presets with exciter 0 and exciter 17 are loaded
- **THEN** both play HandStrike

#### Scenario: Every valid exciter
- **WHEN** presets with exciters 1 to 16 are loaded
- **THEN** each plays a different exciter, and saving it again stores the same number

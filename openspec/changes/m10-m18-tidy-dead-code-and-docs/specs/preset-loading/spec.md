# Spec Delta

## ADDED Requirements

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

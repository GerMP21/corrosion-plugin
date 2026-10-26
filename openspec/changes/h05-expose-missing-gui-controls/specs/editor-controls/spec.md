# Spec Delta

## Purpose

Defines which plugin parameters the editor must let the user see and change, so no parameter that changes the sound is reachable only through host automation.

## ADDED Requirements

### Requirement: Sound-shaping parameters have editor controls
The editor SHALL show a control for each of: resonator Brightness, Thickness, Sync, Play Mode, Limiter Mode, and the Mass, Corrosion, Violence and Brightness macros. Changing a control SHALL change its parameter, and automating the parameter SHALL move the control.

#### Scenario: Control changes the parameter
- **WHEN** the user drags or selects each of these controls in the editor
- **THEN** the corresponding plugin parameter takes the new value

#### Scenario: Automation moves the control
- **WHEN** the host automates each of these parameters
- **THEN** the editor's control shows the automated value

### Requirement: Every audible parameter is reachable
Every plugin parameter that affects the sound SHALL either have an editor control or be listed, with a reason, as intentionally host-only.

#### Scenario: Coverage check
- **WHEN** the editor's controls are compared with the plugin's parameter list
- **THEN** every parameter is either shown in the editor or on the documented host-only list

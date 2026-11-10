# editor-controls Specification

## Purpose
Defines which plugin parameters the editor must let the user see and change, so no parameter that changes the sound is reachable only through host automation.

## Requirements

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

### Requirement: Parameter controls explain themselves on hover
Each parameter control the editor draws (knob, fader or slider) SHALL show a small information badge (a circled "i") immediately to the right of its label. The badge SHALL be drawn in the same muted colour as the label text, and it SHALL scale with the editor's UI scale. Hovering the badge SHALL show a tooltip of one or two sentences that says what the parameter does and how the sound changes as it goes up. Hovering or dragging the control itself SHALL NOT show the tooltip.

#### Scenario: Hovering the badge shows help
- **WHEN** the pointer rests on the information badge next to a control's label
- **THEN** a tooltip with that parameter's help text appears

#### Scenario: Hovering the control does not show help
- **WHEN** the pointer rests on, or drags, the knob, fader or slider itself, outside the badge
- **THEN** no help tooltip appears

#### Scenario: Badge scales with the UI
- **WHEN** the UI scale changes
- **THEN** the badge's size changes in proportion, and the editor content still fits the window width

### Requirement: Slot help follows the selected model
For exciter and object slot controls, whose meaning depends on the selected exciter or object, the help text SHALL describe the slot's meaning for the model that is currently selected.

#### Scenario: Help changes with the exciter
- **WHEN** the user switches the exciter from one model to another
- **THEN** each exciter slot's tooltip describes that slot as the new model uses it

#### Scenario: Help changes with the object
- **WHEN** the user switches the object
- **THEN** each object slot's tooltip describes that slot as the new object uses it

### Requirement: Selectors show trial help for the selected option
As a trial feature that may be removed later, each selector (combo box) SHALL show the same information badge to the right of its label. Its tooltip SHALL describe the currently selected option in one or two sentences.

#### Scenario: Selector help describes the selection
- **WHEN** the pointer rests on a selector's information badge
- **THEN** the tooltip describes the option that is currently selected

### Requirement: Every drawn control has help text
Every parameter control and selector that the editor draws, for every exciter, object and Space mode, SHALL have non-empty help text.

#### Scenario: Help coverage check
- **WHEN** the editor is rendered for every exciter, object and Space mode, and the drawn controls are collected
- **THEN** every drawn control has non-empty help text of at most two sentences

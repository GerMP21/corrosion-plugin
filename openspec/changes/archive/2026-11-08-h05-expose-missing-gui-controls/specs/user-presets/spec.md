# Spec Delta

## Purpose

Defines how users save their own sounds as presets, where those presets live, and how they are listed and loaded alongside the factory bank.

## ADDED Requirements

### Requirement: Save the current sound as a user preset
The editor SHALL save the current parameters as a named preset file in the user's preset folder. A later load of that preset SHALL restore every parameter that a preset load sets.

#### Scenario: Save and reload
- **WHEN** the user saves the current sound as "My Grind", changes several parameters, then loads "My Grind"
- **THEN** every parameter returns to the value it had when saved, except UI Scale

#### Scenario: Name that is not a valid file name
- **WHEN** the user saves with a name containing path separators or characters invalid in file names
- **THEN** the preset is saved under a sanitized file name inside the user preset folder, keeps the typed name as its display name, and nothing is written outside that folder

### Requirement: Overwriting asks first
Saving under the name of an existing user preset SHALL ask for confirmation, and SHALL NOT change the file if the user declines.

#### Scenario: Declined overwrite
- **WHEN** the user saves under an existing user preset's name and declines the confirmation
- **THEN** the existing file is unchanged

### Requirement: User presets are listed after the factory bank
The editor's preset list SHALL show the factory presets, then the user's presets, including one saved during the current session.

#### Scenario: New preset appears
- **WHEN** the user saves a new preset
- **THEN** it appears in the user section of the preset list without reopening the editor

### Requirement: Save failures are reported
If a preset can't be saved, the editor SHALL show an error and SHALL NOT crash or block audio.

#### Scenario: Folder not writable
- **WHEN** the user preset folder can't be created or written
- **THEN** the editor shows an error message, and audio processing continues uninterrupted

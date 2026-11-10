# user-presets Specification

## Purpose
Defines how users save their own sounds as presets, where those presets live, and how they are listed and loaded alongside the factory bank.

## Requirements

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
The editor's preset list SHALL show one submenu per factory folder, with the experimental folder nested by exciter, followed by a User submenu that shows the user's presets, including one saved during the current session. The User submenu SHALL mirror the subfolders of the user preset folder as nested submenus, and presets in the folder's root SHALL appear directly in it. The previous and next arrows SHALL step through the presets of the folder that holds the current preset.

#### Scenario: New preset appears
- **WHEN** the user saves a new preset
- **THEN** it appears in the User submenu, in the folder it was saved to, without reopening the editor

#### Scenario: Factory folders are submenus
- **WHEN** the preset list is opened
- **THEN** it shows the submenus Bass, Lead, Chords, Drones, Percussion and Experimental, then User, and Experimental opens one submenu per exciter

#### Scenario: User subfolders are submenus
- **WHEN** the user preset folder contains a subfolder `Basses` with a preset in it, and a preset in its root
- **THEN** the User submenu shows a `Basses` submenu holding that preset, and the root preset directly

#### Scenario: Arrows stay in the folder
- **WHEN** the current preset is the last one in the Bass folder and the next arrow is pressed
- **THEN** the first Bass preset loads

### Requirement: Save failures are reported
If a preset can't be saved, the editor SHALL show an error and SHALL NOT crash or block audio.

#### Scenario: Folder not writable
- **WHEN** the user preset folder can't be created or written
- **THEN** the editor shows an error message, and audio processing continues uninterrupted

### Requirement: Saving targets a user folder
Save SHALL write into the user preset folder or one of its subfolders, chosen in a folder selector next to the name. The selector SHALL offer only user folders, and no save SHALL ever write into the factory bank.

#### Scenario: Save into a subfolder
- **WHEN** the user picks the subfolder `Basses` and saves "Deep Sub"
- **THEN** the file is written in `Basses` inside the user preset folder, and "Deep Sub" appears under User › Basses

#### Scenario: The factory bank is never a target
- **WHEN** the folder selector is opened
- **THEN** it lists only the user preset folder and its subfolders

### Requirement: Open the user preset folder
The editor SHALL show an Open folder button next to the user presets. Pressing it SHALL create the user preset folder if it is missing and show it in the system file manager. A failure SHALL be reported in the editor, and SHALL NOT crash or block audio.

#### Scenario: The folder opens
- **WHEN** the user presses Open folder
- **THEN** the platform's file manager command is run with the user preset folder's path

#### Scenario: No preset folder available
- **WHEN** the platform gives no user preset folder (its variables are unset)
- **THEN** the button is disabled and its tooltip says why

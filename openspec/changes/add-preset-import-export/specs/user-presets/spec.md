# Spec Delta

## ADDED Requirements

### Requirement: Import preset files
The editor SHALL show an Import… button in the preset browser. It SHALL open the system file dialog for one or more `.corrosion-preset` files. Each chosen file that loads as a preset SHALL be copied into the user folder selected in the browser's folder selector. A name clash SHALL be resolved with a numbered file name, never by overwriting. The last imported preset SHALL load. Files that don't load SHALL be skipped and counted in a message. The factory bank SHALL never be a target.

#### Scenario: A preset downloaded from the web app
- **WHEN** the user imports a preset file downloaded from the web app
- **THEN** the file appears in the selected user folder, it is listed in the User submenu, and it loads

#### Scenario: Name clash
- **WHEN** the user imports "Deep Sub" into a folder that already holds "Deep Sub"
- **THEN** the new file is saved as "Deep Sub (2)" and the existing file is unchanged

#### Scenario: A file that isn't a preset
- **WHEN** the user imports two files and one of them doesn't parse as a preset
- **THEN** the valid one is imported and loaded, and the editor reports one file skipped

### Requirement: Export the current sound
The editor SHALL show an Export… button. It SHALL open the system save dialog, proposing the current preset's name with the `.corrosion-preset` extension, and SHALL write the current sound to the chosen path in the format Save writes. A file exported this way SHALL open in the web app and import back into the plugin with every parameter that a preset load sets.

#### Scenario: Round trip
- **WHEN** the user exports the current sound, changes several parameters, then imports the exported file
- **THEN** every parameter returns to the value it had when exported, except UI Scale

### Requirement: Dialog failures never block
If the platform has no file dialog, or the dialog fails, Import… and Export… SHALL report it in the editor with a pointer to Open folder. They SHALL NOT hang the editor, crash, or interrupt audio.

#### Scenario: No portal on Linux
- **WHEN** Import… is pressed on a Linux desktop where no file dialog answers
- **THEN** the editor shows a message naming Open folder as the alternative, and audio continues uninterrupted

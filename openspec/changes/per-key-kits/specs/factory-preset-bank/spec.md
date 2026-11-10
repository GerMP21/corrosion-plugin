# Spec Delta

## MODIFIED Requirements

### Requirement: Folders and counts
The factory bank SHALL consist of the folders `bass`, `lead`, `chords`, `drones` and `percussion` with exactly 12 presets each, the folder `drums` with exactly 10 kit presets, and the folder `experimental`. Every factory preset SHALL load, and preset names SHALL be unique within a folder.

#### Scenario: Curated folders hold twelve presets
- **WHEN** the factory bank is loaded
- **THEN** each of `bass`, `lead`, `chords`, `drones` and `percussion` holds exactly 12 presets, and every one parses as a valid preset

#### Scenario: Ten drum kits
- **WHEN** the `drums` folder is loaded
- **THEN** it holds exactly 10 presets, each in Kit mode with pads on at least keys 36 (kick), 38 (snare), 42 (closed hat) and 46 (open hat), and every pad meets the quality floor when played alone

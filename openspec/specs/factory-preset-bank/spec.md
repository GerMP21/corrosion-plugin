# factory-preset-bank Specification

## Purpose
Defines the factory preset bank: its folders and their contents, the experimental exciter × resonator grid, the quality every factory preset meets, and the character each curated folder promises.

## Requirements

### Requirement: Folders and counts
The factory bank SHALL consist of the folders `bass`, `lead`, `chords`, `drones` and `percussion` with exactly 12 presets each, and the folder `experimental`. Every factory preset SHALL load, and preset names SHALL be unique within a folder.

#### Scenario: Curated folders hold twelve presets
- **WHEN** the factory bank is loaded
- **THEN** each of `bass`, `lead`, `chords`, `drones` and `percussion` holds exactly 12 presets, and every one parses as a valid preset

### Requirement: Experimental grid
The `experimental` folder SHALL hold exactly one preset for every pair of the 16 exciters and the 9 resonator objects (144 presets), grouped by exciter. The seeder SHALL generate them deterministically, so regenerating produces identical files, and SHALL vary every parameter that the pair's exciter and object use, within ranges that keep the preset playable.

#### Scenario: Every pair appears once
- **WHEN** the experimental folder is loaded
- **THEN** each (exciter, object) pair from 1–16 × 0–8 appears in exactly one preset

#### Scenario: Regeneration is deterministic
- **WHEN** the seeder runs twice
- **THEN** both runs write byte-identical experimental presets

#### Scenario: The parameters vary
- **WHEN** the experimental presets are compared
- **THEN** each exciter-specific parameter of an exciter takes more than one value across its 9 presets, and each shared parameter (size, rust, damage, envelopes, post chain) takes more than one value across the folder

### Requirement: Quality floor
Every factory preset SHALL render a held middle-C note through the plugin's output path with finite samples, an RMS level within 6 dB of the bank's target loudness, and no more than 1 % of samples at the output ceiling. Played at full velocity, the same note SHALL keep no more than 1 % of its samples at the ceiling and sit no more than 12 dB below the target.

#### Scenario: No silent, broken or clipping preset
- **WHEN** each factory preset renders a 2-second middle-C note offline at velocity 100
- **THEN** every sample is finite, the RMS is within ±6 dB of the target loudness, and at most 1 % of samples sit at the ceiling

#### Scenario: Headroom at full velocity
- **WHEN** each factory preset renders the same held note at velocity 127
- **THEN** every sample is finite, at most 1 % of samples sit at the ceiling, and the RMS before the limiter is no more than 12 dB below the target loudness

### Requirement: Folder character
Curated presets SHALL match their folder, measured on offline renders:
- `bass`: at C2 most of the energy is below 500 Hz;
- `lead` and `chords`: the strongest partial at middle C is within 50 cents of the played pitch or an octave of it;
- `chords`: also harmonic, sustaining and clean (defined in the scenario below);
- `drones`: they sustain while held;
- `percussion`: they decay on their own.

#### Scenario: Basses sit low
- **WHEN** a bass preset renders a held C2
- **THEN** at least 70 % of its spectral energy is below 500 Hz

#### Scenario: Leads and chords are pitched
- **WHEN** a lead or chord preset renders a held middle C
- **THEN** its strongest spectral peak is within 50 cents of C4, C5 or C3

#### Scenario: Chord patches are harmonic, sustaining and clean
- **WHEN** a chord preset renders a held middle C for 3 seconds
- **THEN** its first six partials lie within 15 cents of integer multiples of the fundamental, its level 2 seconds in is no more than 20 dB below its peak, and its spectral flatness stays below 0.1

#### Scenario: Drones sustain
- **WHEN** a drone preset holds a note for 8 seconds
- **THEN** its RMS over the last second is within 6 dB of its RMS over the second second

#### Scenario: Percussion decays
- **WHEN** a percussion preset plays a held note
- **THEN** its level 1.5 seconds after the onset is at least 30 dB below its peak, without the key being released

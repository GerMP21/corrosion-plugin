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

### Requirement: Quality floor
Every factory preset SHALL render a held middle-C note through the plugin's output path with finite samples, an RMS level within 6 dB of the bank's target loudness, and no more than 1 % of samples at the output ceiling. Played at full velocity, the same note SHALL keep no more than 1 % of its samples at the ceiling and sit no more than 12 dB below the target. A preset in Kit mode is silent on keys without a pad, so it SHALL instead meet the same bounds pad by pad: each pad played alone, held on its own key.

#### Scenario: No silent, broken or clipping preset
- **WHEN** each factory preset not in Kit mode renders a 2-second middle-C note offline at velocity 100
- **THEN** every sample is finite, the RMS is within ±6 dB of the target loudness, and at most 1 % of samples sit at the ceiling

#### Scenario: Headroom at full velocity
- **WHEN** each factory preset not in Kit mode renders the same held note at velocity 127
- **THEN** every sample is finite, at most 1 % of samples sit at the ceiling, and the RMS before the limiter is no more than 12 dB below the target loudness

#### Scenario: Kit presets meet the floor pad by pad
- **WHEN** each pad of a factory kit renders alone, a 2-second note held on its key, at velocity 100 and at velocity 127
- **THEN** both renders meet the bounds above: finite samples, RMS within ±6 dB of the target at 100 and no more than 12 dB below it before the limiter at 127, and at most 1 % of samples at the ceiling in each

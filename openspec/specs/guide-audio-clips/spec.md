# guide-audio-clips Specification

## Purpose
Audio examples in the user guide, rendered by the plugin itself, so readers can hear what each control and model does.

## Requirements

### Requirement: Clips are rendered by the plugin
Every guide clip SHALL be rendered by a repository binary through the plugin's own output path: voices, master drive, post chain, output and limiter, as `render_presets` does. The render runs at 48 kHz in Render quality, so each clip sounds as the plugin does.

#### Scenario: Regenerate
- **WHEN** the maintainer runs the clip generator after a sound change
- **THEN** every clip and the manifest are rewritten from the current engine

### Requirement: Parameter clips
Each parameter that shapes the sound SHALL have three clips over a fixed base patch: at its minimum, middle and maximum, with every other parameter at the base value. The note is C3 (MIDI 48) at velocity 100, 2–4 s long. Model-dependent slots SHALL be rendered with their model selected, and mode-dependent controls with their mode on.

#### Scenario: A slot of one exciter
- **WHEN** Bow Pressure's clips are rendered
- **THEN** the base patch has Bow selected, and only Bow Pressure differs between the three clips

### Requirement: Demos for context-dependent parameters
A parameter whose effect needs something else to be heard SHALL use a declared demo that exercises it:
- velocity sensitivities: two velocities;
- slow modulation: a longer note;
- tempo sync: a set tempo.

#### Scenario: Velocity sensitivity
- **WHEN** Vel to Level's clips are rendered
- **THEN** each clip plays the note at a soft and a hard velocity, so the setting's effect is audible

### Requirement: Every parameter is planned or excluded
Every host parameter SHALL be in the clip plan or in its exclusion list with a reason (for example UI Scale: no effect on the sound). A test in the CI lane SHALL fail otherwise.

#### Scenario: Unplanned parameter
- **WHEN** a parameter is added to the plugin and the clip plan has no entry or exclusion for it
- **THEN** the clip-plan coverage test fails, naming the parameter

### Requirement: Model and preset clips
Each exciter SHALL have a clip on one shared object, and each object a clip with one shared exciter, all on the same note. Each exciter and object page SHALL also have a clip of a representative factory preset. Each factory drum kit SHALL have a clip of its pads in turn.

#### Scenario: Comparing exciters
- **WHEN** a reader plays the 16 exciter clips
- **THEN** only the exciter differs between them (same object, note, velocity and length)

### Requirement: Level, format and manifest
Each set of clips (the three of a parameter, or a comparison set) SHALL share one gain, chosen so the set's reference clip sits at `TARGET_RMS`, lowered if needed to keep every clip's peak at or under −1 dBFS. Level differences inside a set stay audible. Clips SHALL be stereo OGG Opus. A manifest SHALL list every clip with its source (parameter or model, value, patch), RMS and peak before encoding, and the plan's fingerprint.

#### Scenario: Output parameter
- **WHEN** the Output parameter's three clips are rendered
- **THEN** they share one gain, so the minimum is quieter than the middle, as the control does

### Requirement: Clips are complete and audible
A test in the CI lane SHALL fail when:
- a planned clip is missing from the manifest or from disk;
- a clip's RMS is under −60 dBFS, unless the plan declares it as expected to be quiet, with a reason;
- the manifest's plan fingerprint differs from the current plan.

The guide check SHALL fail when a page references a clip that doesn't exist.

#### Scenario: Silent clip
- **WHEN** a change makes a parameter's minimum render silent, and the clips are regenerated
- **THEN** the audibility test fails, naming the clip, unless the plan declares it quiet

#### Scenario: Plan changed, clips not regenerated
- **WHEN** the base patch or a planned value changes and the clips are not regenerated
- **THEN** the fingerprint test fails and says to rerun the generator

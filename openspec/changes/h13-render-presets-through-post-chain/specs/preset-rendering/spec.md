# Spec Delta

## Purpose

Defines what offline preset renders must match, so the audition WAVs and the level checks hear exactly what the plugin outputs.

## ADDED Requirements

### Requirement: Offline renders equal the plugin's output
Rendering a preset offline SHALL produce, sample for sample, the same stereo output the plugin produces for the same preset, notes, sample rate and block of time, including the post chain, the macros, output gain and the selected limiter mode.

#### Scenario: Post-chain preset
- **WHEN** a factory preset that uses Space, the post-chain drive and BODY is rendered offline, and the same note is played through the plugin's processing path at the same sample rate
- **THEN** the two stereo outputs are bit-identical

#### Scenario: Lookahead limiter
- **WHEN** a preset is rendered with Limiter Mode set to Lookahead
- **THEN** the offline output equals the plugin's output, including the limiter's delay

### Requirement: The audition set reflects the plugin
The rendered audition files SHALL be produced by the offline renderer from the current factory presets.

#### Scenario: Regenerated audition set
- **WHEN** the preset render tool is run over the factory bank
- **THEN** it writes one WAV per preset whose samples equal the offline renderer's output for that preset, quantized to the file's bit depth

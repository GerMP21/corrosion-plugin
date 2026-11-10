# Proposal

## Why

Three parameter findings from the fable review (2026-07-07):

- **M1.** The friction MSEG's Onset knob is a time (0.001–1 s) and is also passed, clamped to 0–1, as the Onset stage's level. One knob sets two unrelated things: lengthening the onset also raises the level the envelope starts from.
- **M4.** Two parameters are called "Brightness" (resonator and macro) and two "Wire Stiffness" (wire brush and spring reverb). IDs are unique, so state is safe, but host automation lists show indistinguishable entries.
- **M5.** Output is a linear gain from 0 to 100 (up to +40 dB) with no dB display. The useful −20 to +6 dB region takes about 1 % of the knob's travel.

## What Changes

- **New parameter Onset Level** (`mseg_onset_level`, 0–1) sets the Onset stage's level; Onset (renamed "Onset Time" for display) sets only its duration. Default 0.01, today's default level, so the default patch doesn't change.
- **Migration:** user presets and saved projects without Onset Level get Onset Level = their old Onset value clamped to 0–1, so they sound as before (both the preset loader and `filter_state`).
- **Unique display names:** "Resonator Brightness" and "Macro Brightness"; "Brush Wire Stiffness" and "Spring Wire Stiffness"; "Onset Time" and "Onset Level". IDs don't change.
- **Output in dB:** shown and typed in dB, with a gain-skewed taper that puts 0 dB a little past the middle of the knob. The maximum stays +40 dB and the stored value stays the linear gain, so presets and projects play at the same level.
- **BREAKING:** existing host automation of Output follows the new taper (the same automation position now means a different gain). Automation lanes can't be migrated.

## Capabilities

### New Capabilities
- `host-parameters`: how parameters present themselves to the host: unique names, units and display, and the Output control's taper.

### Modified Capabilities
- `mseg-envelope`: adds that the Onset stage's level and time are separate controls, and how older state maps onto them.

## Impact

- **Code:** `src/params.rs` (new parameter, names, Output range and formatters), `src/voice/mod.rs` (Onset level from its own control), `src/lib.rs` (`VoiceControls` wiring, `filter_state` migration), `src/presets/mod.rs` (field, `migrate_preset_json`, `into_params`), `src/gui/editor.rs` (Onset Level control), `src/randomizer` (range for the new control), `src/bin/seed_presets.rs` (regenerate the factory bank).
- **Docs:** README (MSEG and Output), `docs/ARCHITECTURE.md`, the review's M1, M4 and M5 status.

# Design

## Context

- `Voice` (friction family) calls `mseg.set_stage_times(controls.mseg_onset, …)` and `mseg.set_stage_levels(controls.mseg_onset.clamp(0.0, 1.0), peak, sustain, 0.0)`.
- Host state stores plain parameter values by ID (`ParamValue::F32`), and `filter_state` already migrates one parameter this way (`migrate_host_wander_rate`). Preset files migrate in `migrate_preset_json` on the raw JSON; the schema stays at `"4"` for optional additive fields.
- `output` is `FloatParam` "Output", `FloatRange::Linear { 0, db_to_gain(40) }`, no unit or formatter. `Preset::into_params` rebuilds it with the same linear range.
- `editor-controls` requires every audible parameter to have an editor control (or be listed as host-only with a reason), and `patch-randomizer` keeps each parameter inside its mode's ranges.

## Goals / Non-Goals

**Goals:** one meaning per control; unambiguous host names; a usable Output taper in dB; old presets and projects unchanged in sound.

**Non-Goals:**
- Lowering Output's +40 dB ceiling (the maintainer kept it).
- Renaming parameter IDs.
- Renaming other parameters that are distinct but similar ("Mass" vs "Hand Mass").

## Decisions

1. **`mseg_onset_level`: `FloatParam` "Onset Level", linear 0–1, default 0.01.** The default equals today's default level (Onset Time default 0.01 s, clamped), so the default patch is bit-identical. *Alternative:* default 0 (start from silence). Cleaner, but it changes every patch that never touched Onset.
2. **Migration rule: Onset Level = clamp(Onset, 0, 1) when absent,** applied in `migrate_preset_json` (preset `extra` lacking `mseg_onset_level`) and in `filter_state` (host map lacking it). Same rule as H9's wander rate. New presets save the field.
3. **Names:** "Resonator Brightness", "Macro Brightness", "Brush Wire Stiffness", "Spring Wire Stiffness", "Onset Time", "Onset Level". `into_params` uses the same names. A test asserts every display name is unique.
4. **Output taper:** `FloatRange::Skewed { min: 0, max: db_to_gain(40), factor: FloatRange::gain_skew_factor(-60.0, 40.0) }`, unit " dB", `v2s_f32_gain_to_db(2)` and `s2v_f32_gain_to_db()`. This puts −10 dB at the knob's middle, 0 dB at about 57 % of the travel, and −20 to +6 dB across about 19 % (from 1 %). Gain 0 still reads −∞ dB. `into_params` builds the same range.
5. **Editor and randomizer:** an Onset Level knob beside Onset Time in the MSEG section; the randomizer treats it like the other MSEG levels (0–1).
6. **Factory presets** are regenerated with `seed_presets` so they carry the new field (they are placeholders; their sound isn't a constraint).

## Risks / Trade-offs

- **[BREAKING: Output automation]** → Called out in the proposal and the README; values in presets and projects are unaffected because the stored value is the gain.
- **[Hosts that cache parameter names show the old names until the project reloads]** → Cosmetic; IDs are unchanged.
- **[The migration can't tell an old file that left Onset at a long time on purpose]** → It doesn't need to: the rule reproduces the old level exactly, whatever the intent.

## Migration Plan

1. Preset files: `migrate_preset_json` adds `mseg_onset_level` from `mseg_onset` when missing.
2. Saved projects: `filter_state` adds `mseg_onset_level` from `mseg_onset` when missing.
3. Host automation of Output: not migratable (BREAKING, documented).

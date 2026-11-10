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

## Measurements

### Before (task 1.1)

| Check | Current code |
|---|---|
| Onset stage level with Onset Level 0.2 and Onset Time 0.01 s / 0.5 s | 0.010 / 0.500 (it follows the time) |
| Shared display names | "Brightness" (`res_brightness`, `macro_brightness`), "Wire Stiffness" (`brush_wire_stiffness`, `wire_stiffness`) |
| Output display at gain 1 | "1" (no unit) |
| Output knob position of 0 dB; span of −20 to +6 dB; −10 dB | 0.010; 0.019; 0.003 |
| Old preset / old project with Onset 0.4 and no Onset Level | Onset Level 0.01 (the default) |

Reference for the bit-identity check: `print_48k_exciter_render_hashes` on a clean worktree of the current code (gnu), exciters 1 and 9–12 are the friction family:

```
 1 0xe17759a0fedfec21   9 0xccbfd36d767722d3  10 0x7d89acefa233fa35
11 0x47921988fb9817dc  12 0x35eef15cb95ee67a
 2 0x5f2fd9cfe80cf8b3   3 0xa79576a0c51b3b5e   4 0x0fb53ddb23de6159
 5 0xad0b72a08b224000   6 0x31b405110c2b2e4c   7 0xb499c6ff3c260c09
 8 0xbe483283e18b51f3  13 0xdd9b3057f2a32add  14 0xdc000e609357e154
15 0x4639f12c4ac351ee  16 0xc4ad8765915bcb5d
```

### After

| Check | After |
|---|---|
| Onset stage level with Onset Level 0.2 and Onset Time 0.01 s / 0.5 s | 0.200 / 0.200, over 480 / 24 000 samples |
| Shared display names | none |
| Output display at gain 1 / 0.5 / 0 | "0.00 dB" / "−6.02 dB" / "-inf dB"; typing "-6" gives gain 0.501 |
| Output knob position of 0 dB; span of −20 to +6 dB; −10 dB | 0.575; 0.189; 0.500 |
| Old preset / old project with Onset 0.4 and no Onset Level | Onset Level 0.4 on both paths |
| Default renders, all 16 exciters (`print_48k_exciter_render_hashes`, gnu) | identical to the reference hashes above |

Checks: fmt, clippy `-D warnings` (CI lane), 410 tests (CI lane, gnu and musl), `cargo doc` without warnings, 366 lib tests on the GUI lane (no new clippy warnings there: still 13 `falling back to f32`), `no_alloc`, `preset_levels_are_sane`.

### As built

- **Randomizer (decision 5):** the randomizer touches none of the MSEG controls (only Size, Rust, Damage, Drive, Width, Body and Output), so Onset Level is left alone like the other MSEG levels; it needed no range.
- **Editor (decision 5):** the fader is labelled "Onset Lvl" next to "Onset" (the host names are "Onset Level" and "Onset Time"). The MSEG panel's first row now has four faders at the ADSR panel's spacing; the second row keeps three. The panel's height is unchanged.
- **Builders:** `mseg_onset_level_param` and `output_param` build the parameters for both the live set and `Preset::into_params`, so their ranges and formatters can't drift; `into_params_uses_the_live_names_and_output_range` checks names and the Output taper.
- **Factory presets:** regenerated; besides the new field, all 50 now store `ui_scale` 1 instead of 2 (the default had changed since the last seeding; `ui_scale` isn't applied when a preset loads).
- **The friction-note bit-identity check** is the render-hash comparison above, not an asserted test: hashes aren't portable across libm. `default_onset_level_is_the_old_shared_level` asserts the property that makes it hold.

# Preset levels after h10

Measured with `preset_levels_are_sane` (C4, velocity 127, one second held and one released, 48 kHz, through the real post chain), on a clean worktree of the commit before this change and on this change. "vs median" is against each run's own bank median.

## The five Factory-reverb presets

| Preset | RMS before | RMS after | Peak before | Peak after | Limiter hits | vs median after |
|---|---|---|---|---|---|---|
| Chain Bell Tower | 0.05404 | 0.05379 | 0.2804 | 0.2639 | 0 → 0 | −9.9 dB |
| Hollow Smokestack Hum | 0.14221 | 0.16611 | 0.3683 | 0.3698 | 0 → 0 | −0.1 dB |
| Plate Resonance Bath | 0.24015 | 0.17146 | 0.7314 | 0.5491 | 0 → 0 | +0.2 dB |
| Sheet Lightning | 0.15282 | 0.16278 | 0.9179 | 0.8380 | 0 → 0 | −0.3 dB |
| Sheet Metal Thunder | 0.15935 | 0.17774 | 0.9178 | 0.9182 | 0 → 0 | +0.5 dB |

None hits the limiter and none is more than 12 dB below the median, so the level pass trims none of them.

## Other presets

- **Hanging Chain Echo** (Echo, Delay Time 0.25): RMS 0.05867 → 0.05871, peak 0.2818 unchanged. In f32 its 0.215 s is 10 319.9995 samples at 48 kHz: the old code truncated it to 10 319, the new code rounds it to 10 320, so the echo comes one sample (0.02 ms) later.
- **The other 44 presets** render identically (same RMS, peak and hits).

## `PRESET_LEVELS`

Unchanged. `preset_levels_table` re-derives a 0.3 dB lower output for the five presets the floor rule raised (Cable Tension Rise, Sheet Bend, Suspension Cable Pluck, Thin Sheet Roll, Workshop Plate Scrape), because the bank median moved down slightly. The rule only requires a raise when a preset is below the floor, and these are now about 0.3 dB above it, so they are left as they are and stay bit-identical. `seed_presets` regenerates the bank with no diff.

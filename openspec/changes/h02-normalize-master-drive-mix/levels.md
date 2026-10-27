# Preset levels after `h02-normalize-master-drive-mix`

The 11 factory presets that use the master Drive, each rendered with its stored BODY and output (`preset_levels_are_sane`, 48 kHz, note 60 at full velocity).

- **Before:** commit `b2a3b72`, with the old mix (`shaped·d + dry·(1 − d/2)`).
- **After:** this change's crossfade (`shaped·m + dry·(1 − m)`, `m = min(d, 1)`).

No preset hits the limiter and none is more than 12 dB under the median, so no output trims were needed. The other 39 presets have Drive 0 and are unchanged.

| Preset | Drive | RMS before | RMS after | Change | Peak before | Peak after | Limiter hits |
|---|---|---|---|---|---|---|---|
| Beam Grind | 1.6 | 0.53978 | 0.48972 | -0.85 dB | 0.6442 | 0.6214 | 0 → 0 |
| Beam Strike Industrial | 1.0 | 0.17383 | 0.16826 | -0.28 dB | 0.5974 | 0.5895 | 0 → 0 |
| Chain Whip Snap | 1.4 | 0.09360 | 0.07787 | -1.60 dB | 0.7329 | 0.6831 | 0 → 0 |
| Cog Mesh Grind | 1.3 | 0.49879 | 0.45690 | -0.76 dB | 0.6797 | 0.6512 | 0 → 0 |
| Cog Tooth Strike | 1.0 | 0.18620 | 0.17350 | -0.61 dB | 0.3427 | 0.3067 | 0 → 0 |
| Ironforge Anvil Strike | 1.5 | 0.15858 | 0.14358 | -0.86 dB | 0.5897 | 0.5761 | 0 → 0 |
| Sheet Lightning | 1.5 | 0.17854 | 0.15282 | -1.35 dB | 0.9182 | 0.9179 | 0 → 0 |
| Sheet Metal Thunder | 1.2 | 0.17141 | 0.15935 | -0.63 dB | 0.9179 | 0.9178 | 0 → 0 |
| Snapped Cable | 1.2 | 0.34666 | 0.30756 | -1.04 dB | 0.6548 | 0.6369 | 0 → 0 |
| Spring Grind | 1.4 | 0.43237 | 0.38700 | -0.96 dB | 0.6484 | 0.6318 | 0 → 0 |
| Steel Tank Rumble | 1.0 | 0.31393 | 0.28857 | -0.73 dB | 0.6094 | 0.6007 | 0 → 0 |

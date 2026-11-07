# Preset levels after `resolve-exciter-tradeoffs`

Each factory preset rendered with its stored BODY and output (`preset_levels_table`, 48 kHz).

- **Before:** the presets as `normalize-object-loudness` left them, played on this change's engine.
- **After:** this pass. It used the same rules as before:
  - trim any preset that hits the limiter, in 5 % steps;
  - raise any preset more than 12 dB below the median RMS to 12 dB below.

  First, outputs above 1 (the earlier pass's raises, which compensated for exciters that are now at a common level) were reset to 1.

The median floor is RMS 0.04374.

| Preset | BODY | Output | RMS before | RMS after | Peak before | Peak after | Limiter hits before → after |
|---|---|---|---|---|---|---|---|
| Beam Drag | 0.48 | 1.000 | 0.18828 | 0.18828 | 0.3385 | 0.3385 | 0 → 0 |
| Beam Grind | 0.92 | 1.000 | 0.56714 | 0.56714 | 0.6774 | 0.6774 | 0 → 0 |
| Beam Strike Industrial | 0.97 | 1.000 | 0.17413 | 0.17413 | 0.5993 | 0.5993 | 0 → 0 |
| Boiler Pipe Tap | 2.57 | 1.000 | 0.17888 | 0.17888 | 0.3707 | 0.3707 | 0 → 0 |
| Boiler Tank Whoosh | 0.49 | 1.000 | 0.07824 | 0.07824 | 0.1039 | 0.1039 | 0 → 0 |
| Bow on Beam | 0.48 | 1.000 | 0.14960 | 0.14960 | 0.2660 | 0.2660 | 0 → 0 |
| Bowed Spring | 0.52 | 1.000 | 0.09328 | 0.09328 | 0.1308 | 0.1308 | 0 → 0 |
| Brine Tank Pulse | 0.48 | 1.000 | 0.23791 | 0.23791 | 0.3484 | 0.3484 | 0 → 0 |
| Brushed Beam | 1.90 | 1.000 | 0.24572 | 0.24572 | 0.3543 | 0.3543 | 0 → 0 |
| Cable Bow | 0.00 | 1.000 | 0.15334 | 0.04579 | 0.2622 | 0.0783 | 0 → 0 |
| Cable Scrape | 0.00 | 1.000 | 0.25121 | 0.09824 | 0.5827 | 0.2279 | 0 → 0 |
| Cable Tension Rise | 0.00 | 1.307 | 0.03055 | 0.04374 | 0.0792 | 0.1134 | 0 → 0 |
| Chain Bell Tower | 1.16 | 1.000 | 0.14867 | 0.05404 | 0.7713 | 0.2804 | 0 → 0 |
| Chain Drag | 0.73 | 1.000 | 0.08810 | 0.08810 | 0.1965 | 0.1965 | 0 → 0 |
| Chain Whip Snap | 2.01 | 0.950 | 0.09402 | 0.09402 | 0.7378 | 0.7378 | 0 → 0 |
| Cog Hum | 0.49 | 1.000 | 0.17951 | 0.17951 | 0.3178 | 0.3178 | 0 → 0 |
| Cog Mesh Grind | 1.22 | 1.000 | 0.50090 | 0.50090 | 0.6828 | 0.6828 | 0 → 0 |
| Cog Scrape | 0.50 | 1.000 | 0.11452 | 0.11452 | 0.2152 | 0.2152 | 0 → 0 |
| Cog Tooth Pluck | 1.39 | 1.000 | 0.26432 | 0.26432 | 0.4463 | 0.4463 | 0 → 0 |
| Cog Tooth Strike | 0.94 | 0.950 | 0.18652 | 0.18652 | 0.3437 | 0.3437 | 0 → 0 |
| Cold Pipe Bow | 0.53 | 1.000 | 0.09781 | 0.09781 | 0.1550 | 0.1550 | 0 → 0 |
| Deep Tank Bell | 1.29 | 1.000 | 0.22340 | 0.22340 | 0.4470 | 0.4470 | 0 → 0 |
| Distant Plate Roll | 2.68 | 1.000 | 0.14311 | 0.14311 | 0.4249 | 0.4249 | 0 → 0 |
| Drum Beam | 1.57 | 1.000 | 0.26303 | 0.26303 | 0.3903 | 0.3903 | 0 → 0 |
| Empty Drum Cavern | 2.55 | 1.000 | 0.38088 | 0.38088 | 0.4444 | 0.4444 | 0 → 0 |
| Falling Chain Cluster | 2.62 | 1.000 | 0.55989 | 0.13385 | 0.9661 | 0.3816 | 57848 → 0 |
| Hanging Chain Echo | 0.94 | 1.000 | 0.05867 | 0.05867 | 0.2818 | 0.2818 | 0 → 0 |
| Hollow Smokestack Hum | 0.47 | 1.000 | 0.14221 | 0.14221 | 0.3683 | 0.3683 | 0 → 0 |
| Industrial Plate Bow | 2.40 | 1.000 | 0.23467 | 0.23467 | 0.3413 | 0.3413 | 0 → 0 |
| Ironforge Anvil Strike | 0.78 | 1.000 | 0.16398 | 0.16398 | 0.6184 | 0.6184 | 0 → 0 |
| Liquid Tank Drone | 0.50 | 1.000 | 0.23873 | 0.23873 | 0.3507 | 0.3507 | 0 → 0 |
| Pipe Bell Cluster | 2.06 | 1.000 | 0.17364 | 0.17364 | 0.4024 | 0.4024 | 0 → 0 |
| Plate Resonance Bath | 2.65 | 1.000 | 0.24015 | 0.24015 | 0.7314 | 0.7314 | 0 → 0 |
| Pneumatic Whistle | 0.41 | 1.000 | 0.14506 | 0.14506 | 0.2844 | 0.2844 | 0 → 0 |
| Rusted Plate Strike | 2.58 | 1.000 | 0.20109 | 0.20109 | 0.4523 | 0.4523 | 0 → 0 |
| Sheet Bend | 0.00 | 1.833 | 0.07397 | 0.04374 | 0.2383 | 0.1409 | 0 → 0 |
| Sheet Bow | 2.65 | 1.000 | 0.33740 | 0.33740 | 0.5416 | 0.5416 | 0 → 0 |
| Sheet Brush | 2.67 | 1.000 | 0.29657 | 0.29657 | 0.4191 | 0.4191 | 0 → 0 |
| Sheet Lightning | 2.52 | 0.950 | 0.18880 | 0.17936 | 0.9661 | 0.9182 | 251 → 0 |
| Sheet Metal Thunder | 2.65 | 0.950 | 0.18097 | 0.17192 | 0.9661 | 0.9179 | 3 → 0 |
| Snapped Cable | 1.23 | 1.000 | 0.34846 | 0.34846 | 0.6588 | 0.6588 | 0 → 0 |
| Spring Drag | 0.52 | 1.000 | 0.08172 | 0.08172 | 0.1710 | 0.1710 | 0 → 0 |
| Spring Grind | 0.95 | 0.950 | 0.45003 | 0.45003 | 0.6745 | 0.6745 | 0 → 0 |
| Spring Pluck | 2.61 | 1.000 | 0.33539 | 0.33539 | 0.4370 | 0.4370 | 0 → 0 |
| Spring Reverb Tap | 2.26 | 0.950 | 0.08745 | 0.08745 | 0.3934 | 0.3934 | 0 → 0 |
| Steel Tank Rumble | 1.05 | 1.000 | 0.31467 | 0.31467 | 0.6113 | 0.6113 | 0 → 0 |
| Suspension Cable Pluck | 0.15 | 1.201 | 0.19803 | 0.04374 | 0.9661 | 0.2811 | 4554 → 0 |
| Sympathetic Chain | 2.48 | 1.000 | 0.10218 | 0.10218 | 0.4102 | 0.4102 | 0 → 0 |
| Thin Sheet Roll | 0.00 | 1.564 | 0.05271 | 0.04374 | 0.1147 | 0.0952 | 0 → 0 |
| Workshop Plate Scrape | 0.00 | 3.058 | 0.07197 | 0.04374 | 0.2174 | 0.1321 | 0 → 0 |

# Factory preset levels

Placeholder presets, levelled lightly (design Decision 4). Each is rendered at C4 and velocity 127 through the full chain: 1 s held, then 1 s released.

Columns:
- **Before:** the old engine, with the C5 levels, much of it clipped inside the voice.
- **Normalized:** the normalized engine with the stored `output` unchanged.
- **After:** the final `output`.

Rules:
- `output` changes only for a preset that hits the limiter (none does now) or sits more than 12 dB below the median RMS. Those are raised to 12 dB below it.
- The limiter-hit counts in the "Before" column are the C5 table's.

The median RMS was 0.3741 before and is 0.0667 after normalization (-15.0 dB). The −12 dB floor the level pass used is 0.01701 (12 dB under the upper-middle of the 50 RMS values).

| Preset | Output before → after | RMS before | RMS normalized | RMS after | dB after vs before | Peak after | Limiter hits before → after |
|---|---|---|---|---|---|---|---|
| Beam Drag | 1.000 → 1.000 | 0.52363 | 0.06563 | 0.06563 | -18.0 | 0.0875 | 0 → 0 |
| Beam Grind | 1.000 → 1.000 | 0.11008 | 0.42549 | 0.42549 | +11.7 | 0.6389 | 0 → 0 |
| Beam Strike Industrial | 1.000 → 1.000 | 0.58629 | 0.05628 | 0.05628 | -20.4 | 0.1431 | 0 → 0 |
| Boiler Pipe Tap | 1.000 → 1.000 | 0.12902 | 0.10110 | 0.10110 | -2.1 | 0.3527 | 0 → 0 |
| Boiler Tank Whoosh | 1.000 → 1.000 | 0.53003 | 0.05261 | 0.05261 | -20.1 | 0.0794 | 0 → 0 |
| Bow on Beam | 1.000 → 1.000 | 0.52385 | 0.06952 | 0.06952 | -17.5 | 0.0924 | 0 → 0 |
| Bowed Spring | 1.000 → 1.000 | 0.50670 | 0.04996 | 0.04996 | -20.1 | 0.0768 | 0 → 0 |
| Brine Tank Pulse | 1.000 → 1.000 | 0.52687 | 0.07783 | 0.07783 | -16.6 | 0.1084 | 0 → 0 |
| Brushed Beam | 1.000 → 1.000 | 0.37032 | 0.24653 | 0.24653 | -3.5 | 0.3192 | 0 → 0 |
| Cable Bow | 0.952 → 2.960 | 0.42342 | 0.00547 | 0.01701 | -27.9 | 0.0397 | 0 → 0 |
| Cable Scrape | 0.676 → 2.260 | 0.28857 | 0.00509 | 0.01701 | -24.6 | 0.0496 | 0 → 0 |
| Cable Tension Rise | 0.913 → 0.913 | 0.37793 | 0.06772 | 0.06772 | -14.9 | 0.1595 | 0 → 0 |
| Chain Bell Tower | 0.950 → 2.751 | 0.52614 | 0.00587 | 0.01701 | -29.8 | 0.1050 | 0 → 0 |
| Chain Drag | 1.000 → 1.000 | 0.45342 | 0.02651 | 0.02651 | -24.7 | 0.1000 | 0 → 0 |
| Chain Whip Snap | 0.950 → 0.950 | 0.14180 | 0.06938 | 0.06938 | -6.2 | 0.6064 | 0 → 0 |
| Cog Hum | 1.000 → 1.000 | 0.52852 | 0.07717 | 0.07717 | -16.7 | 0.1109 | 0 → 0 |
| Cog Mesh Grind | 1.000 → 1.000 | 0.10190 | 0.43688 | 0.43688 | +12.6 | 0.6538 | 0 → 0 |
| Cog Scrape | 1.000 → 1.000 | 0.52926 | 0.06898 | 0.06898 | -17.7 | 0.0971 | 0 → 0 |
| Cog Tooth Pluck | 1.000 → 1.000 | 0.42081 | 0.07944 | 0.07944 | -14.5 | 0.3086 | 0 → 0 |
| Cog Tooth Strike | 0.950 → 0.950 | 0.55769 | 0.03915 | 0.03915 | -23.1 | 0.1251 | 0 → 0 |
| Cold Pipe Bow | 1.000 → 1.000 | 0.50528 | 0.05263 | 0.05263 | -19.6 | 0.0816 | 0 → 0 |
| Deep Tank Bell | 1.000 → 1.000 | 0.40776 | 0.11738 | 0.11738 | -10.8 | 0.2849 | 0 → 0 |
| Distant Plate Roll | 1.000 → 1.000 | 0.04903 | 0.11416 | 0.11416 | +7.3 | 0.3884 | 0 → 0 |
| Drum Beam | 1.000 → 1.000 | 0.29371 | 0.07661 | 0.07661 | -11.7 | 0.2696 | 0 → 0 |
| Empty Drum Cavern | 1.000 → 1.000 | 0.30815 | 0.06290 | 0.06290 | -13.8 | 0.1154 | 0 → 0 |
| Falling Chain Cluster | 1.000 → 35.986 | 0.00181 | 0.00047 | 0.01701 | +19.5 | 0.1205 | 0 → 0 |
| Hanging Chain Echo | 1.000 → 1.000 | 0.33652 | 0.07665 | 0.07665 | -12.8 | 0.2339 | 0 → 0 |
| Hollow Smokestack Hum | 1.000 → 1.000 | 0.63572 | 0.13163 | 0.13163 | -13.7 | 0.2755 | 20006 → 0 |
| Industrial Plate Bow | 1.000 → 1.000 | 0.29549 | 0.06152 | 0.06152 | -13.6 | 0.1255 | 0 → 0 |
| Ironforge Anvil Strike | 1.000 → 1.000 | 0.59690 | 0.04927 | 0.04927 | -21.7 | 0.1227 | 0 → 0 |
| Liquid Tank Drone | 1.000 → 1.000 | 0.53198 | 0.09632 | 0.09632 | -14.8 | 0.1394 | 0 → 0 |
| Pipe Bell Cluster | 1.000 → 1.000 | 0.19267 | 0.10719 | 0.10719 | -5.1 | 0.2855 | 0 → 0 |
| Plate Resonance Bath | 1.000 → 1.000 | 0.28506 | 0.11956 | 0.11956 | -7.5 | 0.4574 | 0 → 0 |
| Pneumatic Whistle | 1.000 → 1.000 | 0.50800 | 0.06277 | 0.06277 | -18.2 | 0.1021 | 0 → 0 |
| Rusted Plate Strike | 1.000 → 1.000 | 0.11679 | 0.12087 | 0.12087 | +0.3 | 0.3465 | 0 → 0 |
| Sheet Bend | 0.570 → 2.739 | 0.23379 | 0.00354 | 0.01701 | -22.8 | 0.0555 | 0 → 0 |
| Sheet Bow | 1.000 → 1.000 | 0.14182 | 0.20952 | 0.20952 | +3.4 | 0.3517 | 0 → 0 |
| Sheet Brush | 1.000 → 1.000 | 0.30111 | 0.25367 | 0.25367 | -1.5 | 0.3776 | 0 → 0 |
| Sheet Lightning | 1.000 → 1.000 | 0.07507 | 0.06116 | 0.06116 | -1.8 | 0.4148 | 0 → 0 |
| Sheet Metal Thunder | 1.000 → 1.000 | 0.01930 | 0.05340 | 0.05340 | +8.8 | 0.3704 | 0 → 0 |
| Snapped Cable | 1.000 → 1.000 | 0.41817 | 0.26471 | 0.26471 | -4.0 | 0.6420 | 0 → 0 |
| Spring Drag | 1.000 → 1.000 | 0.48682 | 0.03225 | 0.03225 | -23.6 | 0.0755 | 0 → 0 |
| Spring Grind | 0.950 → 0.950 | 0.11399 | 0.36125 | 0.36125 | +10.0 | 0.6215 | 0 → 0 |
| Spring Pluck | 1.000 → 1.000 | 0.02590 | 0.04672 | 0.04672 | +5.1 | 0.1345 | 0 → 0 |
| Spring Reverb Tap | 0.950 → 0.950 | 0.56506 | 0.02409 | 0.02409 | -27.4 | 0.0861 | 0 → 0 |
| Steel Tank Rumble | 1.000 → 1.000 | 0.60923 | 0.13034 | 0.13034 | -13.4 | 0.2148 | 0 → 0 |
| Suspension Cable Pluck | 1.000 → 6.833 | 0.46502 | 0.00249 | 0.01701 | -28.7 | 0.0592 | 0 → 0 |
| Sympathetic Chain | 1.000 → 1.000 | 0.17424 | 0.05663 | 0.05663 | -9.8 | 0.3398 | 0 → 0 |
| Thin Sheet Roll | 0.627 → 1.666 | 0.20547 | 0.00640 | 0.01701 | -21.6 | 0.0712 | 0 → 0 |
| Workshop Plate Scrape | 0.665 → 4.448 | 0.28057 | 0.00254 | 0.01701 | -24.3 | 0.0662 | 0 → 0 |

Raised to the −12 dB floor (8): Cable Bow, Cable Scrape, Chain Bell Tower, Falling Chain Cluster, Sheet Bend, Suspension Cable Pluck, Thin Sheet Roll, Workshop Plate Scrape.

No preset hits the limiter after the change; before it, 1 did (with the limiter counts the C5 table recorded). `preset_levels_are_sane` checks both rules. That check is ignored, like the other render-heavy checks.

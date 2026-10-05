# Pre-C5 factory preset baseline

Per-preset post-chain output measured on commit `45dd964` (before C5's routing
change), copied from `openspec/changes/fix-post-chain-body-routing/baseline.md`
(see there for method and column definitions). The ignored harness tests in
`src/lib.rs` (`c5_measurements`) read RMS and limiter hits from this table to
check that every re-voiced preset stays within 0.5 dB of its old level and
hits the limiter no more often.

| preset | peak | rms | limiter hits | old body/dry | implied BODY |
| Beam Drag | 0.6938 | 0.52362 | 0 | 3.101 | 6.33 |
| Beam Grind | 0.7814 | 0.11008 | 0 | 1.565 | 4.71 |
| Beam Strike Industrial | 0.6458 | 0.58627 | 0 | 1.733 | 4.78 |
| Boiler Pipe Tap | 0.3569 | 0.12901 | 0 | 5475.155 | 8.75 |
| Boiler Tank Whoosh | 0.5362 | 0.53003 | 0 | 3.062 | 6.32 |
| Bow on Beam | 0.6586 | 0.52384 | 0 | 3.111 | 6.34 |
| Bowed Spring | 0.5480 | 0.50670 | 0 | 3.483 | 6.45 |
| Brine Tank Pulse | 0.6306 | 0.52689 | 0 | 3.060 | 6.32 |
| Brushed Beam | 0.5013 | 0.37030 | 0 | 54.385 | 8.74 |
| Cable Bow | 0.5565 | 0.42342 | 0 | 3.055 | 6.30 |
| Cable Scrape | 0.5454 | 0.28857 | 0 | 2.876 | 6.22 |
| Cable Tension Rise | 0.6012 | 0.37793 | 0 | 3.512 | 6.38 |
| Chain Bell Tower | 0.9661 | 0.55356 | 44943 | 7.229 | 6.55 |
| Chain Drag | 0.6938 | 0.45341 | 0 | 4.257 | 6.41 |
| Chain Whip Snap | 0.6732 | 0.14927 | 0 | 13.684 | 6.75 |
| Cog Hum | 0.6938 | 0.52854 | 0 | 3.063 | 6.32 |
| Cog Mesh Grind | 0.6344 | 0.10190 | 0 | 1.666 | 4.40 |
| Cog Scrape | 0.6938 | 0.52925 | 0 | 3.068 | 6.27 |
| Cog Tooth Pluck | 0.5369 | 0.42079 | 0 | 10.249 | 6.62 |
| Cog Tooth Strike | 0.7074 | 0.58704 | 0 | 1.827 | 4.87 |
| Cold Pipe Bow | 0.5416 | 0.50528 | 0 | 3.563 | 6.47 |
| Deep Tank Bell | 0.4775 | 0.40776 | 0 | 20.041 | 8.75 |
| Distant Plate Roll | 0.2928 | 0.04903 | 0 | 1559.499 | 8.74 |
| Drum Beam | 0.5379 | 0.29372 | 0 | 19.991 | 7.91 |
| Empty Drum Cavern | 0.3484 | 0.30815 | 0 | 6139.484 | 8.75 |
| Falling Chain Cluster | 0.0126 | 0.00181 | 0 | 33447.562 | 8.67 |
| Hanging Chain Echo | 0.7575 | 0.33651 | 0 | 9.194 | 7.17 |
| Hollow Smokestack Hum | 0.9661 | 0.63571 | 20813 | 3.067 | 6.33 |
| Industrial Plate Bow | 0.3795 | 0.29549 | 0 | 291.229 | 8.75 |
| Ironforge Anvil Strike | 0.6805 | 0.59690 | 0 | 1.493 | 4.80 |
| Liquid Tank Drone | 0.6159 | 0.53198 | 0 | 3.069 | 6.33 |
| Pipe Bell Cluster | 0.4024 | 0.19268 | 0 | 101.095 | 8.75 |
| Plate Resonance Bath | 0.8602 | 0.28504 | 0 | 2108.691 | 8.69 |
| Pneumatic Whistle | 0.5798 | 0.50800 | 0 | 3.049 | 6.31 |
| Rusted Plate Strike | 0.3444 | 0.11679 | 0 | 17261.049 | 8.75 |
| Sheet Bend | 0.6924 | 0.23379 | 0 | 2.832 | 6.11 |
| Sheet Bow | 0.5323 | 0.14181 | 0 | 0.570 | 1.18 |
| Sheet Brush | 0.5337 | 0.30110 | 0 | 2.356 | 1.53 |
| Sheet Lightning | 0.4305 | 0.07507 | 0 | 908.289 | 8.84 |
| Sheet Metal Thunder | 0.2901 | 0.01930 | 0 | 3785.380 | 8.74 |
| Snapped Cable | 0.5881 | 0.41818 | 0 | 7.890 | 6.58 |
| Spring Drag | 0.5766 | 0.48683 | 0 | 3.812 | 6.53 |
| Spring Grind | 0.8700 | 0.11999 | 0 | 1.551 | 4.62 |
| Spring Pluck | 0.0746 | 0.02590 | 0 | 52888.523 | 8.73 |
| Spring Reverb Tap | 0.9661 | 0.56506 | 25758 | 10.194 | 6.57 |
| Steel Tank Rumble | 0.6852 | 0.60924 | 0 | 1.578 | 4.60 |
| Suspension Cable Pluck | 0.5381 | 0.46501 | 0 | 3.063 | 6.30 |
| Sympathetic Chain | 0.4010 | 0.17425 | 0 | 875.732 | 8.72 |
| Thin Sheet Roll | 0.5801 | 0.20547 | 0 | 1.832 | 5.91 |
| Workshop Plate Scrape | 0.5375 | 0.28057 | 0 | 2.975 | 6.25 |

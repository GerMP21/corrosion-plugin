# Baseline measurements (pre-C5 routing)

Captured 2026-10-05 on commit `45dd964` (after C1–C4), with
`cargo test --release --no-default-features --lib c5_preset_measurements -- --ignored --nocapture`
(`src/lib.rs`, module `c5_measurements`). Each preset plays MIDI note 60 at full velocity
for 1 s, then releases for 1 s, at 48 kHz, through the real voice path and post chain
(`Corrosion::apply_post_chain_params`, the same wiring `process()` uses).

- **peak / rms**: post-chain output after output gain and the hard limiter.
- **limiter hits**: samples whose pre-limiter magnitude exceeded `LIMITER_THRESHOLD` (0.9661).
- **old body/dry**: energy ratio E(0.7·body) / E(0.3·dry) of the pre-C5 mono routing,
  measured with a stage-by-stage replica (filter → drive → body on the mono sum).
- **implied BODY**: the BODY value (0–5 → 0–100 %) that would give the same body/dry energy
  ratio under the new parallel routing *with the body at unity gain*. Estimated from the
  mono replica, so only indicative; task 4.1 re-measures through the new chain.

Default-note reference: `hosted_normalized_note_velocity_reaches_output_chain` peak = **0.118**
(post-C1).

| preset | peak | rms | limiter hits | old body/dry | implied BODY |
|---|---|---|---|---|---|
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

## Reading

- Implied BODY ranges 1.18–8.84, median 6.41. Most of it sits **above**
  BODY's 0–5 range. The old routing mixed the body at 0.7 × 0.75 = 0.525 against 0.3 dry,
  a 1.75 gain ratio, so matching it at unity body gain needs `amount` ≈ 1.75 (BODY ≈ 8.75).
  The spread below that comes from presets whose filter/drive change the dry energy.
- Per design Decision 6, the fix is the body's internal gain constant, not BODY's mapping.
- Three presets already hit the limiter heavily *before* C5: Chain Bell Tower (44 943),
  Hollow Smokestack Hum (20 813), Spring Reverb Tap (25 758). Task 4.3 holds every preset to
  at most its count here.

## CPU: post-chain benchmarks (task 3.2)

`cargo bench --no-default-features --bench performance -- post_processing_chain --warm-up-time 1 --measurement-time 3`,
before (C1–C4 head) and after the C5 routing. Each iteration builds a chain and processes
4 096 samples (85 ms at 48 kHz); the median time is shown.

| bench | before | after | share of one core in real time (after) |
|---|---|---|---|
| `post_processing_chain` (Normal) | 643 µs | 844 µs | ≈ 1.0 % |
| `post_processing_chain_eco` | 210 µs | 574 µs | ≈ 0.67 % |
| `post_processing_chain_idle` | 358 µs | 380 µs | — |

Normal gains the second filter/drive pair (+31 %). Eco now runs body, spread and space
(+174 %), and stays the cheapest mode thanks to 1× clipper oversampling.


## Re-voicing (tasks 4.1–4.3)

**What changed from the plan.** The mono-replica estimate above was the wrong instrument. Measured
through the new chain, full BODY at unity body gain is already *louder* than the old mix (most
presets reach the limiter, RMS ≈ 0.95 against a baseline of ≈ 0.5). So no internal body-gain
change was needed (Decision 6). A single BODY value can't serve all presets (Decision 5): with
BODY 0, the dry output ranges from RMS 0.00003 (Falling Chain Cluster) to 0.58 (Hollow Smokestack
Hum), because struck presets relied on the body bank for nearly all of their level.

**Method** (`c5_revoice_table`, `src/lib.rs`): for each preset, bisect BODY until the output RMS
matches the baseline above. If the preset is already louder at BODY 0, keep BODY 0 and trim
`output` to the baseline RMS. Then trim `output` in 5 % steps until limiter hits are at or below
the baseline count. The table is applied by `src/bin/seed_presets.rs` (`C5_REVOICE`), and
regeneration otherwise reproduces the committed files byte for byte.

**Default patch** (`c5_default_body`): BODY default **2.62** reproduces the pre-C5 default-note
peak: 0.1179 against 0.118 (BODY 0 → 0.0028, BODY 5 → 0.2251).

**Verification** (`c5_verify_revoiced_presets`, stored presets as loaded from disk): every preset
is within 0.5 dB of its baseline RMS and hits the limiter no more than before. Chain Bell Tower
goes from 44 943 hits to 0, Spring Reverb Tap from 25 758 to 0, and Hollow Smokestack Hum from
20 813 to 20 006.

| preset | BODY | output | rms | Δ vs baseline | peak | limiter hits | baseline hits |
|---|---|---|---|---|---|---|---|
| Beam Drag | 0.48 | 1.000 | 0.52383 | +0.00 | 0.9123 | 0 | 0 |
| Beam Grind | 0.92 | 1.000 | 0.11010 | +0.00 | 0.8595 | 0 | 0 |
| Beam Strike Industrial | 0.97 | 1.000 | 0.58688 | +0.01 | 0.6925 | 0 | 0 |
| Boiler Pipe Tap | 2.57 | 1.000 | 0.12881 | -0.01 | 0.3862 | 0 | 0 |
| Boiler Tank Whoosh | 0.49 | 1.000 | 0.52988 | -0.00 | 0.5336 | 0 | 0 |
| Bow on Beam | 0.48 | 1.000 | 0.52368 | -0.00 | 0.8976 | 0 | 0 |
| Bowed Spring | 0.52 | 1.000 | 0.50715 | +0.01 | 0.5766 | 0 | 0 |
| Brine Tank Pulse | 0.48 | 1.000 | 0.52700 | +0.00 | 0.6812 | 0 | 0 |
| Brushed Beam | 1.90 | 1.000 | 0.37086 | +0.01 | 0.7121 | 0 | 0 |
| Cable Bow | 0.00 | 0.952 | 0.42355 | +0.00 | 0.4987 | 0 | 0 |
| Cable Scrape | 0.00 | 0.676 | 0.28862 | +0.00 | 0.3412 | 0 | 0 |
| Cable Tension Rise | 0.00 | 0.913 | 0.37809 | +0.00 | 0.5876 | 0 | 0 |
| Chain Bell Tower | 1.16 | 0.950 | 0.52619 | -0.44 | 0.9185 | 0 | 44943 |
| Chain Drag | 0.73 | 1.000 | 0.45357 | +0.00 | 0.9463 | 0 | 0 |
| Chain Whip Snap | 2.01 | 0.950 | 0.14199 | -0.43 | 0.9179 | 0 | 0 |
| Cog Hum | 0.49 | 1.000 | 0.52902 | +0.01 | 0.9154 | 0 | 0 |
| Cog Mesh Grind | 1.22 | 1.000 | 0.10194 | +0.00 | 0.6390 | 0 | 0 |
| Cog Scrape | 0.50 | 1.000 | 0.52956 | +0.01 | 0.9163 | 0 | 0 |
| Cog Tooth Pluck | 1.39 | 1.000 | 0.42085 | +0.00 | 0.6507 | 0 | 0 |
| Cog Tooth Strike | 0.94 | 0.950 | 0.55749 | -0.45 | 0.9178 | 0 | 0 |
| Cold Pipe Bow | 0.53 | 1.000 | 0.50547 | +0.00 | 0.5566 | 0 | 0 |
| Deep Tank Bell | 1.29 | 1.000 | 0.40801 | +0.01 | 0.6219 | 0 | 0 |
| Distant Plate Roll | 2.68 | 1.000 | 0.04900 | -0.01 | 0.2962 | 0 | 0 |
| Drum Beam | 1.57 | 1.000 | 0.29372 | -0.00 | 0.6743 | 0 | 0 |
| Empty Drum Cavern | 2.55 | 1.000 | 0.30817 | +0.00 | 0.3552 | 0 | 0 |
| Falling Chain Cluster | 2.62 | 1.000 | 0.00181 | +0.01 | 0.0126 | 0 | 0 |
| Hanging Chain Echo | 0.94 | 1.000 | 0.33670 | +0.01 | 0.8650 | 0 | 0 |
| Hollow Smokestack Hum | 0.47 | 1.000 | 0.63594 | +0.00 | 0.9661 | 20060 | 20813 |
| Industrial Plate Bow | 2.40 | 1.000 | 0.29555 | +0.00 | 0.4396 | 0 | 0 |
| Ironforge Anvil Strike | 0.78 | 1.000 | 0.59634 | -0.01 | 0.7791 | 0 | 0 |
| Liquid Tank Drone | 0.50 | 1.000 | 0.53205 | +0.00 | 0.7307 | 0 | 0 |
| Pipe Bell Cluster | 2.06 | 1.000 | 0.19244 | -0.01 | 0.4718 | 0 | 0 |
| Plate Resonance Bath | 2.65 | 1.000 | 0.28452 | -0.02 | 0.8391 | 0 | 0 |
| Pneumatic Whistle | 0.41 | 1.000 | 0.50848 | +0.01 | 0.6186 | 0 | 0 |
| Rusted Plate Strike | 2.58 | 1.000 | 0.11684 | +0.00 | 0.3459 | 0 | 0 |
| Sheet Bend | 0.00 | 0.570 | 0.23363 | -0.01 | 0.4345 | 0 | 0 |
| Sheet Bow | 2.65 | 1.000 | 0.14177 | -0.00 | 0.8098 | 0 | 0 |
| Sheet Brush | 2.67 | 1.000 | 0.30096 | -0.00 | 0.6025 | 0 | 0 |
| Sheet Lightning | 2.52 | 1.000 | 0.07511 | +0.01 | 0.4602 | 0 | 0 |
| Sheet Metal Thunder | 2.65 | 1.000 | 0.01930 | +0.00 | 0.2913 | 0 | 0 |
| Snapped Cable | 1.23 | 1.000 | 0.41859 | +0.01 | 0.6664 | 0 | 0 |
| Spring Drag | 0.52 | 1.000 | 0.48651 | -0.01 | 0.6567 | 0 | 0 |
| Spring Grind | 0.95 | 0.950 | 0.11404 | -0.44 | 0.9180 | 0 | 0 |
| Spring Pluck | 2.61 | 1.000 | 0.02588 | -0.01 | 0.0745 | 0 | 0 |
| Spring Reverb Tap | 1.60 | 0.950 | 0.53724 | -0.44 | 0.9183 | 0 | 25758 |
| Steel Tank Rumble | 1.05 | 1.000 | 0.60986 | +0.01 | 0.9293 | 0 | 0 |
| Suspension Cable Pluck | 0.15 | 1.000 | 0.46500 | -0.00 | 0.4957 | 0 | 0 |
| Sympathetic Chain | 2.48 | 1.000 | 0.17422 | -0.00 | 0.5217 | 0 | 0 |
| Thin Sheet Roll | 0.00 | 0.627 | 0.20542 | -0.00 | 0.3846 | 0 | 0 |
| Workshop Plate Scrape | 0.00 | 0.665 | 0.28064 | +0.00 | 0.3182 | 0 | 0 |

> **Later (C6):** fixing the spring reverb's shared state moved Spring Reverb Tap to −0.63 dB, so its BODY was re-matched to 2.26 (output 0.950 unchanged; +0.00 dB, 0 hits). See `openspec/changes/fix-spring-reverb-stereo/audition.md`.

# Spring presets: before and after

The factory presets with Space = Spring are **Distant Plate Roll** and **Spring Reverb Tap**. Both use the default spring tank size (0.5), so the configured delay is 80 ms (3,840 samples at 48 kHz).

These numbers come from `c6_spring_preset_measurements` in `src/lib.rs` (`#[ignore]`, run with `cargo test --release --no-default-features --lib c6_spring_preset_measurements -- --ignored --nocapture`). The render plays one note (C4, velocity 1) for 1 s, then 1 s of release, at 48 kHz, through the real post chain at each preset's stored BODY and output.

Column definitions:
- **spring/output**, **spring side/mid**, **spring L/R corr**: measured on the spring's own contribution, which is the render minus the same render with space amount 0.
- **first echo**: an impulse through the preset's post chain, spring on minus spring off. It includes about 0.5 ms of chain latency.

## Before (shared spring)

| preset | peak | rms | limiter hits | spring/output (dB) | spring side/mid (dB) | spring L/R corr | first echo (ms) |
|---|---|---|---|---|---|---|---|
| Distant Plate Roll | 0.2962 | 0.04900 | 0 | -15.3 | -43.3 | 1.000 | 40.5 |
| Spring Reverb Tap | 0.9183 | 0.53724 | 0 | -8.4 | -50.2 | 1.000 | 40.5 |

## After (one spring per channel)

| preset | peak | rms | limiter hits | spring/output (dB) | spring side/mid (dB) | spring L/R corr | first echo (ms) |
|---|---|---|---|---|---|---|---|
| Distant Plate Roll | 0.3035 | 0.04879 | 0 | -15.9 | -51.9 | 1.000 | 80.5 |
| Spring Reverb Tap | 0.9182 | 0.52532 | 0 | -7.8 | -58.1 | 1.000 | 80.5 |

## After, with Spring Reverb Tap re-voiced

The fix moved Spring Reverb Tap to −0.63 dB from its pre-C5 RMS. That is outside the 0.5 dB that C5's `c5_verify_revoiced_presets` holds every factory preset to; C5 had left it at −0.44 dB. Re-running C5's matching procedure (`c5_revoice_table`) on the fixed chain puts its BODY at 2.26 (was 1.60), with the same 0.95 output trim. All 50 presets then pass `c5_verify_revoiced_presets` again. Distant Plate Roll stays at −0.04 dB, so it isn't touched.

| preset | peak | rms | limiter hits | spring/output (dB) | spring side/mid (dB) | spring L/R corr | first echo (ms) |
|---|---|---|---|---|---|---|---|
| Distant Plate Roll | 0.3035 | 0.04879 | 0 | -15.9 | -51.9 | 1.000 | 80.5 |
| Spring Reverb Tap | 0.9183 | 0.56508 | 0 | -8.5 | -57.0 | 1.000 | 80.5 |

## What changes audibly

- **Echo spacing doubles, from 40 ms to 80 ms.** This is the main audible change: the spring now repeats at its configured delay. Before, it effectively ran at twice the sample rate, so its dispersion low-pass also sat about an octave higher. Expect the tails to be darker as well as more widely spaced. That follows from the model; it hasn't been checked by ear.
- **Level is unchanged.** RMS moves by −0.04 dB (Distant Plate Roll) and −0.19 dB (Spring Reverb Tap). Re-voicing Spring Reverb Tap's BODY brings it back to its pre-C5 level (+0.00 dB). Neither preset hits the limiter, before or after.
- **Cross-talk goes away, but it was barely audible on these presets.** Both run spread width 0.5, which feeds the spring a nearly mono signal, so the spring's tail was already almost mono. Its side energy falls by a further 8 dB, from −43/−50 dB to −52/−58 dB below mid. The isolation itself is pinned by the unit tests. A left-only input now leaves the right tail at exactly 0.0, where before it leaked from the first echo onward.

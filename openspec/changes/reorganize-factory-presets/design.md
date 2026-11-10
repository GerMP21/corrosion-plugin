# Design

## Context

- The factory bank is empty today. `presets/factory/` holds only a `.gitkeep`, and 14 CI tests plus 10 GUI tests (9 of them the same) fail because they read specific factory files.
- `src/bin/seed_presets.rs` builds presets as typed `Preset` values, one function each, and writes them flat.
- The editor embeds `presets/factory` with `include_dir!`. It lists every preset in one alphabetical `ComboBox`, followed by a "USER" separator and the user's presets.
- User presets live in a per-user folder (`presets::user_preset_dir`). `list_user_presets` reads only its root.
- `offline::render_preset` renders a preset through the plugin's own output path (`Corrosion::process_block`), so the seeder and the tests can measure exactly what a user hears.
- The parameters:
  - 16 exciters, each with its own parameter slots;
  - 9 objects, each with its own character parameters;
  - shared controls (size, rust, damage, envelopes, MSEG, global exciter controls, strike position, macros);
  - the post chain (filter, drive, body, spread, Space, diode stage, limiter).

## Goals / Non-Goals

**Goals:**
- A bank organized by purpose: five curated folders of 12, plus a complete exciter × resonator grid.
- Every factory preset playable and at a common loudness. The curated ones are measurably in character.
- A browser that stays usable with about 204 presets, and user folders with a way to reach them on disk.
- Tests that don't break when the bank changes.

**Non-Goals:**
- Drum kits and the `drums/` folder (`per-key-kits`).
- Tags, search or favourites in the browser.
- Creating, renaming or deleting folders from the editor; the file manager does that.
- Changing the preset schema.

## Decisions

### 1. Layout on disk
```
presets/factory/
  bass/<slug>.corrosion-preset            (12)
  lead/…  chords/…  drones/…  percussion/…  (12 each)
  experimental/<exciter-slug>/<object-slug>.corrosion-preset   (16 × 9)
```
The folder is the category. There is no category field in the file, so there is no schema change. The display name is the preset's `name`; experimental presets are named `<Exciter> · <Object>` (for example "Bow · Pipe").
- *Alternative:* a `category` field in the JSON. Rejected: it duplicates the folder and needs a schema change.

### 2. Preset definitions live in the library
The curated presets and the experimental generator move into a library module, `presets::factory` (compiled into the plugin, but not used by the audio path). The `seed_presets` binary becomes a thin writer over it.
- This lets the bank tests (counts, grid, quality, character) build the same presets without reading files, so a stale file can't hide a regression.
- A test also compares the generated presets with the files on disk, so the files can't drift from the code.

### 3. Experimental generator
For each (exciter, object):
- **Seed:** a deterministic seed from the pair's indices, through `dsp::utils::noise` or a small integer hash. No `rand` dependency; the output is identical on every platform.
- **What varies:**
  - **every** parameter the exciter's slots use (from the exciter panel specs, `exciter_panel(…).controls`, so a new slot is picked up automatically);
  - the object's character parameters;
  - the shared voice controls;
  - the post chain.
- **Ranges:** the values come from per-parameter "playable" ranges. These are an inner part of each parameter's range (for example Rust 0–0.7, Damage 0–0.6, Output decided by levelling, Space mix ≤ 0.5). They are written next to the generator and start from `randomizer::full_ranges` where it has them.
- **Play mode** follows the exciter family: friction and continuous exciters (bow, grinding, drag, tension rise, jet, hum, particle rain) get Drone half the time (decided by the seed); strikes stay Tonal. Quality is Normal.

### 4. Levelling and the quality floor in the seeder
After building a preset, the seeder renders a 2-second middle C (velocity 100/127) with `render_preset` and sets Output so the RMS hits the target loudness. The target is chosen once: the median of the curated presets at Output 0 dB, rounded, and written as a constant.
- It then checks the quality floor (finite, within ±6 dB, ≤ 1 % of samples at the ceiling).
- An experimental preset that fails is re-drawn with the next seed (up to 8 tries, and the seed used is recorded so regeneration stays deterministic). A curated preset that fails stops the seeder with a message: those get fixed by hand.

### 5. Curated presets: how they are designed
By hand, one function per preset with a comment on its intent, from what each parameter does (README, `help_for`, the parameter docs):
- **Bass:**
  - resonators: Pipe, Tank, Taut Cable and Coil Spring, at large Size;
  - exciters: soft (Felt Mallet, Hand Strike, Bow);
  - controlled Rust; low-pass filter; moderate drive;
  - short Space or none.
- **Lead:**
  - resonators: Pipe, Taut Cable and Sheet Metal;
  - exciters: brighter (Hard Mallet, Stiff Point, Bow with more speed), more drive;
  - Tonal; the echo synced to the host tempo.
- **Chords:**
  - near-harmonic resonators (Pipe, Taut Cable), with Coil Spring and Plate only where the measurement says they stay within the partial bound;
  - exciters: Bow and Felt Mallet;
  - Rust ≤ 0.15, Damage 0, Heat low;
  - long decay or Drone sustain; Space reverb for width.
- **Drones:**
  - Drone mode with friction or continuous exciters (Bow, Heavy Grinding, Electromagnetic Hum, Pneumatic Jet, Tension Rise);
  - every resonator family represented;
  - slow MSEG loops; Space.
- **Percussion:**
  - strikes (Drumstick, Hard Mallet, Metal Pipe, Metal Chain, Tension Snap, Wire Brush) on Plate, IBeam, Chain, Sheet Metal and Industrial Cog;
  - short decays; Tonal.

Each preset's measurements go in a table in `## Measurements`: level, spectral centroid at its test note, decay to −30 dB, partial deviation, spectral flatness, and whether it passed its character test. The maintainer listens and asks for changes.

### 6. Character tests
Each scenario in the spec becomes a test over the preset definitions (decision 2). It renders offline at 48 kHz (2–8 s per preset) and measures with small helpers:
- RMS windows;
- a windowed FFT for the energy split, the strongest peak and the partials (an FFT helper written in the test module, as `offline` already does for aliasing);
- spectral flatness.

These run in the CI lane on gnu and musl; the bounds have margin, so libm differences don't matter. The 60 curated presets get the character tests; all 204 get the quality floor. If the CI time grows noticeably, the experimental floor test moves to an `#[ignore]` measurement, and the seeder still enforces it.

### 7. Browser
`ComboBox` is replaced by a menu button that shows the current preset as `Folder / Name`, with nested `ui.menu_button` submenus:
- the factory folders in a fixed order (Bass, Lead, Chords, Drones, Percussion, Experimental › exciter);
- then User, with its subfolders.

`include_dir`'s `Dir::dirs()` and `files()` give the tree, and `FactoryPresetEntry` gains its folder path. The arrows step within the current preset's folder, wrapping around.

### 8. User folders, Save and Open folder
- `list_user_presets` walks the user folder recursively (depth ≤ 4; hidden folders and unreadable files are skipped and counted, as today) and returns each preset with its relative folder.
- **Save:** a small folder `ComboBox` before the name, with "User" (the root) and each existing subfolder, then Save. The path is always built under `user_preset_dir()`, which is why the factory bank can't be a target.
- **Open folder:**
  - `std::fs::create_dir_all`, then a detached `std::process::Command`: `explorer` on Windows, `open` on macOS, `xdg-open` elsewhere;
  - spawn errors are shown in the save message line;
  - it runs on the GUI thread and never touches audio.

### 9. Tests that read factory files
Each of the 24 failing test runs (14 CI, 10 GUI, 9 shared) needed "some preset with the post chain on", or "with the lookahead limiter", or "every parameter non-default". Each gets a fixture built in the test (`Preset::from_params` plus the fields it needs) and a comment saying what it needs. The benchmark and `render_presets` default to a fixture written by the test, or take a path argument.

*As built (task 1.1):* instead of fixtures built in each test, the old 50-preset bank is kept as a test corpus in `tests/fixtures/presets/legacy/` (restored from the commit before the deletion). The failing tests were written against those exact files (named presets, their Space modes, limiter settings, every-parameter coverage), so pointing them at the same files keeps each test's original claim without re-deriving 24 fixtures, and the corpus doubles as a migration check: old presets must keep loading. Specific-name loads (`chain_whip_snap`, `plate_resonance_bath`, `beam_grind`, `beam_drag`, `sheet_lightning`, the offline renders' stems) read the corpus; the benchmark and `render_presets` default to `legacy/sheet_lightning`. The whole-bank correctness tests (`loaded_values_match_the_preset_on_every_parameter`, `preset_built_parameters_display_like_the_live_ones`) walk the factory bank recursively plus the corpus, so they cover the new bank once it exists. `factory_preset_paths` is now recursive (the levelling and DC measurements run over the new bank); the C6 Spring measurement reads the corpus. The editor's `factory_preset_loader_finds_factory_bank` now checks only that every embedded file loads, sorted; the bank's shape moves to the `presets::factory` tests (1.2) and the browser tests (1.5). With the bank empty: CI lane 404 + integration tests green, GUI lane 448 green, clippy clean.

## Risks / Trade-offs

- **Measured is not musical.** The character bounds catch a bass that isn't low, or a chord patch that buzzes; they can't tell good from merely in range. *Mitigation:* the measurements table and the maintainer's listening pass. Expect a revision round.
- **The experimental generator can make ugly but valid presets.** That's the folder's point; the floor only guarantees audible, unbroken and levelled.
- **Render time.** 204 presets × 2 s, plus the character renders, take tens of seconds in debug. *Mitigation:* `[profile.test]` uses opt-level 1; decision 6's fallback.
- **The partial bound for chords may exclude every non-Pipe resonator.** If fewer than 12 good chord patches pass with the bound at 15 cents, stop and ask (bound vs variety), rather than relaxing it silently.
- **Browser depth.** Experimental takes two levels; egui submenus open on hover, which is fine with a mouse. Check it in REAPER.
- **`xdg-open` may be missing** on minimal Linux setups; the error is reported, not hidden.

## Measurements

### Target loudness and levelling
- `TARGET_RMS` = **0.13** (−17.7 dBFS, both channels over the 2 s floor render): the median of the 60 curated presets at Output 0 dB was 0.12542 (`seed_presets --target`), rounded to two digits. At Output 0 dB they span 0.0061–0.44 RMS, so levelling sets Output from +31 dB down to −10.6 dB.
- Every preset is levelled exactly (one render at Output 1; the level before the limiter scales with Output), so the floor's ±6 dB holds by construction unless the limiter acts. No preset had samples at the ceiling.
- **Re-drawn experimental seeds:** 2 of 144, both Pneumatic Jet (`pneumatic_jet/pipe` and `pneumatic_jet/taut_cable`, try 1). None needed more than one re-draw. No curated preset failed the floor.
- Seeder run time: 6.5 s (release, all cores) for the 204 presets.
- Bank tests in the CI lane (debug, `opt-level = 1`): 18 tests in 12.6 s wall-clock. Compiling dominates the run, so the experimental floor test stays in CI (decision 6's fallback was not needed).

### Curated presets (held middle C unless stated; Output after levelling)
RMS @0 dB is the floor render at Output 1. Centroid and decay come from a 4 s held C4. Pitch error is the strongest peak against C3/C4/C5. "Low" is the share of energy under 500 Hz at C2. Partial deviation, sustain and flatness follow the chord scenario. Drone drift compares the 8th second with the 2nd. Decay is the level 1.5 s in against the peak. Every folder's own check passes. The other columns are shown for reference.

| Folder | Preset | Output | RMS @0 dB | Centroid (Hz) | Decay −30 dB (s) | Pitch err (c) | Low < 500 Hz | Partial dev (c) | Sustain 2 s (dB) | Flatness | Drone drift (dB) | Decay 1.5 s (dB) | Pass |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bass | Felt Cable Upright | 3.087 | 0.0421 | 209 | 0.35 | 3 | 1.00 | 0.0 | -37.9 | 0.0000 | -28.6 | -35.5 | yes |
| bass | Palm Pipe Thump | 0.369 | 0.3526 | 371 | — | 1 | 0.98 | 0.1 | -2.1 | 0.0000 | -23.9 | -2.0 | yes |
| bass | Bowed Cable Sub | 7.686 | 0.0169 | 263 | — | 8 | 0.99 | 0.0 | -12.4 | 0.0000 | +18.4 | -14.0 | yes |
| bass | Coil Spring Bass | 1.914 | 0.0679 | 206 | 0.30 | 4 | 1.00 | 27.8 | -45.7 | 0.0000 | -49.6 | -41.5 | yes |
| bass | Snapped Cable Bass | 0.385 | 0.3375 | 379 | — | 12 | 0.98 | 0.6 | -1.6 | 0.0000 | -16.9 | -1.5 | yes |
| bass | Tank Boom Bass | 1.854 | 0.0701 | 214 | 0.35 | 110 | 1.00 | 0.0 | -32.4 | 0.0001 | -13.0 | -31.3 | yes |
| bass | Plate Sub Mallet | 1.936 | 0.0671 | 157 | 0.25 | 0 | 1.00 | 0.0 | -73.9 | 0.0015 | -164.5 | -60.3 | yes |
| bass | Driven Beam Bass | 0.900 | 0.1444 | 262 | — | 0 | 1.00 | 0.0 | -17.8 | 0.0000 | -36.5 | -14.7 | yes |
| bass | Hollow Pipe Sub | 1.585 | 0.0820 | 126 | 0.50 | 1 | 1.00 | 0.0 | -29.4 | 0.0000 | -38.6 | -26.2 | yes |
| bass | Hand Slap Cable | 0.402 | 0.3236 | 377 | — | 10 | 0.99 | 0.4 | -1.2 | 0.0001 | -19.4 | -1.1 | yes |
| bass | Low Sheet Bow | 0.503 | 0.2585 | 131 | — | 239 | 1.00 | 0.4 | -0.1 | 0.0000 | +0.0 | -0.2 | yes |
| bass | Heavy Felt Beam | 1.894 | 0.0686 | 207 | 0.35 | 0 | 1.00 | 0.0 | -40.3 | 0.0001 | -37.5 | -37.2 | yes |
| lead | Bowed Cable Lead | 0.508 | 0.2556 | 288 | — | 9 | 0.99 | 1.0 | -0.6 | 0.0000 | +3.0 | -0.2 | yes |
| lead | Hard Cable Pluck | 1.362 | 0.0955 | 262 | 3.35 | 3 | 1.00 | 20.4 | -23.8 | 0.0000 | -26.9 | -21.5 | yes |
| lead | Pipe Bow Lead | 1.011 | 0.1286 | 262 | — | 1 | 1.00 | 218.7 | -13.9 | 0.0000 | -38.1 | -10.7 | yes |
| lead | Glass Sheet Lead | 0.498 | 0.2608 | 773 | — | 0 | 0.94 | 0.1 | -2.0 | 0.0000 | +0.1 | -1.9 | yes |
| lead | Plate Strike Lead | 1.469 | 0.0885 | 262 | 0.80 | 0 | 1.00 | 0.0 | -58.7 | 0.0000 | -144.8 | -46.6 | yes |
| lead | Driven Pipe Lead | 0.390 | 0.3337 | 379 | — | 2 | 0.98 | 1.6 | -1.6 | 0.0001 | -26.6 | -1.5 | yes |
| lead | Echo Cable Pluck | 0.432 | 0.3008 | 443 | — | 11 | 0.98 | 1.0 | -3.3 | 0.0002 | -12.9 | -3.1 | yes |
| lead | Felt Plate Bell | 2.014 | 0.0645 | 199 | 0.15 | 0 | 1.00 | 239.8 | -61.1 | 0.0000 | -106.8 | -52.1 | yes |
| lead | Jet Pipe Flute | 0.343 | 0.3790 | 360 | — | 1 | 0.98 | 218.4 | -2.7 | 0.0004 | +0.8 | -0.0 | yes |
| lead | Bowed Spring Lead | 1.179 | 0.1103 | 292 | 3.65 | 45 | 1.00 | 186.3 | -16.5 | 0.0000 | -47.1 | -12.4 | yes |
| lead | Metal Pipe Pluck | 2.140 | 0.0608 | 262 | 0.30 | 3 | 1.00 | 0.0 | -40.5 | 0.0000 | -27.0 | -38.3 | yes |
| lead | Bowed Beam Lead | 1.064 | 0.1222 | 262 | — | 0 | 1.00 | 0.0 | -13.7 | 0.0000 | -35.7 | -10.7 | yes |
| chords | Bowed Cable Pad | 3.657 | 0.0355 | 263 | — | 10 | 1.00 | 0.0 | -3.6 | 0.0000 | +9.3 | -4.0 | yes |
| chords | Glass Cable Choir | 4.737 | 0.0274 | 262 | — | 8 | 1.00 | 0.0 | -2.5 | 0.0000 | +8.4 | -4.3 | yes |
| chords | Plate Bow Pad | 21.010 | 0.0062 | 260 | — | 4 | 1.00 | 4.3 | -3.7 | 0.0000 | -0.4 | -6.7 | yes |
| chords | Sheet Bow Organ | 0.790 | 0.1646 | 265 | — | 18 | 1.00 | 0.0 | -0.4 | 0.0000 | -0.1 | -0.7 | yes |
| chords | Warm Cable Strings | 3.623 | 0.0359 | 263 | — | 10 | 1.00 | 0.0 | -3.6 | 0.0000 | +9.4 | -4.0 | yes |
| chords | Soft Plate Strings | 21.230 | 0.0061 | 258 | — | 4 | 1.00 | 4.3 | -3.8 | 0.0000 | -0.3 | -6.7 | yes |
| chords | Sheet Bow Shimmer | 0.796 | 0.1633 | 532 | — | 11 | 0.98 | 0.1 | -2.4 | 0.0000 | -0.2 | -1.6 | yes |
| chords | Cable Echo Pad | 4.339 | 0.0300 | 262 | — | 10 | 1.00 | 0.0 | -5.3 | 0.0000 | +11.2 | -4.8 | yes |
| chords | Rosin Cable Ensemble | 4.765 | 0.0273 | 262 | — | 8 | 1.00 | 0.0 | -2.4 | 0.0000 | +8.4 | -4.3 | yes |
| chords | Beam Bow Chorale | 15.370 | 0.0085 | 260 | — | 3 | 1.00 | 7.8 | -3.0 | 0.0000 | -0.1 | -10.6 | yes |
| chords | Cable Spring Pad | 3.772 | 0.0345 | 263 | — | 9 | 1.00 | 0.0 | -3.5 | 0.0000 | +9.4 | -4.1 | yes |
| chords | Sheet Spring Organ | 0.880 | 0.1477 | 269 | — | 17 | 1.00 | 4.4 | -0.2 | 0.0000 | -0.0 | -0.3 | yes |
| drones | Smokestack Hum | 0.394 | 0.3298 | 225 | — | 237 | 0.99 | 315.5 | -1.7 | 0.0002 | -0.0 | -1.7 | yes |
| drones | Grinding Beam | 0.306 | 0.4252 | 253 | — | 600 | 0.99 | 0.1 | -0.7 | 0.0003 | -0.5 | -0.2 | yes |
| drones | Tank Breath | 0.387 | 0.3358 | 115 | — | 715 | 1.00 | 184.5 | -0.1 | 0.0000 | -0.0 | -1.0 | yes |
| drones | Bowed Beam Drone | 0.466 | 0.2789 | 213 | — | 591 | 0.98 | 65.7 | -0.8 | 0.0000 | -0.1 | -0.9 | yes |
| drones | Spring Bow Drone | 0.487 | 0.2668 | 234 | — | 495 | 0.99 | 116.5 | -1.7 | 0.0000 | +0.1 | -3.0 | yes |
| drones | Cog Hum Drone | 0.295 | 0.4400 | 62 | — | 2128 | 1.00 | 440.2 | -0.3 | 0.0001 | +0.0 | -0.2 | yes |
| drones | Chain Drag Drone | 1.077 | 0.1207 | 266 | — | 5 | 1.00 | 315.9 | -2.7 | 0.0000 | -0.4 | -8.1 | yes |
| drones | Plate Tension Swell | 0.365 | 0.3563 | 327 | — | 5 | 0.99 | 525.0 | -4.1 | 0.0002 | +0.3 | -1.9 | yes |
| drones | Sheet Bow Drone | 0.344 | 0.3775 | 511 | — | 490 | 0.95 | 3.1 | -0.1 | 0.0007 | -0.2 | -0.1 | yes |
| drones | Cable Scrape Drone | 0.404 | 0.3216 | 309 | — | 15 | 0.98 | 333.2 | -0.6 | 0.0001 | +1.0 | -1.9 | yes |
| drones | Particle Rain Tank | 0.372 | 0.3496 | 124 | — | 698 | 1.00 | 155.8 | -1.0 | 0.0000 | -0.2 | -0.3 | yes |
| drones | Pipe Wind Drone | 0.339 | 0.3838 | 197 | — | 41 | 1.00 | 0.3 | -1.0 | 0.0001 | -0.8 | -0.3 | yes |
| percussion | Plate Snare Crack | 1.319 | 0.0985 | 523 | 0.90 | 0 | 1.00 | 0.0 | -75.2 | 0.0000 | -187.6 | -55.2 | yes |
| percussion | Beam Kick Thud | 0.686 | 0.1896 | 95 | 1.20 | 583 | 1.00 | 10.7 | -63.4 | 0.0000 | -197.3 | -42.8 | yes |
| percussion | Chain Rattle Hit | 0.883 | 0.1473 | 312 | 0.90 | 0 | 1.00 | 340.7 | -75.4 | 0.0003 | -193.5 | -50.5 | yes |
| percussion | Rusted Sheet Clang | 1.974 | 0.0658 | 565 | 0.40 | 133 | 1.00 | 0.0 | -40.6 | 0.0000 | -38.7 | -37.4 | yes |
| percussion | Cog Tick | 0.602 | 0.2159 | 47 | 1.15 | 1844 | 1.00 | 326.0 | -69.7 | 0.0000 | -196.7 | -46.1 | yes |
| percussion | Brush Plate Swish | 1.624 | 0.0800 | 524 | 0.95 | 0 | 1.00 | 0.0 | -62.8 | 0.0000 | -190.5 | -46.9 | yes |
| percussion | Beam Anvil | 1.864 | 0.0697 | 185 | 1.20 | 599 | 1.00 | 0.0 | -37.1 | 0.0000 | -49.8 | -32.9 | yes |
| percussion | Chain Snap | 1.154 | 0.1127 | 273 | 0.75 | 0 | 1.00 | 546.0 | -89.4 | 0.0000 | -181.5 | -64.6 | yes |
| percussion | Cog Gong Hit | 0.918 | 0.1416 | 28 | 0.30 | 2686 | 1.00 | 597.5 | -58.9 | 0.0000 | -204.5 | -38.2 | yes |
| percussion | Sheet Tick Hat | 2.521 | 0.0516 | 1130 | 0.20 | 1333 | 1.00 | 0.0 | -61.6 | 0.0000 | -52.5 | -57.3 | yes |
| percussion | Chain Mallet Clank | 1.881 | 0.0691 | 264 | 0.35 | 0 | 1.00 | 546.0 | -110.8 | 0.0000 | -164.1 | -86.2 | yes |
| percussion | Rusted Beam Knock | 0.629 | 0.2067 | 135 | 1.30 | 0 | 1.00 | 4.1 | -59.7 | 0.0000 | -200.5 | -39.2 | yes |

### What the measurements found (design guesses that didn't hold)
- **Pitch is not the key at Size 1 for most objects.** The fundamental is roughly the key ÷ Size, with an offset per object: Taut Cable +3–10 cents, Pipe +42 cents (Size 1.025), Plate an octave up (Size 2), Coil Spring +148 cents (Size 1.09), I-Beam a tritone down (Size 0.707), Tank 19 semitones down (Size 0.333). Sheet Metal's pitch goes as Size^−0.75 (Size 3.49 on the key, 1.371 an octave up). The tuned sizes are constants in `curated.rs`.
- **Pipe and Coil Spring can't be chord patches.** A Pipe adds a mode at a fixed 800–1030 Hz whatever the note (it moves with Tube Diameter), and the Spring is inharmonic. The chord folder uses Taut Cable (6), Sheet Metal (3), Plate (2) and I-Beam (1), all bowed in Drone mode with BODY 0. The design's list was Pipe and Taut Cable first.
- **BODY and every Space mode add partials of their own**: BODY near 190 and 390 Hz; Factory, Spring and Echo at 98–144 Hz. Under a loud bowed cable they stay more than 40 dB down. Under a quiet plate or beam they don't, so the Plate and I-Beam chord patches are dry and widened by Spread.
- **A Taut Cable's third partial sits about 20 cents sharp** (stiffness) whenever it is within 40 dB of the strongest. The chord bows keep it under (pressure 0.9–1.1, speed ≤ 1.05).
- **Bowed cables swell** by 5–10 dB over 8 s held, so they suit the chord folder but not the drone folder.
- **Strikes on Pipe, I-Beam and Taut Cable ring for seconds** at velocity 100, and sit at the voice's clamp (about 0.31 RMS) for 2–3 s before decaying, even with Damping 1. This holds for the default patch too: Hand Strike on Pipe, held or released, stays at that level for about 3 s; at velocity 25 it decays from the start. It is not self-oscillation (it decays exponentially once under the clamp), but it is loud. Percussion therefore uses Plate and Chain, or heavy Rust (2) and Sludge (1) on beams and cogs. Noted for the engine backlog; not changed here.
- **The flatness bound never binds.** The largest flatness measured was 0.031 (old corpus), 0.0025 with exciters pushed to be as noisy as possible, and 0.0056 for a pure felt-mallet tone. The modal output always has a large spectral dynamic range, so "< 0.1" can't tell clean from noisy in this instrument. The bound is implemented as specified. **Open question for the maintainer:** keep it, or replace "clean" with a measure that separates harmonic from non-harmonic energy (for example, energy outside ±25 cents of the harmonics over total, < 10 %).

### As built
- *Generator in parameter-ID space (decision 3).* `exciter_panel` lives in the GUI module, which the no-GUI library can't reach. So the generator keeps its own per-exciter slot and per-object character lists (`exciter_slot_ids`, `object_character_ids`), and a GUI-lane test (`experimental_generator_varies_what_the_panels_show`) checks they equal the panels' controls. It goes red when a slot is dropped. Every parameter ID is a key of the preset JSON, so values are written by ID and parsed back through `Preset::from_json_str`, which clamps them like a loaded file. Fixed parameters are listed with reasons (`EXPERIMENTAL_FIXED_IDS`: output, quality mode, limiter mode, ceiling, UI scale). Everything else varies, `sync_rate` included: without a host tempo it falls back to Delay Time.
- *Stored levels (decision 4).* The seeder writes each preset's Output and seed try to `src/presets/factory/levels.rs`. `presets::factory::bank()` rebuilds the exact files without rendering, so the disk-equality test is cheap, and gnu/musl libm differences in a re-render can't make the files drift.
- *Seeds:* SplitMix64 over (exciter, object, try), mixed with an FNV hash of the parameter ID, so adding a parameter doesn't change the others' draws.
- *Measurement helpers:* a 4-term Blackman–Harris window (side lobes −92 dB, so window side lobes aren't counted as partials), radix-2 FFT of 65 536 points, peaks within 40 dB of the strongest (local maxima within ±20 Hz). The chord partials are the first six such peaks above 60 Hz, each compared with the nearest integer multiple of the first.
- *Character tests reject wrong presets* (`character_checks_reject_wrong_presets`): a detuned (Size 1.4) cable fails the pitch check, a dissonant cog fails the partials, a felt bell fails the chord sustain and the drone, a swelling bowed cable fails the drone, a bowed pipe fails percussion, and a small bright scraped sheet fails the bass check. No preset could be made to fail the flatness bound (see above). Its helper test checks it on white noise.


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

## Risks / Trade-offs

- **Measured is not musical.** The character bounds catch a bass that isn't low, or a chord patch that buzzes; they can't tell good from merely in range. *Mitigation:* the measurements table and the maintainer's listening pass. Expect a revision round.
- **The experimental generator can make ugly but valid presets.** That's the folder's point; the floor only guarantees audible, unbroken and levelled.
- **Render time.** 204 presets × 2 s, plus the character renders, take tens of seconds in debug. *Mitigation:* `[profile.test]` uses opt-level 1; decision 6's fallback.
- **The partial bound for chords may exclude every non-Pipe resonator.** If fewer than 12 good chord patches pass with the bound at 15 cents, stop and ask (bound vs variety), rather than relaxing it silently.
- **Browser depth.** Experimental takes two levels; egui submenus open on hover, which is fine with a mouse. Check it in REAPER.
- **`xdg-open` may be missing** on minimal Linux setups; the error is reported, not hidden.

## Measurements

(to be filled during implementation: target loudness, per-preset table, re-drawn experimental seeds, CI time of the bank tests, REAPER browser check)

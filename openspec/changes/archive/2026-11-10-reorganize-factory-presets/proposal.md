# Proposal

## Why

The factory bank was deleted to be rebuilt (2026-11-10). The old one was 50 placeholders in one flat, alphabetical list, with no way to find a bass or a drone. The maintainer wants a real bank, organized by purpose: a folder per musical role, and an experimental folder that shows every exciter on every resonator. User presets should get folders too, a button to reach them on disk, and no way to write into the factory bank.

## What Changes

- **Factory bank in folders**, embedded in the plugin as today:
  - `bass/`, `lead/`, `chords/`, `drones/` and `percussion/`, 12 presets each, designed by hand from the parameters' behaviour and checked by measurement (60 in all).
  - `experimental/`, one preset for every exciter × resonator pair: 16 × 9 = 144, grouped by exciter. The seeder generates them deterministically and varies every available parameter.
  - Drum kits are not in this change: a kit preset needs `per-key-kits`, which adds the `drums/` folder.
- **Every factory preset meets a quality floor**: finite, audible, not clipping, and levelled to a common loudness by the seeder. Curated presets also meet their folder's character:
  - bass: low register;
  - lead and chords: clearly pitched;
  - chords: harmonic and sustaining, without noise;
  - drones: sustaining indefinitely;
  - percussion: decaying.
- **Preset browser with submenus**: one submenu per factory folder (experimental nested by exciter), then a User submenu. The arrows step through the current folder.
- **User folders**:
  - the User submenu mirrors the subfolders of the user preset folder;
  - Save gets a folder selector (the user folder or one of its subfolders, never a factory folder);
  - an **Open folder** button shows the user preset folder in the system file manager.
- **Tests stop depending on the bank**: the tests that read specific factory files (14 in the CI lane, 10 in the GUI lane, 9 of them shared, all failing today) use fixtures built in the tests instead.
- **Factory levels matter again**: the rule that factory presets are placeholders ends with this bank. `preset_levels_are_sane` is updated to the new bank and must pass.

No parameter or schema change. User presets and DAW projects are untouched; presets saved before keep loading from the user folder's root.

## Capabilities

### New Capabilities
- `factory-preset-bank`: what the factory bank contains (folders, counts, the experimental grid), the quality floor and each folder's character, and how the seeder produces it.

### Modified Capabilities
- `user-presets`: the preset list becomes folder submenus, user presets can live in subfolders, Save picks a user folder and can never target the factory bank, and an Open folder button reveals the user folder.

## Impact

- `src/bin/seed_presets.rs`:
  - rewritten: 60 curated presets in five folders, the experimental generator, levelling by offline render;
  - preset definitions may move into the library so tests and the seeder share them.
- `presets/factory/<folder>/…`: 204 files, regenerated with `cargo run --bin seed_presets`.
- `src/gui/editor.rs`: the loader reads folders recursively (`include_dir`), the browser gets submenus and folder-scoped arrows, Save gets the folder selector, plus the Open folder button.
- `src/presets/mod.rs`: user presets are listed recursively, with their folder; saving into a subfolder; opening the folder per platform (`explorer`, `open`, `xdg-open`).
- Tests:
  - `src/lib.rs`, `src/offline/preset.rs`, `src/presets/mod.rs`, `tests/preset_loading.rs` and `tests/preset_roundtrip.rs` switch to in-test fixtures;
  - new bank tests (counts, grid coverage, quality floor, folder character);
  - `benches/performance.rs` and `render_presets` get a fixture instead of `sheet_lightning`.
- CLAUDE.md's factory-preset rules, the README's preset section, `docs/ARCHITECTURE.md`.

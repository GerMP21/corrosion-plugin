# Corrosion: notes for Claude

## Factory presets are placeholders

The 50 factory presets (`presets/factory/`, seeded by `src/bin/seed_presets.rs`) are irrelevant for now: the maintainer will regenerate them all when the project is finished.

- Don't spend effort keeping them bit-identical, re-levelling them, or measuring how a change affects them.
- Don't treat "a factory preset changes" as a risk or a reason to avoid a change.
- Keep them loading and passing the existing tests (`preset_levels_are_sane`, `tests/preset_roundtrip.rs`). If a schema change adds a field, regenerate them with `cargo run --bin seed_presets` so they stay valid.
- User presets and saved DAW projects still matter: keep their migrations.

//! Stored levels, written by `cargo run --bin seed_presets`. Do not edit.
//! (relative path, seed try, Output)

use super::StoredLevel;

const LEVELS: &[(&str, u32, f32)] = &[];

/// The stored level of the preset at `path` (relative to `presets/factory`).
pub(super) fn stored(path: &str) -> Option<StoredLevel> {
    LEVELS
        .iter()
        .find(|(p, _, _)| *p == path)
        .map(|&(_, try_index, output)| StoredLevel { try_index, output })
}

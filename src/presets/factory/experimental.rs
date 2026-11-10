//! The experimental grid: one generated preset per exciter × resonator pair.

use crate::params::Object;
use crate::presets::Preset;

/// How many seeds the seeder tries for a pair before giving up.
pub const MAX_SEED_TRIES: u32 = 8;

/// Parameters the generator does not vary, with the reason: `(id, why)`.
pub const EXPERIMENTAL_FIXED_IDS: &[(&str, &str)] = &[];

/// The exciter-specific parameters of exciter `exciter` (1–16).
pub fn exciter_slot_ids(_exciter: i32) -> &'static [&'static str] {
    &[]
}

/// The object-specific character parameters of `object`.
pub fn object_character_ids(_object: Object) -> &'static [&'static str] {
    &[]
}

/// The experimental preset for (`exciter`, `object`), drawn with seed try
/// `try_index`, before levelling.
pub fn experimental_preset(exciter: i32, object: Object, _try_index: u32) -> Preset {
    let mut preset = super::base(&format!(
        "{} · {}",
        super::exciter_name(exciter),
        object.name()
    ));
    preset.exciter = exciter;
    preset.object = object;
    preset
}

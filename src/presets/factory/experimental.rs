//! The experimental grid: one generated preset per exciter × resonator pair
//! (design decision 3).
//!
//! Each pair draws its values from a deterministic seed (a small integer
//! hash of the exciter, the object, the seed try and the parameter), so the
//! output is the same on every platform and needs no `rand` dependency.
//! Every parameter is drawn, within a playable part of its range, except the
//! few in [`EXPERIMENTAL_FIXED_IDS`].
//!
//! Values are written by parameter ID: every parameter ID is a key of the
//! preset JSON (top level or under `extra`), so the generator sets plain
//! values there and parses the result as a preset, which also clamps and
//! sanitizes them exactly as loading a file does.

use std::collections::HashMap;

use nih_plug::prelude::{ParamPtr, Params};

use crate::params::{CorrosionParams, Object};
use crate::presets::Preset;

/// How many seeds the seeder tries for a pair before giving up.
pub const MAX_SEED_TRIES: u32 = 8;

/// Parameters the generator does not vary, with the reason: `(id, why)`.
pub const EXPERIMENTAL_FIXED_IDS: &[(&str, &str)] = &[
    ("output", "set by levelling"),
    ("quality_mode", "Normal for every factory preset"),
    (
        "limiter_mode",
        "the default Hard limiter, as every factory preset",
    ),
    (
        "analog_ceiling",
        "the default ceiling, so levels compare across the bank",
    ),
    ("ui_scale", "an editor preference, not part of a sound"),
];

/// The exciter-specific parameters of exciter `exciter` (1–16), as the
/// editor's exciter panel shows them (a GUI-lane test keeps the two equal).
pub fn exciter_slot_ids(exciter: i32) -> &'static [&'static str] {
    match exciter {
        1 => &["bow_pressure", "bow_speed", "rosin_grip", "slip_curve"],
        2 => &[
            "hand_mass",
            "flesh_stiffness",
            "flesh_damping",
            "mute_decay",
        ],
        3 => &[
            "mallet_mass",
            "felt_softness",
            "core_hardness",
            "compression_curve",
        ],
        4 => &["mallet_mass", "material_stiffness", "impact_damping"],
        5 => &[
            "stick_mass",
            "tip_stiffness",
            "restitution_bounciness",
            "micro_bounce_limit",
        ],
        6 => &[
            "wire_density",
            "spread_duration",
            "brush_wire_stiffness",
            "amplitude_randomization",
        ],
        7 => &[
            "pipe_mass",
            "metal_stiffness",
            "pipe_pitch",
            "pipe_ring_decay",
        ],
        8 => &[
            "link_count",
            "chain_mass",
            "drop_envelope_spread",
            "internal_rattle",
            "rattle_color",
        ],
        9 => &[
            "scrape_speed",
            "point_pressure",
            "chatter_pitch",
            "chatter_damping",
        ],
        10 => &[
            "grind_speed",
            "grind_pressure",
            "surface_grit",
            "grit_color",
        ],
        11 => &[
            "drag_speed",
            "ridge_spacing",
            "ridge_depth",
            "drag_exciter_mass",
        ],
        12 => &[
            "pull_speed",
            "break_threshold",
            "slip_stochasticity",
            "creak_sharpness",
        ],
        13 => &["air_pressure", "nozzle_width", "turbulence_chaos"],
        14 => &["mains_frequency", "coil_proximity", "voltage_sag"],
        15 => &["pull_distance", "hook_stiffness", "snap_force"],
        16 => &["flow_rate", "particle_mass", "mass_variance"],
        _ => &[],
    }
}

/// The object-specific character parameters of `object`, as the editor's
/// resonator panel shows them, without the shared Size and Damping.
pub fn object_character_ids(object: Object) -> &'static [&'static str] {
    match object {
        Object::Pipe => &["pipe_diameter"],
        Object::Plate => &["plate_aspect", "plate_stiffness"],
        Object::Tank => &["tank_volume", "tank_cavity_mix"],
        Object::Chain => &["chain_link_mass", "chain_instability"],
        Object::IBeam => &["beam_shear"],
        Object::TautCable => &["cable_braid", "cable_tension_drop"],
        Object::CoilSpring => &["spring_dispersion", "spring_slosh"],
        Object::SheetMetal => &["sheet_thinness"],
        Object::IndustrialCog => &["cog_dissonance"],
    }
}

/// The part of a parameter's range the generator draws from.
#[derive(Clone, Copy, Debug)]
enum Range {
    /// Normalized (host) values, so a skewed range is drawn as the knob is.
    Normalized(f32, f32),
    /// Plain values, linearly.
    Plain(f32, f32),
    /// Plain values, geometrically (frequencies).
    Log(f32, f32),
    /// One of a list of integer values.
    Choice(&'static [i32]),
}

/// The inner part of the range of every exciter slot and object character
/// parameter.
const SPECIFIC_RANGE: Range = Range::Normalized(0.1, 0.9);

/// Playable ranges of the shared voice controls and the post chain. Every
/// parameter that is neither exciter- nor object-specific, nor fixed, nor
/// the grid's own exciter/object/play mode, must be listed here (a test
/// checks it varies).
const SHARED_RANGES: &[(&str, Range)] = &[
    // Shared voice controls.
    ("size", Range::Plain(0.5, 2.5)),
    ("rust", Range::Plain(0.0, 0.7)),
    ("damage", Range::Plain(0.0, 0.6)),
    ("drive", Range::Plain(0.0, 1.5)),
    ("width", Range::Plain(0.2, 0.9)),
    ("body", Range::Plain(0.3, 3.0)),
    ("env_attack", Range::Plain(0.001, 0.15)),
    ("env_decay", Range::Plain(0.1, 2.5)),
    ("env_sustain", Range::Plain(0.4, 1.0)),
    ("env_release", Range::Plain(0.1, 2.0)),
    ("mseg_onset", Range::Plain(0.001, 0.05)),
    ("mseg_onset_level", Range::Plain(0.0, 0.3)),
    ("mseg_attack", Range::Plain(0.001, 0.3)),
    ("mseg_hold", Range::Plain(0.0, 0.3)),
    ("mseg_decay", Range::Plain(0.1, 2.5)),
    ("mseg_sustain", Range::Plain(0.3, 1.0)),
    ("mseg_release", Range::Plain(0.05, 1.5)),
    ("env_amount", Range::Plain(0.5, 1.0)),
    ("velocity_to_peak", Range::Plain(0.2, 0.8)),
    ("loop_mode", Range::Choice(&[0, 0, 1, 2])),
    ("loop_start_stage", Range::Choice(&[1, 2, 3])),
    ("loop_end_stage", Range::Choice(&[4, 5])),
    ("sync_rate", Range::Plain(0.0, 1.0)),
    ("global_time_scale", Range::Plain(0.5, 2.0)),
    ("velocity_to_level", Range::Plain(0.5, 1.0)),
    ("velocity_to_time", Range::Plain(0.0, 0.5)),
    ("curve_tension", Range::Plain(-0.5, 0.5)),
    ("exciter_pressure", Range::Plain(0.2, 0.8)),
    ("exciter_speed", Range::Plain(0.2, 0.8)),
    ("exciter_roughness", Range::Plain(0.1, 0.7)),
    ("strike_position", Range::Plain(0.1, 0.9)),
    ("coupling_stiffness", Range::Plain(0.4, 1.0)),
    ("position_wander", Range::Plain(0.0, 0.4)),
    ("position_wander_rate", Range::Plain(0.1, 2.0)),
    ("position_envelope", Range::Plain(0.0, 0.5)),
    ("fundamental_anchor", Range::Plain(0.1, 0.7)),
    ("res_damping", Range::Plain(0.2, 0.8)),
    ("res_brightness", Range::Plain(0.2, 0.8)),
    ("thickness", Range::Plain(0.2, 0.8)),
    ("heat", Range::Plain(0.0, 0.4)),
    ("sludge", Range::Plain(0.0, 0.4)),
    ("macro_mass", Range::Plain(0.25, 0.75)),
    ("macro_corrosion", Range::Plain(0.25, 0.75)),
    ("macro_violence", Range::Plain(0.25, 0.75)),
    ("macro_brightness", Range::Plain(0.25, 0.75)),
    // Post chain.
    ("filter_cutoff", Range::Log(1_500.0, 20_000.0)),
    ("filter_resonance", Range::Plain(0.0, 0.5)),
    ("component_tolerance", Range::Plain(0.0, 0.5)),
    ("drive_amount", Range::Plain(0.0, 1.2)),
    ("bias_starvation", Range::Plain(0.0, 0.4)),
    ("chaos_depth", Range::Plain(0.0, 0.3)),
    ("spread_width", Range::Plain(0.2, 0.9)),
    ("listener_proximity", Range::Plain(0.2, 0.8)),
    ("chassis_material", Range::Plain(0.2, 0.8)),
    ("chassis_volume", Range::Plain(0.2, 0.8)),
    ("space_mode", Range::Choice(&[0, 1, 2, 3])),
    ("space_amount", Range::Plain(0.0, 0.5)),
    ("factory_size", Range::Plain(0.1, 0.9)),
    ("machinery_clutter", Range::Plain(0.1, 0.9)),
    ("wall_impedance", Range::Plain(0.1, 0.9)),
    ("spring_tension", Range::Plain(0.1, 0.9)),
    ("wire_stiffness", Range::Plain(0.1, 0.9)),
    ("spring_tank_size", Range::Plain(0.1, 0.9)),
    ("delay_time", Range::Plain(0.1, 0.9)),
    ("machinery_movement", Range::Plain(0.1, 0.9)),
    ("high_frequency_damping", Range::Plain(0.1, 0.9)),
    ("diode_softness", Range::Plain(0.0, 1.0)),
    ("diode_asymmetry", Range::Plain(0.0, 0.5)),
];

/// Exciters whose contact is continuous (friction, air, field, particles):
/// their presets play in Drone mode when the seed says so. Strikes stay
/// Tonal.
const CONTINUOUS_EXCITERS: [i32; 7] = [1, 10, 11, 12, 13, 14, 16];

const PLAY_MODE_TONAL: i32 = 0;
const PLAY_MODE_DRONE: i32 = 2;

/// SplitMix64: a small, well-mixed integer hash.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// A value in `0.0..1.0` for draw `index` of the pair's seed.
fn unit(seed: u64, index: u64) -> f64 {
    (mix(seed ^ mix(index)) >> 11) as f64 / (1u64 << 53) as f64
}

fn pair_seed(exciter: i32, object: Object, try_index: u32) -> u64 {
    mix(((exciter as u64) << 32) | ((object.to_int() as u64) << 16) | try_index as u64)
}

/// Draw index of a parameter: a hash of its ID, so adding a parameter
/// doesn't change the others' draws.
fn draw_index(id: &str) -> u64 {
    id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Round to 4 significant digits, so the files stay readable.
fn round4(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = 10f64.powi(3 - x.abs().log10().floor() as i32);
    (x * scale).round() / scale
}

/// The plain value `range` gives for `t` in `0.0..1.0`.
fn draw(range: Range, ptr: ParamPtr, t: f64) -> f64 {
    match range {
        Range::Normalized(lo, hi) => {
            let normalized = lo as f64 + t * (hi - lo) as f64;
            // SAFETY: `ptr` comes from a `CorrosionParams` alive for the call.
            round4(unsafe { ptr.preview_plain(normalized as f32) } as f64)
        }
        Range::Plain(lo, hi) => round4(lo as f64 + t * (hi - lo) as f64),
        Range::Log(lo, hi) => round4((lo as f64) * ((hi / lo) as f64).powf(t)),
        Range::Choice(values) => {
            values[((t * values.len() as f64) as usize).min(values.len() - 1)] as f64
        }
    }
}

/// Set `id` to `value` in a preset's JSON: at the top level if it is a
/// top-level key, otherwise under `extra`. Integer parameters are written as
/// integers.
fn set(json: &mut serde_json::Value, id: &str, value: f64, integer: bool) {
    let number = if integer {
        serde_json::Value::from(value.round() as i64)
    } else {
        serde_json::Value::from(value)
    };
    if json.get(id).is_some() {
        json[id] = number;
    } else {
        json["extra"][id] = number;
    }
}

/// The experimental preset for (`exciter`, `object`), drawn with seed try
/// `try_index`, before levelling. Named `<Exciter> · <Object>`.
pub fn experimental_preset(exciter: i32, object: Object, try_index: u32) -> Preset {
    let name = format!("{} · {}", super::exciter_name(exciter), object.name());
    let defaults = CorrosionParams::default();
    let ptrs: HashMap<String, ParamPtr> = defaults
        .param_map()
        .into_iter()
        .map(|(id, ptr, _)| (id, ptr))
        .collect();
    let seed = pair_seed(exciter, object, try_index);

    let mut preset = super::base(&name);
    preset.exciter = exciter;
    preset.object = object;
    preset.play_mode = if CONTINUOUS_EXCITERS.contains(&exciter) && unit(seed, 0) < 0.5 {
        PLAY_MODE_DRONE
    } else {
        PLAY_MODE_TONAL
    };
    let mut json = serde_json::to_value(&preset).expect("serialize preset");

    let specific = exciter_slot_ids(exciter)
        .iter()
        .chain(object_character_ids(object))
        .map(|id| (*id, SPECIFIC_RANGE));
    for (id, range) in specific.chain(SHARED_RANGES.iter().copied()) {
        let ptr = ptrs[id];
        let value = draw(range, ptr, unit(seed, draw_index(id)));
        set(&mut json, id, value, matches!(ptr, ParamPtr::IntParam(_)));
    }
    Preset::from_json_str(&json.to_string()).expect("generated preset parses")
}

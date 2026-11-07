//! Factory preset seeder for Corrosion.
//!
//! Run with `cargo run --bin seed_presets` to regenerate the 50 factory
//! presets under `presets/factory/`. Each preset is constructed
//! programmatically from a typed `Preset` so future schema changes are
//! caught at compile time and so the design intent of each preset stays
//! readable next to its parameters.
//!
//! Naming conventions:
//! - Title-case display name (used by the GUI loader and by tests).
//! - Filename = lowercase + underscore-separated.
//!
//! Each preset is anchored on an Object × Exciter pair and tweaks only the
//! fields that give it identity; everything else inherits from
//! `CorrosionParams::default()`.
use std::path::PathBuf;

use corrosion::{CorrosionParams, Object, Preset};

// --- Exciter id constants (mirrors `params::ExciterType::to_int`) ----------
const EX_BOW: i32 = 1;
const EX_HAND_STRIKE: i32 = 2;
const EX_FELT_MALLET: i32 = 3;
const EX_HARD_MALLET: i32 = 4;
const EX_DRUMSTICK: i32 = 5;
const EX_WIRE_BRUSH: i32 = 6;
const EX_METAL_PIPE: i32 = 7;
const EX_METAL_CHAIN: i32 = 8;
const EX_STIFF_POINT: i32 = 9;
const EX_HEAVY_GRINDING: i32 = 10;
const EX_CORRUGATED_DRAG: i32 = 11;
const EX_TENSION_RISE: i32 = 12;
const EX_PNEUMATIC_JET: i32 = 13;
const EX_ELECTROMAGNETIC_HUM: i32 = 14;
const EX_TENSION_SNAP: i32 = 15;

// PlayMode::Drone for sustained presets — see params::PlayMode.
const PLAY_MODE_DRONE: i32 = 2;

fn base(name: &str) -> Preset {
    Preset::from_params(name, &CorrosionParams::default())
}

fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

// --- Pipe family -----------------------------------------------------------

fn boiler_pipe_tap() -> Preset {
    // Small bright pipe percussion — wood drumstick on a thin pipe.
    let mut p = base("Boiler Pipe Tap");
    p.object = Object::Pipe;
    p.exciter = EX_DRUMSTICK;
    p.size = 0.55;
    p.rust = 0.15;
    p.body = 0.4;
    p.extra.pipe_diameter = 0.3;
    p.extra.stick_mass = 0.4;
    p.extra.tip_stiffness = 4.2;
    p.extra.macro_brightness = 0.65;
    p
}

fn cold_pipe_bow() -> Preset {
    // Bowed pipe — singing, slowly evolving timbre. Drone-ready.
    let mut p = base("Cold Pipe Bow");
    p.object = Object::Pipe;
    p.exciter = EX_BOW;
    p.size = 1.4;
    p.body = 0.8;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.pipe_diameter = 0.55;
    p.extra.bow_pressure = 0.9;
    p.extra.bow_speed = 0.8;
    p.extra.rosin_grip = 0.55;
    p.extra.macro_brightness = 0.55;
    p.extra.macro_mass = 0.55;
    p
}

fn hollow_smokestack_hum() -> Preset {
    // Massive industrial pipe with electromagnetic hum — dark sustained drone.
    let mut p = base("Hollow Smokestack Hum");
    p.object = Object::Pipe;
    p.exciter = EX_ELECTROMAGNETIC_HUM;
    p.size = 3.5;
    p.rust = 1.2;
    p.body = 1.5;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.pipe_diameter = 0.85;
    p.extra.mains_frequency = 50.0;
    p.extra.coil_proximity = 1.4;
    p.extra.voltage_sag = 0.45;
    p.extra.macro_mass = 0.8;
    p.extra.macro_corrosion = 0.65;
    p.extra.space_mode = 1; // Factory
    p.extra.space_amount = 0.45;
    p
}

fn ironforge_anvil_strike() -> Preset {
    // Hard mallet on iron pipe — bright forge strike with bite.
    let mut p = base("Ironforge Anvil Strike");
    p.object = Object::Pipe;
    p.exciter = EX_HARD_MALLET;
    p.size = 1.8;
    p.rust = 0.3;
    p.damage = 0.5;
    p.drive = 1.5;
    p.body = 1.0;
    p.extra.pipe_diameter = 0.4;
    p.extra.material_stiffness = 4.0;
    p.extra.impact_damping = 0.35;
    p.extra.macro_violence = 0.75;
    p.extra.drive_amount = 0.5;
    p
}

fn pipe_bell_cluster() -> Preset {
    // Felt mallet on pipe — bell-like, melodic, sweet.
    let mut p = base("Pipe Bell Cluster");
    p.object = Object::Pipe;
    p.exciter = EX_FELT_MALLET;
    p.size = 1.2;
    p.body = 1.0;
    p.extra.pipe_diameter = 0.5;
    p.extra.mallet_mass = 1.8;
    p.extra.felt_softness = 0.7;
    p.extra.core_hardness = 2.0;
    p.extra.macro_brightness = 0.6;
    p
}

fn pneumatic_whistle() -> Preset {
    // Pneumatic jet through a long pipe — breathy, sustained.
    let mut p = base("Pneumatic Whistle");
    p.object = Object::Pipe;
    p.exciter = EX_PNEUMATIC_JET;
    p.size = 2.0;
    p.body = 0.6;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.pipe_diameter = 0.45;
    p.extra.air_pressure = 1.3;
    p.extra.nozzle_width = 0.7;
    p.extra.turbulence_chaos = 0.5;
    p.extra.macro_brightness = 0.7;
    p
}

// --- Plate family ----------------------------------------------------------

fn distant_plate_roll() -> Preset {
    // Wire brush on plate — soft, distant texture with reverb.
    let mut p = base("Distant Plate Roll");
    p.object = Object::Plate;
    p.exciter = EX_WIRE_BRUSH;
    p.size = 1.8;
    p.body = 0.5;
    p.extra.plate_aspect = 1.6;
    p.extra.wire_density = 60.0;
    p.extra.spread_duration = 180.0;
    p.extra.brush_wire_stiffness = 0.25;
    p.extra.amplitude_randomization = 0.4;
    p.extra.space_mode = 2; // Spring
    p.extra.space_amount = 0.55;
    p.extra.macro_brightness = 0.55;
    p
}

fn industrial_plate_bow() -> Preset {
    // Bowed plate — long sustained tone with character.
    let mut p = base("Industrial Plate Bow");
    p.object = Object::Plate;
    p.exciter = EX_BOW;
    p.size = 2.0;
    p.body = 1.0;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.plate_aspect = 1.4;
    p.extra.plate_stiffness = 1.6;
    p.extra.bow_pressure = 1.1;
    p.extra.bow_speed = 0.9;
    p.extra.rosin_grip = 0.5;
    p.extra.macro_brightness = 0.55;
    p
}

fn plate_resonance_bath() -> Preset {
    // Tension-rise on a plate — slow swell into bloom.
    let mut p = base("Plate Resonance Bath");
    p.object = Object::Plate;
    p.exciter = EX_TENSION_RISE;
    p.size = 2.2;
    p.body = 1.6;
    p.extra.plate_aspect = 1.2;
    p.extra.plate_stiffness = 1.0;
    p.extra.pull_speed = 0.6;
    p.extra.break_threshold = 0.75;
    p.extra.creak_sharpness = 0.4;
    p.extra.space_mode = 1; // Factory
    p.extra.space_amount = 0.6;
    p.extra.macro_brightness = 0.6;
    p
}

fn rusted_plate_strike() -> Preset {
    // Aged plate with felt mallet — warm, slightly dampened.
    let mut p = base("Rusted Plate Strike");
    p.object = Object::Plate;
    p.exciter = EX_FELT_MALLET;
    p.size = 1.5;
    p.rust = 2.5;
    p.damage = 1.0;
    p.body = 0.8;
    p.extra.plate_aspect = 1.3;
    p.extra.plate_stiffness = 0.7;
    p.extra.mallet_mass = 1.5;
    p.extra.felt_softness = 1.1;
    p.extra.macro_corrosion = 0.7;
    p.extra.macro_brightness = 0.4;
    p
}

fn sheet_metal_thunder() -> Preset {
    // Hand strike on large plate — booming, ominous.
    let mut p = base("Sheet Metal Thunder");
    p.object = Object::Plate;
    p.exciter = EX_HAND_STRIKE;
    p.size = 3.0;
    p.damage = 2.0;
    p.drive = 1.2;
    p.body = 1.4;
    p.extra.plate_aspect = 2.0;
    p.extra.plate_stiffness = 0.8;
    p.extra.hand_mass = 2.2;
    p.extra.flesh_stiffness = 0.35;
    p.extra.macro_violence = 0.7;
    p.extra.macro_mass = 0.7;
    p.extra.space_mode = 1; // Factory
    p.extra.space_amount = 0.5;
    p
}

fn workshop_plate_scrape() -> Preset {
    // Stiff point scrape across a plate — high-friction whine.
    let mut p = base("Workshop Plate Scrape");
    p.object = Object::Plate;
    p.exciter = EX_STIFF_POINT;
    p.size = 1.4;
    p.body = 0.5;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.plate_aspect = 1.5;
    p.extra.plate_stiffness = 1.4;
    p.extra.scrape_speed = 1.0;
    p.extra.point_pressure = 0.95;
    p.extra.chatter_pitch = 0.65;
    p.extra.chatter_damping = 0.5;
    p.extra.macro_brightness = 0.7;
    p
}

// --- Tank family -----------------------------------------------------------

fn boiler_tank_whoosh() -> Preset {
    // Pneumatic jet inside a tank — atmospheric pressure release.
    let mut p = base("Boiler Tank Whoosh");
    p.object = Object::Tank;
    p.exciter = EX_PNEUMATIC_JET;
    p.size = 2.5;
    p.body = 1.0;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.tank_volume = 0.8;
    p.extra.tank_cavity_mix = 0.7;
    p.extra.air_pressure = 1.5;
    p.extra.nozzle_width = 0.9;
    p.extra.turbulence_chaos = 0.65;
    p.extra.macro_brightness = 0.45;
    p
}

fn brine_tank_pulse() -> Preset {
    // Electromagnetic hum inside a tank — low pulsing drone.
    let mut p = base("Brine Tank Pulse");
    p.object = Object::Tank;
    p.exciter = EX_ELECTROMAGNETIC_HUM;
    p.size = 2.8;
    p.body = 1.4;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.tank_volume = 0.75;
    p.extra.tank_cavity_mix = 0.85;
    p.extra.mains_frequency = 60.0;
    p.extra.coil_proximity = 1.2;
    p.extra.voltage_sag = 0.55;
    p.extra.macro_mass = 0.7;
    p.extra.macro_brightness = 0.35;
    p
}

fn deep_tank_bell() -> Preset {
    // Felt mallet on tank — sub-octave bell tone with cavity bloom.
    let mut p = base("Deep Tank Bell");
    p.object = Object::Tank;
    p.exciter = EX_FELT_MALLET;
    p.size = 2.0;
    p.body = 1.4;
    p.extra.tank_volume = 0.6;
    p.extra.tank_cavity_mix = 0.75;
    p.extra.mallet_mass = 2.0;
    p.extra.felt_softness = 0.9;
    p.extra.core_hardness = 2.2;
    p.extra.macro_mass = 0.6;
    p
}

fn empty_drum_cavern() -> Preset {
    // Hand strike on hollow tank — dry, dark cavern slap.
    let mut p = base("Empty Drum Cavern");
    p.object = Object::Tank;
    p.exciter = EX_HAND_STRIKE;
    p.size = 2.2;
    p.body = 0.6;
    p.extra.tank_volume = 0.55;
    p.extra.tank_cavity_mix = 0.5;
    p.extra.hand_mass = 1.8;
    p.extra.flesh_damping = 0.85;
    p.extra.macro_brightness = 0.35;
    p
}

fn liquid_tank_drone() -> Preset {
    // Bowed tank — long sustained liquid-like drone.
    let mut p = base("Liquid Tank Drone");
    p.object = Object::Tank;
    p.exciter = EX_BOW;
    p.size = 2.5;
    p.body = 1.2;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.tank_volume = 0.7;
    p.extra.tank_cavity_mix = 0.65;
    p.extra.bow_pressure = 0.85;
    p.extra.bow_speed = 0.7;
    p.extra.rosin_grip = 0.6;
    p.extra.macro_brightness = 0.45;
    p
}

fn steel_tank_rumble() -> Preset {
    // Hard mallet on steel tank — powerful low impact.
    let mut p = base("Steel Tank Rumble");
    p.object = Object::Tank;
    p.exciter = EX_HARD_MALLET;
    p.size = 2.8;
    p.damage = 1.5;
    p.drive = 1.0;
    p.body = 1.5;
    p.extra.tank_volume = 0.7;
    p.extra.tank_cavity_mix = 0.6;
    p.extra.material_stiffness = 3.5;
    p.extra.impact_damping = 0.4;
    p.extra.macro_mass = 0.75;
    p.extra.macro_violence = 0.6;
    p
}

// --- Chain family ----------------------------------------------------------

fn chain_bell_tower() -> Preset {
    // Hard mallet on a chain — bell-tower cluster of pitches.
    let mut p = base("Chain Bell Tower");
    p.object = Object::Chain;
    p.exciter = EX_HARD_MALLET;
    p.size = 1.6;
    p.body = 0.8;
    p.extra.chain_link_mass = 0.6;
    p.extra.chain_instability = 0.4;
    p.extra.material_stiffness = 3.0;
    p.extra.macro_brightness = 0.6;
    p.extra.space_mode = 1;
    p.extra.space_amount = 0.4;
    p
}

fn chain_drag() -> Preset {
    // Chain dragged across corrugated surface — clattery scrape.
    let mut p = base("Chain Drag");
    p.object = Object::Chain;
    p.exciter = EX_CORRUGATED_DRAG;
    p.size = 1.4;
    p.damage = 1.5;
    p.body = 0.6;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.chain_link_mass = 0.5;
    p.extra.chain_instability = 0.5;
    p.extra.drag_speed = 1.0;
    p.extra.ridge_spacing = 0.18;
    p.extra.ridge_depth = 0.7;
    p.extra.macro_corrosion = 0.6;
    p
}

fn chain_whip_snap() -> Preset {
    // Chain under tension that snaps — sharp aggressive transient.
    let mut p = base("Chain Whip Snap");
    p.object = Object::Chain;
    p.exciter = EX_TENSION_SNAP;
    p.size = 1.2;
    p.damage = 2.5;
    p.drive = 1.4;
    p.extra.chain_link_mass = 0.45;
    p.extra.chain_instability = 0.6;
    p.extra.pull_distance = 0.9;
    p.extra.hook_stiffness = 1.6;
    p.extra.snap_force = 0.85;
    p.extra.macro_violence = 0.75;
    p
}

fn falling_chain_cluster() -> Preset {
    // Self-exciter: a chain dropping on itself.
    let mut p = base("Falling Chain Cluster");
    p.object = Object::Chain;
    p.exciter = EX_METAL_CHAIN;
    p.size = 1.5;
    p.rust = 0.5;
    p.damage = 1.0;
    p.body = 0.6;
    p.extra.chain_link_mass = 0.55;
    p.extra.chain_instability = 0.55;
    p.extra.link_count = 12.0;
    p.extra.chain_mass = 0.9;
    p.extra.drop_envelope_spread = 280.0;
    p.extra.internal_rattle = 0.45;
    p.extra.macro_brightness = 0.55;
    p
}

fn hanging_chain_echo() -> Preset {
    // Drumstick hit on hanging chain with echo space.
    let mut p = base("Hanging Chain Echo");
    p.object = Object::Chain;
    p.exciter = EX_DRUMSTICK;
    p.size = 1.7;
    p.body = 0.5;
    p.extra.chain_link_mass = 0.55;
    p.extra.chain_instability = 0.35;
    p.extra.stick_mass = 0.7;
    p.extra.tip_stiffness = 3.5;
    p.extra.restitution_bounciness = 0.35;
    p.extra.space_mode = 3; // Echo
    p.extra.space_amount = 0.5;
    p.extra.sync_rate = 0.5; // 1/4 tempo-synced
    p
}

fn sympathetic_chain() -> Preset {
    // Metal pipe striking a chain — sympathetic resonance.
    let mut p = base("Sympathetic Chain");
    p.object = Object::Chain;
    p.exciter = EX_METAL_PIPE;
    p.size = 1.8;
    p.rust = 0.4;
    p.body = 0.9;
    p.extra.chain_link_mass = 0.6;
    p.extra.chain_instability = 0.3;
    p.extra.pipe_mass = 1.6;
    p.extra.metal_stiffness = 3.2;
    p.extra.pipe_pitch = 1.0;
    p.extra.pipe_ring_decay = 0.98;
    p.extra.macro_brightness = 0.55;
    p
}

// --- IBeam family ----------------------------------------------------------

fn beam_drag() -> Preset {
    // Corrugated drag across a beam — industrial mechanical texture.
    let mut p = base("Beam Drag");
    p.object = Object::IBeam;
    p.exciter = EX_CORRUGATED_DRAG;
    p.size = 1.8;
    p.damage = 1.0;
    p.body = 0.8;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.beam_shear = 0.6;
    p.extra.drag_speed = 0.85;
    p.extra.ridge_spacing = 0.14;
    p.extra.ridge_depth = 0.6;
    p.extra.drag_exciter_mass = 1.2;
    p.extra.macro_corrosion = 0.55;
    p
}

fn beam_grind() -> Preset {
    // Heavy grinding against a beam — industrial machining roar.
    let mut p = base("Beam Grind");
    p.object = Object::IBeam;
    p.exciter = EX_HEAVY_GRINDING;
    p.size = 2.0;
    p.damage = 1.5;
    p.drive = 1.6;
    p.body = 1.0;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.beam_shear = 0.7;
    p.extra.grind_speed = 0.9;
    p.extra.grind_pressure = 1.2;
    p.extra.surface_grit = 0.6;
    p.extra.macro_violence = 0.75;
    p.extra.drive_amount = 0.55;
    p
}

fn beam_strike_industrial() -> Preset {
    // Hard mallet on I-beam — high-velocity construction impact.
    let mut p = base("Beam Strike Industrial");
    p.object = Object::IBeam;
    p.exciter = EX_HARD_MALLET;
    p.size = 1.7;
    p.damage = 0.8;
    p.drive = 1.0;
    p.body = 1.2;
    p.extra.beam_shear = 0.55;
    p.extra.material_stiffness = 3.8;
    p.extra.impact_damping = 0.3;
    p.extra.macro_mass = 0.65;
    p.extra.macro_violence = 0.6;
    p
}

fn bow_on_beam() -> Preset {
    // Bowed steel beam — slow industrial drone, hollow & metallic.
    let mut p = base("Bow on Beam");
    p.object = Object::IBeam;
    p.exciter = EX_BOW;
    p.size = 2.2;
    p.body = 1.0;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.beam_shear = 0.5;
    p.extra.bow_pressure = 1.0;
    p.extra.bow_speed = 0.85;
    p.extra.rosin_grip = 0.55;
    p.extra.macro_brightness = 0.5;
    p
}

fn brushed_beam() -> Preset {
    // Wire brush across a beam — gentle rolling texture.
    let mut p = base("Brushed Beam");
    p.object = Object::IBeam;
    p.exciter = EX_WIRE_BRUSH;
    p.size = 1.6;
    p.body = 0.6;
    p.extra.beam_shear = 0.45;
    p.extra.wire_density = 55.0;
    p.extra.spread_duration = 160.0;
    p.extra.brush_wire_stiffness = 0.3;
    p.extra.amplitude_randomization = 0.35;
    p.extra.macro_brightness = 0.55;
    p
}

fn drum_beam() -> Preset {
    // Drumstick on a beam — clean wood-on-metal hit.
    let mut p = base("Drum Beam");
    p.object = Object::IBeam;
    p.exciter = EX_DRUMSTICK;
    p.size = 1.4;
    p.body = 0.8;
    p.extra.beam_shear = 0.5;
    p.extra.stick_mass = 0.7;
    p.extra.tip_stiffness = 4.0;
    p.extra.restitution_bounciness = 0.45;
    p.extra.micro_bounce_limit = 3.0;
    p.extra.macro_brightness = 0.6;
    p
}

// --- TautCable family ------------------------------------------------------

fn cable_bow() -> Preset {
    // Bowed steel cable — singing string-like tone.
    let mut p = base("Cable Bow");
    p.object = Object::TautCable;
    p.exciter = EX_BOW;
    p.size = 1.4;
    p.body = 0.5;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.cable_braid = 0.4;
    p.extra.cable_tension_drop = 0.2;
    p.extra.bow_pressure = 0.95;
    p.extra.bow_speed = 0.95;
    p.extra.rosin_grip = 0.45;
    p.extra.macro_brightness = 0.65;
    p
}

fn cable_scrape() -> Preset {
    // Stiff point scrape across cable — high friction whine.
    let mut p = base("Cable Scrape");
    p.object = Object::TautCable;
    p.exciter = EX_STIFF_POINT;
    p.size = 1.2;
    p.damage = 0.8;
    p.body = 0.4;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.cable_braid = 0.5;
    p.extra.scrape_speed = 1.05;
    p.extra.point_pressure = 0.9;
    p.extra.chatter_pitch = 0.7;
    p.extra.chatter_damping = 0.45;
    p
}

fn cable_tension_rise() -> Preset {
    // Cable being pulled to breaking point — anticipation builds.
    let mut p = base("Cable Tension Rise");
    p.object = Object::TautCable;
    p.exciter = EX_TENSION_RISE;
    p.size = 1.5;
    p.damage = 1.2;
    p.body = 0.6;
    p.extra.cable_braid = 0.4;
    p.extra.cable_tension_drop = 0.6;
    p.extra.pull_speed = 0.7;
    p.extra.break_threshold = 0.8;
    p.extra.slip_stochasticity = 0.35;
    p.extra.creak_sharpness = 0.55;
    p.extra.macro_brightness = 0.55;
    p
}

fn snapped_cable() -> Preset {
    // Sharp cable snap — sudden release of tension. Snap_force needs to be low
    // enough that the tension build trips it inside the voice's ADSR attack
    // window; otherwise the single-sample impulse lands at ~0 envelope.
    let mut p = base("Snapped Cable");
    p.object = Object::TautCable;
    p.exciter = EX_TENSION_SNAP;
    p.size = 1.3;
    p.damage = 1.8;
    p.drive = 1.2;
    p.body = 0.8;
    // Stretch envelope so the snap impulse fires inside an audible window.
    p.extra.env_attack = 0.005;
    p.extra.env_decay = 1.5;
    p.extra.env_sustain = 0.8;
    p.extra.env_release = 0.5;
    p.extra.cable_braid = 0.4;
    p.extra.cable_tension_drop = 0.3;
    p.extra.pull_distance = 0.5;
    p.extra.hook_stiffness = 2.0;
    p.extra.snap_force = 0.3;
    p.extra.macro_violence = 0.7;
    p
}

fn suspension_cable_pluck() -> Preset {
    // Hard mallet pluck on heavy suspension cable.
    let mut p = base("Suspension Cable Pluck");
    p.object = Object::TautCable;
    p.exciter = EX_HARD_MALLET;
    p.size = 2.0;
    p.body = 1.0;
    p.extra.cable_braid = 0.45;
    p.extra.cable_tension_drop = 0.35;
    p.extra.material_stiffness = 3.2;
    p.extra.impact_damping = 0.4;
    p.extra.macro_mass = 0.65;
    p.extra.macro_brightness = 0.5;
    p
}

// --- CoilSpring family -----------------------------------------------------

fn bowed_spring() -> Preset {
    // Bowed coil spring — boing-y sustained tone with dispersion.
    let mut p = base("Bowed Spring");
    p.object = Object::CoilSpring;
    p.exciter = EX_BOW;
    p.size = 1.6;
    p.body = 0.7;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.spring_dispersion = 0.7;
    p.extra.spring_slosh = 0.5;
    p.extra.bow_pressure = 0.9;
    p.extra.bow_speed = 0.85;
    p.extra.rosin_grip = 0.5;
    p.extra.macro_brightness = 0.55;
    p
}

fn spring_drag() -> Preset {
    // Drag along a spring — clattering metal coils.
    let mut p = base("Spring Drag");
    p.object = Object::CoilSpring;
    p.exciter = EX_CORRUGATED_DRAG;
    p.size = 1.4;
    p.damage = 1.0;
    p.body = 0.5;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.spring_dispersion = 0.6;
    p.extra.spring_slosh = 0.45;
    p.extra.drag_speed = 1.0;
    p.extra.ridge_spacing = 0.12;
    p.extra.ridge_depth = 0.55;
    p
}

fn spring_grind() -> Preset {
    // Heavy grinding against a spring — sustained machine noise. Drop Drone
    // mode here: the friction MSEG sustains the grinding by default for the
    // 2 s render window, and forcing loop_start_stage=3/end=4 on top of the
    // CoilSpring's dispersive resonator was producing a self-cancelling
    // standing wave at our default mseg_sustain level. Higher grind_speed
    // keeps the relative velocity above the zero-crossing dead-band.
    let mut p = base("Spring Grind");
    p.object = Object::CoilSpring;
    p.exciter = EX_HEAVY_GRINDING;
    p.size = 1.8;
    p.damage = 1.5;
    p.drive = 1.4;
    p.body = 0.6;
    p.extra.spring_dispersion = 0.5;
    p.extra.spring_slosh = 0.3;
    p.extra.grind_speed = 1.5;
    p.extra.grind_pressure = 1.4;
    p.extra.surface_grit = 0.6;
    p.extra.grit_color = 0.45;
    p.extra.mseg_sustain = 0.8;
    p.extra.macro_violence = 0.75;
    p.extra.drive_amount = 0.4;
    p
}

fn spring_pluck() -> Preset {
    // Hand strike on coil spring — vintage spring reverb pluck.
    let mut p = base("Spring Pluck");
    p.object = Object::CoilSpring;
    p.exciter = EX_HAND_STRIKE;
    p.size = 1.5;
    p.body = 0.6;
    p.extra.spring_dispersion = 0.6;
    p.extra.spring_slosh = 0.4;
    p.extra.hand_mass = 1.5;
    p.extra.flesh_stiffness = 0.45;
    p.extra.macro_brightness = 0.6;
    p
}

fn spring_reverb_tap() -> Preset {
    // Classic spring-reverb-style tap excited with hard mallet.
    let mut p = base("Spring Reverb Tap");
    p.object = Object::CoilSpring;
    p.exciter = EX_HARD_MALLET;
    p.size = 1.4;
    p.body = 1.2;
    p.extra.spring_dispersion = 0.75;
    p.extra.spring_slosh = 0.6;
    p.extra.material_stiffness = 3.5;
    p.extra.impact_damping = 0.35;
    p.extra.space_mode = 2; // Spring
    p.extra.space_amount = 0.65;
    p.extra.macro_brightness = 0.6;
    p
}

// --- SheetMetal family -----------------------------------------------------

fn sheet_bend() -> Preset {
    // Tension rise on a sheet — bending pre-buckle.
    let mut p = base("Sheet Bend");
    p.object = Object::SheetMetal;
    p.exciter = EX_TENSION_RISE;
    p.size = 1.8;
    p.damage = 1.0;
    p.body = 0.8;
    p.extra.sheet_thinness = 0.5;
    p.extra.pull_speed = 0.65;
    p.extra.break_threshold = 0.75;
    p.extra.creak_sharpness = 0.5;
    p.extra.macro_brightness = 0.6;
    p
}

fn sheet_bow() -> Preset {
    // Bowed sheet metal — singing, slow-bloom drone.
    let mut p = base("Sheet Bow");
    p.object = Object::SheetMetal;
    p.exciter = EX_BOW;
    p.size = 2.0;
    p.body = 1.0;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.sheet_thinness = 0.45;
    p.extra.bow_pressure = 0.95;
    p.extra.bow_speed = 0.8;
    p.extra.rosin_grip = 0.55;
    p.extra.macro_brightness = 0.6;
    p
}

fn sheet_brush() -> Preset {
    // Wire brush across thin sheet — soft swept noise.
    let mut p = base("Sheet Brush");
    p.object = Object::SheetMetal;
    p.exciter = EX_WIRE_BRUSH;
    p.size = 1.5;
    p.body = 0.5;
    p.extra.sheet_thinness = 0.55;
    p.extra.wire_density = 70.0;
    p.extra.spread_duration = 200.0;
    p.extra.brush_wire_stiffness = 0.2;
    p.extra.amplitude_randomization = 0.5;
    p.extra.macro_brightness = 0.65;
    p
}

fn sheet_lightning() -> Preset {
    // Hand strike on large sheet — booming thunder-sheet effect.
    let mut p = base("Sheet Lightning");
    p.object = Object::SheetMetal;
    p.exciter = EX_HAND_STRIKE;
    p.size = 2.8;
    p.damage = 2.5;
    p.drive = 1.5;
    p.body = 1.6;
    p.extra.sheet_thinness = 0.5;
    p.extra.hand_mass = 2.0;
    p.extra.flesh_stiffness = 0.4;
    p.extra.macro_violence = 0.75;
    p.extra.macro_mass = 0.7;
    p.extra.space_mode = 1; // Factory
    p.extra.space_amount = 0.55;
    p
}

fn thin_sheet_roll() -> Preset {
    // Drumstick rolls on thin sheet metal — bouncy clatter.
    let mut p = base("Thin Sheet Roll");
    p.object = Object::SheetMetal;
    p.exciter = EX_DRUMSTICK;
    p.size = 1.3;
    p.body = 0.6;
    p.extra.sheet_thinness = 0.6;
    p.extra.stick_mass = 0.55;
    p.extra.tip_stiffness = 3.8;
    p.extra.restitution_bounciness = 0.6;
    p.extra.micro_bounce_limit = 5.0;
    p.extra.macro_brightness = 0.7;
    p
}

// --- IndustrialCog family --------------------------------------------------

fn cog_hum() -> Preset {
    // Electromagnetic hum coupling to a cog — mechanical drone.
    let mut p = base("Cog Hum");
    p.object = Object::IndustrialCog;
    p.exciter = EX_ELECTROMAGNETIC_HUM;
    p.size = 2.0;
    p.rust = 0.8;
    p.body = 1.0;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.cog_dissonance = 0.25;
    p.extra.mains_frequency = 50.0;
    p.extra.coil_proximity = 1.1;
    p.extra.voltage_sag = 0.5;
    p.extra.macro_corrosion = 0.55;
    p.extra.macro_brightness = 0.45;
    p
}

fn cog_mesh_grind() -> Preset {
    // Two cogs grinding against each other — engaged gears.
    let mut p = base("Cog Mesh Grind");
    p.object = Object::IndustrialCog;
    p.exciter = EX_HEAVY_GRINDING;
    p.size = 1.7;
    p.damage = 1.2;
    p.drive = 1.3;
    p.body = 0.8;
    p.play_mode = PLAY_MODE_DRONE;
    p.extra.cog_dissonance = 0.35;
    p.extra.grind_speed = 1.0;
    p.extra.grind_pressure = 1.1;
    p.extra.surface_grit = 0.5;
    p.extra.macro_violence = 0.65;
    p
}

fn cog_scrape() -> Preset {
    // Stiff point scrape across cog teeth — chattering metal.
    let mut p = base("Cog Scrape");
    p.object = Object::IndustrialCog;
    p.exciter = EX_STIFF_POINT;
    p.size = 1.5;
    p.damage = 0.6;
    p.body = 0.6;
    p.extra.cog_dissonance = 0.3;
    p.extra.scrape_speed = 0.9;
    p.extra.point_pressure = 0.8;
    p.extra.chatter_pitch = 0.6;
    p.extra.chatter_damping = 0.55;
    p.extra.macro_brightness = 0.55;
    p
}

fn cog_tooth_pluck() -> Preset {
    // Drumstick striking individual teeth — pitched mechanical tap.
    let mut p = base("Cog Tooth Pluck");
    p.object = Object::IndustrialCog;
    p.exciter = EX_DRUMSTICK;
    p.size = 1.2;
    p.body = 0.5;
    p.extra.cog_dissonance = 0.15;
    p.extra.stick_mass = 0.6;
    p.extra.tip_stiffness = 4.2;
    p.extra.restitution_bounciness = 0.4;
    p.extra.macro_brightness = 0.65;
    p
}

fn cog_tooth_strike() -> Preset {
    // Hard mallet hammering a cog tooth — sharp metallic strike.
    let mut p = base("Cog Tooth Strike");
    p.object = Object::IndustrialCog;
    p.exciter = EX_HARD_MALLET;
    p.size = 1.6;
    p.damage = 0.7;
    p.drive = 1.0;
    p.body = 1.0;
    p.extra.cog_dissonance = 0.2;
    p.extra.material_stiffness = 3.6;
    p.extra.impact_damping = 0.35;
    p.extra.macro_violence = 0.6;
    p.extra.macro_brightness = 0.6;
    p
}

fn presets() -> Vec<Preset> {
    vec![
        // Pipe
        boiler_pipe_tap(),
        cold_pipe_bow(),
        hollow_smokestack_hum(),
        ironforge_anvil_strike(),
        pipe_bell_cluster(),
        pneumatic_whistle(),
        // Plate
        distant_plate_roll(),
        industrial_plate_bow(),
        plate_resonance_bath(),
        rusted_plate_strike(),
        sheet_metal_thunder(),
        workshop_plate_scrape(),
        // Tank
        boiler_tank_whoosh(),
        brine_tank_pulse(),
        deep_tank_bell(),
        empty_drum_cavern(),
        liquid_tank_drone(),
        steel_tank_rumble(),
        // Chain
        chain_bell_tower(),
        chain_drag(),
        chain_whip_snap(),
        falling_chain_cluster(),
        hanging_chain_echo(),
        sympathetic_chain(),
        // IBeam
        beam_drag(),
        beam_grind(),
        beam_strike_industrial(),
        bow_on_beam(),
        brushed_beam(),
        drum_beam(),
        // TautCable
        cable_bow(),
        cable_scrape(),
        cable_tension_rise(),
        snapped_cable(),
        suspension_cable_pluck(),
        // CoilSpring
        bowed_spring(),
        spring_drag(),
        spring_grind(),
        spring_pluck(),
        spring_reverb_tap(),
        // SheetMetal
        sheet_bend(),
        sheet_bow(),
        sheet_brush(),
        sheet_lightning(),
        thin_sheet_roll(),
        // IndustrialCog
        cog_hum(),
        cog_mesh_grind(),
        cog_scrape(),
        cog_tooth_pluck(),
        cog_tooth_strike(),
    ]
}

// --- Preset levels ------------------------------------------------------
//
// BODY (0–5) and output gain per preset. These presets are placeholders, to
// be replaced by real ones, so the levels follow a light rule rather than a
// careful re-voice (OpenSpec change `normalize-object-loudness`, design
// Decision 4; re-run by `resolve-exciter-tradeoffs`, which first reset the
// earlier raises above 1 to 1):
// - BODY is as C5 left it (`fix-post-chain-body-routing`).
// - `output` is kept unless the preset hits the limiter (trimmed in 5 %
//   steps until it doesn't) or its RMS is more than 12 dB below the bank's
//   median (raised to 12 dB below).
// Values and the per-preset levels come from `cargo test --release
// --no-default-features --lib preset_levels_table -- --ignored --nocapture`.
// (name, body, output)
const PRESET_LEVELS: [(&str, f32, f32); 50] = [
    ("Beam Drag", 0.48, 1.000), // rms 0.18828 (was 0.18828), peak 0.3385 (was 0.3385), hits 0 (was 0)
    ("Beam Grind", 0.92, 1.000), // rms 0.56714 (was 0.56714), peak 0.6774 (was 0.6774), hits 0 (was 0)
    ("Beam Strike Industrial", 0.97, 1.000), // rms 0.17413 (was 0.17413), peak 0.5993 (was 0.5993), hits 0 (was 0)
    ("Boiler Pipe Tap", 2.57, 1.000), // rms 0.17888 (was 0.17888), peak 0.3707 (was 0.3707), hits 0 (was 0)
    ("Boiler Tank Whoosh", 0.49, 1.000), // rms 0.07824 (was 0.07824), peak 0.1039 (was 0.1039), hits 0 (was 0)
    ("Bow on Beam", 0.48, 1.000), // rms 0.14960 (was 0.14960), peak 0.2660 (was 0.2660), hits 0 (was 0)
    ("Bowed Spring", 0.52, 1.000), // rms 0.09328 (was 0.09328), peak 0.1308 (was 0.1308), hits 0 (was 0)
    ("Brine Tank Pulse", 0.48, 1.000), // rms 0.23791 (was 0.23791), peak 0.3484 (was 0.3484), hits 0 (was 0)
    ("Brushed Beam", 1.90, 1.000), // rms 0.24572 (was 0.24572), peak 0.3543 (was 0.3543), hits 0 (was 0)
    ("Cable Bow", 0.00, 1.000), // rms 0.04579 (was 0.04579), peak 0.0783 (was 0.0783), hits 0 (was 0)
    ("Cable Scrape", 0.00, 1.000), // rms 0.09824 (was 0.09824), peak 0.2279 (was 0.2279), hits 0 (was 0)
    ("Cable Tension Rise", 0.00, 1.307), // rms 0.04374 (was 0.03055), peak 0.1134 (was 0.0792), hits 0 (was 0)
    ("Chain Bell Tower", 1.16, 1.000), // rms 0.05404 (was 0.05404), peak 0.2804 (was 0.2804), hits 0 (was 0)
    ("Chain Drag", 0.73, 1.000), // rms 0.08810 (was 0.08810), peak 0.1965 (was 0.1965), hits 0 (was 0)
    ("Chain Whip Snap", 2.01, 0.950), // rms 0.09402 (was 0.09402), peak 0.7378 (was 0.7378), hits 0 (was 0)
    ("Cog Hum", 0.49, 1.000), // rms 0.17951 (was 0.17951), peak 0.3178 (was 0.3178), hits 0 (was 0)
    ("Cog Mesh Grind", 1.22, 1.000), // rms 0.50090 (was 0.50090), peak 0.6828 (was 0.6828), hits 0 (was 0)
    ("Cog Scrape", 0.50, 1.000), // rms 0.11452 (was 0.11452), peak 0.2152 (was 0.2152), hits 0 (was 0)
    ("Cog Tooth Pluck", 1.39, 1.000), // rms 0.26432 (was 0.26432), peak 0.4463 (was 0.4463), hits 0 (was 0)
    ("Cog Tooth Strike", 0.94, 0.950), // rms 0.18652 (was 0.18652), peak 0.3437 (was 0.3437), hits 0 (was 0)
    ("Cold Pipe Bow", 0.53, 1.000), // rms 0.09781 (was 0.09781), peak 0.1550 (was 0.1550), hits 0 (was 0)
    ("Deep Tank Bell", 1.29, 1.000), // rms 0.22340 (was 0.22340), peak 0.4470 (was 0.4470), hits 0 (was 0)
    ("Distant Plate Roll", 2.68, 1.000), // rms 0.14311 (was 0.14311), peak 0.4249 (was 0.4249), hits 0 (was 0)
    ("Drum Beam", 1.57, 1.000), // rms 0.26303 (was 0.26303), peak 0.3903 (was 0.3903), hits 0 (was 0)
    ("Empty Drum Cavern", 2.55, 1.000), // rms 0.38088 (was 0.38088), peak 0.4444 (was 0.4444), hits 0 (was 0)
    ("Falling Chain Cluster", 2.62, 1.000), // rms 0.13385 (was 0.13385), peak 0.3816 (was 0.3816), hits 0 (was 0)
    ("Hanging Chain Echo", 0.94, 1.000), // rms 0.05867 (was 0.05867), peak 0.2818 (was 0.2818), hits 0 (was 0)
    ("Hollow Smokestack Hum", 0.47, 1.000), // rms 0.14221 (was 0.14221), peak 0.3683 (was 0.3683), hits 0 (was 0)
    ("Industrial Plate Bow", 2.40, 1.000), // rms 0.23467 (was 0.23467), peak 0.3413 (was 0.3413), hits 0 (was 0)
    ("Ironforge Anvil Strike", 0.78, 1.000), // rms 0.16398 (was 0.16398), peak 0.6184 (was 0.6184), hits 0 (was 0)
    ("Liquid Tank Drone", 0.50, 1.000), // rms 0.23873 (was 0.23873), peak 0.3507 (was 0.3507), hits 0 (was 0)
    ("Pipe Bell Cluster", 2.06, 1.000), // rms 0.17364 (was 0.17364), peak 0.4024 (was 0.4024), hits 0 (was 0)
    ("Plate Resonance Bath", 2.65, 1.000), // rms 0.24015 (was 0.24015), peak 0.7314 (was 0.7314), hits 0 (was 0)
    ("Pneumatic Whistle", 0.41, 1.000), // rms 0.14506 (was 0.14506), peak 0.2844 (was 0.2844), hits 0 (was 0)
    ("Rusted Plate Strike", 2.58, 1.000), // rms 0.20109 (was 0.20109), peak 0.4523 (was 0.4523), hits 0 (was 0)
    ("Sheet Bend", 0.00, 1.833), // rms 0.04374 (was 0.02387), peak 0.1409 (was 0.0769), hits 0 (was 0)
    ("Sheet Bow", 2.65, 1.000), // rms 0.33740 (was 0.33740), peak 0.5416 (was 0.5416), hits 0 (was 0)
    ("Sheet Brush", 2.67, 1.000), // rms 0.29657 (was 0.29657), peak 0.4191 (was 0.4191), hits 0 (was 0)
    ("Sheet Lightning", 2.52, 0.950), // rms 0.17936 (was 0.18880), peak 0.9182 (was 0.9661), hits 0 (was 251)
    ("Sheet Metal Thunder", 2.65, 0.950), // rms 0.17192 (was 0.18097), peak 0.9179 (was 0.9661), hits 0 (was 3)
    ("Snapped Cable", 1.23, 1.000), // rms 0.34846 (was 0.34846), peak 0.6588 (was 0.6588), hits 0 (was 0)
    ("Spring Drag", 0.52, 1.000), // rms 0.08172 (was 0.08172), peak 0.1710 (was 0.1710), hits 0 (was 0)
    ("Spring Grind", 0.95, 0.950), // rms 0.45003 (was 0.45003), peak 0.6745 (was 0.6745), hits 0 (was 0)
    ("Spring Pluck", 2.61, 1.000), // rms 0.33539 (was 0.33539), peak 0.4370 (was 0.4370), hits 0 (was 0)
    ("Spring Reverb Tap", 2.26, 0.950), // rms 0.08745 (was 0.08745), peak 0.3934 (was 0.3934), hits 0 (was 0)
    ("Steel Tank Rumble", 1.05, 1.000), // rms 0.31467 (was 0.31467), peak 0.6113 (was 0.6113), hits 0 (was 0)
    ("Suspension Cable Pluck", 0.15, 1.201), // rms 0.04374 (was 0.03643), peak 0.2811 (was 0.2341), hits 0 (was 0)
    ("Sympathetic Chain", 2.48, 1.000), // rms 0.10218 (was 0.10218), peak 0.4102 (was 0.4102), hits 0 (was 0)
    ("Thin Sheet Roll", 0.00, 1.564), // rms 0.04374 (was 0.02796), peak 0.0952 (was 0.0608), hits 0 (was 0)
    ("Workshop Plate Scrape", 0.00, 3.058), // rms 0.04374 (was 0.01430), peak 0.1321 (was 0.0432), hits 0 (was 0)
];

fn apply_preset_levels(preset: &mut Preset) {
    let (_, body, output) = PRESET_LEVELS
        .iter()
        .find(|(name, _, _)| *name == preset.name)
        .unwrap_or_else(|| panic!("no level entry for preset {}", preset.name));
    preset.body = *body;
    preset.output = *output;
}

fn main() -> std::io::Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("presets/factory");
    std::fs::create_dir_all(&dir)?;

    let mut written = 0usize;
    for mut preset in presets() {
        apply_preset_levels(&mut preset);
        let filename = format!("{}.corrosion-preset", slug(&preset.name));
        let path = dir.join(filename);
        preset.save(&path)?;
        written += 1;
    }
    println!("Seeded {written} factory presets in {}", dir.display());
    assert_eq!(written, 50, "expected to seed exactly 50 presets");
    Ok(())
}

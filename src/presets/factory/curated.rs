//! The curated presets, designed by hand (design decision 5).
//!
//! Each preset starts from the plugin's defaults and sets what gives it its
//! identity; the comment above it says what it is for. Output is set later
//! by levelling. Tuning notes, measured on held notes (`zz` probes in the
//! bank tests, recorded in the change's design):
//! - A note's fundamental is roughly the key's frequency divided by Size.
//!   At Size 1 a Taut Cable plays the key (+3–10 cents), a Pipe 42 cents
//!   sharp (Size 1.025 corrects it), a Plate an octave up (Size 2 plays the
//!   key), a Coil Spring 148 cents sharp (Size 1.09), an I-Beam a tritone
//!   down (Size 0.707), Sheet Metal 425 cents up (Size 3.49 plays the key,
//!   1.371 the octave up; its pitch goes as Size^−0.75, not 1/Size), a Tank
//!   19 semitones down (Size 0.333).
//! - Every Space mode adds faint low partials of its own (98–144 Hz); they
//!   stay 40 dB under a bowed cable but not under a quiet bowed plate, so
//!   the plate chord patches are dry.
//! - A Pipe adds a fixed mode between 800 and 1030 Hz (with its diameter)
//!   whatever the note, and a Coil Spring is inharmonic: neither suits the
//!   chord folder.
//! - Pipe, I-Beam and Taut Cable ring for seconds; a strike that must decay
//!   on its own uses a Plate or a Chain, or heavy Rust and Sludge.
//! - BODY adds its own resonances (near 190 and 390 Hz); the chord patches
//!   keep it at 0 so their partials stay harmonic.
//! - Bowed notes sustain while held in Drone mode; strikes need Damping,
//!   Rust or a short envelope to decay on their own.

use crate::params::Object;
use crate::presets::Preset;

const BOW: i32 = 1;
const HAND_STRIKE: i32 = 2;
const FELT_MALLET: i32 = 3;
const HARD_MALLET: i32 = 4;
const DRUMSTICK: i32 = 5;
const WIRE_BRUSH: i32 = 6;
const METAL_PIPE: i32 = 7;
const METAL_CHAIN: i32 = 8;
const STIFF_POINT: i32 = 9;
const HEAVY_GRINDING: i32 = 10;
const CORRUGATED_DRAG: i32 = 11;
const TENSION_RISE: i32 = 12;
const PNEUMATIC_JET: i32 = 13;
const ELECTROMAGNETIC_HUM: i32 = 14;
const TENSION_SNAP: i32 = 15;
const PARTICLE_RAIN: i32 = 16;

const TONAL: i32 = 0;
const DRONE: i32 = 2;

const SPACE_FACTORY: i32 = 1;
const SPACE_SPRING: i32 = 2;
const SPACE_ECHO: i32 = 3;

/// `sync_rate` values for tempo-synced echoes (falls back to Delay Time
/// when the host reports no tempo).
const SYNC_EIGHTH: f32 = 0.29;
const SYNC_QUARTER: f32 = 0.45;

/// Sizes that put each object's fundamental on the key.
const PIPE_ON_KEY: f32 = 1.025;
const PLATE_ON_KEY: f32 = 2.0;
const SPRING_ON_KEY: f32 = 1.09;
const BEAM_ON_KEY: f32 = 0.707;
const SHEET_ON_KEY: f32 = 3.49;
const SHEET_OCTAVE_UP: f32 = 1.371;
const SHEET_OCTAVE_DOWN: f32 = 8.8;
const TANK_ON_KEY: f32 = 0.333;

/// A preset named `name`: `exciter` on `object` at `size`, in `play_mode`,
/// then `edit`.
fn make(
    name: &str,
    object: Object,
    exciter: i32,
    size: f32,
    play_mode: i32,
    edit: impl FnOnce(&mut Preset),
) -> Preset {
    let mut p = super::base(name);
    p.object = object;
    p.exciter = exciter;
    p.size = size;
    p.play_mode = play_mode;
    edit(&mut p);
    p
}

fn bass() -> Vec<Preset> {
    vec![
        // Upright-bass pluck: a heavy, soft felt head on a cable, dark filter.
        make(
            "Felt Cable Upright",
            Object::TautCable,
            FELT_MALLET,
            1.0,
            TONAL,
            |p| {
                p.body = 1.5;
                p.extra.mallet_mass = 2.4;
                p.extra.felt_softness = 1.1;
                p.extra.res_damping = 0.55;
                p.extra.filter_cutoff = 2_500.0;
            },
        ),
        // Palm thump on a pipe: short, round, a little drive.
        make(
            "Palm Pipe Thump",
            Object::Pipe,
            HAND_STRIKE,
            PIPE_ON_KEY,
            TONAL,
            |p| {
                p.extra.hand_mass = 2.4;
                p.extra.flesh_damping = 1.2;
                p.extra.filter_cutoff = 1_800.0;
                p.extra.drive_amount = 0.3;
            },
        ),
        // Slow bowed sub: a gentle bow on a cable, low-passed.
        make("Bowed Cable Sub", Object::TautCable, BOW, 1.0, TONAL, |p| {
            p.body = 1.0;
            p.extra.bow_pressure = 0.9;
            p.extra.bow_speed = 0.7;
            p.extra.env_attack = 0.03;
            p.extra.filter_cutoff = 1_200.0;
        }),
        // Springy felt bass with a little chirp.
        make(
            "Coil Spring Bass",
            Object::CoilSpring,
            FELT_MALLET,
            SPRING_ON_KEY,
            TONAL,
            |p| {
                p.extra.mallet_mass = 2.0;
                p.extra.spring_dispersion = 0.2;
                p.extra.filter_cutoff = 2_000.0;
            },
        ),
        // A snapped cable: hard attack, driven, short.
        make(
            "Snapped Cable Bass",
            Object::TautCable,
            TENSION_SNAP,
            1.0,
            TONAL,
            |p| {
                p.extra.snap_force = 0.8;
                p.extra.filter_cutoff = 2_200.0;
                p.extra.drive_amount = 0.5;
                p.extra.res_damping = 0.6;
            },
        ),
        // Boom of a struck tank, tuned to the key.
        make(
            "Tank Boom Bass",
            Object::Tank,
            FELT_MALLET,
            TANK_ON_KEY,
            TONAL,
            |p| {
                p.extra.mallet_mass = 2.5;
                p.extra.tank_volume = 0.6;
                p.extra.filter_cutoff = 900.0;
            },
        ),
        // Soft mallet on a big plate: a round sub with a metallic edge.
        make(
            "Plate Sub Mallet",
            Object::Plate,
            FELT_MALLET,
            PLATE_ON_KEY,
            TONAL,
            |p| {
                p.extra.mallet_mass = 2.2;
                p.extra.res_damping = 0.6;
                p.extra.filter_cutoff = 1_500.0;
            },
        ),
        // Driven hard-mallet beam: a growling, industrial bass.
        make(
            "Driven Beam Bass",
            Object::IBeam,
            HARD_MALLET,
            BEAM_ON_KEY,
            TONAL,
            |p| {
                p.extra.mallet_mass = 2.5;
                p.extra.drive_amount = 0.8;
                p.extra.filter_cutoff = 1_800.0;
            },
        ),
        // Felt on a pipe an octave down: a hollow, round sub.
        make(
            "Hollow Pipe Sub",
            Object::Pipe,
            FELT_MALLET,
            2.0 * PIPE_ON_KEY,
            TONAL,
            |p| {
                p.extra.mallet_mass = 2.0;
                p.extra.filter_cutoff = 1_500.0;
            },
        ),
        // Slap bass: a stiff hand on a cable.
        make(
            "Hand Slap Cable",
            Object::TautCable,
            HAND_STRIKE,
            1.0,
            TONAL,
            |p| {
                p.extra.flesh_stiffness = 0.7;
                p.extra.hand_mass = 2.0;
                p.extra.filter_cutoff = 3_000.0;
                p.extra.res_damping = 0.6;
            },
        ),
        // Bowed sheet an octave down: a reedy low drone-bass.
        make(
            "Low Sheet Bow",
            Object::SheetMetal,
            BOW,
            SHEET_OCTAVE_DOWN,
            TONAL,
            |p| {
                p.body = 1.0;
                p.extra.bow_pressure = 0.9;
                p.extra.filter_cutoff = 1_500.0;
            },
        ),
        // A heavy felt on a beam with a touch of rust: a dull, woody bass.
        make(
            "Heavy Felt Beam",
            Object::IBeam,
            FELT_MALLET,
            BEAM_ON_KEY,
            TONAL,
            |p| {
                p.rust = 0.15;
                p.extra.mallet_mass = 2.6;
                p.extra.filter_cutoff = 2_000.0;
            },
        ),
    ]
}

fn lead() -> Vec<Preset> {
    vec![
        // Singing bowed cable with a tempo-synced eighth echo.
        make(
            "Bowed Cable Lead",
            Object::TautCable,
            BOW,
            1.0,
            TONAL,
            |p| {
                p.extra.bow_speed = 1.4;
                p.extra.drive_amount = 0.6;
                p.extra.space_mode = SPACE_ECHO;
                p.extra.space_amount = 0.25;
                p.extra.sync_rate = SYNC_EIGHTH;
            },
        ),
        // Bright hard-mallet cable: a plucked, biting lead.
        make(
            "Hard Cable Pluck",
            Object::TautCable,
            HARD_MALLET,
            1.0,
            TONAL,
            |p| {
                p.extra.material_stiffness = 4.0;
                p.extra.drive_amount = 0.4;
                p.extra.space_mode = SPACE_ECHO;
                p.extra.space_amount = 0.2;
                p.extra.sync_rate = SYNC_QUARTER;
            },
        ),
        // Bowed pipe: a reedy, clarinet-like lead.
        make(
            "Pipe Bow Lead",
            Object::Pipe,
            BOW,
            PIPE_ON_KEY,
            TONAL,
            |p| {
                p.extra.bow_speed = 1.2;
                p.extra.drive_amount = 0.3;
            },
        ),
        // Bowed sheet an octave up: a glassy, hollow lead.
        make(
            "Glass Sheet Lead",
            Object::SheetMetal,
            BOW,
            SHEET_OCTAVE_UP,
            TONAL,
            |p| {
                p.extra.bow_speed = 1.3;
                p.extra.space_mode = SPACE_ECHO;
                p.extra.space_amount = 0.2;
                p.extra.sync_rate = SYNC_EIGHTH;
            },
        ),
        // Hard mallet on a plate, driven: a bell-lead.
        make(
            "Plate Strike Lead",
            Object::Plate,
            HARD_MALLET,
            PLATE_ON_KEY,
            TONAL,
            |p| {
                p.extra.drive_amount = 0.7;
                p.extra.material_stiffness = 3.5;
            },
        ),
        // Drumstick on a pipe through heavy drive: a gritty synth-like lead.
        make(
            "Driven Pipe Lead",
            Object::Pipe,
            DRUMSTICK,
            PIPE_ON_KEY,
            TONAL,
            |p| {
                p.extra.drive_amount = 1.0;
                p.extra.filter_cutoff = 8_000.0;
            },
        ),
        // Plucked cable with a quarter echo: a clean, delay-pluck lead.
        make(
            "Echo Cable Pluck",
            Object::TautCable,
            DRUMSTICK,
            1.0,
            TONAL,
            |p| {
                p.extra.space_mode = SPACE_ECHO;
                p.extra.space_amount = 0.35;
                p.extra.sync_rate = SYNC_QUARTER;
            },
        ),
        // Felt on a lightly damped plate: a soft, bell-like top line.
        make(
            "Felt Plate Bell",
            Object::Plate,
            FELT_MALLET,
            PLATE_ON_KEY,
            TONAL,
            |p| {
                p.extra.res_damping = 0.2;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.2;
            },
        ),
        // Air through a pipe: a breathy flute.
        make(
            "Jet Pipe Flute",
            Object::Pipe,
            PNEUMATIC_JET,
            PIPE_ON_KEY,
            TONAL,
            |p| {
                p.extra.air_pressure = 1.2;
                p.extra.turbulence_chaos = 0.3;
            },
        ),
        // Bowed spring: a wobbly, singing lead.
        make(
            "Bowed Spring Lead",
            Object::CoilSpring,
            BOW,
            SPRING_ON_KEY,
            TONAL,
            |p| {
                p.extra.bow_speed = 1.2;
                p.extra.spring_slosh = 0.2;
            },
        ),
        // Metal pipe striking a cable: a clangy, metallic pluck.
        make(
            "Metal Pipe Pluck",
            Object::TautCable,
            METAL_PIPE,
            1.0,
            TONAL,
            |p| {
                p.extra.drive_amount = 0.4;
            },
        ),
        // Bowed beam on the key: a rough, cello-like lead.
        make(
            "Bowed Beam Lead",
            Object::IBeam,
            BOW,
            BEAM_ON_KEY,
            TONAL,
            |p| {
                p.extra.bow_speed = 1.3;
                p.extra.drive_amount = 0.5;
            },
        ),
    ]
}

fn chords() -> Vec<Preset> {
    vec![
        // Warm bowed-cable pad in the factory room.
        make("Bowed Cable Pad", Object::TautCable, BOW, 1.0, DRONE, |p| {
            p.body = 0.0;
            p.extra.space_mode = SPACE_FACTORY;
            p.extra.space_amount = 0.35;
        }),
        // Light bow, more rosin: an airy choir. (A stronger third partial
        // would sit 20 cents sharp, from the cable's stiffness; this bow
        // keeps it under the others.)
        make(
            "Glass Cable Choir",
            Object::TautCable,
            BOW,
            1.0,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.bow_pressure = 0.9;
                p.extra.bow_speed = 1.0;
                p.extra.rosin_grip = 0.6;
                p.extra.filter_cutoff = 6_000.0;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.4;
            },
        ),
        // Bowed plate: a shimmering metallic pad, dry, widened by Spread.
        make(
            "Plate Bow Pad",
            Object::Plate,
            BOW,
            PLATE_ON_KEY,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.spread_width = 0.8;
            },
        ),
        // Bowed sheet on the key: a hollow, organ-like chord.
        make(
            "Sheet Bow Organ",
            Object::SheetMetal,
            BOW,
            SHEET_ON_KEY,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.3;
            },
        ),
        // Slow-attack, slow-bowed cable strings.
        make(
            "Warm Cable Strings",
            Object::TautCable,
            BOW,
            1.0,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.bow_speed = 0.8;
                p.extra.env_attack = 0.3;
                p.extra.env_release = 1.5;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.3;
            },
        ),
        // A darker plate pad, low-passed and dry.
        make(
            "Soft Plate Strings",
            Object::Plate,
            BOW,
            PLATE_ON_KEY,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.bow_speed = 0.8;
                p.extra.filter_cutoff = 3_000.0;
                p.extra.spread_width = 0.7;
            },
        ),
        // Bowed sheet an octave up: a bright shimmer.
        make(
            "Sheet Bow Shimmer",
            Object::SheetMetal,
            BOW,
            SHEET_OCTAVE_UP,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.45;
            },
        ),
        // Cable pad with a slow quarter echo.
        make("Cable Echo Pad", Object::TautCable, BOW, 1.0, DRONE, |p| {
            p.body = 0.0;
            p.extra.space_mode = SPACE_ECHO;
            p.extra.space_amount = 0.3;
            p.extra.sync_rate = SYNC_QUARTER;
        }),
        // More rosin, less pressure: a fuller, ensemble-like chord.
        make(
            "Rosin Cable Ensemble",
            Object::TautCable,
            BOW,
            1.0,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.bow_pressure = 0.9;
                p.extra.rosin_grip = 0.6;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.35;
            },
        ),
        // A bowed beam on the key: a dark, reedy chorale, dry and wide.
        make(
            "Beam Bow Chorale",
            Object::IBeam,
            BOW,
            BEAM_ON_KEY,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.spread_width = 0.8;
            },
        ),
        // Cable pad through the spring reverb.
        make(
            "Cable Spring Pad",
            Object::TautCable,
            BOW,
            1.0,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.space_mode = SPACE_SPRING;
                p.extra.space_amount = 0.3;
            },
        ),
        // The sheet organ through the spring reverb.
        make(
            "Sheet Spring Organ",
            Object::SheetMetal,
            BOW,
            SHEET_ON_KEY,
            DRONE,
            |p| {
                p.body = 0.0;
                p.extra.bow_speed = 0.9;
                p.extra.space_mode = SPACE_SPRING;
                p.extra.space_amount = 0.25;
            },
        ),
    ]
}

fn drones() -> Vec<Preset> {
    vec![
        // Electromagnetic hum through a smokestack.
        make(
            "Smokestack Hum",
            Object::Pipe,
            ELECTROMAGNETIC_HUM,
            2.0,
            DRONE,
            |p| {
                p.extra.mains_frequency = 50.0;
                p.extra.voltage_sag = 0.4;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.4;
            },
        ),
        // A grinding wheel against a beam.
        make(
            "Grinding Beam",
            Object::IBeam,
            HEAVY_GRINDING,
            1.0,
            DRONE,
            |p| {
                p.extra.grind_pressure = 0.8;
                p.extra.surface_grit = 0.4;
                p.extra.filter_cutoff = 6_000.0;
            },
        ),
        // Air breathing into a tank.
        make(
            "Tank Breath",
            Object::Tank,
            PNEUMATIC_JET,
            1.0,
            DRONE,
            |p| {
                p.extra.air_pressure = 1.0;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.4;
            },
        ),
        // A bowed beam: a low, groaning drone.
        make("Bowed Beam Drone", Object::IBeam, BOW, 1.0, DRONE, |p| {
            p.extra.bow_pressure = 1.0;
            p.extra.space_mode = SPACE_FACTORY;
            p.extra.space_amount = 0.35;
        }),
        // A bowed spring: a wobbling, metallic drone.
        make(
            "Spring Bow Drone",
            Object::CoilSpring,
            BOW,
            1.5,
            DRONE,
            |p| {
                p.extra.spring_slosh = 0.4;
                p.extra.space_mode = SPACE_SPRING;
                p.extra.space_amount = 0.35;
            },
        ),
        // A humming cog: a dark machine-room drone.
        make(
            "Cog Hum Drone",
            Object::IndustrialCog,
            ELECTROMAGNETIC_HUM,
            1.0,
            DRONE,
            |p| {
                p.extra.mains_frequency = 60.0;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.3;
            },
        ),
        // A chain dragged over ribs, with a slow MSEG loop.
        make(
            "Chain Drag Drone",
            Object::Chain,
            CORRUGATED_DRAG,
            1.0,
            DRONE,
            |p| {
                p.extra.drag_speed = 0.6;
                p.extra.loop_mode = 1;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.3;
            },
        ),
        // Tension slowly building in a plate.
        make(
            "Plate Tension Swell",
            Object::Plate,
            TENSION_RISE,
            2.0,
            DRONE,
            |p| {
                p.extra.pull_speed = 0.5;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.45;
            },
        ),
        // A bowed sheet: a bright, steady drone.
        make(
            "Sheet Bow Drone",
            Object::SheetMetal,
            BOW,
            2.0,
            DRONE,
            |p| {
                p.extra.bow_pressure = 0.9;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.35;
            },
        ),
        // A point scraping a cable: a thin, squealing drone.
        make(
            "Cable Scrape Drone",
            Object::TautCable,
            STIFF_POINT,
            1.0,
            DRONE,
            |p| {
                p.extra.scrape_speed = 0.6;
                p.extra.filter_cutoff = 5_000.0;
                p.extra.space_mode = SPACE_ECHO;
                p.extra.space_amount = 0.3;
            },
        ),
        // Grit raining into a tank.
        make(
            "Particle Rain Tank",
            Object::Tank,
            PARTICLE_RAIN,
            1.0,
            DRONE,
            |p| {
                p.extra.flow_rate = 1.2;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.4;
            },
        ),
        // Air through a long pipe: a hollow wind drone.
        make(
            "Pipe Wind Drone",
            Object::Pipe,
            PNEUMATIC_JET,
            2.0,
            DRONE,
            |p| {
                p.extra.air_pressure = 1.1;
                p.extra.turbulence_chaos = 0.5;
                p.extra.space_mode = SPACE_FACTORY;
                p.extra.space_amount = 0.4;
            },
        ),
    ]
}

fn percussion() -> Vec<Preset> {
    /// Short-envelope, damped strike settings shared by the percussion.
    fn short(p: &mut Preset, damping: f32) {
        p.extra.res_damping = damping;
        p.extra.env_sustain = 0.0;
        p.extra.env_decay = 0.3;
    }
    /// For objects that ring for seconds (beam, cog): heavy Rust and
    /// Sludge, so the strike dies within a second.
    fn deadened(p: &mut Preset) {
        short(p, 0.9);
        p.rust = 2.0;
        p.extra.sludge = 1.0;
    }
    vec![
        // A drumstick crack on a small plate.
        make(
            "Plate Snare Crack",
            Object::Plate,
            DRUMSTICK,
            1.0,
            TONAL,
            |p| {
                short(p, 0.8);
                p.rust = 0.3;
            },
        ),
        // A palm on a heavy, sludge-coated beam: a dull kick-like thud.
        make(
            "Beam Kick Thud",
            Object::IBeam,
            HAND_STRIKE,
            1.4,
            TONAL,
            |p| {
                deadened(p);
                p.extra.hand_mass = 2.6;
                p.extra.filter_cutoff = 2_000.0;
            },
        ),
        // A dropped chain on a chain.
        make(
            "Chain Rattle Hit",
            Object::Chain,
            METAL_CHAIN,
            1.0,
            TONAL,
            |p| {
                short(p, 0.6);
            },
        ),
        // A hard clang on a rusted sheet.
        make(
            "Rusted Sheet Clang",
            Object::SheetMetal,
            HARD_MALLET,
            1.0,
            TONAL,
            |p| {
                short(p, 0.7);
                p.rust = 0.4;
            },
        ),
        // A drumstick tick on a small, deadened cog.
        make(
            "Cog Tick",
            Object::IndustrialCog,
            DRUMSTICK,
            0.6,
            TONAL,
            |p| {
                deadened(p);
            },
        ),
        // A wire brush sweep on a plate.
        make(
            "Brush Plate Swish",
            Object::Plate,
            WIRE_BRUSH,
            1.0,
            TONAL,
            |p| {
                short(p, 0.7);
                p.extra.spread_duration = 120.0;
            },
        ),
        // A metal pipe striking a beam: a clanging anvil.
        make("Beam Anvil", Object::IBeam, METAL_PIPE, 1.0, TONAL, |p| {
            short(p, 0.7);
            p.rust = 0.2;
        }),
        // A snapping hook on a chain.
        make("Chain Snap", Object::Chain, TENSION_SNAP, 1.0, TONAL, |p| {
            short(p, 0.6);
        }),
        // A felt mallet on a large, rusted cog: a short, dark gong.
        make(
            "Cog Gong Hit",
            Object::IndustrialCog,
            FELT_MALLET,
            1.0,
            TONAL,
            |p| {
                short(p, 0.9);
                p.rust = 1.5;
                p.extra.sludge = 0.9;
            },
        ),
        // A hard mallet on a small sheet: a hi-hat-like tick.
        make(
            "Sheet Tick Hat",
            Object::SheetMetal,
            HARD_MALLET,
            0.5,
            TONAL,
            |p| {
                short(p, 0.9);
                p.extra.material_stiffness = 4.5;
            },
        ),
        // A hard mallet clank on a chain.
        make(
            "Chain Mallet Clank",
            Object::Chain,
            HARD_MALLET,
            1.0,
            TONAL,
            |p| {
                short(p, 0.6);
            },
        ),
        // A drumstick knock on a rusty, deadened beam.
        make(
            "Rusted Beam Knock",
            Object::IBeam,
            DRUMSTICK,
            1.0,
            TONAL,
            |p| {
                deadened(p);
            },
        ),
    ]
}

/// Builds one folder's presets.
type FolderBuilder = fn() -> Vec<Preset>;

/// Every curated preset with its folder, in folder order.
pub(super) fn presets() -> Vec<(&'static str, Preset)> {
    let folders: [(&str, FolderBuilder); 5] = [
        ("bass", bass),
        ("lead", lead),
        ("chords", chords),
        ("drones", drones),
        ("percussion", percussion),
    ];
    folders
        .into_iter()
        .flat_map(|(folder, build)| build().into_iter().map(move |p| (folder, p)))
        .collect()
}

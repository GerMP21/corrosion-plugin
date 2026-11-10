//! The factory drum kits (`per-key-kits`): ten Kit-mode presets on the
//! General MIDI drum layout, designed by hand. Each pad's level is set by
//! the seeder (stored in `levels.rs`) so it meets the quality floor alone.
//!
//! Every kit has the same 12 pads: 36 kick, 37 side stick, 38 snare,
//! 39 clap, 41 / 45 / 48 low, mid and high toms, 42 / 44 / 46 closed,
//! pedal and open hats, 49 crash and 51 ride. Each pad is tuned for its
//! role: kicks an octave down, snares and claps an octave up, hats and
//! cymbals three to four octaves up (the engine's pitch follows the key
//! ÷ Size, and a hat at its own key would ring at a few hundred Hz). The
//! toms stay at their keys, so they rise with them.
//!
//! The hats sit far above the engine's normalized strike range and play
//! quietly, so their stored levels are high (up to about +50 dB): the
//! seeder levels each pad alone to the same floor as every preset.

use crate::kit::{KitPad, KitState};
use crate::params::{Object, PlayMode};
use crate::presets::Preset;

const HAND_STRIKE: i32 = 2;
const FELT_MALLET: i32 = 3;
const HARD_MALLET: i32 = 4;
const DRUMSTICK: i32 = 5;
const WIRE_BRUSH: i32 = 6;
const METAL_PIPE: i32 = 7;
const METAL_CHAIN: i32 = 8;
const TENSION_SNAP: i32 = 15;
const PARTICLE_RAIN: i32 = 16;

pub const KICK: u8 = 36;
pub const SIDE_STICK: u8 = 37;
pub const SNARE: u8 = 38;
pub const CLAP: u8 = 39;
pub const LOW_TOM: u8 = 41;
pub const CLOSED_HAT: u8 = 42;
pub const PEDAL_HAT: u8 = 44;
pub const MID_TOM: u8 = 45;
pub const OPEN_HAT: u8 = 46;
pub const HIGH_TOM: u8 = 48;
pub const CRASH: u8 = 49;
pub const RIDE: u8 = 51;

/// A pad's sound: `exciter` on `object` at `size`, a short one-shot
/// envelope, resonator damping `damping` and brightness `brightness`,
/// then `edit`.
fn sound(
    object: Object,
    exciter: i32,
    size: f32,
    damping: f32,
    brightness: f32,
    edit: impl FnOnce(&mut Preset),
) -> Preset {
    let mut p = super::base("pad");
    p.object = object;
    p.exciter = exciter;
    p.size = size;
    p.extra.res_damping = damping;
    p.extra.res_brightness = brightness;
    p.extra.env_sustain = 0.0;
    p.extra.env_decay = 0.3;
    edit(&mut p);
    p
}

/// Heavy Rust and Sludge: a strike that would ring for seconds dies
/// within one, darker.
fn deaden(p: &mut Preset) {
    p.rust = 2.0;
    p.extra.sludge = 1.0;
}

fn no_edit(_: &mut Preset) {}

/// A pad: its key, its tune in semitones and its sound.
type PadDef = (u8, i32, Preset);

/// Tunes per role, in semitones from the key.
const KICK_TUNE: i32 = -12;
const SNARE_TUNE: i32 = 12;
const SIDE_STICK_TUNE: i32 = 24;
const HAT_TUNE: i32 = 48;
const CYMBAL_TUNE: i32 = 36;

fn kick(object: Object, size: f32, edit: impl FnOnce(&mut Preset)) -> PadDef {
    (
        KICK,
        KICK_TUNE,
        sound(object, FELT_MALLET, size, 0.9, 0.3, edit),
    )
}

fn snare(object: Object, exciter: i32, edit: impl FnOnce(&mut Preset)) -> PadDef {
    (
        SNARE,
        SNARE_TUNE,
        sound(object, exciter, 1.0, 0.9, 0.7, edit),
    )
}

fn side_stick(object: Object, edit: impl FnOnce(&mut Preset)) -> PadDef {
    (
        SIDE_STICK,
        SIDE_STICK_TUNE,
        sound(object, DRUMSTICK, 0.8, 0.95, 0.8, edit),
    )
}

fn clap(object: Object, exciter: i32, edit: impl FnOnce(&mut Preset)) -> PadDef {
    (
        CLAP,
        SNARE_TUNE,
        sound(object, exciter, 1.0, 0.92, 0.8, edit),
    )
}

/// The three toms: one sound on 41, 45 and 48, rising with the keys.
fn toms(object: Object, size: f32, edit: impl Fn(&mut Preset)) -> [PadDef; 3] {
    [LOW_TOM, MID_TOM, HIGH_TOM]
        .map(|key| (key, 0, sound(object, FELT_MALLET, size, 0.9, 0.5, &edit)))
}

/// Closed, pedal and open hats on one object: the pedal hat a little
/// lower and duller, the open hat with less damping. A hard mallet's
/// strike this high dies in 0.2 s whatever the damping, so those kits
/// open the hat with a drumstick instead.
fn hats(object: Object, exciter: i32, edit: impl Fn(&mut Preset)) -> [PadDef; 3] {
    let open = if exciter == HARD_MALLET {
        DRUMSTICK
    } else {
        exciter
    };
    [
        (
            CLOSED_HAT,
            HAT_TUNE,
            sound(object, exciter, 0.5, 0.95, 1.0, &edit),
        ),
        (
            PEDAL_HAT,
            HAT_TUNE - 5,
            sound(object, exciter, 0.5, 0.95, 0.8, &edit),
        ),
        (
            OPEN_HAT,
            HAT_TUNE,
            sound(object, open, 0.5, 0.4, 1.0, &edit),
        ),
    ]
}

fn crash(object: Object, exciter: i32, edit: impl FnOnce(&mut Preset)) -> PadDef {
    (
        CRASH,
        CYMBAL_TUNE,
        sound(object, exciter, 0.8, 0.55, 0.9, edit),
    )
}

fn ride(object: Object, exciter: i32, edit: impl FnOnce(&mut Preset)) -> PadDef {
    (
        RIDE,
        CYMBAL_TUNE,
        sound(object, exciter, 0.6, 0.6, 0.9, edit),
    )
}

/// A kit preset named `name` in Kit mode: `post` sets its shared post
/// chain (BODY 0.4 unless `post` changes it: the default adds low
/// partials to every pad), and each pad is stored at level 0 dB (the
/// seeder levels them).
fn kit(name: &str, post: impl FnOnce(&mut Preset), pads: Vec<PadDef>) -> Preset {
    let mut preset = super::base(name);
    preset.play_mode = PlayMode::Kit.to_int();
    preset.body = 0.4;
    post(&mut preset);
    let mut state = KitState::default();
    for (key, tune, sound) in pads {
        state.set(KitPad::from_params(key, &sound.into_params(), tune, 0.0));
    }
    preset.kit = Some(state);
    preset
}

/// Builds a pad list from single pads and role groups.
macro_rules! pads {
    ($($e:expr),* $(,)?) => {{
        let mut v: Vec<PadDef> = Vec::new();
        $(v.extend(IntoPads::into_pads($e));)*
        v
    }};
}

trait IntoPads {
    fn into_pads(self) -> Vec<PadDef>;
}

impl IntoPads for PadDef {
    fn into_pads(self) -> Vec<PadDef> {
        vec![self]
    }
}

impl IntoPads for [PadDef; 3] {
    fn into_pads(self) -> Vec<PadDef> {
        self.into()
    }
}

use Object::*;

/// The ten kits, in browser order.
pub(super) fn kits() -> Vec<Preset> {
    vec![
        // A heavy shop floor: plate kick and snare, tank toms, chain hats.
        kit(
            "Foundry Kit",
            no_edit,
            pads![
                kick(Plate, 2.0, no_edit),
                side_stick(Plate, no_edit),
                snare(Plate, DRUMSTICK, no_edit),
                clap(SheetMetal, HAND_STRIKE, |p| p.rust = 1.0),
                toms(Tank, 1.2, no_edit),
                hats(Chain, METAL_CHAIN, no_edit),
                crash(SheetMetal, HARD_MALLET, no_edit),
                ride(Plate, DRUMSTICK, no_edit),
            ],
        ),
        // Struck scrap: a dead cog kick, a pipe on a rusty plate, brushed chain.
        kit(
            "Scrapyard Kit",
            |p| p.drive = 0.4,
            pads![
                kick(IndustrialCog, 2.0, deaden),
                side_stick(IBeam, |p| p.rust = 2.0),
                snare(Plate, METAL_PIPE, deaden),
                clap(Plate, HAND_STRIKE, |p| p.rust = 1.5),
                toms(Pipe, 1.2, no_edit),
                hats(Chain, WIRE_BRUSH, |p| p.rust = 1.0),
                crash(Plate, METAL_PIPE, no_edit),
                ride(Chain, DRUMSTICK, no_edit),
            ],
        ),
        // Tuned pipes: every drum a length of tube.
        kit(
            "Pipe Kit",
            |p| p.extra.space_amount = 0.15,
            pads![
                kick(Pipe, 2.0, |p| {
                    deaden(p);
                    p.extra.res_damping = 0.95;
                }),
                side_stick(Pipe, |p| p.rust = 2.0),
                snare(Pipe, DRUMSTICK, deaden),
                clap(Pipe, HAND_STRIKE, deaden),
                toms(Pipe, 1.2, deaden),
                hats(Pipe, HARD_MALLET, no_edit),
                crash(Pipe, HARD_MALLET, no_edit),
                ride(Pipe, DRUMSTICK, |p| p.rust = 1.5),
            ],
        ),
        // Corroded everything: high Rust and Damage, gritty and short.
        kit(
            "Rust Belt Kit",
            |p| {
                p.drive = 0.6;
                p.extra.filter_cutoff = 9_000.0;
            },
            pads![
                kick(Plate, 2.0, deaden),
                side_stick(Plate, |p| p.rust = 1.5),
                snare(Chain, DRUMSTICK, |p| {
                    p.rust = 1.5;
                    p.damage = 2.0;
                }),
                clap(SheetMetal, WIRE_BRUSH, |p| p.rust = 1.5),
                toms(Plate, 1.5, |p| p.rust = 1.2),
                hats(Chain, DRUMSTICK, |p| p.rust = 2.0),
                crash(SheetMetal, HARD_MALLET, |p| {
                    p.rust = 1.0;
                    p.damage = 2.5;
                }),
                ride(Plate, DRUMSTICK, |p| p.rust = 1.0),
            ],
        ),
        // A boiler room: big sheet kick and tank toms in the Factory space.
        kit(
            "Boiler Room Kit",
            |p| {
                p.extra.space_mode = 1;
                p.extra.space_amount = 0.2;
            },
            pads![
                kick(SheetMetal, 3.0, no_edit),
                side_stick(Tank, |p| p.extra.res_damping = 0.98),
                snare(Plate, DRUMSTICK, |p| p.rust = 1.0),
                clap(Plate, HAND_STRIKE, no_edit),
                toms(Tank, 1.4, no_edit),
                hats(Plate, METAL_CHAIN, no_edit),
                crash(SheetMetal, HARD_MALLET, no_edit),
                ride(Plate, DRUMSTICK, no_edit),
            ],
        ),
        // A rail yard: beams and rails struck with pipes and hammers.
        kit(
            "Rail Yard Kit",
            |p| p.extra.space_amount = 0.1,
            pads![
                kick(IBeam, 2.0, |p| {
                    deaden(p);
                    p.extra.res_damping = 1.0;
                }),
                side_stick(IBeam, |p| p.rust = 2.0),
                snare(IBeam, METAL_PIPE, deaden),
                clap(IBeam, HAND_STRIKE, deaden),
                toms(IBeam, 1.2, deaden),
                hats(IBeam, HARD_MALLET, |p| p.size = 0.4),
                crash(IBeam, HARD_MALLET, no_edit),
                ride(IBeam, DRUMSTICK, |p| p.rust = 2.0),
            ],
        ),
        // Wires and springs: a spring kick and snare, chained cable hats.
        kit(
            "Wire Kit",
            |p| {
                p.extra.space_mode = 2;
                p.extra.space_amount = 0.15;
            },
            pads![
                kick(CoilSpring, 2.0, |p| {
                    deaden(p);
                    p.extra.res_damping = 0.95;
                }),
                side_stick(TautCable, |p| p.rust = 2.0),
                snare(CoilSpring, DRUMSTICK, deaden),
                clap(CoilSpring, WIRE_BRUSH, deaden),
                toms(TautCable, 1.2, deaden),
                hats(Chain, WIRE_BRUSH, |p| p.rust = 2.0),
                crash(TautCable, HARD_MALLET, no_edit),
                ride(CoilSpring, DRUMSTICK, no_edit),
            ],
        ),
        // Sheet metal throughout: thin, bright and splashy.
        kit(
            "Sheet Storm Kit",
            |p| p.width = 0.8,
            pads![
                kick(SheetMetal, 4.0, no_edit),
                side_stick(SheetMetal, no_edit),
                snare(SheetMetal, DRUMSTICK, |p| {
                    deaden(p);
                    p.extra.res_damping = 0.98;
                }),
                clap(SheetMetal, HAND_STRIKE, no_edit),
                toms(SheetMetal, 1.5, no_edit),
                hats(SheetMetal, HARD_MALLET, no_edit),
                crash(SheetMetal, HARD_MALLET, |p| p.extra.sheet_thinness = 0.9),
                ride(SheetMetal, DRUMSTICK, no_edit),
            ],
        ),
        // Clockwork: cogs and chains, tight ticks and knocks.
        kit(
            "Clockwork Kit",
            |p| p.extra.filter_cutoff = 12_000.0,
            pads![
                kick(IndustrialCog, 2.0, deaden),
                side_stick(IndustrialCog, |p| p.rust = 2.0),
                snare(Chain, DRUMSTICK, no_edit),
                clap(Chain, TENSION_SNAP, no_edit),
                toms(Chain, 1.2, no_edit),
                hats(Chain, DRUMSTICK, no_edit),
                crash(Chain, METAL_CHAIN, no_edit),
                ride(Chain, DRUMSTICK, no_edit),
            ],
        ),
        // Junk percussion: rattles, snaps and rain more than drums.
        kit(
            "Junk Percussion Kit",
            |p| {
                p.extra.space_mode = 3;
                p.extra.space_amount = 0.15;
            },
            pads![
                kick(Chain, 2.0, no_edit),
                side_stick(Chain, |p| p.exciter = TENSION_SNAP),
                snare(Plate, PARTICLE_RAIN, |p| {
                    p.extra.flow_rate = 2.5;
                    p.rust = 1.0;
                }),
                clap(Plate, TENSION_SNAP, no_edit),
                toms(Chain, 1.2, no_edit),
                hats(Plate, METAL_CHAIN, |p| p.rust = 1.0),
                crash(Chain, METAL_CHAIN, no_edit),
                ride(Plate, METAL_CHAIN, no_edit),
            ],
        ),
    ]
}

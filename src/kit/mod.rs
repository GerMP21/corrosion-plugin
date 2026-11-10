//! Per-key kits (`per-key-kits`).
//!
//! In Kit play mode every key can hold a **pad**: a stored sound (exciter,
//! object and every voice-level parameter) played at a fixed pitch, the key
//! plus the pad's tune. Keys without a pad are silent. The post chain stays
//! shared by the whole kit.
//!
//! Two forms of the same kit:
//! - [`KitState`] is the serialisable form (parameter values by ID), saved
//!   in presets and in the host's project state. It lives behind a lock and
//!   is only touched off the audio thread.
//! - [`KitTable`] is what the audio thread reads at note-on: 128 slots of
//!   ready-to-play [`Pad`]s, one seqlock per slot, written by the editor or
//!   by `Plugin::initialize` after a state restore and never locked or
//!   allocated on the audio side.

use std::cell::UnsafeCell;
use std::collections::{BTreeMap, HashMap};
use std::mem::MaybeUninit;
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Mutex;

use nih_plug::prelude::{ParamPtr, Params};
use serde::{Deserialize, Serialize};

use crate::params::{CorrosionParams, Object, PlayMode};
use crate::presets::Preset;
use crate::NoteSetup;

/// The General MIDI drum zone the editor plays and selects in Kit mode,
/// unless Full keyboard is on (MIDI 36–59; C2–B3 in the plugin's naming).
pub const KIT_ZONE: std::ops::RangeInclusive<u8> = 36..=59;

/// The largest pad tune, in semitones either way.
pub const MAX_TUNE: i32 = 48;

/// Parameters that are not part of a pad: the post chain, the master
/// section, the play and quality modes, the tempo sync and editor settings.
/// Every other parameter is voice-level and is stored in a pad. The macros
/// are stored too (their voice-side bias is part of the sound), but loading
/// a pad into the editor leaves them alone ([`PAD_MACRO_PARAMS`]).
pub const NON_PAD_PARAMS: &[&str] = &[
    "quality_mode",
    "play_mode",
    "limiter_mode",
    "drive",
    "output",
    "width",
    "body",
    "ui_scale",
    "sync_rate",
    "filter_cutoff",
    "filter_resonance",
    "component_tolerance",
    "drive_amount",
    "bias_starvation",
    "chaos_depth",
    "spread_width",
    "listener_proximity",
    "chassis_material",
    "chassis_volume",
    "space_mode",
    "space_amount",
    "factory_size",
    "machinery_clutter",
    "wall_impedance",
    "spring_tension",
    "wire_stiffness",
    "spring_tank_size",
    "delay_time",
    "machinery_movement",
    "high_frequency_damping",
    "analog_ceiling",
    "diode_softness",
    "diode_asymmetry",
];

/// The macros: stored in a pad for their voice-side bias, not loaded back
/// into the editor (they also drive the shared post chain).
pub const PAD_MACRO_PARAMS: &[&str] = &[
    "macro_mass",
    "macro_corrosion",
    "macro_violence",
    "macro_brightness",
];

/// Whether parameter `id` is stored in a pad.
pub fn is_pad_param(id: &str) -> bool {
    !NON_PAD_PARAMS.contains(&id)
}

/// One pad as saved: the plain values of the pad parameters, the tune
/// (semitones) and the level (dB).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KitPad {
    /// MIDI key, 0–127.
    pub key: u8,
    #[serde(default)]
    pub tune: i32,
    #[serde(default)]
    pub level_db: f32,
    /// Plain values by parameter ID ([`is_pad_param`]).
    pub values: BTreeMap<String, f32>,
}

/// A whole kit as saved: its pads, sorted by key, at most one per key.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KitState {
    #[serde(default)]
    pub pads: Vec<KitPad>,
}

impl KitState {
    /// The pad on `key`, if any.
    pub fn pad(&self, key: u8) -> Option<&KitPad> {
        self.pads.iter().find(|p| p.key == key)
    }

    /// Store `pad` on its key, replacing any pad there.
    pub fn set(&mut self, pad: KitPad) {
        self.pads.retain(|p| p.key != pad.key);
        let at = self.pads.partition_point(|p| p.key < pad.key);
        self.pads.insert(at, pad);
    }

    /// Remove the pad on `key`.
    pub fn clear(&mut self, key: u8) {
        self.pads.retain(|p| p.key != key);
    }

    pub fn is_empty(&self) -> bool {
        self.pads.is_empty()
    }
}

impl KitPad {
    /// A pad on `key` holding `params`' current sound.
    pub fn from_params(key: u8, params: &CorrosionParams, tune: i32, level_db: f32) -> Self {
        let values = params
            .param_map()
            .into_iter()
            .filter(|(id, _, _)| is_pad_param(id))
            // SAFETY: the pointers come from `params`, borrowed for this call.
            .map(|(id, ptr, _)| (id, unsafe { ptr.modulated_plain_value() }))
            .collect();
        Self {
            key,
            tune: tune.clamp(-MAX_TUNE, MAX_TUNE),
            level_db,
            values,
        }
    }

    /// Parameters at their defaults with this pad's values applied, through
    /// the preset loader (so values are clamped like a loaded file).
    pub fn params(&self) -> CorrosionParams {
        let defaults = CorrosionParams::default();
        let ints: HashMap<String, bool> = defaults
            .param_map()
            .into_iter()
            .map(|(id, ptr, _)| {
                let int = matches!(ptr, ParamPtr::IntParam(_) | ParamPtr::EnumParam(_));
                (id, int)
            })
            .collect();
        let mut json = serde_json::to_value(Preset::from_params("pad", &defaults))
            .expect("serialize default preset");
        for (id, &value) in &self.values {
            let Some(&int) = ints.get(id) else {
                continue; // a parameter this build doesn't have
            };
            let number = if id == "object" {
                serde_json::to_value(Object::from_int(value.round() as i32)).expect("object")
            } else if int {
                serde_json::Value::from(value.round() as i64)
            } else {
                serde_json::Value::from(value as f64)
            };
            if json.get(id.as_str()).is_some() {
                json[id.as_str()] = number;
            } else {
                json["extra"][id.as_str()] = number;
            }
        }
        Preset::from_json_str(&json.to_string())
            .expect("pad values parse as a preset")
            .into_params()
    }

    /// The ready-to-play pad.
    pub fn build(&self) -> Pad {
        let params = self.params();
        let object = Object::from_int(params.object.value());
        Pad {
            setup: crate::note_setup(&params, object, PlayMode::Kit),
            tune: self.tune.clamp(-MAX_TUNE, MAX_TUNE),
            gain: 10f32.powf(self.level_db / 20.0),
        }
    }
}

/// A pad as the audio thread plays it: the note setup built from its
/// values, its tune and its linear gain. Plain `Copy` data.
#[derive(Clone, Copy, Debug)]
pub struct Pad {
    pub(crate) setup: NoteSetup,
    pub tune: i32,
    pub gain: f32,
}

impl Pad {
    /// Start this pad on `key` in `voices`: its stored sound at the key plus
    /// its tune, with the live quality's mode-count scale (quality sets CPU
    /// cost, not the sound's design). Returns `false`, starting nothing,
    /// when the tune takes the note off the MIDI range. Real-time safe.
    pub fn play(
        &self,
        voices: &mut crate::voice::manager::VoiceManager,
        address: crate::voice::manager::NoteAddress,
        key: u8,
        velocity: f32,
        mode_count_scale: f32,
    ) -> bool {
        let Some(pitch_note) = self.pitch_note(key) else {
            return false;
        };
        let mut controls = self.setup.controls;
        controls.mode_count_scale = mode_count_scale;
        voices.note_on_pad(
            address,
            key,
            pitch_note,
            velocity,
            self.setup.profile,
            self.setup.size,
            self.setup.rust,
            self.setup.damage,
            self.setup.exciter,
            controls,
            self.gain,
        );
        true
    }

    /// The MIDI note `key` sounds at with this pad, or `None` when the tune
    /// takes it off the MIDI range.
    pub fn pitch_note(&self, key: u8) -> Option<u8> {
        u8::try_from(key as i32 + self.tune)
            .ok()
            .filter(|&n| n <= 127)
    }
}

struct Slot {
    /// Even while the slot is stable, odd while it is being written.
    seq: AtomicU32,
    pad: UnsafeCell<MaybeUninit<Option<Pad>>>,
}

/// How many times a reader retries a slot that is being written before it
/// gives up (the note then plays nothing).
const READ_TRIES: usize = 4;

/// The audio thread's pad table: one seqlock slot per MIDI key. Writers
/// (the editor, a state restore) are serialised by a mutex the audio thread
/// never takes; readers copy a slot and retry while a write is in flight.
pub struct KitTable {
    slots: Box<[Slot]>,
    writer: Mutex<()>,
}

// SAFETY: a slot's `pad` is written only under `writer`, between two
// sequence bumps; readers copy it with volatile reads and keep the copy
// only if the sequence was even and unchanged around the read.
unsafe impl Sync for KitTable {}

impl Default for KitTable {
    fn default() -> Self {
        Self::new()
    }
}

impl KitTable {
    /// An empty table (allocates its 128 slots once).
    pub fn new() -> Self {
        let slots = (0..128)
            .map(|_| Slot {
                seq: AtomicU32::new(0),
                pad: UnsafeCell::new(MaybeUninit::new(None)),
            })
            .collect();
        Self {
            slots,
            writer: Mutex::new(()),
        }
    }

    /// Store (or, with `None`, clear) the pad on `key`. Not for the audio
    /// thread: it takes the writer lock.
    pub fn set(&self, key: u8, pad: Option<Pad>) {
        let _writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        self.write(key, pad);
    }

    fn write(&self, key: u8, pad: Option<Pad>) {
        let slot = &self.slots[key as usize & 127];
        let seq = slot.seq.load(Ordering::Relaxed);
        slot.seq.store(seq.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        // SAFETY: writers are serialised by `writer`; readers discard what
        // they read while the sequence is odd or has moved.
        unsafe { std::ptr::write_volatile(slot.pad.get(), MaybeUninit::new(pad)) };
        slot.seq.store(seq.wrapping_add(2), Ordering::Release);
    }

    /// The pad on `key`, if any. Real-time safe: no lock, no allocation.
    /// Returns `None` if a write keeps racing the read ([`READ_TRIES`]).
    pub fn get(&self, key: u8) -> Option<Pad> {
        let slot = &self.slots[key as usize & 127];
        for _ in 0..READ_TRIES {
            let before = slot.seq.load(Ordering::Acquire);
            if before & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }
            // SAFETY: the copy stays `MaybeUninit` (no validity assumed)
            // until the sequence check shows no write overlapped it; then it
            // is a whole value `write` stored.
            let raw = unsafe { std::ptr::read_volatile(slot.pad.get()) };
            fence(Ordering::Acquire);
            if slot.seq.load(Ordering::Relaxed) == before {
                return unsafe { raw.assume_init() };
            }
        }
        None
    }

    /// Rebuild every slot from `state`: its pads, and no pad elsewhere. Not
    /// for the audio thread (it builds pads, which allocates).
    pub fn load(&self, state: &KitState) {
        let pads: Vec<(u8, Pad)> = state.pads.iter().map(|p| (p.key, p.build())).collect();
        let _writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        for key in 0..128u8 {
            let pad = pads.iter().find(|(k, _)| *k == key).map(|&(_, p)| p);
            self.write(key, pad);
        }
    }

    /// Keys that have a pad.
    pub fn keys(&self) -> impl Iterator<Item = u8> + '_ {
        (0..128u8).filter(|&k| self.get(k).is_some())
    }
}

#[cfg(test)]
mod tests;

//! The factory preset bank, built in code (`reorganize-factory-presets`).
//!
//! The bank is five curated folders of 12 presets designed by hand
//! (`curated`), plus `experimental`: one generated preset for every exciter ×
//! resonator pair (`experimental`). `bank()` builds every preset with its
//! stored level (`levels.rs`), exactly as `cargo run --bin seed_presets`
//! writes it to `presets/factory/`, so the bank tests check the definitions
//! without reading files and a test compares them with the files on disk.
//!
//! Levelling (`level`) renders each preset through the plugin's own output
//! path and sets Output so a held middle C sits at `TARGET_RMS`. It runs
//! only in the seeder; the results are written to `levels.rs`.

pub mod character;
mod curated;
mod drums;
mod experimental;
mod levels;
pub mod measure;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use crate::offline::{render_preset_pre_limiter, OfflineNote, PresetRenderer};
use crate::params::{CorrosionParams, Object};
use crate::presets::{Preset, PRESET_EXTENSION};

pub use experimental::{
    exciter_slot_ids, experimental_preset, object_character_ids, EXPERIMENTAL_FIXED_IDS,
    MAX_SEED_TRIES,
};

/// The curated folders, in browser order. Each holds exactly 12 presets.
pub const CURATED_FOLDERS: [&str; 5] = ["bass", "lead", "chords", "drones", "percussion"];

/// The folder of the generated exciter × resonator grid.
pub const EXPERIMENTAL_FOLDER: &str = "experimental";

/// The folder of the drum kits (`per-key-kits`) and how many it holds.
pub const DRUMS_FOLDER: &str = "drums";
pub const DRUM_KIT_COUNT: usize = 10;

pub use drums::{
    CLAP, CLOSED_HAT, CRASH, HIGH_TOM, KICK, LOW_TOM, MID_TOM, OPEN_HAT, PEDAL_HAT, RIDE,
    SIDE_STICK, SNARE,
};

/// How many presets each curated folder holds.
pub const CURATED_FOLDER_SIZE: usize = 12;

/// Number of exciter models (parameter values 1–16).
pub const EXCITER_COUNT: i32 = 16;

/// Number of resonator objects (parameter values 0–8).
pub const OBJECT_COUNT: i32 = 9;

/// One factory preset and where it lives in the bank.
#[derive(Clone, Debug, PartialEq)]
pub struct FactoryPreset {
    /// Folder relative to `presets/factory`, `/`-separated
    /// (`"bass"`, `"experimental/bow"`).
    pub folder: String,
    /// File stem.
    pub slug: String,
    pub preset: Preset,
}

impl FactoryPreset {
    /// The file's path relative to `presets/factory`, `/`-separated.
    pub fn relative_path(&self) -> String {
        format!("{}/{}.{PRESET_EXTENSION}", self.folder, self.slug)
    }

    /// The file's path under `root`.
    pub fn path_under(&self, root: &std::path::Path) -> PathBuf {
        let mut path = root.to_path_buf();
        for part in self.folder.split('/') {
            path.push(part);
        }
        path.push(format!("{}.{PRESET_EXTENSION}", self.slug));
        path
    }
}

/// Lowercase, `_`-separated file stem for a display name.
pub fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// The display name of exciter `exciter` (1–16).
pub fn exciter_name(exciter: i32) -> &'static str {
    crate::params::ExciterType::from_int(exciter).name()
}

/// A preset at the plugin's defaults, named `name`.
pub(crate) fn base(name: &str) -> Preset {
    Preset::from_params(name, &CorrosionParams::default())
}

/// The curated presets, unlevelled, in folder order.
pub fn curated_presets() -> Vec<FactoryPreset> {
    curated::presets()
        .into_iter()
        .map(|(folder, preset)| FactoryPreset {
            folder: folder.to_string(),
            slug: slug(&preset.name),
            preset,
        })
        .collect()
}

/// The drum kits, their pads unlevelled (level 0 dB), in browser order.
pub fn drum_kits() -> Vec<FactoryPreset> {
    drums::kits()
        .into_iter()
        .map(|preset| FactoryPreset {
            folder: DRUMS_FOLDER.to_string(),
            slug: slug(&preset.name),
            preset,
        })
        .collect()
}

/// The experimental preset for one pair, drawn with seed try `try_index`,
/// unlevelled.
pub fn experimental_entry(exciter: i32, object: Object, try_index: u32) -> FactoryPreset {
    let preset = experimental_preset(exciter, object, try_index);
    FactoryPreset {
        folder: format!("{EXPERIMENTAL_FOLDER}/{}", slug(exciter_name(exciter))),
        slug: slug(object.name()),
        preset,
    }
}

/// Every factory preset as the seeder writes it: the curated folders, the
/// drum kits, then the experimental grid (by exciter, then object), each
/// with its stored level (a kit's per pad) and, for experimental presets,
/// its stored seed try.
pub fn bank() -> Vec<FactoryPreset> {
    let mut bank = curated_presets();
    for mut entry in drum_kits() {
        let path = entry.relative_path();
        if let Some(kit) = entry.preset.kit.as_mut() {
            for pad in &mut kit.pads {
                if let Some(level_db) = levels::stored_pad(&path, pad.key) {
                    pad.level_db = level_db;
                }
            }
        }
        bank.push(entry);
    }
    for exciter in 1..=EXCITER_COUNT {
        for object in 0..OBJECT_COUNT {
            let object = Object::from_int(object);
            let path = experimental_entry(exciter, object, 0).relative_path();
            let try_index = levels::stored(&path).map_or(0, |l| l.try_index);
            bank.push(experimental_entry(exciter, object, try_index));
        }
    }
    for entry in &mut bank {
        if let Some(stored) = levels::stored(&entry.relative_path()) {
            entry.preset.output = stored.output;
        }
    }
    bank
}

/// A stored level: the Output the seeder chose and the seed try it used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoredLevel {
    pub try_index: u32,
    pub output: f32,
}

// --- Levelling and the quality floor -------------------------------------

/// The RMS (both channels, the whole render) every factory preset's floor
/// render is levelled to (−17.7 dBFS). Chosen once (design decision 4): the
/// median of the 60 curated presets at Output 0 dB was 0.1254
/// (`seed_presets --target`), rounded to two digits.
pub const TARGET_RMS: f32 = 0.13;

/// How far (dB) a preset's floor-render RMS may sit from [`TARGET_RMS`].
pub const FLOOR_TOLERANCE_DB: f32 = 6.0;

/// The largest share of samples allowed at the output ceiling.
pub const MAX_CEILING_FRACTION: f32 = 0.01;

/// The floor render's rate, note, velocity (host scale) and length.
pub const FLOOR_SAMPLE_RATE: u32 = 48_000;
pub const FLOOR_NOTE: u8 = 60;
pub const FLOOR_VELOCITY: f32 = 100.0 / 127.0;
pub const FLOOR_SECONDS: f32 = 2.0;

/// The headroom render's velocity: the same held note at full velocity must
/// also keep at most [`MAX_CEILING_FRACTION`] of its samples at the ceiling,
/// so a hard-played note does not drive the limiter.
pub const HEADROOM_VELOCITY: f32 = 1.0;

/// How far (dB) below [`TARGET_RMS`] the full-velocity note may sit, so a
/// preset that goes quiet when played hard is rejected.
pub const MAX_FULL_VELOCITY_DROP_DB: f32 = 12.0;

/// The largest Output levelling may set (+40 dB, the parameter's top).
pub const MAX_OUTPUT: f32 = 100.0;

/// A floor render's measurements.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorReport {
    /// Every output sample is finite.
    pub finite: bool,
    /// RMS of the limited output, both channels.
    pub rms: f32,
    /// Share of samples (both channels) whose level before the limiter is
    /// above the output ceiling: how often the limiter acts.
    pub ceiling_fraction: f32,
    /// The same share for the held note at [`HEADROOM_VELOCITY`].
    pub full_velocity_ceiling_fraction: f32,
    /// RMS before the limiter of the held note at [`HEADROOM_VELOCITY`].
    pub full_velocity_rms: f32,
}

impl FloorReport {
    /// The RMS relative to [`TARGET_RMS`], in dB.
    pub fn level_db(&self) -> f32 {
        20.0 * (self.rms.max(1e-12) / TARGET_RMS).log10()
    }

    /// Whether the floor holds, and if not, why.
    pub fn check(&self) -> Result<(), String> {
        if !self.finite {
            return Err("non-finite samples".into());
        }
        if self.level_db().abs() > FLOOR_TOLERANCE_DB {
            return Err(format!(
                "level {:+.1} dB from the target (±{FLOOR_TOLERANCE_DB} dB)",
                self.level_db()
            ));
        }
        if self.ceiling_fraction > MAX_CEILING_FRACTION {
            return Err(format!(
                "{:.2} % of samples at the ceiling (≤ {:.0} %)",
                self.ceiling_fraction * 100.0,
                MAX_CEILING_FRACTION * 100.0
            ));
        }
        let full_db = 20.0 * (self.full_velocity_rms.max(1e-12) / TARGET_RMS).log10();
        if full_db < -MAX_FULL_VELOCITY_DROP_DB {
            return Err(format!(
                "level {full_db:+.1} dB from the target at full velocity \
                 (≥ −{MAX_FULL_VELOCITY_DROP_DB} dB)"
            ));
        }
        if self.full_velocity_ceiling_fraction > MAX_CEILING_FRACTION {
            return Err(format!(
                "{:.2} % of samples at the ceiling at full velocity (≤ {:.0} %)",
                self.full_velocity_ceiling_fraction * 100.0,
                MAX_CEILING_FRACTION * 100.0
            ));
        }
        Ok(())
    }
}

fn floor_notes() -> [OfflineNote; 1] {
    floor_notes_at(FLOOR_NOTE)
}

fn headroom_notes() -> [OfflineNote; 1] {
    headroom_notes_at(FLOOR_NOTE)
}

fn floor_notes_at(note: u8) -> [OfflineNote; 1] {
    [OfflineNote::on(0, note, FLOOR_VELOCITY)]
}

fn headroom_notes_at(note: u8) -> [OfflineNote; 1] {
    [OfflineNote::on(0, note, HEADROOM_VELOCITY)]
}

fn frames() -> usize {
    (FLOOR_SAMPLE_RATE as f32 * FLOOR_SECONDS) as usize
}

/// Share of samples (both channels) above `ceiling`.
fn fraction_above(frames: &[(f32, f32)], ceiling: f32) -> f32 {
    let over = frames
        .iter()
        .flat_map(|&(l, r)| [l, r])
        .filter(|x| x.abs() > ceiling)
        .count();
    over as f32 / (2 * frames.len().max(1)) as f32
}

fn rms(frames: &[(f32, f32)]) -> f32 {
    let sum: f64 = frames
        .iter()
        .map(|&(l, r)| (l as f64).powi(2) + (r as f64).powi(2))
        .sum();
    (sum / (2 * frames.len().max(1)) as f64).sqrt() as f32
}

/// Render `preset`'s floor note (a held middle C, 2 s) and the same note at
/// full velocity, and measure them.
pub fn measure_floor(preset: &Preset) -> FloorReport {
    measure_floor_at(preset, FLOOR_NOTE)
}

/// The floor measurements with `note` held instead of middle C (a drum
/// pad's key).
pub fn measure_floor_at(preset: &Preset, note: u8) -> FloorReport {
    let rendered =
        PresetRenderer::new(preset).render(&floor_notes_at(note), FLOOR_SAMPLE_RATE, frames());
    let full = render_preset_pre_limiter(
        preset,
        &headroom_notes_at(note),
        FLOOR_SAMPLE_RATE,
        FLOOR_SECONDS,
    );
    let ceiling = preset.extra.analog_ceiling;
    FloorReport {
        finite: rendered
            .output
            .iter()
            .chain(&full)
            .all(|&(l, r)| l.is_finite() && r.is_finite()),
        rms: rms(&rendered.output),
        ceiling_fraction: fraction_above(&rendered.pre_limiter, ceiling),
        full_velocity_ceiling_fraction: fraction_above(&full, ceiling),
        full_velocity_rms: rms(&full),
    }
}

/// Set `preset.output` so its floor render sits at [`TARGET_RMS`], lowered
/// if needed so the full-velocity note keeps at most
/// [`MAX_CEILING_FRACTION`] of its samples at the ceiling, and return the
/// levelled render's measurements.
///
/// The output gain is applied before the limiter, so the level before the
/// limiter scales exactly with Output: one render of each note at Output 1
/// gives both gains (the headroom gain puts the full-velocity note's
/// 99th-percentile sample, less a margin, at the ceiling). Output is rounded
/// to 4 significant digits so the files stay readable, and capped at
/// [`MAX_OUTPUT`].
pub fn level(preset: &mut Preset) -> FloorReport {
    preset.output = 1.0;
    let pre = render_preset_pre_limiter(preset, &floor_notes(), FLOOR_SAMPLE_RATE, FLOOR_SECONDS);
    let at_unity = rms(&pre);
    let mut gain = if at_unity.is_finite() && at_unity > 0.0 {
        (TARGET_RMS / at_unity).min(MAX_OUTPUT)
    } else {
        1.0
    };
    let full =
        render_preset_pre_limiter(preset, &headroom_notes(), FLOOR_SAMPLE_RATE, FLOOR_SECONDS);
    let quantile = allowed_peak(&full);
    if quantile.is_finite() && quantile > 0.0 {
        gain = gain.min(HEADROOM_MARGIN * preset.extra.analog_ceiling / quantile);
    }
    preset.output = round_significant(gain, 4);
    measure_floor(preset)
}

/// The most a levelled experimental preset's floor level may move when Size,
/// Damage, Drive or Pressure changes by 0.01 % or 0.1 %. A preset past it sits
/// on a knife edge between two behaviours, which each platform's libm
/// rounding can pick differently (`felt_mallet/tank` measured 0 dB on Linux
/// and -8 dB on Windows), so the seeder re-draws it.
pub const MAX_LEVEL_INSTABILITY_DB: f32 = 2.0;

/// The largest change of [`FloorReport::level_db`] over tiny relative changes
/// (+0.01 %, -0.1 %) of Size, Damage, Drive and Pressure.
pub fn level_instability_db(preset: &Preset) -> f32 {
    let base = measure_floor(preset).level_db();
    let mut worst = 0.0_f32;
    for rel in [1e-4_f32, -1e-3] {
        for which in 0..4 {
            let mut p = preset.clone();
            match which {
                0 => p.size *= 1.0 + rel,
                1 => p.damage *= 1.0 + rel,
                2 => p.drive *= 1.0 + rel,
                _ => p.extra.exciter_pressure *= 1.0 + rel,
            }
            worst = worst.max((measure_floor(&p).level_db() - base).abs());
        }
    }
    worst
}

/// Headroom kept under the ceiling by [`level`], for the rounding of Output.
const HEADROOM_MARGIN: f32 = 0.995;

/// The sample level that [`MAX_CEILING_FRACTION`] of the samples exceed.
fn allowed_peak(frames: &[(f32, f32)]) -> f32 {
    let mut levels: Vec<f32> = frames
        .iter()
        .flat_map(|&(l, r)| [l.abs(), r.abs()])
        .collect();
    if levels.is_empty() {
        return 0.0;
    }
    levels.sort_by(f32::total_cmp);
    let allowed = (levels.len() as f32 * MAX_CEILING_FRACTION) as usize;
    levels[levels.len() - 1 - allowed.min(levels.len() - 1)]
}

/// The kit `preset` with only its pad on `key` (a pad's floor is measured
/// alone).
pub fn pad_alone(preset: &Preset, key: u8) -> Preset {
    let mut alone = preset.clone();
    if let Some(kit) = alone.kit.as_mut() {
        kit.pads.retain(|p| p.key == key);
    }
    alone
}

/// The range a drum pad's level may take.
pub const PAD_LEVEL_RANGE_DB: std::ops::RangeInclusive<f32> = -40.0..=60.0;

/// Set the level of the pad on `key` of the kit `preset` so the pad, held
/// alone on its key, sits at [`TARGET_RMS`] with the full-velocity headroom
/// of [`level`], and return its floor measurements. The pad's gain comes
/// before the shared post chain, which may not be linear, so the level is
/// found in a few correction steps; it is rounded to 0.1 dB.
pub fn level_pad(preset: &mut Preset, key: u8) -> FloorReport {
    let ceiling = preset.extra.analog_ceiling;
    let set_level = |preset: &mut Preset, level_db: f32| {
        if let Some(pad) = preset
            .kit
            .as_mut()
            .and_then(|kit| kit.pads.iter_mut().find(|p| p.key == key))
        {
            pad.level_db = level_db;
        }
    };
    let mut level_db = 0.0f32;
    for _ in 0..5 {
        set_level(preset, level_db);
        let alone = pad_alone(preset, key);
        let pre = render_preset_pre_limiter(
            &alone,
            &floor_notes_at(key),
            FLOOR_SAMPLE_RATE,
            FLOOR_SECONDS,
        );
        let full = render_preset_pre_limiter(
            &alone,
            &headroom_notes_at(key),
            FLOOR_SAMPLE_RATE,
            FLOOR_SECONDS,
        );
        let level = rms(&pre);
        let peak = allowed_peak(&full);
        if !(level.is_finite() && level > 0.0) {
            break;
        }
        let mut step = TARGET_RMS / level;
        if peak.is_finite() && peak > 0.0 {
            step = step.min(HEADROOM_MARGIN * ceiling / peak);
        }
        let step_db = 20.0 * step.log10();
        let next =
            (level_db + step_db).clamp(*PAD_LEVEL_RANGE_DB.start(), *PAD_LEVEL_RANGE_DB.end());
        let done = (next - level_db).abs() < 0.1;
        level_db = next;
        if done {
            break;
        }
    }
    level_db = (level_db * 10.0).round() / 10.0;
    set_level(preset, level_db);
    measure_floor_at(&pad_alone(preset, key), key)
}

fn round_significant(x: f32, digits: i32) -> f32 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = 10f64.powi(digits - 1 - (x.abs() as f64).log10().floor() as i32);
    ((x as f64 * scale).round() / scale) as f32
}

//! The factory preset bank, built in code (`reorganize-factory-presets`).
//!
//! The bank is five curated folders of 12 presets designed by hand
//! (`curated`), plus `experimental`: one generated preset for every exciter ×
//! resonator pair (`experimental`). [`bank`] builds every preset with its
//! stored level ([`levels`]), exactly as `cargo run --bin seed_presets`
//! writes it to `presets/factory/`, so the bank tests check the definitions
//! without reading files and a test compares them with the files on disk.
//!
//! Levelling ([`level`]) renders each preset through the plugin's own output
//! path and sets Output so a held middle C sits at [`TARGET_RMS`]. It runs
//! only in the seeder; the results are written to `levels.rs`.

pub mod character;
mod curated;
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

/// Every factory preset as the seeder writes it: the curated folders, then
/// the experimental grid (by exciter, then object), each with its stored
/// level and, for experimental presets, its stored seed try.
pub fn bank() -> Vec<FactoryPreset> {
    let mut bank = curated_presets();
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
        Ok(())
    }
}

fn floor_notes() -> [OfflineNote; 1] {
    [OfflineNote::on(0, FLOOR_NOTE, FLOOR_VELOCITY)]
}

fn rms(frames: &[(f32, f32)]) -> f32 {
    let sum: f64 = frames
        .iter()
        .map(|&(l, r)| (l as f64).powi(2) + (r as f64).powi(2))
        .sum();
    (sum / (2 * frames.len().max(1)) as f64).sqrt() as f32
}

/// Render `preset`'s floor note (a held middle C, 2 s) and measure it.
pub fn measure_floor(preset: &Preset) -> FloorReport {
    let frames = (FLOOR_SAMPLE_RATE as f32 * FLOOR_SECONDS) as usize;
    let rendered = PresetRenderer::new(preset).render(&floor_notes(), FLOOR_SAMPLE_RATE, frames);
    let ceiling = preset.extra.analog_ceiling;
    let over = rendered
        .pre_limiter
        .iter()
        .flat_map(|&(l, r)| [l, r])
        .filter(|x| x.abs() > ceiling)
        .count();
    FloorReport {
        finite: rendered
            .output
            .iter()
            .all(|&(l, r)| l.is_finite() && r.is_finite()),
        rms: rms(&rendered.output),
        ceiling_fraction: over as f32 / (2 * frames.max(1)) as f32,
    }
}

/// Set `preset.output` so its floor render sits at [`TARGET_RMS`], and
/// return the levelled render's measurements.
///
/// The output gain is applied before the limiter, so the level before the
/// limiter scales exactly with Output: one render at Output 1 gives the
/// gain. Output is rounded to 4 significant digits so the files stay
/// readable, and capped at [`MAX_OUTPUT`].
pub fn level(preset: &mut Preset) -> FloorReport {
    preset.output = 1.0;
    let pre = render_preset_pre_limiter(preset, &floor_notes(), FLOOR_SAMPLE_RATE, FLOOR_SECONDS);
    let at_unity = rms(&pre);
    let gain = if at_unity.is_finite() && at_unity > 0.0 {
        (TARGET_RMS / at_unity).min(MAX_OUTPUT)
    } else {
        1.0
    };
    preset.output = round_significant(gain, 4);
    measure_floor(preset)
}

fn round_significant(x: f32, digits: i32) -> f32 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let scale = 10f64.powi(digits - 1 - (x.abs() as f64).log10().floor() as i32);
    ((x as f64 * scale).round() / scale) as f32
}

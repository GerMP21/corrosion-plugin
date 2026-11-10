//! Bank tests (`factory-preset-bank`): folders and counts, the experimental
//! grid, determinism, parameter variation, and the files on disk.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use nih_plug::prelude::Params;

use super::*;

/// A preset's plain value for parameter `id`, read from its JSON (every
/// parameter ID is a preset key, at the top level or under `extra`).
pub(super) fn value_of(preset: &Preset, id: &str) -> f64 {
    let json = serde_json::to_value(preset).unwrap();
    json.get(id)
        .or_else(|| json["extra"].get(id))
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| panic!("{id} is not a numeric preset key"))
}

fn experimental(bank: &[FactoryPreset]) -> Vec<&FactoryPreset> {
    bank.iter()
        .filter(|e| e.folder.starts_with(EXPERIMENTAL_FOLDER))
        .collect()
}

fn parameter_ids() -> BTreeSet<String> {
    CorrosionParams::default()
        .param_map()
        .into_iter()
        .map(|(id, _, _)| id)
        .collect()
}

#[test]
fn curated_folders_hold_twelve_presets() {
    let curated = curated_presets();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in &curated {
        assert!(
            CURATED_FOLDERS.contains(&entry.folder.as_str()),
            "{} is in an unknown folder {}",
            entry.preset.name,
            entry.folder
        );
        *counts.entry(entry.folder.as_str()).or_default() += 1;
        // Parses as a valid preset: the saved JSON loads back unchanged.
        let json = serde_json::to_string_pretty(&entry.preset).unwrap();
        let loaded = Preset::from_json_str(&json).expect("parse preset");
        assert_eq!(
            loaded, entry.preset,
            "{} changes on load",
            entry.preset.name
        );
    }
    for folder in CURATED_FOLDERS {
        assert_eq!(
            counts.get(folder).copied().unwrap_or(0),
            CURATED_FOLDER_SIZE,
            "folder {folder}"
        );
    }
}

#[test]
fn names_and_files_are_unique_within_a_folder() {
    let bank = bank();
    let mut names: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    let mut paths = BTreeSet::new();
    for entry in &bank {
        assert!(
            names
                .entry(entry.folder.as_str())
                .or_default()
                .insert(entry.preset.name.as_str()),
            "duplicate name {} in {}",
            entry.preset.name,
            entry.folder
        );
        assert!(
            paths.insert(entry.relative_path()),
            "duplicate file {}",
            entry.relative_path()
        );
    }
}

#[test]
fn experimental_grid_covers_every_pair_once() {
    let bank = bank();
    let grid = experimental(&bank);
    assert_eq!(grid.len(), (EXCITER_COUNT * OBJECT_COUNT) as usize);
    let mut pairs = BTreeSet::new();
    for entry in &grid {
        let pair = (entry.preset.exciter, entry.preset.object.to_int());
        assert!(pairs.insert(pair), "pair {pair:?} appears twice");
        assert_eq!(
            entry.folder,
            format!("{EXPERIMENTAL_FOLDER}/{}", slug(exciter_name(pair.0))),
            "{} is grouped under the wrong exciter",
            entry.preset.name
        );
        assert_eq!(
            entry.preset.name,
            format!("{} · {}", exciter_name(pair.0), entry.preset.object.name())
        );
    }
    for exciter in 1..=EXCITER_COUNT {
        for object in 0..OBJECT_COUNT {
            assert!(
                pairs.contains(&(exciter, object)),
                "missing {exciter} × {object}"
            );
        }
    }
}

#[test]
fn regeneration_is_deterministic() {
    let render = |bank: Vec<FactoryPreset>| -> Vec<(String, String)> {
        bank.into_iter()
            .map(|e| {
                (
                    e.relative_path(),
                    serde_json::to_string_pretty(&e.preset).unwrap(),
                )
            })
            .collect()
    };
    let first = render(bank());
    let second = render(bank());
    assert_eq!(first, second);
    // Every seed try draws a different preset, so re-drawing changes it.
    let a = experimental_preset(1, Object::Pipe, 0);
    let b = experimental_preset(1, Object::Pipe, 1);
    assert_ne!(a, b, "seed try 1 draws the same preset as try 0");
}

#[test]
fn generator_parameter_lists_name_real_parameters() {
    let ids = parameter_ids();
    for exciter in 1..=EXCITER_COUNT {
        let slots = exciter_slot_ids(exciter);
        assert!(
            !slots.is_empty(),
            "exciter {exciter} has no slot parameters"
        );
        for id in slots {
            assert!(
                ids.contains(*id),
                "exciter {exciter}: {id} is not a parameter"
            );
        }
    }
    for object in 0..OBJECT_COUNT {
        let object = Object::from_int(object);
        let character = object_character_ids(object);
        assert!(
            !character.is_empty(),
            "{} has no character parameters",
            object.name()
        );
        for id in character {
            assert!(
                ids.contains(*id),
                "{}: {id} is not a parameter",
                object.name()
            );
        }
    }
    for (id, _) in EXPERIMENTAL_FIXED_IDS {
        assert!(ids.contains(*id), "fixed {id} is not a parameter");
    }
}

#[test]
fn experimental_parameters_vary() {
    let bank = bank();
    let grid = experimental(&bank);
    let distinct = |entries: &[&&FactoryPreset], id: &str| -> usize {
        entries
            .iter()
            .map(|e| value_of(&e.preset, id).to_bits())
            .collect::<BTreeSet<_>>()
            .len()
    };
    let mut specific = BTreeSet::new();
    // Each exciter-specific parameter varies across that exciter's 9 presets.
    for exciter in 1..=EXCITER_COUNT {
        let own: Vec<_> = grid
            .iter()
            .filter(|e| e.preset.exciter == exciter)
            .collect();
        for id in exciter_slot_ids(exciter) {
            specific.insert(*id);
            assert!(
                distinct(&own, id) > 1,
                "{}: {id} takes one value across its presets",
                exciter_name(exciter)
            );
        }
    }
    // Each object character parameter varies across that object's 16.
    for object in 0..OBJECT_COUNT {
        let object = Object::from_int(object);
        let own: Vec<_> = grid.iter().filter(|e| e.preset.object == object).collect();
        for id in object_character_ids(object) {
            specific.insert(*id);
            assert!(
                distinct(&own, id) > 1,
                "{}: {id} takes one value across its presets",
                object.name()
            );
        }
    }
    // Every other parameter (shared voice controls, post chain), except the
    // documented fixed ones and the grid's own exciter/object, varies
    // across the folder.
    let all: Vec<_> = grid.iter().collect();
    let fixed: BTreeSet<&str> = EXPERIMENTAL_FIXED_IDS.iter().map(|(id, _)| *id).collect();
    for id in parameter_ids() {
        let id = id.as_str();
        if specific.contains(id) || fixed.contains(id) || id == "exciter" || id == "object" {
            continue;
        }
        assert!(
            distinct(&all, id) > 1,
            "shared {id} takes one value across the folder"
        );
    }
}

/// Every preset file under `dir`, keyed by its `/`-separated path relative to
/// `root`.
fn files_under(root: &std::path::Path, dir: &std::path::Path, out: &mut BTreeMap<String, String>) {
    for entry in std::fs::read_dir(dir).expect("read presets/factory") {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(root, &path, out);
        } else if path.extension().is_some_and(|e| e == PRESET_EXTENSION) {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(relative, std::fs::read_to_string(&path).unwrap());
        }
    }
}

#[test]
fn generated_bank_equals_the_files_on_disk() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("presets/factory");
    let mut on_disk = BTreeMap::new();
    files_under(&root, &root, &mut on_disk);
    let generated: BTreeMap<String, String> = bank()
        .into_iter()
        .map(|e| {
            (
                e.relative_path(),
                serde_json::to_string_pretty(&e.preset).unwrap(),
            )
        })
        .collect();
    let missing: Vec<_> = generated
        .keys()
        .filter(|k| !on_disk.contains_key(*k))
        .collect();
    let extra: Vec<_> = on_disk
        .keys()
        .filter(|k| !generated.contains_key(*k))
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "run `cargo run --release --bin seed_presets`: {} missing (first {:?}), {} stale (first {:?})",
        missing.len(),
        missing.iter().take(5).collect::<Vec<_>>(),
        extra.len(),
        extra.iter().take(5).collect::<Vec<_>>()
    );
    for (path, json) in &generated {
        assert_eq!(&on_disk[path], json, "{path} differs from its definition");
    }
}

// --- Quality floor and folder character -------------------------------------

#[test]
fn every_factory_preset_meets_the_quality_floor() {
    let bank = bank();
    assert_eq!(
        bank.len(),
        CURATED_FOLDERS.len() * CURATED_FOLDER_SIZE
            + DRUM_KIT_COUNT
            + (EXCITER_COUNT * OBJECT_COUNT) as usize
    );
    // A kit is silent on C4; its pads are held to the floor one by one in
    // `every_drum_pad_meets_the_quality_floor`.
    let failures: Vec<String> = bank
        .iter()
        .filter(|e| e.folder != DRUMS_FOLDER)
        .filter_map(|e| {
            let report = measure_floor(&e.preset);
            report
                .check()
                .err()
                .map(|why| format!("{}: {why}", e.relative_path()))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_quality_floor_rejects_silent_and_clipping_presets() {
    let mut preset = experimental_preset(2, Object::Pipe, 0);
    let levelled = level(&mut preset);
    assert!(levelled.check().is_ok(), "levelled: {levelled:?}");
    let mut silent = preset.clone();
    silent.output = 0.0;
    assert!(measure_floor(&silent).check().is_err());
    let mut quiet = preset.clone();
    quiet.output *= 0.4; // −8 dB
    assert!(measure_floor(&quiet).check().is_err());
    let mut loud = preset.clone();
    loud.output = MAX_OUTPUT;
    let report = measure_floor(&loud);
    assert!(
        report.ceiling_fraction > MAX_CEILING_FRACTION && report.check().is_err(),
        "{report:?}"
    );
    // Quiet or clipping only when played hard.
    let held = measure_floor(&preset);
    let hard_quiet = FloorReport {
        full_velocity_rms: TARGET_RMS * 0.2, // −14 dB
        ..held
    };
    assert!(hard_quiet.check().is_err(), "{hard_quiet:?}");
    let hard_clipping = FloorReport {
        full_velocity_ceiling_fraction: 2.0 * MAX_CEILING_FRACTION,
        ..held
    };
    assert!(hard_clipping.check().is_err(), "{hard_clipping:?}");
}

/// Every curated preset of `folder` passes its character checks; the folder
/// must be full, so an empty folder fails.
fn folder_has_its_character(folder: &str) {
    let presets: Vec<_> = curated_presets()
        .into_iter()
        .filter(|e| e.folder == folder)
        .collect();
    assert_eq!(presets.len(), CURATED_FOLDER_SIZE, "{folder}");
    let failures: Vec<String> = presets
        .iter()
        .filter_map(|e| {
            character::measure(folder, &e.preset)
                .check()
                .err()
                .map(|why| format!("{}: {why}", e.preset.name))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn basses_sit_low() {
    folder_has_its_character("bass");
}

#[test]
fn leads_are_pitched() {
    folder_has_its_character("lead");
}

#[test]
fn chord_patches_are_pitched_harmonic_sustaining_and_clean() {
    folder_has_its_character("chords");
}

#[test]
fn drones_sustain() {
    folder_has_its_character("drones");
}

#[test]
fn percussion_decays() {
    folder_has_its_character("percussion");
}

/// A preset for the checks below: `exciter` on `object` at `size`.
fn probe(exciter: i32, object: Object, size: f32) -> Preset {
    let mut p = base("Probe");
    p.exciter = exciter;
    p.object = object;
    p.size = size;
    p
}

const BOW: i32 = 1;
const FELT_MALLET: i32 = 3;
const HARD_MALLET: i32 = 4;
const STIFF_POINT: i32 = 9;

#[test]
fn character_checks_reject_wrong_presets() {
    let fails = |folder: &str, preset: &Preset| {
        let report = character::measure(folder, preset);
        assert!(
            report.check().is_err(),
            "{folder} accepts a wrong preset: {report:?}"
        );
    };
    let passes = |folder: &str, preset: &Preset| {
        let report = character::measure(folder, preset);
        assert!(report.check().is_ok(), "{folder}: {report:?}");
    };
    // A bowed taut cable held in Drone mode at Size 1, without BODY, is a
    // clean, pitched, harmonic chord patch ...
    let mut bowed_cable = probe(BOW, Object::TautCable, 1.0);
    bowed_cable.play_mode = 2; // Drone
    bowed_cable.body = 0.0;
    passes("chords", &bowed_cable);
    passes("lead", &bowed_cable);
    // ... but it swells by 10 dB over 8 s, so it is not a steady drone.
    fails("drones", &bowed_cable);
    // The old "Cold Pipe Bow" holds its level.
    let mut bowed_pipe = probe(BOW, Object::Pipe, 1.4);
    bowed_pipe.play_mode = 2;
    bowed_pipe.extra.pipe_diameter = 0.55;
    bowed_pipe.extra.bow_pressure = 0.9;
    bowed_pipe.extra.bow_speed = 0.8;
    bowed_pipe.extra.rosin_grip = 0.55;
    passes("drones", &bowed_pipe);
    // Neither sustained patch is percussion.
    fails("percussion", &bowed_pipe);
    fails("percussion", &bowed_cable);
    // Size scales the pitch down: at 1.4 it is 580 cents under the key.
    let mut detuned = bowed_cable.clone();
    detuned.size = 1.4;
    fails("lead", &detuned);
    // A cog is inharmonic.
    let mut cog = bowed_cable.clone();
    cog.object = Object::IndustrialCog;
    cog.extra.cog_dissonance = 1.0;
    fails("chords", &cog);
    // A felt-mallet bell dies away: not a sustaining chord, not a drone.
    let mut bell = probe(FELT_MALLET, Object::Pipe, 1.0);
    bell.extra.mallet_mass = 0.5;
    bell.extra.res_damping = 0.9;
    fails("chords", &bell);
    fails("drones", &bell);
    passes("percussion", &bell);
    // A small, bright scraped sheet is not a bass.
    let mut sheet = probe(STIFF_POINT, Object::SheetMetal, 0.2);
    sheet.extra.macro_brightness = 1.0;
    fails("bass", &sheet);
    // A hard-mallet strike on a large pipe is low.
    passes("bass", &probe(HARD_MALLET, Object::Pipe, 1.0));
}

/// The curated presets' measurements (design `## Measurements`): Output
/// chosen by levelling, the floor-render RMS at Output 0 dB, the centroid
/// and decay to −30 dB of a held middle C, and each character value, with
/// whether the folder's check passes.
#[test]
#[ignore]
fn curated_measurements_table() {
    use super::measure::*;
    println!("| Folder | Preset | Output | RMS @0 dB | Centroid (Hz) | Decay −30 dB (s) | Pitch err (c) | Low < 500 Hz | Partial dev (c) | Sustain 2 s (dB) | Flatness | Drone drift (dB) | Decay 1.5 s (dB) | Pass |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let entries = bank();
    let curated: Vec<_> = entries
        .iter()
        .filter(|e| !e.folder.starts_with(EXPERIMENTAL_FOLDER))
        .collect();
    let rows: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = curated.iter().map(|e| scope.spawn(move || {
            let mut unity = e.preset.clone();
            unity.output = 1.0;
            let rms0 = measure_floor(&unity).rms;
            let s = mono(&render_held(&e.preset, 60, 4.0).output);
            let centroid = Spectrum::of(&s, (0.1 * SAMPLE_RATE as f32) as usize, 65_536).centroid();
            let decay = decay_to_minus_30(&s).map_or("—".to_string(), |d| format!("{d:.2}"));
            let mut r = character::CharacterReport::default();
            character::measure_pitch(&e.preset, &mut r);
            character::measure_bass(&e.preset, &mut r);
            character::measure_chord(&e.preset, &mut r);
            character::measure_drone(&e.preset, &mut r);
            character::measure_percussion(&e.preset, &mut r);
            let pass = character::measure(&e.folder, &e.preset).check();
            format!("| {} | {} | {:.3} | {:.4} | {:.0} | {} | {:.0} | {:.2} | {:.1} | {:.1} | {:.4} | {:+.1} | {:.1} | {} |",
                e.folder, e.preset.name, e.preset.output, rms0, centroid, decay,
                r.pitch_error_cents.unwrap(), r.low_fraction.unwrap(), r.partial_deviation_cents.unwrap(),
                r.sustain_db.unwrap(), r.flatness.unwrap(), r.drone_drift_db.unwrap(), r.decay_db.unwrap(),
                match pass { Ok(()) => "yes".to_string(), Err(why) => format!("**no**: {why}") })
        })).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for row in rows {
        println!("{row}");
    }
}

/// Sweeps every parameter to both ends of its range on the default patch,
/// for each exciter on three objects, and prints the settings where a held
/// C4 at velocity 127 is silent or more than 6 dB under velocity 64, or
/// where the setting silences both velocities (`fix-pneumatic-jet-saturation`).
#[test]
#[ignore]
fn print_velocity_sweep() {
    use crate::offline::{render_preset_pre_limiter, OfflineNote};
    use nih_plug::prelude::ParamPtr;
    let db = |p: &Preset, v: f32| {
        let f = render_preset_pre_limiter(p, &[OfflineNote::on(0, 60, v)], 48_000, 1.0);
        let s: f64 = f.iter().map(|&(l, r)| (l * l + r * r) as f64).sum();
        10.0 * (s / (2 * f.len()) as f64).max(1e-24).log10() as f32
    };
    let skip = ["output", "ui_scale", "play_mode", "exciter", "object"];
    let objects = [Object::Pipe, Object::Plate, Object::TautCable];
    let lines: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (1..=EXCITER_COUNT)
            .map(|exciter| {
                scope.spawn(move || {
                    let defaults = CorrosionParams::default();
                    let params: Vec<(String, ParamPtr)> = defaults
                        .param_map()
                        .into_iter()
                        .map(|(id, ptr, _)| (id, ptr))
                        .collect();
                    let mut out = Vec::new();
                    for object in objects {
                        let mut base = super::base("sweep");
                        base.exciter = exciter;
                        base.object = object;
                        let (b64, b127) = (db(&base, 64.0 / 127.0), db(&base, 1.0));
                        out.push(format!(
                            "BASE {} {:?}: v64 {b64:.1} v127 {b127:.1}",
                            exciter_name(exciter),
                            object
                        ));
                        for (id, ptr) in &params {
                            if skip.contains(&id.as_str()) {
                                continue;
                            }
                            let other_slot = (1..=EXCITER_COUNT)
                                .any(|e| e != exciter && exciter_slot_ids(e).contains(&id.as_str()))
                                && !exciter_slot_ids(exciter).contains(&id.as_str());
                            let other_char = (0..OBJECT_COUNT).map(Object::from_int).any(|o| {
                                o != object && object_character_ids(o).contains(&id.as_str())
                            }) && !object_character_ids(object).contains(&id.as_str());
                            if other_slot || other_char {
                                continue;
                            }
                            for end in [0.0f32, 1.0] {
                                // SAFETY: `ptr` belongs to `defaults`, alive here.
                                let plain = unsafe { ptr.preview_plain(end) };
                                let mut json = serde_json::to_value(&base).unwrap();
                                let value = if matches!(ptr, ParamPtr::IntParam(_)) {
                                    serde_json::Value::from(plain.round() as i64)
                                } else {
                                    serde_json::Value::from(plain as f64)
                                };
                                if json.get(id.as_str()).is_some() {
                                    json[id.as_str()] = value;
                                } else {
                                    json["extra"][id.as_str()] = value;
                                }
                                let p = Preset::from_json_str(&json.to_string()).unwrap();
                                let (v64, v127) = (db(&p, 64.0 / 127.0), db(&p, 1.0));
                                let flag = if v127 < -120.0 && v64 >= -120.0 {
                                    Some("SILENT AT 127")
                                } else if v127 < -120.0 && b127 >= -120.0 {
                                    Some("SILENCES")
                                } else if v127 < v64 - 6.0 {
                                    Some("QUIETER AT 127")
                                } else {
                                    None
                                };
                                if let Some(flag) = flag {
                                    out.push(format!(
                                        "FLAG {flag}: {} {:?} {id}={plain:.4}: v64 {v64:.1} v127 {v127:.1} (default v64 {b64:.1} v127 {b127:.1})",
                                        exciter_name(exciter),
                                        object
                                    ));
                                }
                            }
                        }
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    for line in lines {
        println!("{line}");
    }
}

/// Prints each drum pad's level, energy below 500 Hz, centroid and decay
/// (held alone on its key, 2 s): the table in the `per-key-kits` design.
#[test]
#[ignore]
fn drum_pad_measurements_table() {
    use super::measure::{decay_to_minus_30, mono, render_held, Spectrum};
    for entry in bank().into_iter().filter(|e| e.folder == DRUMS_FOLDER) {
        let kit = entry.preset.kit.clone().unwrap();
        for pad in &kit.pads {
            let alone = pad_alone(&entry.preset, pad.key);
            let samples = mono(&render_held(&alone, pad.key, 2.0).output);
            let spectrum = Spectrum::of(&samples, 0, 65_536);
            let decay =
                decay_to_minus_30(&samples).map_or("—".to_string(), |d| format!("{d:.2} s"));
            let floor = measure_floor_at(&alone, pad.key);
            println!(
                "PAD | {} | {} | {:+.1} dB | {:+.1} dB | {:.0} % | {:.0} Hz | {} |",
                entry.preset.name,
                pad.key,
                pad.level_db,
                floor.level_db(),
                100.0 * spectrum.fraction_below(500.0),
                spectrum.centroid(),
                decay
            );
        }
    }
}

// --- Drum kits (`per-key-kits`) -------------------------------------------

/// Spec "Ten drum kits": the folder holds 10 Kit-mode presets with pads on
/// at least the kick, snare, closed hat and open hat.
#[test]
fn the_drums_folder_holds_ten_kits() {
    let kits: Vec<_> = bank()
        .into_iter()
        .filter(|e| e.folder == DRUMS_FOLDER)
        .collect();
    assert_eq!(kits.len(), DRUM_KIT_COUNT);
    for entry in &kits {
        assert_eq!(
            entry.preset.play_mode,
            crate::params::PlayMode::Kit.to_int(),
            "{}",
            entry.preset.name
        );
        let kit = entry.preset.kit.as_ref().expect("a kit");
        for key in [KICK, SNARE, CLOSED_HAT, OPEN_HAT] {
            assert!(
                kit.pad(key).is_some(),
                "{}: no pad on {key}",
                entry.preset.name
            );
        }
    }
}

/// Spec "Ten drum kits": every pad, played alone on its key, meets the
/// quality floor.
#[test]
fn every_drum_pad_meets_the_quality_floor() {
    let failures: Vec<String> = bank()
        .into_iter()
        .filter(|e| e.folder == DRUMS_FOLDER)
        .flat_map(|entry| {
            let keys: Vec<u8> = entry
                .preset
                .kit
                .as_ref()
                .map_or_else(Vec::new, |k| k.pads.iter().map(|p| p.key).collect());
            keys.into_iter().filter_map(move |key| {
                measure_floor_at(&pad_alone(&entry.preset, key), key)
                    .check()
                    .err()
                    .map(|why| format!("{} pad {key}: {why}", entry.preset.name))
            })
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A drum pad's character, held alone on its key for 2 s.
struct PadCharacter {
    below_500: f64,
    centroid: f32,
    pitch: f32,
    decay: Option<f32>,
}

fn pad_character(preset: &Preset, key: u8) -> PadCharacter {
    use super::measure::{decay_to_minus_30, mono, render_held, Spectrum};
    let samples = mono(&render_held(&pad_alone(preset, key), key, 2.0).output);
    let spectrum = Spectrum::of(&samples, 0, 65_536);
    PadCharacter {
        below_500: spectrum.fraction_below(500.0),
        centroid: spectrum.centroid(),
        pitch: spectrum.strongest_peak(),
        decay: decay_to_minus_30(&samples),
    }
}

/// The kits sound like drum kits (design, "Drum kits"): a low, short kick;
/// a short snare above it; bright hats, the closed one short and the open
/// one longer; toms rising in pitch.
#[test]
fn drum_kits_have_drum_character() {
    let mut failures = Vec::new();
    for entry in bank().into_iter().filter(|e| e.folder == DRUMS_FOLDER) {
        let name = &entry.preset.name;
        let pad = |key| pad_character(&entry.preset, key);
        let (kick, snare) = (pad(KICK), pad(SNARE));
        let (closed, open) = (pad(CLOSED_HAT), pad(OPEN_HAT));
        let toms = [pad(LOW_TOM), pad(MID_TOM), pad(HIGH_TOM)];
        let mut fail = |why: String| failures.push(format!("{name}: {why}"));
        if kick.below_500 < 0.7 {
            fail(format!("kick {:.0} % below 500 Hz", 100.0 * kick.below_500));
        }
        if !kick.decay.is_some_and(|d| d <= 1.5) {
            fail(format!("kick decay {:?}", kick.decay));
        }
        if !snare.decay.is_some_and(|d| d <= 1.5) {
            fail(format!("snare decay {:?}", snare.decay));
        }
        if snare.centroid <= kick.centroid {
            fail(format!(
                "snare centroid {:.0} ≤ kick {:.0} Hz",
                snare.centroid, kick.centroid
            ));
        }
        for (hat, what) in [(&closed, "closed hat"), (&open, "open hat")] {
            if hat.centroid < 2_000.0 || hat.below_500 > 0.3 {
                fail(format!(
                    "{what} centroid {:.0} Hz, {:.0} % below 500 Hz",
                    hat.centroid,
                    100.0 * hat.below_500
                ));
            }
        }
        if !closed.decay.is_some_and(|d| d <= 1.0) {
            fail(format!("closed hat decay {:?}", closed.decay));
        }
        if open
            .decay
            .is_some_and(|o| closed.decay.is_some_and(|c| o <= c))
        {
            fail(format!(
                "open hat {:?} not longer than closed {:?}",
                open.decay, closed.decay
            ));
        }
        if !(toms[0].pitch < toms[1].pitch && toms[1].pitch < toms[2].pitch) {
            fail(format!(
                "toms {:.0} / {:.0} / {:.0} Hz don't rise",
                toms[0].pitch, toms[1].pitch, toms[2].pitch
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Diagnostic for `experimental/felt_mallet/tank`, whose floor level is about
/// 0 dB on Linux and −8 dB on Windows (CI, 2026-11-10). Prints the level per
/// quarter second, and the level with each parameter reset to the base
/// preset's value, so the platforms can be compared side by side.
#[test]
#[ignore]
fn print_felt_mallet_tank_divergence() {
    let target = "experimental/felt_mallet/tank.corrosion-preset";
    let entry = bank()
        .into_iter()
        .find(|e| e.relative_path() == target)
        .expect("preset in the bank");
    let preset = entry.preset.clone();
    println!("level {:+.2} dB", measure_floor(&preset).level_db());

    let pre = crate::offline::render_preset_pre_limiter(
        &preset,
        &floor_notes(),
        FLOOR_SAMPLE_RATE,
        FLOOR_SECONDS,
    );
    let quarter = FLOOR_SAMPLE_RATE as usize / 4;
    for (i, chunk) in pre.chunks(quarter).enumerate() {
        let db = 20.0 * (rms(chunk).max(1e-12) / TARGET_RMS).log10();
        println!("t={:.2}s {db:+.2} dB", i as f32 * 0.25);
    }

    let base = serde_json::to_value(super::base("base")).unwrap();
    let json = serde_json::to_value(&preset).unwrap();
    let mut keys: Vec<(bool, String)> = Vec::new();
    for (k, v) in json.as_object().unwrap() {
        if k != "extra" && k != "name" && k != "output" && base.get(k) != Some(v) {
            keys.push((false, k.clone()));
        }
    }
    for (k, v) in json["extra"].as_object().unwrap() {
        if base["extra"].get(k) != Some(v) {
            keys.push((true, k.clone()));
        }
    }
    for (extra, k) in keys {
        let mut j = json.clone();
        if extra {
            j["extra"][&k] = base["extra"][&k].clone();
        } else {
            j[&k] = base[&k].clone();
        }
        let p = Preset::from_json_str(&j.to_string()).unwrap();
        println!("reset {k}: {:+.2} dB", measure_floor(&p).level_db());
    }
}

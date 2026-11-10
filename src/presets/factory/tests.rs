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
        CURATED_FOLDERS.len() * CURATED_FOLDER_SIZE + (EXCITER_COUNT * OBJECT_COUNT) as usize
    );
    let failures: Vec<String> = bank
        .iter()
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

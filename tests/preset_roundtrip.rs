use corrosion::presets::{Preset, PresetParameters, PRESET_VERSION};
use corrosion::Object;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("corrosion_{name}_{nanos}.corrosion-preset"))
}

#[test]
fn preset_roundtrip_save_and_load() {
    let preset = Preset {
        name: "Iron Strike".to_string(),
        version: PRESET_VERSION.to_string(),
        object: Object::Pipe,
        exciter: 2,
        size: 1.0,
        rust: 0.3,
        damage: 0.1,
        drive: 0.5,
        output: 1.0,
        width: 0.5,
        body: 0.2,
        quality_mode: 1,
        play_mode: 0,
        extra: PresetParameters::default(),
    };

    let path = unique_temp_path("roundtrip");
    preset.save(&path).unwrap();
    let loaded = Preset::load(&path).unwrap();
    assert_eq!(preset, loaded);
    let _ = fs::remove_file(path);
}

#[test]
fn version_1_preset_loads_with_default_extended_parameters() {
    let json = r#"
    {
        "name": "Iron Strike",
        "version": "1",
        "object": "Pipe",
        "exciter": 0,
        "size": 1.0,
        "rust": 0.3,
        "damage": 0.1,
        "drive": 0.5,
        "output": 1.0,
        "width": 0.5,
        "body": 0.2
    }
    "#;

    let preset: Preset = serde_json::from_str(json).unwrap();
    assert_eq!(preset.name, "Iron Strike");
    assert_eq!(preset.extra, PresetParameters::default());
}

#[test]
fn expanded_parameters_roundtrip() {
    let mut preset = Preset {
        name: "Expanded State".to_string(),
        version: PRESET_VERSION.to_string(),
        object: Object::Tank,
        exciter: 16,
        size: 7.5,
        rust: 4.0,
        damage: 8.0,
        drive: 4.5,
        output: 3.0,
        width: 2.0,
        body: 4.0,
        quality_mode: 3,
        play_mode: 0,
        extra: PresetParameters::default(),
    };
    preset.extra.drive_amount = 3.2;
    preset.extra.space_mode = 2;
    preset.extra.space_amount = 0.8;
    preset.extra.analog_ceiling = 0.7;
    preset.extra.diode_softness = 0.9;
    preset.extra.hand_mass = 2.4;
    preset.extra.pipe_ring_decay = 0.991;
    preset.extra.ridge_spacing = 0.08;
    preset.extra.flow_rate = 2.2;
    preset.extra.plate_aspect = 2.5;
    preset.extra.cog_dissonance = 0.7;

    let path = unique_temp_path("expanded_roundtrip");
    preset.save(&path).unwrap();
    let loaded = Preset::load(&path).unwrap();
    assert_eq!(preset, loaded);

    let params = loaded.into_params();
    assert_eq!(params.drive.value(), 4.5);
    assert_eq!(params.width.value(), 2.0);
    assert_eq!(params.space_mode.value(), 2);
    assert_eq!(params.space_amount.value(), 0.8);
    assert_eq!(params.analog_ceiling.value(), 0.7);
    assert_eq!(params.diode_softness.value(), 0.9);
    assert_eq!(params.hand_mass.value(), 2.4);
    assert_eq!(params.pipe_ring_decay.value(), 0.991);
    assert_eq!(params.ridge_spacing.value(), 0.08);
    assert_eq!(params.flow_rate.value(), 2.2);
    assert_eq!(params.plate_aspect.value(), 2.5);
    assert_eq!(params.cog_dissonance.value(), 0.7);
    let _ = fs::remove_file(path);
}

#[test]
fn invalid_preset_values_are_sanitized_on_load() {
    let json = r#"
    {
        "name": "Broken State",
        "version": "999",
        "object": "Pipe",
        "exciter": 999,
        "size": -100.0,
        "rust": 999.0,
        "damage": -5.0,
        "drive": 999.0,
        "output": 999.0,
        "width": -999.0,
        "body": 999.0,
        "extra": {
            "ui_scale": 99,
            "loop_mode": 99,
            "loop_start_stage": -99,
            "loop_end_stage": 99,
            "space_mode": 99,
            "filter_cutoff": -10.0,
            "analog_ceiling": 9.0,
            "diode_softness": -3.0
        }
    }
    "#;

    let path = unique_temp_path("sanitized_load");
    fs::write(&path, json).unwrap();
    let loaded = Preset::load(&path).unwrap();
    let params = loaded.into_params();

    assert_eq!(params.exciter.value(), 2);
    assert_eq!(params.size.value(), 0.05);
    assert_eq!(params.rust.value(), 5.0);
    assert_eq!(params.damage.value(), 0.0);
    assert_eq!(params.drive.value(), 5.0);
    assert_eq!(params.output.value(), 100.0);
    assert_eq!(params.width.value(), -2.0);
    assert_eq!(params.body.value(), 5.0);
    assert_eq!(params.ui_scale.value(), 4);
    assert_eq!(params.loop_mode.value(), 2);
    assert_eq!(params.loop_start_stage.value(), 0);
    assert_eq!(params.loop_end_stage.value(), 5);
    assert_eq!(params.space_mode.value(), 3);
    assert_eq!(params.filter_cutoff.value(), 20.0);
    assert_eq!(params.analog_ceiling.value(), 1.0);
    assert_eq!(params.diode_softness.value(), 0.0);
    assert_eq!(params.quality_mode.value(), 1);

    let _ = fs::remove_file(path);
}

#[test]
fn quality_mode_preset_roundtrip() {
    let preset = Preset {
        name: "Quality Test".to_string(),
        version: PRESET_VERSION.to_string(),
        object: Object::Plate,
        exciter: 5,
        size: 1.5,
        rust: 0.5,
        damage: 0.5,
        drive: 0.5,
        output: 1.0,
        width: 0.0,
        body: 0.0,
        quality_mode: 0,
        play_mode: 0,
        extra: PresetParameters::default(),
    };

    let path = unique_temp_path("quality_roundtrip");
    preset.save(&path).unwrap();
    let loaded = Preset::load(&path).unwrap();
    assert_eq!(preset.quality_mode, loaded.quality_mode);

    let params = loaded.into_params();
    assert_eq!(params.quality_mode.value(), 0);
    let _ = fs::remove_file(path);
}

#[test]
fn legacy_complex_algo_field_is_ignored_on_load() {
    // The retired `complex_algo` field must load silently from old presets,
    // and the new per-object character fields default when absent.
    let json = r#"
    {
        "name": "Legacy",
        "version": "3",
        "object": "Plate",
        "exciter": 2,
        "size": 1.0,
        "rust": 0.0,
        "damage": 0.0,
        "drive": 0.0,
        "output": 1.0,
        "width": 0.5,
        "body": 0.0,
        "quality_mode": 1,
        "complex_algo": 1
    }
    "#;

    let path = unique_temp_path("legacy_complex_algo");
    fs::write(&path, json).unwrap();
    let loaded = Preset::load(&path).unwrap();
    let params = loaded.into_params();
    // New character params fall back to their defaults.
    assert_eq!(params.plate_aspect.value(), 1.0);
    assert_eq!(params.pipe_diameter.value(), 0.5);
    let _ = fs::remove_file(path);
}

#[test]
fn unstamped_legacy_preset_migrates_to_current_schema() {
    // A preset saved before versioning existed (no `version` field). The
    // migration step must stamp it to the current schema and the typed
    // deserialization must succeed with neutral defaults filled in.
    let json = r#"
    {
        "name": "Unstamped",
        "object": "Pipe",
        "exciter": 2,
        "size": 1.0,
        "rust": 0.0,
        "damage": 0.0,
        "drive": 0.0,
        "output": 1.0,
        "width": 0.5,
        "body": 0.0
    }
    "#;
    let loaded = Preset::from_json_str(json).unwrap();
    let params = loaded.into_params();
    // Macros default to the neutral midpoint.
    assert!((params.macro_mass.value() - 0.5).abs() < 1e-6);
    assert!((params.macro_corrosion.value() - 0.5).abs() < 1e-6);
    assert!((params.macro_violence.value() - 0.5).abs() < 1e-6);
    assert!((params.macro_brightness.value() - 0.5).abs() < 1e-6);
    // Play mode defaults to Tonal (0).
    assert_eq!(params.play_mode.value(), 0);
    // Limiter mode defaults to Hard (0).
    assert_eq!(params.limiter_mode.value(), 0);
}

#[test]
fn v3_preset_migrates_to_v4_without_dropping_fields() {
    // A v3 preset is the most common form on disk today. Loading it through
    // the migration must reach v4 and preserve every v3 field unchanged.
    let json = r#"
    {
        "name": "From V3",
        "version": "3",
        "object": "Tank",
        "exciter": 7,
        "size": 1.5,
        "rust": 0.7,
        "damage": 1.2,
        "drive": 0.3,
        "output": 1.0,
        "width": 0.5,
        "body": 1.0,
        "quality_mode": 2
    }
    "#;
    let loaded = Preset::from_json_str(json).unwrap();
    assert_eq!(loaded.size, 1.5);
    assert_eq!(loaded.rust, 0.7);
    assert_eq!(loaded.damage, 1.2);
    assert_eq!(loaded.quality_mode, 2);
    // New schema-level fields show up at their defaults.
    assert_eq!(loaded.play_mode, 0);
    assert!((loaded.extra.macro_mass - 0.5).abs() < 1e-6);
    assert!((loaded.extra.macro_corrosion - 0.5).abs() < 1e-6);
}

#[test]
fn migration_idempotent_on_current_version() {
    // Calling the migration on a current-version preset must be a no-op
    // (stable shape, stable version stamp). Re-serializing after a round
    // trip must match the original.
    let json = r#"
    {
        "name": "Current",
        "version": "4",
        "object": "Pipe",
        "exciter": 2,
        "size": 1.0,
        "rust": 0.0,
        "damage": 0.0,
        "drive": 0.0,
        "output": 1.0,
        "width": 0.5,
        "body": 0.0,
        "quality_mode": 1,
        "play_mode": 0
    }
    "#;
    let first = Preset::from_json_str(json).unwrap();
    let re_serialized = serde_json::to_string(&first).unwrap();
    let second = Preset::from_json_str(&re_serialized).unwrap();
    assert_eq!(first, second, "migration must be idempotent on v4 input");
}

#[test]
fn preset_without_wander_rate_keeps_its_wander() {
    // Before `position_wander_rate`, Position Envelope set the wander's rate
    // (`0.1 + position_envelope · 4.9` Hz). Such a preset loads that rate,
    // with Position Envelope 0 (`h09-route-envelope-to-strike-position`).
    let json = r#"
    {
        "name": "Old Wander",
        "version": "4",
        "object": "Pipe",
        "exciter": 1,
        "size": 1.0,
        "rust": 0.0,
        "damage": 0.0,
        "drive": 0.0,
        "output": 1.0,
        "width": 0.5,
        "body": 1.0,
        "extra": { "position_wander": 0.3, "position_envelope": 0.5 }
    }
    "#;
    let loaded = Preset::from_json_str(json).unwrap();
    assert!(
        (loaded.extra.position_wander_rate - 2.55).abs() < 1e-5,
        "rate {}",
        loaded.extra.position_wander_rate
    );
    assert_eq!(loaded.extra.position_envelope, 0.0);
    assert_eq!(loaded.extra.position_wander, 0.3);
    let params = loaded.into_params();
    assert!((params.position_wander_rate.value() - 2.55).abs() < 1e-5);
    assert_eq!(params.position_envelope.value(), 0.0);
}

#[test]
fn preset_with_wander_rate_keeps_both_fields() {
    let json = r#"
    {
        "name": "New Wander",
        "version": "4",
        "object": "Pipe",
        "exciter": 1,
        "size": 1.0,
        "rust": 0.0,
        "damage": 0.0,
        "drive": 0.0,
        "output": 1.0,
        "width": 0.5,
        "body": 1.0,
        "extra": { "position_envelope": 0.5, "position_wander_rate": 1.5 }
    }
    "#;
    let loaded = Preset::from_json_str(json).unwrap();
    assert_eq!(loaded.extra.position_wander_rate, 1.5);
    assert_eq!(loaded.extra.position_envelope, 0.5);
}

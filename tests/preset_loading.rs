//! What loading a preset sets (`h04-load-every-preset-field-in-gui`): every
//! parameter except the editor-only ones, at the preset's value, the same as
//! converting the preset to parameters directly.

use corrosion::presets::PRESET_SKIPPED_PARAMS;
use corrosion::{CorrosionParams, Preset};
use nih_plug::prelude::Params;
use std::collections::HashMap;

fn factory_preset(name: &str) -> Preset {
    let path = format!(
        "{}/presets/factory/{name}.corrosion-preset",
        env!("CARGO_MANIFEST_DIR")
    );
    Preset::load(path).expect("load factory preset")
}

/// Plain values of `params`, by ID.
fn plain_values(params: &CorrosionParams) -> HashMap<String, f32> {
    params
        .param_map()
        .into_iter()
        // SAFETY: the pointers come from `params`, borrowed for this call.
        .map(|(id, ptr, _)| (id, unsafe { ptr.unmodulated_plain_value() }))
        .collect()
}

/// The plain value the live parameter `id` would take from `normalized`.
fn live_plain(live: &CorrosionParams, id: &str, normalized: f32) -> f32 {
    let (_, ptr, _) = live
        .param_map()
        .into_iter()
        .find(|(pid, _, _)| pid == id)
        .expect("live parameter");
    // SAFETY: the pointer comes from `live`, borrowed for this call.
    unsafe { ptr.preview_plain(normalized) }
}

#[test]
fn loading_sets_play_mode_limiter_and_macros() {
    // The editor's old hand-written list skipped exactly these six.
    let mut preset = factory_preset("beam_drag");
    preset.play_mode = 1; // Kit
    preset.extra.limiter_mode = 1; // Lookahead
    preset.extra.macro_mass = 0.9;
    preset.extra.macro_corrosion = 0.1;
    preset.extra.macro_violence = 0.8;
    preset.extra.macro_brightness = 0.2;
    let live = CorrosionParams::default();
    let values: HashMap<String, f32> = preset.parameter_values(&live).into_iter().collect();
    for (id, expected) in [
        ("play_mode", 1.0),
        ("limiter_mode", 1.0),
        ("macro_mass", 0.9),
        ("macro_corrosion", 0.1),
        ("macro_violence", 0.8),
        ("macro_brightness", 0.2),
    ] {
        let normalized = *values
            .get(id)
            .unwrap_or_else(|| panic!("{id} is not set by a load"));
        let plain = live_plain(&live, id, normalized);
        assert!(
            (plain - expected).abs() < 1e-4,
            "{id}: {plain}, preset has {expected}"
        );
    }
}

#[test]
fn loading_covers_every_parameter_but_ui_scale() {
    let live = CorrosionParams::default();
    let set: Vec<String> = factory_preset("beam_drag")
        .parameter_values(&live)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let mut expected: Vec<String> = live
        .param_map()
        .into_iter()
        .map(|(id, _, _)| id)
        .filter(|id| !PRESET_SKIPPED_PARAMS.contains(&id.as_str()))
        .collect();
    let mut set_sorted = set.clone();
    set_sorted.sort();
    expected.sort();
    assert_eq!(set_sorted, expected);
    assert!(!set.iter().any(|id| id == "ui_scale"));
}

#[test]
fn loaded_values_match_the_preset_on_every_parameter() {
    // Every factory preset: each value a load sets is the value the preset's
    // own parameter conversion holds.
    let dir = format!("{}/presets/factory", env!("CARGO_MANIFEST_DIR"));
    let live = CorrosionParams::default();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let preset = Preset::load(&path).unwrap();
        let expected = plain_values(&preset.clone().into_params());
        for (id, normalized) in preset.parameter_values(&live) {
            let plain = live_plain(&live, &id, normalized);
            let want = expected[&id];
            assert!(
                (plain - want).abs() <= 1e-4 * want.abs().max(1.0),
                "{}: {id} loads as {plain}, preset holds {want}",
                preset.name
            );
        }
    }
}

//! The user guide's data, exported from the plugin (`add-user-guide`).
//!
//! `export_guide_data` writes [`guide_files`](crate::guide::guide_files) into `web/guide/src/data/`,
//! and `guide_data_equals_the_files_on_disk` fails when the committed files
//! drift from what the plugin would write.

pub mod clip_plan;

use crate::gui::controls::{
    exciter_panel, exciter_param_ref, help_for, resonator_panel, resonator_param_ref,
    selector_help, selector_items, FixedControl, Panel, Shown, FIXED_CONTROLS, LIMITER_MODE_ITEMS,
    PLAY_MODE_ITEMS, QUALITY_MODE_ITEMS, SPACE_MODE_ITEMS,
};
use crate::params::{exciter_model_items, CorrosionParams, ExciterFamily, ExciterType, Object};
use crate::presets::factory;
use crate::randomizer::RandomizerMode;
use nih_plug::prelude::{FloatParam, Param, ParamPtr, Params};
use serde_json::{json, Value};

/// The number of objects (`object` parameter values 0..9).
const OBJECT_COUNT: i32 = 9;

/// Where the parameter reference's data goes, relative to the repository.
pub const PARAMETERS_PATH: &str = "web/guide/src/data/parameters.json";
/// Where the models and modes go, relative to the repository.
pub const MODELS_PATH: &str = "web/guide/src/data/models.json";

/// The parameter reference: every host parameter, in the editor's order
/// (panel by panel; a panel's model slots follow its first section), then any
/// parameter the editor doesn't draw, marked `hidden`.
///
/// Each entry has `id`, `name`, `panel`, `group`, `kind`, `min`, `max`,
/// `default` and `unit` (display strings from the parameter's own
/// formatter), and either `controls` (fixed controls: label, section and when
/// shown, with the ⓘ `help`) or `slots` (model-dependent controls: model,
/// label, section, range and help per model). Selectors list `options`
/// (value, name and, when they differ, help) instead of `min` and `max`.
pub fn parameters_json() -> String {
    let params = CorrosionParams::default();
    let map = params.param_map();
    let ptr_of = |id: &str| -> ParamPtr {
        map.iter()
            .find(|(i, _, _)| i == id)
            .map(|(_, p, _)| *p)
            .unwrap_or_else(|| panic!("`{id}` isn't a host parameter"))
    };
    let id_of_float = |param: &FloatParam| -> String {
        map.iter()
            .find(|(_, p, _)| matches!(p, ParamPtr::FloatParam(f) if std::ptr::eq(*f, param)))
            .map(|(id, _, _)| id.clone())
            .expect("every slot control is a host parameter")
    };

    // Model slots: parameter ID → its slots, in first-seen order.
    let mut exciter_slots: Vec<(String, Vec<Value>)> = Vec::new();
    for &(value, _) in exciter_model_items() {
        let exciter = ExciterType::from_int(value);
        let panel = exciter_panel(exciter);
        for spec in panel.controls {
            let param = exciter_param_ref(&params, spec.id);
            let slot = slot_json(
                exciter.name(),
                panel.title,
                spec.label,
                param,
                spec.min,
                spec.max,
            );
            push_slot(&mut exciter_slots, id_of_float(param), slot);
        }
    }
    let mut object_slots: Vec<(String, Vec<Value>)> = Vec::new();
    for value in 0..OBJECT_COUNT {
        let object = Object::from_int(value);
        let panel = resonator_panel(object);
        for spec in panel.controls {
            let param = resonator_param_ref(&params, spec.id);
            let slot = slot_json(
                object.name(),
                panel.title,
                spec.label,
                param,
                spec.min,
                spec.max,
            );
            push_slot(&mut object_slots, id_of_float(param), slot);
        }
    }

    let mut entries: Vec<Value> = Vec::new();
    let mut done: Vec<String> = Vec::new();
    let panels = [
        Panel::Header,
        Panel::Macros,
        Panel::Exciter,
        Panel::Resonator,
        Panel::Processing,
    ];
    for panel in panels {
        let fixed: Vec<&FixedControl> =
            FIXED_CONTROLS.iter().filter(|c| c.panel == panel).collect();
        let first_group = fixed.first().map(|c| c.group);
        let slots = match panel {
            Panel::Exciter => Some(&exciter_slots),
            Panel::Resonator => Some(&object_slots),
            _ => None,
        };
        let mut slots_written = slots.is_none();
        for (i, control) in fixed.iter().enumerate() {
            if !done.iter().any(|d| d == control.id) {
                let controls: Vec<Value> = FIXED_CONTROLS
                    .iter()
                    .filter(|c| c.id == control.id)
                    .map(fixed_json)
                    .collect();
                let mut entry = common_json(control.id, ptr_of(control.id), panel, control.group);
                entry["controls"] = Value::Array(controls);
                let options = selector_items(control.id);
                if !options.is_empty() {
                    // A selector's options carry their own help; when every
                    // option has the same text it goes on the control.
                    let helps: Vec<&str> = options
                        .iter()
                        .map(|&(_, name)| selector_help(control.label, name))
                        .collect();
                    let shared = helps.iter().all(|h| *h == helps[0]);
                    if shared {
                        for c in entry["controls"].as_array_mut().unwrap() {
                            if c["help"] == "" {
                                c["help"] = Value::from(helps[0]);
                            }
                        }
                    }
                    entry["options"] = options
                        .iter()
                        .zip(&helps)
                        .map(|(&(value, name), help)| {
                            let mut option = json!({ "value": value, "name": name });
                            if !shared {
                                option["help"] = Value::from(*help);
                            }
                            option
                        })
                        .collect();
                    // The range is the options list; the host's min and max
                    // are value indices, not the menu's ends.
                    let object = entry.as_object_mut().unwrap();
                    object.remove("min");
                    object.remove("max");
                }
                entries.push(entry);
                done.push(control.id.to_string());
            }
            let next_group = fixed.get(i + 1).map(|c| c.group);
            if !slots_written && next_group != first_group {
                for (id, slots) in slots.unwrap() {
                    let mut entry = common_json(id, ptr_of(id), panel, "Model controls");
                    entry["slots"] = Value::Array(slots.clone());
                    entries.push(entry);
                    done.push(id.clone());
                }
                slots_written = true;
            }
        }
    }
    for (id, ptr, _) in &map {
        if !done.iter().any(|d| d == id) {
            let mut entry = common_json(id, *ptr, Panel::Header, "");
            entry["panel"] = Value::Null;
            entry["group"] = Value::Null;
            entry["hidden"] = Value::Bool(true);
            entries.push(entry);
        }
    }
    to_file(&json!({ "parameters": entries }))
}

fn push_slot(slots: &mut Vec<(String, Vec<Value>)>, id: String, slot: Value) {
    match slots.iter_mut().find(|(i, _)| *i == id) {
        Some((_, list)) => list.push(slot),
        None => slots.push((id, vec![slot])),
    }
}

/// What every entry has, from the parameter itself.
fn common_json(id: &str, ptr: ParamPtr, panel: Panel, group: &str) -> Value {
    // SAFETY: `ptr` points into a `CorrosionParams` the caller keeps alive.
    let (name, unit, min, max, default, kind) = unsafe {
        (
            ptr.name().to_string(),
            ptr.unit(),
            ptr.normalized_value_to_string(0.0, true),
            ptr.normalized_value_to_string(1.0, true),
            ptr.normalized_value_to_string(ptr.default_normalized_value(), true),
            match ptr {
                ParamPtr::FloatParam(_) => "float",
                ParamPtr::IntParam(_) => "int",
                ParamPtr::BoolParam(_) => "bool",
                ParamPtr::EnumParam(_) => "enum",
            },
        )
    };
    json!({
        "id": id,
        "name": name,
        "panel": panel.key(),
        "group": group,
        "kind": kind,
        "min": min,
        "max": max,
        "default": default,
        "unit": unit.trim(),
    })
}

fn fixed_json(control: &FixedControl) -> Value {
    let shown = match control.shown {
        Shown::Always => "always".to_string(),
        Shown::Family(family) => format!("family:{}", family_key(family)),
        Shown::SpaceMode(mode) => format!(
            "space:{}",
            SPACE_MODE_ITEMS
                .iter()
                .find(|(v, _)| *v == mode)
                .map_or("?", |(_, n)| n)
                .to_ascii_lowercase()
        ),
    };
    json!({
        "label": control.label,
        "group": control.group,
        "shown": shown,
        "help": help_for(control.label),
    })
}

fn slot_json(
    model: &str,
    group: &str,
    label: &str,
    param: &FloatParam,
    min: f32,
    max: f32,
) -> Value {
    let show = |v: f32| param.normalized_value_to_string(param.preview_normalized(v), true);
    json!({
        "model": model,
        "group": group,
        "label": label,
        "min": show(min),
        "max": show(max),
        "help": help_for(label),
    })
}

fn family_key(family: ExciterFamily) -> &'static str {
    match family {
        ExciterFamily::Hit => "hit",
        ExciterFamily::Friction => "friction",
        ExciterFamily::Specialty => "specialty",
    }
}

fn items_json(items: &[(i32, &str)]) -> Value {
    items
        .iter()
        .map(|&(value, name)| json!({ "value": value, "name": name }))
        .collect()
}

/// The models, modes, factory folders (with their presets, in browser order)
/// and drum kits.
pub fn models_json() -> String {
    let exciters: Value = exciter_model_items()
        .iter()
        .map(|&(value, name)| {
            json!({
                "value": value,
                "name": name,
                "family": family_key(ExciterType::from_int(value).family()),
            })
        })
        .collect();
    let objects: Vec<(i32, &str)> = (0..OBJECT_COUNT)
        .map(|v| (v, Object::from_int(v).name()))
        .collect();
    let randomizer: Vec<(i32, &str)> = [
        RandomizerMode::Safe,
        RandomizerMode::Object,
        RandomizerMode::Damage,
        RandomizerMode::Full,
    ]
    .iter()
    .enumerate()
    .map(|(i, m)| (i as i32, m.name()))
    .collect();

    let mut folders: Vec<(String, Vec<Value>)> = Vec::new();
    for entry in factory::bank() {
        let preset = json!({ "name": entry.preset.name, "slug": entry.slug });
        match folders.iter_mut().find(|(f, _)| *f == entry.folder) {
            Some((_, list)) => list.push(preset),
            None => folders.push((entry.folder, vec![preset])),
        }
    }
    let folders: Value = folders
        .into_iter()
        .map(|(folder, presets)| json!({ "folder": folder, "presets": presets }))
        .collect();
    let kits: Value = factory::drum_kits()
        .into_iter()
        .map(|kit| {
            json!({
                "name": kit.preset.name,
                "slug": kit.slug,
                "pads": kit.preset.kit.as_ref().map_or(0, |k| k.pads.len()),
            })
        })
        .collect();

    to_file(&json!({
        "exciters": exciters,
        "objects": items_json(&objects),
        "space_modes": items_json(&SPACE_MODE_ITEMS),
        "quality_modes": items_json(&QUALITY_MODE_ITEMS),
        "note_modes": items_json(&PLAY_MODE_ITEMS),
        "limiter_modes": items_json(&LIMITER_MODE_ITEMS),
        "randomizer_modes": items_json(&randomizer),
        "factory": { "folders": folders, "drum_kits": kits },
        "kit": kit_json(),
    }))
}

/// Kit mode's facts: the drum zone, the largest pad tune, and which
/// parameters a pad stores (every voice-level one) and which stay global.
fn kit_json() -> Value {
    let params = CorrosionParams::default();
    let (global, pad): (Vec<String>, Vec<String>) = params
        .param_map()
        .into_iter()
        .map(|(id, _, _)| id)
        .partition(|id| crate::kit::NON_PAD_PARAMS.contains(&id.as_str()));
    json!({
        "zone": [crate::kit::KIT_ZONE.start(), crate::kit::KIT_ZONE.end()],
        "max_tune": crate::kit::MAX_TUNE,
        "pad_params": pad,
        "global_params": global,
    })
}

/// Pretty-printed with sorted keys (`serde_json`'s map is a `BTreeMap`
/// without `preserve_order`), ending in a newline.
fn to_file(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("JSON values serialize");
    text.push('\n');
    text
}

/// The exciters' (value, name) pairs in editor order, for the binaries.
pub fn exciter_names() -> Vec<(i32, &'static str)> {
    exciter_model_items().to_vec()
}

/// Every file the exporter writes, as (path relative to the repository,
/// contents).
pub fn guide_files() -> Vec<(&'static str, String)> {
    vec![
        (PARAMETERS_PATH, parameters_json()),
        (MODELS_PATH, models_json()),
    ]
}

#[cfg(test)]
mod tests;

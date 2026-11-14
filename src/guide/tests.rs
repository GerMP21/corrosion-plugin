use super::*;
use crate::params::CorrosionParams;
use nih_plug::prelude::Params;
use serde_json::Value;
use std::collections::BTreeSet;

fn parse(json: &str, what: &str) -> Value {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("{what} isn't JSON: {e}"))
}

/// The first place two exports differ: a parameter ID for the reference, a
/// top-level key for the models.
fn first_difference(path: &str, on_disk: &Value, generated: &Value) -> String {
    if path == PARAMETERS_PATH {
        let by_id = |v: &Value| -> Vec<(String, Value)> {
            v["parameters"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|p| (p["id"].as_str().unwrap_or("?").to_string(), p.clone()))
                        .collect()
                })
                .unwrap_or_default()
        };
        let (old, new) = (by_id(on_disk), by_id(generated));
        for (id, param) in &new {
            match old.iter().find(|(o, _)| o == id) {
                None => return format!("parameter `{id}` is missing from the file"),
                Some((_, o)) if o != param => return format!("parameter `{id}` differs"),
                _ => {}
            }
        }
        for (id, _) in &old {
            if !new.iter().any(|(n, _)| n == id) {
                return format!("parameter `{id}` is no longer exported");
            }
        }
        return "the parameters' order differs".to_string();
    }
    if let (Some(old), Some(new)) = (on_disk.as_object(), generated.as_object()) {
        for key in old.keys().chain(new.keys()) {
            if old.get(key) != new.get(key) {
                return format!("`{key}` differs");
            }
        }
    }
    "the formatting differs".to_string()
}

#[test]
fn guide_data_equals_the_files_on_disk() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut stale = Vec::new();
    for (path, generated) in guide_files() {
        match std::fs::read_to_string(root.join(path)) {
            Err(_) => stale.push(format!("{path} is missing")),
            Ok(on_disk) if on_disk != generated => {
                let what = first_difference(
                    path,
                    &parse(&on_disk, path),
                    &parse(&generated, "the export"),
                );
                stale.push(format!("{path} is stale ({what})"));
            }
            Ok(_) => {}
        }
    }
    assert!(
        stale.is_empty(),
        "{}: run `cargo run --bin export_guide_data`",
        stale.join("; ")
    );
}

#[test]
fn guide_data_lists_every_host_parameter() {
    let params = CorrosionParams::default();
    let host: BTreeSet<String> = params
        .param_map()
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();
    let data = parse(&parameters_json(), "the export");
    let exported: BTreeSet<String> = data["parameters"]
        .as_array()
        .expect("a `parameters` array")
        .iter()
        .map(|p| p["id"].as_str().expect("an `id`").to_string())
        .collect();
    let missing: Vec<_> = host.difference(&exported).collect();
    let extra: Vec<_> = exported.difference(&host).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "missing from the export: {missing:?}; not host parameters: {extra:?}"
    );
    assert_eq!(
        data["parameters"].as_array().unwrap().len(),
        host.len(),
        "a parameter is exported twice"
    );
}

#[test]
fn guide_data_lists_every_model_and_mode() {
    let data = parse(&models_json(), "the export");
    let count = |key: &str| data[key].as_array().map_or(0, Vec::len);
    assert_eq!(count("exciters"), 16);
    assert_eq!(count("objects"), 9);
    assert_eq!(count("space_modes"), 4);
    assert_eq!(count("quality_modes"), 4);
    assert_eq!(count("note_modes"), 3);
    assert_eq!(count("limiter_modes"), 2);
    assert_eq!(count("randomizer_modes"), 4);
    let presets: usize = data["factory"]["folders"]
        .as_array()
        .expect("factory folders")
        .iter()
        .map(|f| f["presets"].as_array().map_or(0, Vec::len))
        .sum();
    assert_eq!(presets, crate::presets::factory::bank().len());
    assert_eq!(
        data["factory"]["drum_kits"].as_array().map_or(0, Vec::len),
        crate::presets::factory::drum_kits().len()
    );
}

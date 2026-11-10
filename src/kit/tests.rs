//! `per-key-kits` tests: pad playback, the pad table, persistence and
//! migration.

use std::sync::Arc;

use super::*;
use crate::offline::{OfflineNote, PresetRenderer};
use crate::params::PlayMode;
use crate::presets::Preset;

const SR: u32 = 48_000;

/// Felt Mallet on Tank, with some voice values off their defaults.
fn mallet_on_tank() -> Preset {
    let mut p = crate::presets::factory::base("Mallet Tank");
    p.exciter = 3;
    p.object = Object::Tank;
    p.size = 0.8;
    p.rust = 0.3;
    p.extra.felt_softness = 0.7;
    p.extra.res_damping = 0.6;
    p
}

/// Drumstick on Sheet Metal.
fn stick_on_sheet() -> Preset {
    let mut p = crate::presets::factory::base("Stick Sheet");
    p.exciter = 5;
    p.object = Object::SheetMetal;
    p.size = 1.3;
    p.extra.res_brightness = 0.8;
    p
}

/// The live sound of the kit tests: Bow on Pipe, in Kit mode.
fn bow_on_pipe_kit() -> Preset {
    let mut p = crate::presets::factory::base("Bow Pipe");
    p.exciter = 1;
    p.object = Object::Pipe;
    p.play_mode = PlayMode::Kit.to_int();
    p
}

/// A pad on `key` with `preset`'s sound and `tune`.
fn pad_from(preset: &Preset, key: u8, tune: i32) -> KitPad {
    KitPad::from_params(key, &preset.clone().into_params(), tune, 0.0)
}

/// `preset`'s parameters with `pads` as the kit, the table loaded.
fn kit_params(preset: &Preset, pads: &[KitPad]) -> crate::params::CorrosionParams {
    let params = preset.clone().into_params();
    {
        let mut kit = params.kit.write().unwrap();
        for pad in pads {
            kit.set(pad.clone());
        }
        params.kit_table.load(&kit);
    }
    params
}

fn render(params: crate::params::CorrosionParams, note: u8, frames: usize) -> Vec<(f32, f32)> {
    PresetRenderer::from_params(params)
        .render(&[OfflineNote::on(0, note, 0.8)], SR, frames)
        .pre_limiter
}

#[test]
fn a_pad_plays_its_own_sound() {
    let mallet = mallet_on_tank();
    let stick = stick_on_sheet();
    let kit = kit_params(
        &bow_on_pipe_kit(),
        &[pad_from(&mallet, 36, 0), pad_from(&stick, 38, 0)],
    );
    let kit_36 = render(kit, 36, 24_000);
    let live_36 = render(mallet.into_params(), 36, 24_000);
    assert!(kit_36.iter().any(|&(l, _)| l != 0.0), "pad 36 is silent");
    assert_eq!(kit_36, live_36, "pad 36 differs from the live mallet note");

    let kit = kit_params(
        &bow_on_pipe_kit(),
        &[pad_from(&mallet_on_tank(), 36, 0), pad_from(&stick, 38, 0)],
    );
    assert_eq!(
        render(kit, 38, 24_000),
        render(stick.into_params(), 38, 24_000),
        "pad 38 differs from the live drumstick note"
    );
}

#[test]
fn keys_without_a_pad_are_silent() {
    let kit = kit_params(&bow_on_pipe_kit(), &[pad_from(&mallet_on_tank(), 36, 0)]);
    let mut renderer = PresetRenderer::from_params(kit);
    let out = renderer.render(&[OfflineNote::on(0, 40, 1.0)], SR, 4_800);
    assert!(
        renderer
            .plugin_mut()
            .voice_manager
            .voices_for_test()
            .iter()
            .all(|v| !v.is_rendering()),
        "a voice started on a key without a pad"
    );
    assert!(out.pre_limiter.iter().all(|&(l, r)| l == 0.0 && r == 0.0));
}

/// Spec "The kit zone": host MIDI plays any key that has a pad; the zone
/// and the Full keyboard toggle only lock the editor's strip.
#[test]
fn host_notes_play_pads_outside_the_kit_zone() {
    assert!(!KIT_ZONE.contains(&70));
    let mallet = mallet_on_tank();
    let kit = kit_params(&bow_on_pipe_kit(), &[pad_from(&mallet, 70, 0)]);
    let out = render(kit, 70, 24_000);
    assert!(out.iter().any(|&(l, _)| l != 0.0), "pad 70 is silent");
    assert_eq!(out, render(mallet.into_params(), 70, 24_000));
}

#[test]
fn pads_keep_their_pitch() {
    let mallet = mallet_on_tank();
    let kit = kit_params(&bow_on_pipe_kit(), &[pad_from(&mallet, 42, 24)]);
    assert_eq!(
        render(kit, 42, 24_000),
        render(mallet.into_params(), 66, 24_000),
        "pad 42 at +24 should sound like the live note 66"
    );
}

/// Render `note` in 512-frame blocks, setting Damping to 0.05 after the
/// first block when `turn_damping` is set.
fn render_turning_damping(
    params: crate::params::CorrosionParams,
    note: u8,
    turn_damping: bool,
) -> Vec<(f32, f32)> {
    let mut renderer = PresetRenderer::from_params(params);
    let mut out = renderer
        .render(&[OfflineNote::on(0, note, 0.8)], SR, 512)
        .pre_limiter;
    if turn_damping {
        Arc::get_mut(&mut renderer.plugin_mut().params)
            .expect("unique params")
            .res_damping = crate::params::res_damping_param(0.05);
    }
    out.extend(renderer.render(&[], SR, 24_000).pre_limiter);
    out
}

#[test]
fn live_controls_do_not_change_pad_voices() {
    let kit = || kit_params(&bow_on_pipe_kit(), &[pad_from(&mallet_on_tank(), 36, 0)]);
    assert_eq!(
        render_turning_damping(kit(), 36, true),
        render_turning_damping(kit(), 36, false),
        "turning Damping changed a sounding pad"
    );
    // The same move does reach a live note (the test can see it).
    let live = || mallet_on_tank().into_params();
    assert_ne!(
        render_turning_damping(live(), 36, true),
        render_turning_damping(live(), 36, false),
        "Damping should still reach live notes"
    );
}

// --- The pad table ---------------------------------------------------------

fn marked_pad(mark: f32) -> Pad {
    let mut pad = pad_from(&mallet_on_tank(), 36, 0).build();
    pad.gain = mark;
    pad.setup.size = mark;
    pad.setup.controls.env_attack = mark;
    pad.tune = mark as i32;
    pad
}

#[test]
fn reads_never_see_a_torn_pad() {
    let table = Arc::new(KitTable::new());
    let (a, b) = (marked_pad(1.0), marked_pad(2.0));
    table.set(36, Some(a));
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let writer = {
        let (table, stop) = (Arc::clone(&table), Arc::clone(&stop));
        std::thread::spawn(move || {
            let mut flip = false;
            while !stop.load(Ordering::Relaxed) {
                table.set(36, Some(if flip { a } else { b }));
                flip = !flip;
            }
        })
    };
    let (mut seen, mut missed) = (0usize, 0usize);
    for _ in 0..200_000 {
        match table.get(36) {
            Some(pad) => {
                let mark = pad.gain;
                assert!(mark == 1.0 || mark == 2.0, "unknown pad {mark}");
                assert_eq!(pad.setup.size, mark, "torn pad (size)");
                assert_eq!(pad.setup.controls.env_attack, mark, "torn pad (controls)");
                assert_eq!(pad.tune, mark as i32, "torn pad (tune)");
                seen += 1;
            }
            None => missed += 1,
        }
    }
    stop.store(true, Ordering::Relaxed);
    writer.join().unwrap();
    println!("TABLE reads {seen}, given up {missed}");
    assert!(seen > 0);
}

#[test]
fn load_replaces_every_pad() {
    let table = KitTable::new();
    table.set(50, Some(marked_pad(1.0)));
    let mut state = KitState::default();
    state.set(pad_from(&mallet_on_tank(), 36, 0));
    table.load(&state);
    assert_eq!(table.keys().collect::<Vec<_>>(), vec![36]);
}

// --- Persistence ----------------------------------------------------------

fn three_pad_kit() -> Vec<KitPad> {
    vec![
        pad_from(&mallet_on_tank(), 36, 0),
        pad_from(&stick_on_sheet(), 38, -5),
        pad_from(&stick_on_sheet(), 42, 12),
    ]
}

#[test]
fn preset_round_trip_keeps_the_kit() {
    let params = kit_params(&bow_on_pipe_kit(), &three_pad_kit());
    let preset = Preset::from_params("Kit", &params);
    let json = serde_json::to_string_pretty(&preset).unwrap();
    let loaded = Preset::from_json_str(&json).unwrap();
    let kit = loaded.kit.clone().expect("kit field");
    assert_eq!(
        kit.pads.iter().map(|p| p.key).collect::<Vec<_>>(),
        vec![36, 38, 42]
    );
    assert_eq!(kit, *params.kit.read().unwrap());
    let reloaded = loaded.into_params();
    assert_eq!(
        reloaded.kit_table.keys().collect::<Vec<_>>(),
        vec![36, 38, 42]
    );
    assert_eq!(reloaded.kit_table.get(38).unwrap().tune, -5);
}

#[test]
fn a_preset_without_a_kit_has_no_pads() {
    let preset = Preset::from_params("Plain", &crate::params::CorrosionParams::default());
    let json = serde_json::to_string(&preset).unwrap();
    assert!(!json.contains("\"kit\""), "an empty kit is written: {json}");
    let params = Preset::from_json_str(&json).unwrap().into_params();
    assert!(params.kit.read().unwrap().is_empty());
    assert_eq!(params.kit_table.keys().count(), 0);
}

#[test]
fn project_round_trip_keeps_the_kit() {
    use nih_plug::prelude::Params;
    let params = kit_params(&bow_on_pipe_kit(), &three_pad_kit());
    let fields = params.serialize_fields();
    let restored = crate::params::CorrosionParams::default();
    restored.deserialize_fields(&fields);
    assert_eq!(*restored.kit.read().unwrap(), *params.kit.read().unwrap());
    // The host then reinitialises the plugin, which rebuilds the table.
    let (plugin, _) = crate::host_rate_and_latency_tests::initialized(restored, SR as f32);
    assert_eq!(
        plugin.params.kit_table.keys().collect::<Vec<_>>(),
        vec![36, 38, 42]
    );
}

// --- Migration ------------------------------------------------------------

#[test]
fn an_old_kit_preset_loads_as_tonal() {
    let mut old = crate::presets::factory::base("Old Kit");
    old.play_mode = PlayMode::Kit.to_int();
    old.size = 1.7;
    let mut json = serde_json::to_value(&old).unwrap();
    json.as_object_mut().unwrap().remove("kit");
    let loaded = Preset::from_json_str(&json.to_string()).unwrap();
    assert_eq!(loaded.play_mode, PlayMode::Tonal.to_int());
    let mut expected = old.clone();
    expected.play_mode = PlayMode::Tonal.to_int();
    assert_eq!(loaded, expected, "only Note Mode should change");
}

#[test]
fn an_old_kit_project_loads_as_tonal() {
    use nih_plug::prelude::Plugin;
    use nih_plug::wrapper::state::{ParamValue, PluginState};
    let mut state = PluginState {
        version: String::new(),
        params: [
            ("play_mode".to_string(), ParamValue::I32(1)),
            ("size".to_string(), ParamValue::F32(1.7)),
            ("diode_asymmetry".to_string(), ParamValue::F32(0.25)),
        ]
        .into_iter()
        .collect(),
        fields: Default::default(),
    };
    crate::Corrosion::filter_state(&mut state);
    assert!(matches!(state.params["play_mode"], ParamValue::I32(0)));
    assert!(matches!(state.params["size"], ParamValue::F32(v) if v == 1.7));

    // A project saved with a kit keeps Kit mode.
    state.params.insert("play_mode".into(), ParamValue::I32(1));
    state.fields.insert("kit".into(), "{\"pads\":[]}".into());
    crate::Corrosion::filter_state(&mut state);
    assert!(matches!(state.params["play_mode"], ParamValue::I32(1)));
}

/// Loading a pad into the parameters and storing them again gives the same
/// pad (spec "Selecting a pad loads it": Assign without changes).
#[test]
fn a_pad_round_trips_through_its_parameters() {
    let pad = pad_from(&stick_on_sheet(), 38, -5);
    let again = KitPad::from_params(38, &pad.params(), pad.tune, pad.level_db);
    assert_eq!(again, pad);
}

/// A pad's level scales its output: −36 dB plays about 12 dB under
/// −24 dB. (Both are under the diode stage's knee; a pad near full scale is
/// compressed by the shared post chain, so a level step there moves the
/// output less.)
#[test]
fn a_pads_level_scales_its_output() {
    let rms = |level_db: f32| {
        let mut pad = pad_from(&mallet_on_tank(), 36, 0);
        pad.level_db = level_db;
        let out = render(kit_params(&bow_on_pipe_kit(), &[pad]), 36, 24_000);
        (out.iter()
            .map(|&(l, r)| (l * l + r * r) as f64)
            .sum::<f64>()
            / out.len() as f64)
            .sqrt()
    };
    let drop_db = 20.0 * (rms(-24.0) / rms(-36.0)).log10();
    assert!(
        drop_db > 10.0,
        "12 dB less pad level dropped only {drop_db:.2} dB"
    );
}

// --- Measurements ---------------------------------------------------------

/// Prints the cost of a pad note-on against a live note-on of the same
/// sound, through `handle_note_event` (median of 7 interleaved runs).
#[test]
#[ignore]
fn print_pad_note_on_cost() {
    use nih_plug::prelude::NoteEvent;
    let mallet = mallet_on_tank();
    let (mut live, _) =
        crate::host_rate_and_latency_tests::initialized(mallet.clone().into_params(), SR as f32);
    let mut kit_preset = mallet.clone();
    kit_preset.play_mode = PlayMode::Kit.to_int();
    let (mut pad, _) = crate::host_rate_and_latency_tests::initialized(
        kit_params(&kit_preset, &[pad_from(&mallet, 36, 0)]),
        SR as f32,
    );
    let event = NoteEvent::NoteOn {
        timing: 0,
        voice_id: None,
        channel: 0,
        note: 36,
        velocity: 0.8,
    };
    const N: u32 = 2_000;
    let time = |plugin: &mut crate::Corrosion| {
        let start = std::time::Instant::now();
        for _ in 0..N {
            crate::handle_note_event(plugin, event);
        }
        start.elapsed().as_secs_f64() * 1e6 / f64::from(N)
    };
    let (mut l, mut p) = (Vec::new(), Vec::new());
    for _ in 0..7 {
        l.push(time(&mut live));
        p.push(time(&mut pad));
    }
    l.sort_by(f64::total_cmp);
    p.sort_by(f64::total_cmp);
    println!(
        "NOTE-ON live {:.2} µs, pad {:.2} µs, ratio {:.3}",
        l[3],
        p[3],
        p[3] / l[3]
    );
}

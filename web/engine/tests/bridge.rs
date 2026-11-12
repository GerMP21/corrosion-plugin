//! The editor's changes reach the engine through the bridge, and the
//! engine's report reaches the editor (`add-web-app` task 2.1).

use std::sync::Arc;

use corrosion::dsp::DiodeMeter;
use corrosion::gui_notes::{EditorNoteEvent, EditorNotes};
use corrosion::web::protocol::{decode, Command, Report};
use corrosion::web::{EditorBridge, WebEngine};
use corrosion::{CorrosionParams, Preset};
use nih_plug::prelude::Param;

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

fn render(engine: &mut WebEngine, blocks: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for _ in 0..blocks {
        engine.process(128);
        out.extend_from_slice(&engine.left()[..128]);
    }
    out
}

struct Page {
    bridge: EditorBridge,
    notes: Arc<EditorNotes>,
    engine: WebEngine,
}

impl Page {
    fn new() -> Self {
        let notes = Arc::new(EditorNotes::new());
        let bridge = EditorBridge::new(
            Arc::new(CorrosionParams::default()),
            notes.clone(),
            Arc::new(DiodeMeter::new()),
        );
        Self {
            bridge,
            notes,
            engine: WebEngine::new(48_000.0),
        }
    }

    /// One editor frame: post its batch to the engine.
    fn sync(&mut self) {
        let batch = self.bridge.take_batch();
        self.engine.apply(&batch);
    }
}

#[test]
fn commands_round_trip() {
    let commands = [
        Command::Param {
            index: 7,
            normalized: 0.25,
        },
        Command::Midi([0x90, 60, 100]),
        Command::EditorNoteOn {
            note: 61,
            velocity: 90,
        },
        Command::EditorNoteOff { note: 61 },
        Command::ReleaseEditorNotes,
        Command::Tempo(120.0),
        Command::Kit("{\"pads\":[]}"),
    ];
    let mut batch = Vec::new();
    for command in &commands {
        command.encode(&mut batch);
    }
    assert_eq!(decode(&batch).collect::<Vec<_>>(), commands);
    // A truncated batch stops cleanly.
    assert_eq!(
        decode(&batch[..batch.len() - 3]).count(),
        commands.len() - 1
    );
}

#[test]
fn a_parameter_set_in_the_editor_reaches_the_engine() {
    let mut page = Page::new();
    let params = page.bridge.params().clone();
    {
        let setter = page.bridge.setter();
        setter.begin_set_parameter(&params.output);
        setter.set_parameter_normalized(&params.output, 0.1);
        setter.end_set_parameter(&params.output);
    }
    // The editor's mirror moved (to 0.1 within the dB round trip)...
    let mirror = params.output.unmodulated_normalized_value();
    assert!((mirror - 0.1).abs() < 1e-5, "mirror {mirror}");
    page.sync();
    // ...and the engine's parameter took the same value.
    let index = page.engine.param_index("output").unwrap();
    let engine = page.engine.param(index).unwrap();
    assert!(
        (engine - mirror).abs() < 1e-6,
        "engine {engine}, mirror {mirror}"
    );
}

#[test]
fn a_note_from_the_editor_keyboard_plays_and_stops() {
    let mut page = Page::new();
    assert!(page.notes.try_push(EditorNoteEvent::On {
        note: 60,
        velocity: 100
    }));
    page.sync();
    let played = rms(&render(&mut page.engine, 100));
    assert!(played > 1e-3, "played {played}");
    assert_eq!(page.engine.active_voices(), 1);
    // The engine's report lights the key on the editor's strip.
    let report = page.engine.report().encode();
    page.bridge.apply_report(&report);
    assert!(page.notes.held().contains(60));
    assert_eq!(page.bridge.voices(), 1);
    page.notes.request_release_all();
    page.sync();
    render(&mut page.engine, 2);
    let report = page.engine.report().encode();
    page.bridge.apply_report(&report);
    assert!(!page.notes.held().contains(60));
}

#[test]
fn a_kit_edited_in_the_editor_reaches_the_engine() {
    let mut page = Page::new();
    let params = page.bridge.params().clone();
    let pad = corrosion::kit::KitPad::from_params(48, &params, 0, 0.0);
    params.kit_table.set(48, Some(pad.build()));
    params.kit.write().unwrap().set(pad);
    page.sync();
    // Kit mode: the pad's key plays, a key without a pad doesn't.
    let play_mode = page.engine.param_index("play_mode").unwrap();
    page.engine.set_param(play_mode, 0.5);
    page.engine.midi(0x90, 50, 100);
    assert_eq!(rms(&render(&mut page.engine, 50)), 0.0);
    page.engine.midi(0x90, 48, 100);
    assert!(rms(&render(&mut page.engine, 100)) > 1e-3);
}

#[test]
fn a_preset_loaded_in_the_editor_reaches_the_engine() {
    let mut page = Page::new();
    let mut preset = Preset::from_params("quiet", &CorrosionParams::default());
    preset.output = -40.0;
    page.bridge.load_preset(&preset);
    page.sync();
    let index = page.engine.param_index("output").unwrap();
    let expected = page.bridge.params().output.unmodulated_normalized_value();
    assert!(expected < 0.5);
    let engine = page.engine.param(index).unwrap();
    assert!(
        (engine - expected).abs() < 1e-6,
        "engine {engine}, editor {expected}"
    );
}

#[test]
fn the_report_carries_the_diode_meter() {
    let report = Report {
        diode_reduction_db: 3.5,
        voices: 4,
        held: Default::default(),
    };
    let mut page = Page::new();
    page.bridge.apply_report(&report.encode());
    assert_eq!(page.bridge.voices(), 4);
    assert_eq!(Report::decode(&report.encode()), Some(report));
}

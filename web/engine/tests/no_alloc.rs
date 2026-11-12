//! Real-time safety of the web engine (`add-web-app` task 2.2): a block,
//! and applying parameter, MIDI, editor-note, release and tempo commands,
//! never allocate. A kit command rebuilds the pad table and may allocate,
//! as a state restore does in the plugin.
//!
//! Debug only: NIH-plug installs the checking allocator under
//! `debug_assertions` + `assert_process_allocs`. A violation aborts the test
//! process. Run with `cargo test -p corrosion-web-engine --test no_alloc`.

use assert_no_alloc::assert_no_alloc;
use corrosion::web::protocol::Command;
use corrosion::web::WebEngine;

#[test]
fn commands_and_blocks_do_not_allocate() {
    let mut engine = WebEngine::new(48_000.0);
    let output = engine.param_index("output").unwrap() as u32;
    let mut batch = Vec::new();
    for command in [
        Command::Param {
            index: output,
            normalized: 0.4,
        },
        Command::Midi([0x90, 60, 100]),
        Command::Midi([0xB0, 1, 64]),
        Command::Midi([0xE0, 0, 80]),
        Command::EditorNoteOn {
            note: 64,
            velocity: 90,
        },
        Command::EditorNoteOff { note: 64 },
        Command::ReleaseEditorNotes,
        Command::Tempo(128.0),
    ] {
        command.encode(&mut batch);
    }
    // Warm up: the first block after construction is ordinary too.
    engine.process(128);
    assert_no_alloc(|| {
        for _ in 0..50 {
            engine.apply(&batch);
            engine.process(128);
            let _ = engine.report();
        }
    });
    assert!(engine.active_voices() > 0);
}

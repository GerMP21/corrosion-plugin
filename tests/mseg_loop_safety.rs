//! Regression guard for review finding C2: a friction voice whose MSEG loop
//! starts at Sustain used to recurse forever on the audio thread (stack
//! overflow in debug, endless loop in release). Play it through the voice
//! manager the way the plugin does and require finite output in bounded time.

use corrosion::dsp::ModalProfileId;
use corrosion::voice::manager::VoiceManager;
use corrosion::voice::VoiceControls;
use std::sync::mpsc;
use std::time::Duration;

/// Bow, the friction-family exciter whose force follows the MSEG.
const BOW: i32 = 1;

fn play_bow_note(loop_start_stage: i32, loop_end_stage: i32) -> bool {
    let sample_rate = 48_000u32;
    let mut manager = VoiceManager::new();
    let controls = VoiceControls {
        loop_mode: 1, // Forward
        loop_start_stage,
        loop_end_stage,
        ..VoiceControls::default()
    };
    manager.note_on_with_controls(
        60,
        100.0,
        ModalProfileId::Pipe,
        1.0,
        0.0,
        0.0,
        BOW,
        controls,
    );
    (0..sample_rate * 2).all(|_| manager.process_sample(sample_rate).is_finite())
}

#[test]
fn bow_note_with_loop_starting_at_sustain_renders() {
    for (start, end) in [(4, 4), (5, 5), (4, 5)] {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(play_bow_note(start, end));
        });
        let finite = rx
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|_| {
                panic!("loop {start}->{end}: 2 s of audio did not finish (hang or crash)")
            });
        assert!(finite, "loop {start}->{end}: non-finite output");
    }
}

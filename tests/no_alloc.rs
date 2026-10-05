//! Real-time safety: nothing on the audio thread may allocate or free heap
//! memory (review finding C3).
//!
//! The check only runs in **debug** builds: nih-plug installs the checking
//! allocator under `debug_assertions` + `assert_process_allocs`, and
//! `assert_no_alloc` compiles to a no-op in release by default. Run this file
//! with `cargo test --no-default-features --test no_alloc` (no `--release`).
//! A violation aborts the whole test process rather than failing one test.

use assert_no_alloc::assert_no_alloc;
use corrosion::dsp::ModalProfileId;
use corrosion::voice::manager::VoiceManager;
use corrosion::voice::{VoiceControls, MAX_VOICES};

const SAMPLE_RATE: u32 = 48_000;

const PROFILES: [ModalProfileId; 9] = [
    ModalProfileId::Pipe,
    ModalProfileId::Plate,
    ModalProfileId::Tank,
    ModalProfileId::Chain,
    ModalProfileId::IBeam,
    ModalProfileId::TautCable,
    ModalProfileId::CoilSpring,
    ModalProfileId::SheetMetal,
    ModalProfileId::IndustrialCog,
];

/// QualityMode Eco / Normal / High / Render mode-count scales.
const QUALITY_SCALES: [f32; 4] = [0.5, 1.0, 1.5, 2.0];

/// Exciter selectors 0..=15 cover the hit, friction and specialty families.
const EXCITERS: std::ops::RangeInclusive<i32> = 0..=15;

fn controls(mode_count_scale: f32) -> VoiceControls {
    VoiceControls {
        mode_count_scale,
        ..VoiceControls::default()
    }
}

fn run(manager: &mut VoiceManager, samples: usize) {
    for _ in 0..samples {
        let _ = manager.process_sample(SAMPLE_RATE);
    }
}

#[test]
fn process_callback_is_alloc_free() {
    let total_samples = 480_000; // 10 seconds at 48kHz
    let mut manager = VoiceManager::new();

    // Trigger 8 voices before entering the no-alloc zone.
    // The process callback itself is what must stay allocation-free.
    for i in 0..MAX_VOICES {
        manager.note_on(60 + i as u8, 100.0, ModalProfileId::Pipe, 1.0, 0.0, 0.0, 2);
    }

    assert_no_alloc(|| {
        run(&mut manager, total_samples);
    });
}

#[test]
fn note_on_is_alloc_free_for_every_object_and_quality() {
    let mut manager = VoiceManager::new();
    assert_no_alloc(|| {
        for profile in PROFILES {
            for scale in QUALITY_SCALES {
                for damage in [0.0, 1.0] {
                    manager.note_on_with_controls(
                        60,
                        100.0,
                        profile,
                        1.0,
                        0.3,
                        damage,
                        2,
                        controls(scale),
                    );
                    run(&mut manager, 64);
                }
            }
        }
    });
}

#[test]
fn note_on_is_alloc_free_for_every_exciter() {
    let mut manager = VoiceManager::new();
    assert_no_alloc(|| {
        for exciter in EXCITERS {
            manager.note_on_with_controls(
                48 + exciter as u8,
                100.0,
                ModalProfileId::Plate,
                1.0,
                0.0,
                0.5,
                exciter,
                controls(2.0),
            );
            // 0.1 s per exciter so streaming exciters (particle rain, brush,
            // chain) reach their steady state inside the check.
            run(&mut manager, 4_800);
        }
    });
}

#[test]
fn note_off_and_tail_are_alloc_free() {
    let mut manager = VoiceManager::new();
    for i in 0..4 {
        manager.note_on_with_controls(
            60 + i,
            100.0,
            ModalProfileId::Tank,
            1.0,
            0.0,
            0.0,
            1,
            controls(1.0),
        );
    }
    run(&mut manager, 4_800);
    assert_no_alloc(|| {
        for i in 0..4 {
            manager.note_off(60 + i);
        }
        // Long enough for the released voices to decay and go idle.
        run(&mut manager, 480_000);
    });
}

#[test]
fn voice_stealing_is_alloc_free() {
    let mut manager = VoiceManager::new();
    for i in 0..MAX_VOICES {
        manager.note_on_with_controls(
            40 + i as u8,
            100.0,
            ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
            controls(0.5),
        );
    }
    run(&mut manager, 256);
    assert_no_alloc(|| {
        // Every new note must steal; a larger object at Render quality forces
        // the stolen voice's bank to grow.
        for i in 0..MAX_VOICES {
            manager.note_on_with_controls(
                80 + i as u8,
                100.0,
                ModalProfileId::IndustrialCog,
                1.0,
                0.0,
                1.0,
                1,
                controls(2.0),
            );
            run(&mut manager, 64);
        }
    });
}

#[test]
fn live_expression_is_alloc_free() {
    let mut manager = VoiceManager::new();
    manager.note_on_with_controls(
        57,
        100.0,
        ModalProfileId::TautCable,
        1.0,
        0.0,
        0.0,
        2,
        controls(1.0),
    );
    assert_no_alloc(|| {
        for step in 0..=48 {
            let bend = -24.0 + step as f32;
            manager.set_pitch_bend_semitones(bend);
            manager.set_channel_pressure(step as f32 / 48.0);
            manager.set_poly_pressure(57, 1.0 - step as f32 / 48.0);
            manager.set_mod_wheel(step as f32 / 48.0);
            manager.update_live_controls(0.3, 0.7, 0.4, 0.5, 0.2, 0.1, 0.3, SAMPLE_RATE);
            run(&mut manager, 128);
        }
    });
}

#[test]
fn plugin_reset_is_alloc_free() {
    use nih_plug::prelude::Plugin;

    let mut plugin = corrosion::Corrosion::default();
    assert_no_alloc(|| {
        plugin.reset();
    });
}

#[test]
fn reset_with_sounding_voices_is_alloc_free_and_silent() {
    let mut manager = VoiceManager::new();
    for i in 0..MAX_VOICES {
        manager.note_on_with_controls(
            48 + i as u8,
            110.0,
            PROFILES[i % PROFILES.len()],
            1.0,
            0.2,
            0.5,
            i as i32,
            controls(1.0),
        );
    }
    run(&mut manager, 2_400);
    let mut after_reset = Vec::with_capacity(4_800);
    assert_no_alloc(|| {
        manager.reset();
        for _ in 0..4_800 {
            after_reset.push(manager.process_sample(SAMPLE_RATE));
        }
    });
    assert!(
        after_reset.iter().all(|&s| s == 0.0),
        "output must be silent after reset"
    );
}

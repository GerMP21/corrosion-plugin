use corrosion::dsp::{
    CharacterParams, ModalProfileId, ModalResonator, PostProcessingChain, SpaceMode, WireBrush,
};
use corrosion::offline::{OfflineNote, OfflineRenderer, PresetRenderer, RenderConfig};
use corrosion::presets::Preset;
use corrosion::voice::manager::VoiceManager;
use corrosion::voice::VoiceControls;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

/// Benchmark one active voice through the mono voice manager path.
fn bench_single_voice_manager(c: &mut Criterion) {
    c.bench_function("voice_manager_single_voice", |b| {
        b.iter(|| {
            let mut manager = VoiceManager::new();
            manager.note_on(black_box(60), 100.0, ModalProfileId::Pipe, 1.0, 0.0, 0.0, 2);

            let mut sum = 0.0_f32;
            for _ in 0..4_096 {
                sum += manager.process_sample(48_000);
            }

            black_box(sum)
        })
    });
}

/// Benchmark eight active voices (half the 16-voice pool).
fn bench_eight_voice_manager(c: &mut Criterion) {
    c.bench_function("voice_manager_eight_voices", |b| {
        b.iter(|| {
            let mut manager = VoiceManager::new();
            for note in 60..68 {
                manager.note_on(note, 110.0, ModalProfileId::Pipe, 1.0, 0.1, 0.1, 2);
            }

            let mut sum = 0.0_f32;
            for _ in 0..4_096 {
                sum += manager.process_sample(48_000);
            }

            black_box(sum)
        })
    });
}

/// Benchmark a full 16-voice pool (Normal quality) through the stereo path.
fn bench_sixteen_voice_manager(c: &mut Criterion) {
    c.bench_function("voice_manager_sixteen_voices", |b| {
        b.iter(|| {
            let mut manager = VoiceManager::new();
            for note in 52..68 {
                manager.note_on(note, 110.0, ModalProfileId::Pipe, 1.0, 0.1, 0.1, 2);
            }

            let mut sum = 0.0_f32;
            for _ in 0..4_096 {
                let (left, right) = manager.process_sample_stereo(48_000, 0.5);
                sum += left + right;
            }

            black_box(sum)
        })
    });
}

/// Worst case: 16 Render-quality (2x modes) damaged Plate voices, stereo.
fn bench_sixteen_voice_manager_render(c: &mut Criterion) {
    c.bench_function("voice_manager_sixteen_voices_render", |b| {
        b.iter(|| {
            let mut manager = VoiceManager::new();
            for note in 52..68 {
                manager.note_on_with_controls(
                    note,
                    110.0,
                    ModalProfileId::Plate,
                    1.0,
                    0.3,
                    1.0,
                    2,
                    VoiceControls {
                        mode_count_scale: 2.0,
                        ..VoiceControls::default()
                    },
                );
            }

            let mut sum = 0.0_f32;
            for _ in 0..4_096 {
                let (left, right) = manager.process_sample_stereo(48_000, 0.5);
                sum += left + right;
            }

            black_box(sum)
        })
    });
}

/// The Render benchmark's 16 damaged Plate voices, held with a +2-semitone
/// pitch bend. Its plain twin is `voice_manager_sixteen_voices_render`
/// (`h12-skip-redundant-mode-retunes`).
fn bench_sixteen_voice_manager_bent(c: &mut Criterion) {
    c.bench_function("voice_manager_sixteen_voices_bent", |b| {
        b.iter(|| {
            let mut manager = VoiceManager::new();
            for note in 52..68 {
                manager.note_on_with_controls(
                    note,
                    110.0,
                    ModalProfileId::Plate,
                    1.0,
                    0.3,
                    1.0,
                    2,
                    VoiceControls {
                        mode_count_scale: 2.0,
                        ..VoiceControls::default()
                    },
                );
            }
            manager.set_pitch_bend_semitones(2.0);

            let mut sum = 0.0_f32;
            for _ in 0..4_096 {
                let (left, right) = manager.process_sample_stereo(48_000, 0.5);
                sum += left + right;
            }

            black_box(sum)
        })
    });
}

/// 16 ringing TautCable voices at Render with Cable Tension Drop at
/// `tension_drop`: 0.4 (the default) moves every mode's pitch with the
/// ring's amplitude each sample; 0 holds it still.
fn sixteen_cable_voices(tension_drop: f32) -> f32 {
    let mut manager = VoiceManager::new();
    for note in 52..68 {
        manager.note_on_with_controls(
            note,
            110.0,
            ModalProfileId::TautCable,
            1.0,
            0.0,
            0.0,
            2,
            VoiceControls {
                mode_count_scale: 2.0,
                character: CharacterParams {
                    cable_tension_drop: tension_drop,
                    ..CharacterParams::default()
                },
                ..VoiceControls::default()
            },
        );
    }
    let mut sum = 0.0_f32;
    for _ in 0..4_096 {
        let (left, right) = manager.process_sample_stereo(48_000, 0.5);
        sum += left + right;
    }
    sum
}

/// TautCable with its tension-drop dynamics, next to its static twin
/// (`h12-skip-redundant-mode-retunes`).
fn bench_sixteen_voice_manager_cable_dynamics(c: &mut Criterion) {
    c.bench_function("voice_manager_sixteen_voices_cable_dynamics", |b| {
        b.iter(|| black_box(sixteen_cable_voices(0.4)))
    });
    c.bench_function("voice_manager_sixteen_voices_cable_static", |b| {
        b.iter(|| black_box(sixteen_cable_voices(0.0)))
    });
}

/// Benchmark the stochastic wire-brush exciter in a dense trigger scenario.
fn bench_wire_brush(c: &mut Criterion) {
    c.bench_function("wire_brush_dense_cluster", |b| {
        b.iter(|| {
            let mut brush = WireBrush::new();
            brush.set_sample_rate(48_000.0);
            brush.set_parameters(512, 150.0, 0.9, 0.8);
            brush.trigger(0.95);

            let mut sum = 0.0_f32;
            for _ in 0..8_192 {
                sum += brush.process_sample(0.0, 0.0);
            }

            black_box(sum)
        })
    });
}

/// Benchmark the full post-processing chain on a stereo synthetic input stream.
fn bench_post_chain(c: &mut Criterion) {
    c.bench_function("post_processing_chain", |b| {
        b.iter(|| {
            let mut chain = PostProcessingChain::new();
            chain.set_sample_rate(48_000.0);
            chain.set_filter_params(8_000.0, 0.35, 0.05);
            chain.set_drive_params(1.5, 0.2, 0.15);
            chain.set_body_params(0.4, 0.6);
            chain.set_spread_params(0.6, 0.5);
            chain.set_space_mode(SpaceMode::Factory);
            chain.set_space_amount(0.25);
            chain.set_factory_params(0.5, 0.4, 0.5);
            chain.set_clipper_params(0.9661, 0.5);

            let mut last = (0.0_f32, 0.0_f32);
            for i in 0..4_096 {
                let input = ((i as f32) * 0.013).sin() * 0.5;
                last = chain.process(input, input * 0.8);
            }

            black_box(last)
        })
    });
}

/// Benchmark post-processing chain in Eco quality mode.
fn bench_post_chain_eco(c: &mut Criterion) {
    c.bench_function("post_processing_chain_eco", |b| {
        b.iter(|| {
            let mut chain = PostProcessingChain::new();
            chain.set_sample_rate(48_000.0);
            chain.set_quality_mode(corrosion::dsp::PostQualityMode::Eco);
            chain.set_filter_params(8_000.0, 0.35, 0.05);
            chain.set_drive_params(1.5, 0.2, 0.15);
            chain.set_body_params(0.4, 0.6);
            chain.set_spread_params(0.6, 0.5);
            chain.set_space_mode(SpaceMode::Factory);
            chain.set_space_amount(0.25);
            chain.set_factory_params(0.5, 0.4, 0.5);
            chain.set_clipper_params(0.9661, 0.5);

            let mut last = (0.0_f32, 0.0_f32);
            for i in 0..4_096 {
                let input = ((i as f32) * 0.013).sin() * 0.5;
                last = chain.process(input, input * 0.8);
            }

            black_box(last)
        })
    });
}

/// Benchmark the deterministic offline resonator renderer used by QA.
fn bench_offline_renderer(c: &mut Criterion) {
    c.bench_function("offline_renderer_pipe_family", |b| {
        b.iter(|| {
            let renderer = OfflineRenderer::new(RenderConfig {
                sample_rate: 48_000,
                frame_count: 48_000,
                excitation_frame: 0,
                excitation_amplitude: 1.0,
            });
            let resonator = ModalResonator::with_profile(ModalProfileId::Pipe);
            let (_output, summary) = renderer.render(resonator);
            black_box(summary)
        })
    });
}

/// Benchmark the voice manager with no active voices to measure idle overhead.
fn bench_idle_voice_manager(c: &mut Criterion) {
    c.bench_function("voice_manager_idle", |b| {
        b.iter(|| {
            let mut manager = VoiceManager::new();
            let mut sum = 0.0_f32;
            for _ in 0..4_096 {
                sum += manager.process_sample(48_000);
            }
            black_box(sum)
        })
    });
}

/// Benchmark the post-processing chain with zero input to measure idle overhead.
fn bench_idle_post_chain(c: &mut Criterion) {
    c.bench_function("post_processing_chain_idle", |b| {
        b.iter(|| {
            let mut chain = PostProcessingChain::new();
            chain.set_sample_rate(48_000.0);
            chain.set_filter_params(20_000.0, 0.0, 0.0);
            chain.set_drive_params(0.0, 0.0, 0.0);
            chain.set_body_params(0.0, 0.0);
            chain.set_spread_params(0.0, 0.0);
            chain.set_space_mode(SpaceMode::Off);
            chain.set_space_amount(0.0);
            chain.set_clipper_params(0.9661, 0.5);

            let mut last = (0.0_f32, 0.0_f32);
            for _ in 0..4_096 {
                last = chain.process(0.0, 0.0);
            }
            black_box(last)
        })
    });
}

/// The plugin's whole output path (`Corrosion::process_block`: voices,
/// master drive, post chain, output gain, limiter) for 16 notes of a preset
/// with Space, BODY and master drive, 4 096 samples in 512-sample blocks.
/// The plugin is built outside the timed part (H13).
fn bench_plugin_output_path(c: &mut Criterion) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/presets/factory/sheet_lightning.corrosion-preset"
    );
    let preset = Preset::load(std::path::Path::new(path)).expect("load preset");
    let notes: Vec<OfflineNote> = (52..68)
        .map(|note| OfflineNote::on(0, note, 0.85))
        .collect();
    c.bench_function("plugin_output_path_sixteen_voices", |b| {
        b.iter_batched(
            || PresetRenderer::new(&preset),
            |mut renderer| black_box(renderer.render(&notes, 48_000, 4_096)),
            BatchSize::LargeInput,
        )
    });
    // The same in Render quality (more modes per voice, 16× clipper).
    let mut render = preset.clone();
    render.quality_mode = 3;
    c.bench_function("plugin_output_path_sixteen_voices_render", |b| {
        b.iter_batched(
            || PresetRenderer::new(&render),
            |mut renderer| black_box(renderer.render(&notes, 48_000, 4_096)),
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(
    benches,
    bench_single_voice_manager,
    bench_eight_voice_manager,
    bench_sixteen_voice_manager,
    bench_sixteen_voice_manager_render,
    bench_sixteen_voice_manager_bent,
    bench_sixteen_voice_manager_cable_dynamics,
    bench_wire_brush,
    bench_post_chain,
    bench_post_chain_eco,
    bench_offline_renderer,
    bench_idle_voice_manager,
    bench_plugin_output_path,
    bench_idle_post_chain
);
criterion_main!(benches);

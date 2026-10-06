use corrosion::dsp::{ModalProfileId, ModalResonator, PostProcessingChain, SpaceMode, WireBrush};
use corrosion::offline::{OfflineRenderer, RenderConfig};
use corrosion::voice::manager::VoiceManager;
use corrosion::voice::VoiceControls;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

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

/// Benchmark the eight-voice worst-case polyphonic manager path.
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

criterion_group!(
    benches,
    bench_single_voice_manager,
    bench_eight_voice_manager,
    bench_sixteen_voice_manager,
    bench_sixteen_voice_manager_render,
    bench_wire_brush,
    bench_post_chain,
    bench_post_chain_eco,
    bench_offline_renderer,
    bench_idle_voice_manager,
    bench_idle_post_chain
);
criterion_main!(benches);

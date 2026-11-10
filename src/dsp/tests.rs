use super::{
    offline_peak_shared_mode_limit, realtime_mode_count_estimate, safe_realtime_shared_mode_limit,
    DamageAmount, ExcitationInput, ModalModeSpec, ModalProfile, ModalProfileId, ModalResonator,
    RealtimeModeCountEstimate, RustAmount, SizeScale, PIPE_MODAL_PROFILE_MODES,
    PLATE_MODAL_PROFILE_MODES, TANK_MODAL_PROFILE_MODES,
};
use crate::offline::{
    render_behavior_metrics, write_wav_i16, ComparisonRenderSpec, DamageVariationRenderSpec,
    OfflineRenderer, RenderConfig, RustVariationRenderSpec, DAMAGE_VARIATION_SPECS,
    FAMILY_COMPARISON_SPECS, RUST_VARIATION_SPECS,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct ProfileMetrics {
    weighted_frequency: f32,
    average_decay: f32,
    low_band_gain_ratio: f32,
    harmonic_deviation: f32,
}

fn approx_eq(left: f32, right: f32, epsilon: f32) {
    assert!(
        (left - right).abs() <= epsilon,
        "left={left} right={right} epsilon={epsilon}"
    );
}

fn profile_metrics(profile: ModalProfile) -> ProfileMetrics {
    profile_metrics_for_modes(profile.modes)
}

fn profile_metrics_for_modes(modes: &[ModalModeSpec]) -> ProfileMetrics {
    let weighted_frequency_sum = modes
        .iter()
        .map(|mode| mode.frequency_hz * mode.gain)
        .sum::<f32>();
    let total_gain = modes.iter().map(|mode| mode.gain).sum::<f32>();
    let average_decay =
        modes.iter().map(|mode| mode.decay_seconds).sum::<f32>() / modes.len() as f32;
    let low_band_gain_ratio = modes.iter().take(3).map(|mode| mode.gain).sum::<f32>() / total_gain;
    let fundamental = modes[0].frequency_hz;
    let harmonic_deviation = modes
        .iter()
        .enumerate()
        .skip(1)
        .map(|(index, mode)| {
            let harmonic_number = (index + 1) as f32;
            ((mode.frequency_hz / fundamental) - harmonic_number).abs()
        })
        .sum::<f32>()
        / (modes.len() - 1) as f32;

    ProfileMetrics {
        weighted_frequency: weighted_frequency_sum / total_gain,
        average_decay,
        low_band_gain_ratio,
        harmonic_deviation,
    }
}

fn unique_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("corrosion_{name}_{nanos}"))
}

fn assert_size_scaling_direction(profile_id: ModalProfileId) {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8_192,
        ..RenderConfig::default()
    });
    let base_profile = ModalProfile::from_id(profile_id);
    let smaller_scale = SizeScale::new(0.7);
    let larger_scale = SizeScale::new(1.6);

    let smaller_profile_metrics = profile_metrics_for_modes(&base_profile.scaled_mode_specs(
        smaller_scale,
        RustAmount::default(),
        DamageAmount::default(),
    ));
    let larger_profile_metrics = profile_metrics_for_modes(&base_profile.scaled_mode_specs(
        larger_scale,
        RustAmount::default(),
        DamageAmount::default(),
    ));

    let (smaller_output, smaller_summary) = renderer.render(ModalResonator::with_profile_and_size(
        profile_id,
        smaller_scale,
    ));
    let (larger_output, larger_summary) = renderer.render(ModalResonator::with_profile_and_size(
        profile_id,
        larger_scale,
    ));

    let smaller_render_metrics = render_behavior_metrics(&smaller_output);
    let larger_render_metrics = render_behavior_metrics(&larger_output);

    assert!(smaller_profile_metrics.weighted_frequency > larger_profile_metrics.weighted_frequency);
    assert!(larger_profile_metrics.average_decay > smaller_profile_metrics.average_decay);
    assert!(
        larger_profile_metrics.low_band_gain_ratio > smaller_profile_metrics.low_band_gain_ratio
    );
    approx_eq(
        smaller_profile_metrics.harmonic_deviation,
        larger_profile_metrics.harmonic_deviation,
        1.0e-5,
    );

    assert_ne!(smaller_summary.checksum, larger_summary.checksum);
    assert!(smaller_render_metrics.zero_crossings > larger_render_metrics.zero_crossings);
    assert!(smaller_render_metrics.brightness_proxy > larger_render_metrics.brightness_proxy);
    assert!(
        larger_render_metrics.late_to_early_energy_ratio
            > smaller_render_metrics.late_to_early_energy_ratio
    );
}

#[test]
fn render_is_deterministic_for_same_configuration() {
    let renderer = OfflineRenderer::new(RenderConfig::default());

    let (_, first_summary) = renderer.render(ModalResonator::default());
    let (_, second_summary) = renderer.render(ModalResonator::default());

    assert_eq!(first_summary, second_summary);
}

#[test]
fn deterministic_excitation_is_explicit_and_repeatable() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8,
        excitation_frame: 3,
        excitation_amplitude: 0.75,
        ..RenderConfig::default()
    });

    let first_excitation = renderer.deterministic_excitation();
    let second_excitation = renderer.deterministic_excitation();

    assert_eq!(first_excitation, second_excitation);
    assert_eq!(first_excitation, ExcitationInput::impulse(8, 3, 0.75));
    assert_eq!(
        first_excitation.samples(),
        &[0.0, 0.0, 0.0, 0.75, 0.0, 0.0, 0.0, 0.0],
    );
}

#[test]
fn render_with_explicit_excitation_matches_default_render_path() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 512,
        excitation_frame: 4,
        excitation_amplitude: 0.5,
        ..RenderConfig::default()
    });

    let excitation = renderer.deterministic_excitation();
    let (_, implicit_summary) = renderer.render(ModalResonator::default());
    let (_, explicit_summary) =
        renderer.render_with_excitation(&excitation, &mut ModalResonator::default());

    assert_eq!(implicit_summary, explicit_summary);
}

#[test]
fn family_comparison_specs_cover_pipe_plate_and_tank_in_order() {
    assert_eq!(
        FAMILY_COMPARISON_SPECS,
        [
            ComparisonRenderSpec {
                profile_id: ModalProfileId::Pipe,
                slug: "pipe",
            },
            ComparisonRenderSpec {
                profile_id: ModalProfileId::Plate,
                slug: "plate",
            },
            ComparisonRenderSpec {
                profile_id: ModalProfileId::Tank,
                slug: "tank",
            },
        ]
    );
}

#[test]
fn wav_writer_emits_pcm_header_and_payload() {
    let output_dir = unique_test_dir("wav_writer");
    let wav_path = output_dir.join("test.wav");
    let samples = [1.0_f32, -1.0, 0.0, 0.5];

    write_wav_i16(&wav_path, &samples, 48_000).expect("wav write should succeed");

    let wav_bytes = fs::read(&wav_path).expect("wav should exist");
    assert_eq!(&wav_bytes[0..4], b"RIFF");
    assert_eq!(&wav_bytes[8..12], b"WAVE");
    assert_eq!(&wav_bytes[12..16], b"fmt ");
    assert_eq!(&wav_bytes[36..40], b"data");
    assert_eq!(
        u32::from_le_bytes(wav_bytes[24..28].try_into().unwrap()),
        48_000
    );
    assert_eq!(
        u16::from_le_bytes(wav_bytes[34..36].try_into().unwrap()),
        16
    );
    assert_eq!(u32::from_le_bytes(wav_bytes[40..44].try_into().unwrap()), 8);

    fs::remove_dir_all(output_dir).expect("temp output cleanup should succeed");
}

#[test]
fn family_comparison_render_writes_expected_artifacts() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 4_096,
        ..RenderConfig::default()
    });
    let output_dir = unique_test_dir("family_comparison");

    let artifacts = renderer
        .render_family_comparisons_to_dir(&output_dir)
        .expect("family comparison render should succeed");

    assert_eq!(artifacts.len(), FAMILY_COMPARISON_SPECS.len());

    for (artifact, spec) in artifacts.iter().zip(FAMILY_COMPARISON_SPECS.iter()) {
        assert_eq!(artifact.profile_id, spec.profile_id);
        assert_eq!(artifact.slug, spec.slug);

        let wav_path = output_dir.join(format!("{}_comparison.wav", spec.slug));
        let summary_path = output_dir.join(format!("{}_comparison_summary.txt", spec.slug));

        assert_eq!(artifact.paths.wav_path, wav_path.display().to_string());
        assert_eq!(
            artifact.paths.summary_path,
            summary_path.display().to_string()
        );
        assert!(wav_path.exists());
        assert!(summary_path.exists());

        let summary_report =
            fs::read_to_string(&summary_path).expect("comparison summary should be readable");
        assert!(summary_report.contains(&format!("family={}", spec.slug)));
        assert!(summary_report.contains("Corrosion offline renderer scaffold"));
    }

    let manifest = fs::read_to_string(output_dir.join("family_comparison_manifest.txt"))
        .expect("manifest should be readable");
    assert!(manifest.contains("family=pipe"));
    assert!(manifest.contains("family=plate"));
    assert!(manifest.contains("family=tank"));

    fs::remove_dir_all(output_dir).expect("temp output cleanup should succeed");
}

#[test]
fn rust_variation_specs_cover_low_and_high_pipe_examples_in_order() {
    assert_eq!(
        RUST_VARIATION_SPECS,
        [
            RustVariationRenderSpec {
                profile_id: ModalProfileId::Pipe,
                profile_slug: "pipe",
                variant_slug: "low_rust",
                rust_amount: 0.25,
            },
            RustVariationRenderSpec {
                profile_id: ModalProfileId::Pipe,
                profile_slug: "pipe",
                variant_slug: "high_rust",
                rust_amount: 0.85,
            },
        ]
    );
}

#[test]
fn rust_variation_render_writes_expected_artifacts() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8_192,
        ..RenderConfig::default()
    });
    let output_dir = unique_test_dir("rust_variation");

    let artifacts = renderer
        .render_rust_variations_to_dir(&output_dir)
        .expect("rust variation render should succeed");

    assert_eq!(artifacts.len(), RUST_VARIATION_SPECS.len());

    for (artifact, spec) in artifacts.iter().zip(RUST_VARIATION_SPECS.iter()) {
        assert_eq!(artifact.profile_id, spec.profile_id);
        assert_eq!(artifact.profile_slug, spec.profile_slug);
        assert_eq!(artifact.variant_slug, spec.variant_slug);
        approx_eq(artifact.rust_amount, spec.rust_amount, 1.0e-6);

        let wav_path = output_dir.join(format!("{}_{}.wav", spec.profile_slug, spec.variant_slug));
        let summary_path = output_dir.join(format!(
            "{}_{}_summary.txt",
            spec.profile_slug, spec.variant_slug
        ));

        assert_eq!(artifact.paths.wav_path, wav_path.display().to_string());
        assert_eq!(
            artifact.paths.summary_path,
            summary_path.display().to_string()
        );
        assert!(wav_path.exists());
        assert!(summary_path.exists());

        let summary_report =
            fs::read_to_string(&summary_path).expect("rust summary should be readable");
        assert!(summary_report.contains(&format!("profile={}", spec.profile_slug)));
        assert!(summary_report.contains(&format!("variant={}", spec.variant_slug)));
        assert!(summary_report.contains(&format!("rust_amount={:.3}", spec.rust_amount)));
        assert!(summary_report.contains("brightness_proxy="));
        assert!(summary_report.contains("late_to_early_energy_ratio="));
    }

    let manifest = fs::read_to_string(output_dir.join("rust_variation_manifest.txt"))
        .expect("rust manifest should be readable");
    assert!(manifest.contains("variant=low_rust"));
    assert!(manifest.contains("variant=high_rust"));

    fs::remove_dir_all(output_dir).expect("temp output cleanup should succeed");
}

#[test]
fn damage_variation_specs_cover_low_and_high_pipe_examples_in_order() {
    assert_eq!(
        DAMAGE_VARIATION_SPECS,
        [
            DamageVariationRenderSpec {
                profile_id: ModalProfileId::Pipe,
                profile_slug: "pipe",
                variant_slug: "low_damage",
                damage_amount: 0.25,
            },
            DamageVariationRenderSpec {
                profile_id: ModalProfileId::Pipe,
                profile_slug: "pipe",
                variant_slug: "high_damage",
                damage_amount: 0.85,
            },
        ]
    );
}

#[test]
fn damage_variation_render_writes_expected_artifacts() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8_192,
        ..RenderConfig::default()
    });
    let output_dir = unique_test_dir("damage_variation");

    let artifacts = renderer
        .render_damage_variations_to_dir(&output_dir)
        .expect("damage variation render should succeed");

    assert_eq!(artifacts.len(), DAMAGE_VARIATION_SPECS.len());

    for (artifact, spec) in artifacts.iter().zip(DAMAGE_VARIATION_SPECS.iter()) {
        assert_eq!(artifact.profile_id, spec.profile_id);
        assert_eq!(artifact.profile_slug, spec.profile_slug);
        assert_eq!(artifact.variant_slug, spec.variant_slug);
        approx_eq(artifact.damage_amount, spec.damage_amount, 1.0e-6);

        let wav_path = output_dir.join(format!("{}_{}.wav", spec.profile_slug, spec.variant_slug));
        let summary_path = output_dir.join(format!(
            "{}_{}_summary.txt",
            spec.profile_slug, spec.variant_slug
        ));

        assert_eq!(artifact.paths.wav_path, wav_path.display().to_string());
        assert_eq!(
            artifact.paths.summary_path,
            summary_path.display().to_string()
        );
        assert!(wav_path.exists());
        assert!(summary_path.exists());

        let summary_report =
            fs::read_to_string(&summary_path).expect("damage summary should be readable");
        assert!(summary_report.contains(&format!("profile={}", spec.profile_slug)));
        assert!(summary_report.contains(&format!("variant={}", spec.variant_slug)));
        assert!(summary_report.contains(&format!("damage_amount={:.3}", spec.damage_amount)));
        assert!(summary_report.contains("roughness_proxy="));
        assert!(summary_report.contains("late_to_early_energy_ratio="));
    }

    let manifest = fs::read_to_string(output_dir.join("damage_variation_manifest.txt"))
        .expect("damage manifest should be readable");
    assert!(manifest.contains("variant=low_damage"));
    assert!(manifest.contains("variant=high_damage"));

    fs::remove_dir_all(output_dir).expect("temp output cleanup should succeed");
}

#[test]
fn render_produces_non_silent_decaying_output() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 4_096,
        ..RenderConfig::default()
    });

    let (output, summary) = renderer.render(ModalResonator::default());
    let first_window_energy = output
        .iter()
        .take(512)
        .map(|sample| sample.abs())
        .sum::<f32>();
    let tail_window_energy = output
        .iter()
        .rev()
        .take(512)
        .map(|sample| sample.abs())
        .sum::<f32>();

    assert!(summary.peak > 0.0);
    assert!(summary.rms > 0.0);
    assert!(first_window_energy > tail_window_energy);
}

#[test]
fn rust_amount_clamps_to_supported_range() {
    assert_eq!(RustAmount::new(-0.25), RustAmount::default());
    assert_eq!(RustAmount::new(10.0), RustAmount::new(5.0));
    assert_eq!(RustAmount::new(f32::NAN), RustAmount::default());
}

#[test]
fn damage_amount_clamps_to_supported_range() {
    assert_eq!(DamageAmount::new(-0.25), DamageAmount::default());
    assert_eq!(DamageAmount::new(20.0), DamageAmount::new(10.0));
    assert_eq!(DamageAmount::new(f32::NAN), DamageAmount::default());
}

#[test]
fn default_resonator_uses_explicit_pipe_profile() {
    let resonator = ModalResonator::default();

    assert_eq!(resonator.profile, ModalProfileId::Pipe);
    assert_eq!(resonator.size_scale, SizeScale::default());
    assert_eq!(resonator.rust_amount, RustAmount::default());
    assert_eq!(resonator.damage_amount, DamageAmount::default());
    assert_eq!(resonator.modes.len(), PIPE_MODAL_PROFILE_MODES.len());

    let first_mode = resonator.modes.first().expect("pipe profile has modes");
    let last_mode = resonator.modes.last().expect("pipe profile has modes");

    approx_eq(first_mode.spec.frequency_hz, 220.0, 1.0e-6);
    assert!(first_mode.spec.decay_seconds > last_mode.spec.decay_seconds);
    assert!(first_mode.spec.gain > last_mode.spec.gain);
}

#[test]
fn pipe_profile_is_curated_for_tubular_ring_direction() {
    let profile = ModalProfile::pipe();
    let mut previous_frequency = 0.0;
    let mut previous_gain = f32::INFINITY;
    let fundamental = profile.modes[0].frequency_hz;

    assert_eq!(profile.id, ModalProfileId::Pipe);
    assert!(profile.modes.len() >= 6);

    for mode in profile.modes {
        assert!(mode.frequency_hz > previous_frequency);
        assert!(mode.gain < previous_gain);
        assert!(mode.decay_seconds > 0.0);
        previous_frequency = mode.frequency_hz;
        previous_gain = mode.gain;
    }

    let second_ratio = profile.modes[1].frequency_hz / fundamental;
    let fourth_ratio = profile.modes[3].frequency_hz / fundamental;

    assert!((second_ratio - 2.0).abs() < 0.02);
    assert!((fourth_ratio - 4.0).abs() < 0.04);
    assert!(profile.modes[0].decay_seconds >= 1.5);
}

#[test]
fn plate_profile_is_available_through_shared_profile_system() {
    let profile = ModalProfile::from_id(ModalProfileId::Plate);
    let resonator = ModalResonator::with_profile(ModalProfileId::Plate);

    assert_eq!(profile.id, ModalProfileId::Plate);
    assert_eq!(profile.modes.len(), PLATE_MODAL_PROFILE_MODES.len());
    assert_eq!(resonator.profile, ModalProfileId::Plate);
    assert_eq!(resonator.size_scale, SizeScale::default());
    assert_eq!(resonator.modes.len(), PLATE_MODAL_PROFILE_MODES.len());
}

#[test]
fn plate_profile_is_curated_for_flatter_metallic_ring_direction() {
    let profile = ModalProfile::plate();
    let mut previous_frequency = 0.0;
    let mut previous_gain = f32::INFINITY;
    let fundamental = profile.modes[0].frequency_hz;
    let mut largest_ratio_deviation = 0.0_f32;

    assert_eq!(profile.id, ModalProfileId::Plate);
    assert!(profile.modes.len() >= 7);

    for (index, mode) in profile.modes.iter().enumerate() {
        assert!(mode.frequency_hz > previous_frequency);
        assert!(mode.gain < previous_gain);
        assert!(mode.decay_seconds > 0.0);

        if index > 0 {
            let harmonic_number = (index + 1) as f32;
            let ratio = mode.frequency_hz / fundamental;
            largest_ratio_deviation = largest_ratio_deviation.max((ratio - harmonic_number).abs());
        }

        previous_frequency = mode.frequency_hz;
        previous_gain = mode.gain;
    }

    assert!(largest_ratio_deviation > 0.20);
    assert!(profile.modes[0].decay_seconds < ModalProfile::pipe().modes[0].decay_seconds);
}

#[test]
fn modal_profiles_follow_distinct_family_metric_directions() {
    let pipe_metrics = profile_metrics(ModalProfile::pipe());
    let plate_metrics = profile_metrics(ModalProfile::plate());
    let tank_metrics = profile_metrics(ModalProfile::tank());

    assert!(pipe_metrics.harmonic_deviation < plate_metrics.harmonic_deviation);
    assert!(pipe_metrics.harmonic_deviation < tank_metrics.harmonic_deviation);
    assert!(plate_metrics.weighted_frequency > pipe_metrics.weighted_frequency);
    assert!(pipe_metrics.weighted_frequency > tank_metrics.weighted_frequency);
    assert!(tank_metrics.average_decay > pipe_metrics.average_decay);
    assert!(pipe_metrics.average_decay > plate_metrics.average_decay);
}

#[test]
fn plate_profile_render_stays_deterministic_and_differs_from_pipe() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 4_096,
        ..RenderConfig::default()
    });

    let (_, first_plate_summary) =
        renderer.render(ModalResonator::with_profile(ModalProfileId::Plate));
    let (_, second_plate_summary) =
        renderer.render(ModalResonator::with_profile(ModalProfileId::Plate));
    let (_, pipe_summary) = renderer.render(ModalResonator::default());

    assert_eq!(first_plate_summary, second_plate_summary);
    assert_ne!(first_plate_summary.checksum, pipe_summary.checksum);
    assert_ne!(
        first_plate_summary.first_samples,
        pipe_summary.first_samples
    );
    assert!(first_plate_summary.peak > 0.0);
    assert!(first_plate_summary.rms > 0.0);
}

#[test]
fn tank_profile_is_available_through_shared_profile_system() {
    let profile = ModalProfile::from_id(ModalProfileId::Tank);
    let resonator = ModalResonator::with_profile(ModalProfileId::Tank);

    assert_eq!(profile.id, ModalProfileId::Tank);
    assert_eq!(profile.modes.len(), TANK_MODAL_PROFILE_MODES.len());
    assert_eq!(resonator.profile, ModalProfileId::Tank);
    assert_eq!(resonator.size_scale, SizeScale::default());
    assert_eq!(resonator.modes.len(), TANK_MODAL_PROFILE_MODES.len());
}

#[test]
fn size_scaling_lowers_and_extends_pipe_modes() {
    assert_size_scaling_direction(ModalProfileId::Pipe);
}

#[test]
fn size_scaling_lowers_and_extends_plate_modes() {
    assert_size_scaling_direction(ModalProfileId::Plate);
}

#[test]
fn size_scaling_lowers_and_extends_tank_modes() {
    assert_size_scaling_direction(ModalProfileId::Tank);
}

#[test]
fn tank_profile_is_curated_for_boomier_cavity_like_metallic_direction() {
    let profile = ModalProfile::tank();
    let pipe = ModalProfile::pipe();
    let plate = ModalProfile::plate();
    let mut previous_frequency = 0.0;
    let mut previous_gain = f32::INFINITY;
    let fundamental = profile.modes[0].frequency_hz;
    let mut low_band_gain = 0.0_f32;
    let mut high_band_gain = 0.0_f32;
    let mut largest_harmonic_deviation = 0.0_f32;

    assert_eq!(profile.id, ModalProfileId::Tank);
    assert!(profile.modes.len() >= 8);

    for (index, mode) in profile.modes.iter().enumerate() {
        assert!(mode.frequency_hz > previous_frequency);
        assert!(mode.gain < previous_gain);
        assert!(mode.decay_seconds > 0.0);

        if mode.frequency_hz < 400.0 {
            low_band_gain += mode.gain;
        } else {
            high_band_gain += mode.gain;
        }

        if index > 0 {
            let harmonic_number = (index + 1) as f32;
            let ratio = mode.frequency_hz / fundamental;
            largest_harmonic_deviation =
                largest_harmonic_deviation.max((ratio - harmonic_number).abs());
        }

        previous_frequency = mode.frequency_hz;
        previous_gain = mode.gain;
    }

    assert!(fundamental < pipe.modes[0].frequency_hz);
    assert!(fundamental < plate.modes[0].frequency_hz);
    assert!(profile.modes[0].decay_seconds > pipe.modes[0].decay_seconds);
    assert!(profile.modes[0].decay_seconds > plate.modes[0].decay_seconds);
    assert!(low_band_gain > high_band_gain);
    assert!(largest_harmonic_deviation > 0.35);
}

#[test]
fn tank_profile_render_stays_deterministic_and_differs_from_pipe_and_plate() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 4_096,
        ..RenderConfig::default()
    });

    let (_, first_tank_summary) =
        renderer.render(ModalResonator::with_profile(ModalProfileId::Tank));
    let (_, second_tank_summary) =
        renderer.render(ModalResonator::with_profile(ModalProfileId::Tank));
    let (_, pipe_summary) = renderer.render(ModalResonator::default());
    let (_, plate_summary) = renderer.render(ModalResonator::with_profile(ModalProfileId::Plate));

    assert_eq!(first_tank_summary, second_tank_summary);
    assert_ne!(first_tank_summary.checksum, pipe_summary.checksum);
    assert_ne!(first_tank_summary.checksum, plate_summary.checksum);
    assert_ne!(first_tank_summary.first_samples, pipe_summary.first_samples);
    assert_ne!(
        first_tank_summary.first_samples,
        plate_summary.first_samples
    );
    assert!(first_tank_summary.peak > 0.0);
    assert!(first_tank_summary.rms > 0.0);
}

#[test]
fn family_renders_separate_by_brightness_and_decay_behavior() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8_192,
        ..RenderConfig::default()
    });

    let (pipe_output, pipe_summary) =
        renderer.render(ModalResonator::with_profile(ModalProfileId::Pipe));
    let (plate_output, plate_summary) =
        renderer.render(ModalResonator::with_profile(ModalProfileId::Plate));
    let (tank_output, tank_summary) =
        renderer.render(ModalResonator::with_profile(ModalProfileId::Tank));

    let pipe_metrics = render_behavior_metrics(&pipe_output);
    let plate_metrics = render_behavior_metrics(&plate_output);
    let tank_metrics = render_behavior_metrics(&tank_output);

    assert_ne!(pipe_summary.checksum, plate_summary.checksum);
    assert_ne!(pipe_summary.checksum, tank_summary.checksum);
    assert_ne!(plate_summary.checksum, tank_summary.checksum);

    assert!(plate_metrics.zero_crossings > pipe_metrics.zero_crossings);
    assert!(pipe_metrics.zero_crossings > tank_metrics.zero_crossings);
    assert!(plate_metrics.brightness_proxy > pipe_metrics.brightness_proxy);
    assert!(pipe_metrics.brightness_proxy > tank_metrics.brightness_proxy);
    assert!(tank_metrics.late_to_early_energy_ratio > pipe_metrics.late_to_early_energy_ratio);
    assert!(pipe_metrics.late_to_early_energy_ratio > plate_metrics.late_to_early_energy_ratio);
}

#[test]
fn rust_transform_darkens_and_shortens_shared_pipe_modes() {
    let profile = ModalProfile::pipe();
    let clean_metrics = profile_metrics_for_modes(&profile.scaled_mode_specs(
        SizeScale::default(),
        RustAmount::default(),
        DamageAmount::default(),
    ));
    let rusted_metrics = profile_metrics_for_modes(&profile.scaled_mode_specs(
        SizeScale::default(),
        RustAmount::new(0.85),
        DamageAmount::default(),
    ));

    assert!(rusted_metrics.weighted_frequency < clean_metrics.weighted_frequency);
    assert!(rusted_metrics.average_decay < clean_metrics.average_decay);
    assert!(rusted_metrics.low_band_gain_ratio > clean_metrics.low_band_gain_ratio);
}

#[test]
fn rust_render_is_deterministic_and_reduces_brightness_and_decay() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8_192,
        ..RenderConfig::default()
    });

    let (clean_output, clean_summary) =
        renderer.render(ModalResonator::with_profile_size_and_rust(
            ModalProfileId::Pipe,
            SizeScale::default(),
            RustAmount::default(),
        ));
    let (rusted_output, rusted_summary) =
        renderer.render(ModalResonator::with_profile_size_and_rust(
            ModalProfileId::Pipe,
            SizeScale::default(),
            RustAmount::new(0.85),
        ));
    let (_, repeated_rusted_summary) = renderer.render(ModalResonator::with_profile_size_and_rust(
        ModalProfileId::Pipe,
        SizeScale::default(),
        RustAmount::new(0.85),
    ));

    let clean_metrics = render_behavior_metrics(&clean_output);
    let rusted_metrics = render_behavior_metrics(&rusted_output);

    assert_eq!(rusted_summary, repeated_rusted_summary);
    assert_ne!(clean_summary.checksum, rusted_summary.checksum);
    assert!(rusted_metrics.brightness_proxy < clean_metrics.brightness_proxy);
    assert!(rusted_metrics.late_to_early_energy_ratio < clean_metrics.late_to_early_energy_ratio);
    assert!(rusted_summary.peak < clean_summary.peak);
}

#[test]
fn high_rust_render_is_darker_and_shorter_than_low_rust_render() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8_192,
        ..RenderConfig::default()
    });

    let (low_output, low_summary) = renderer.render(ModalResonator::with_profile_size_and_rust(
        ModalProfileId::Pipe,
        SizeScale::default(),
        RustAmount::new(0.25),
    ));
    let (high_output, high_summary) = renderer.render(ModalResonator::with_profile_size_and_rust(
        ModalProfileId::Pipe,
        SizeScale::default(),
        RustAmount::new(0.85),
    ));

    let low_metrics = render_behavior_metrics(&low_output);
    let high_metrics = render_behavior_metrics(&high_output);

    assert_ne!(low_summary.checksum, high_summary.checksum);
    assert!(high_metrics.brightness_proxy < low_metrics.brightness_proxy);
    assert!(high_metrics.late_to_early_energy_ratio < low_metrics.late_to_early_energy_ratio);
    assert!(high_summary.peak < low_summary.peak);
}

#[test]
fn damage_transform_detunes_and_expands_shared_pipe_modes() {
    let profile = ModalProfile::pipe();
    let clean_modes = profile.scaled_mode_specs(
        SizeScale::default(),
        RustAmount::default(),
        DamageAmount::default(),
    );
    let damaged_modes = profile.scaled_mode_specs(
        SizeScale::default(),
        RustAmount::default(),
        DamageAmount::new(0.85),
    );
    let clean_metrics = profile_metrics_for_modes(&clean_modes);
    let damaged_metrics = profile_metrics_for_modes(&damaged_modes);

    assert_eq!(clean_modes.len(), PIPE_MODAL_PROFILE_MODES.len());
    assert_eq!(damaged_modes.len(), PIPE_MODAL_PROFILE_MODES.len() * 2);
    assert!(damaged_metrics.harmonic_deviation > clean_metrics.harmonic_deviation);
    assert!(damaged_metrics.average_decay < clean_metrics.average_decay);
}

#[test]
fn damage_render_is_deterministic_and_increases_roughness() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8_192,
        ..RenderConfig::default()
    });

    let (clean_output, clean_summary) =
        renderer.render(ModalResonator::with_profile_size_rust_and_damage(
            ModalProfileId::Pipe,
            SizeScale::default(),
            RustAmount::default(),
            DamageAmount::default(),
        ));
    let (damaged_output, damaged_summary) =
        renderer.render(ModalResonator::with_profile_size_rust_and_damage(
            ModalProfileId::Pipe,
            SizeScale::default(),
            RustAmount::default(),
            DamageAmount::new(0.85),
        ));
    let (_, repeated_damaged_summary) =
        renderer.render(ModalResonator::with_profile_size_rust_and_damage(
            ModalProfileId::Pipe,
            SizeScale::default(),
            RustAmount::default(),
            DamageAmount::new(0.85),
        ));

    let clean_metrics = render_behavior_metrics(&clean_output);
    let damaged_metrics = render_behavior_metrics(&damaged_output);

    assert_eq!(damaged_summary, repeated_damaged_summary);
    assert_ne!(clean_summary.checksum, damaged_summary.checksum);
    assert!(damaged_metrics.roughness_proxy > clean_metrics.roughness_proxy);
    assert!(damaged_metrics.late_to_early_energy_ratio < clean_metrics.late_to_early_energy_ratio);
}

#[test]
fn high_damage_render_is_rougher_and_shorter_than_low_damage_render() {
    let renderer = OfflineRenderer::new(RenderConfig {
        frame_count: 8_192,
        ..RenderConfig::default()
    });

    let (low_output, low_summary) =
        renderer.render(ModalResonator::with_profile_size_rust_and_damage(
            ModalProfileId::Pipe,
            SizeScale::default(),
            RustAmount::default(),
            DamageAmount::new(0.25),
        ));
    let (high_output, high_summary) =
        renderer.render(ModalResonator::with_profile_size_rust_and_damage(
            ModalProfileId::Pipe,
            SizeScale::default(),
            RustAmount::default(),
            DamageAmount::new(0.85),
        ));

    let low_metrics = render_behavior_metrics(&low_output);
    let high_metrics = render_behavior_metrics(&high_output);

    assert_ne!(low_summary.checksum, high_summary.checksum);
    assert!(high_metrics.roughness_proxy > low_metrics.roughness_proxy);
    assert!(high_metrics.late_to_early_energy_ratio < low_metrics.late_to_early_energy_ratio);
    assert!(high_summary.peak >= low_summary.peak);
}

#[test]
fn realtime_mode_count_estimates_stay_grounded_in_current_profile_tables() {
    assert_eq!(
        realtime_mode_count_estimate(ModalProfileId::Pipe),
        RealtimeModeCountEstimate {
            profile_id: ModalProfileId::Pipe,
            canonical_mode_count: PIPE_MODAL_PROFILE_MODES.len(),
            safe_realtime_mode_count: PIPE_MODAL_PROFILE_MODES.len(),
            offline_peak_mode_count: PIPE_MODAL_PROFILE_MODES.len() * 2,
        }
    );
    assert_eq!(
        realtime_mode_count_estimate(ModalProfileId::Plate),
        RealtimeModeCountEstimate {
            profile_id: ModalProfileId::Plate,
            canonical_mode_count: PLATE_MODAL_PROFILE_MODES.len(),
            safe_realtime_mode_count: PLATE_MODAL_PROFILE_MODES.len(),
            offline_peak_mode_count: PLATE_MODAL_PROFILE_MODES.len() * 2,
        }
    );
    assert_eq!(
        realtime_mode_count_estimate(ModalProfileId::Tank),
        RealtimeModeCountEstimate {
            profile_id: ModalProfileId::Tank,
            canonical_mode_count: TANK_MODAL_PROFILE_MODES.len(),
            safe_realtime_mode_count: TANK_MODAL_PROFILE_MODES.len(),
            offline_peak_mode_count: TANK_MODAL_PROFILE_MODES.len() * 2,
        }
    );
}

#[test]
fn shared_mode_limits_follow_current_family_extremes() {
    use super::profile::CHAIN_MODAL_PROFILE_MODES;
    assert_eq!(
        safe_realtime_shared_mode_limit(),
        CHAIN_MODAL_PROFILE_MODES.len()
    );
    assert_eq!(
        offline_peak_shared_mode_limit(),
        CHAIN_MODAL_PROFILE_MODES.len() * 2
    );
}

#[test]
fn resonator_coefficients_follow_spec_formula() {
    use super::ResonatorCoefficients;
    let mode = ModalModeSpec::new(330.0, 0.72, 0.12);
    let coefficients = ResonatorCoefficients::for_mode(mode, 48_000);
    let omega = 2.0 * std::f32::consts::PI * mode.frequency_hz / 48_000.0;
    let r = (-1.0 / (mode.decay_seconds * 48_000.0)).exp();

    approx_eq(coefficients.b0, mode.gain, 1.0e-8);
    approx_eq(coefficients.a1, -2.0 * r * omega.cos(), 1.0e-6);
    approx_eq(coefficients.a2, r * r, 1.0e-6);
}

#[test]
fn second_order_mode_obeys_difference_equation() {
    use super::{ResonatorCoefficients, SecondOrderMode};
    let spec = ModalModeSpec::new(220.0, 0.95, 0.18);
    let mut mode = SecondOrderMode::new(spec);
    let sample_rate = 48_000;
    let coefficients = ResonatorCoefficients::for_mode(spec, sample_rate);

    // The mode is the biquad scaled by sin ω / ω, so it rings at the
    // continuous oscillator's amplitude b0/ω (resolve-exciter-tradeoffs).
    let omega = 2.0 * std::f32::consts::PI * spec.frequency_hz / sample_rate as f32;
    let scale = omega.sin() / omega;
    let y0 = mode.process(1.0, sample_rate);
    let y1 = mode.process(0.0, sample_rate);
    let y2 = mode.process(0.0, sample_rate);

    approx_eq(y0, coefficients.b0 * scale, 1.0e-6);
    approx_eq(y1, -coefficients.a1 * y0, 1.0e-5);
    approx_eq(y2, -coefficients.a1 * y1 - coefficients.a2 * y0, 1.0e-5);
}

#[test]
fn resonator_output_stays_finite_over_long_render() {
    let renderer = OfflineRenderer::new(RenderConfig::default());
    let (output, summary) = renderer.render(ModalResonator::default());

    assert!(output.iter().all(|sample| sample.is_finite()));
    assert!(summary.peak.is_finite());
    assert!(summary.rms.is_finite());
}

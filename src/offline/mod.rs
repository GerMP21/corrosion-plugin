mod preset;

pub use preset::{
    render_preset, render_preset_pre_limiter, OfflineNote, OfflineNoteKind, PresetRenderer,
    RenderedFrames, OFFLINE_BLOCK_SIZE,
};

use std::fs;
use std::io;
use std::path::Path;

use crate::dsp::{
    DamageAmount, ModalProfileId, ModalResonator, PostProcessingChain, PostQualityMode, RustAmount,
    SizeScale, SpaceMode,
};

#[derive(Clone, Copy, Debug)]
pub struct RenderConfig {
    pub sample_rate: u32,
    pub frame_count: usize,
    pub excitation_frame: usize,
    pub excitation_amplitude: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            frame_count: 48_000,
            excitation_frame: 0,
            excitation_amplitude: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderSummary {
    pub sample_rate: u32,
    pub frame_count: usize,
    pub peak: f32,
    pub rms: f32,
    pub checksum: u64,
    pub first_samples: Vec<f32>,
}

impl RenderSummary {
    pub fn to_report(&self) -> String {
        let first_samples = self
            .first_samples
            .iter()
            .map(|sample| format!("{sample:.9}"))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            concat!(
                "Corrosion offline renderer scaffold\n",
                "sample_rate={sample_rate}\n",
                "frame_count={frame_count}\n",
                "peak={peak:.9}\n",
                "rms={rms:.9}\n",
                "checksum={checksum}\n",
                "first_samples=[{first_samples}]\n"
            ),
            sample_rate = self.sample_rate,
            frame_count = self.frame_count,
            peak = self.peak,
            rms = self.rms,
            checksum = self.checksum,
            first_samples = first_samples,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderBehaviorMetrics {
    pub zero_crossings: usize,
    pub brightness_proxy: f32,
    pub late_to_early_energy_ratio: f32,
    pub roughness_proxy: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AliasingReport {
    pub sample_rate: u32,
    pub frame_count: usize,
    pub input_frequency_hz: f32,
    pub harmonic_bins: Vec<usize>,
    pub harmonic_energy: f32,
    pub alias_energy: f32,
    pub dc_energy: f32,
    pub alias_ratio_db: f32,
    pub strongest_alias_frequency_hz: f32,
    pub strongest_alias_energy: f32,
}

impl AliasingReport {
    pub fn to_report(&self) -> String {
        let harmonic_bins = self
            .harmonic_bins
            .iter()
            .map(|bin| bin.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            concat!(
                "sample_rate={}\n",
                "frame_count={}\n",
                "input_frequency_hz={:.6}\n",
                "harmonic_bins=[{}]\n",
                "harmonic_energy={:.9}\n",
                "alias_energy={:.9}\n",
                "dc_energy={:.9}\n",
                "alias_ratio_db={:.9}\n",
                "strongest_alias_frequency_hz={:.6}\n",
                "strongest_alias_energy={:.9}\n"
            ),
            self.sample_rate,
            self.frame_count,
            self.input_frequency_hz,
            harmonic_bins,
            self.harmonic_energy,
            self.alias_energy,
            self.dc_energy,
            self.alias_ratio_db,
            self.strongest_alias_frequency_hz,
            self.strongest_alias_energy,
        )
    }
}

impl RenderBehaviorMetrics {
    fn to_report(self) -> String {
        format!(
            concat!(
                "zero_crossings={}\n",
                "brightness_proxy={:.9}\n",
                "late_to_early_energy_ratio={:.9}\n",
                "roughness_proxy={:.9}\n"
            ),
            self.zero_crossings,
            self.brightness_proxy,
            self.late_to_early_energy_ratio,
            self.roughness_proxy,
        )
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComparisonRenderSpec {
    pub profile_id: ModalProfileId,
    pub slug: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonArtifactPaths {
    pub wav_path: String,
    pub summary_path: String,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonRenderArtifact {
    pub profile_id: ModalProfileId,
    pub slug: &'static str,
    pub paths: ComparisonArtifactPaths,
    pub summary: RenderSummary,
}

#[cfg_attr(not(test), allow(dead_code))]
impl ComparisonRenderArtifact {
    fn summary_report(&self) -> String {
        format!(
            concat!("family={}\n", "wav_path={}\n", "summary_path={}\n", "{}"),
            self.slug,
            self.paths.wav_path,
            self.paths.summary_path,
            self.summary.to_report(),
        )
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RustVariationRenderSpec {
    pub profile_id: ModalProfileId,
    pub profile_slug: &'static str,
    pub variant_slug: &'static str,
    pub rust_amount: f32,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Debug, PartialEq)]
pub struct RustVariationRenderArtifact {
    pub profile_id: ModalProfileId,
    pub profile_slug: &'static str,
    pub variant_slug: &'static str,
    pub rust_amount: f32,
    pub paths: ComparisonArtifactPaths,
    pub summary: RenderSummary,
    pub behavior_metrics: RenderBehaviorMetrics,
}

#[cfg_attr(not(test), allow(dead_code))]
impl RustVariationRenderArtifact {
    fn summary_report(&self) -> String {
        format!(
            concat!(
                "profile={}\n",
                "variant={}\n",
                "rust_amount={:.3}\n",
                "wav_path={}\n",
                "summary_path={}\n",
                "{}",
                "{}"
            ),
            self.profile_slug,
            self.variant_slug,
            self.rust_amount,
            self.paths.wav_path,
            self.paths.summary_path,
            self.summary.to_report(),
            self.behavior_metrics.to_report(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageVariationRenderSpec {
    pub profile_id: ModalProfileId,
    pub profile_slug: &'static str,
    pub variant_slug: &'static str,
    pub damage_amount: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DamageVariationRenderArtifact {
    pub profile_id: ModalProfileId,
    pub profile_slug: &'static str,
    pub variant_slug: &'static str,
    pub damage_amount: f32,
    pub paths: ComparisonArtifactPaths,
    pub summary: RenderSummary,
    pub behavior_metrics: RenderBehaviorMetrics,
}

impl DamageVariationRenderArtifact {
    fn summary_report(&self) -> String {
        format!(
            concat!(
                "profile={}\n",
                "variant={}\n",
                "damage_amount={:.3}\n",
                "wav_path={}\n",
                "summary_path={}\n",
                "{}",
                "{}"
            ),
            self.profile_slug,
            self.variant_slug,
            self.damage_amount,
            self.paths.wav_path,
            self.paths.summary_path,
            self.summary.to_report(),
            self.behavior_metrics.to_report(),
        )
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub const FAMILY_COMPARISON_SPECS: [ComparisonRenderSpec; 3] = [
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
];

#[cfg_attr(not(test), allow(dead_code))]
pub const RUST_VARIATION_SPECS: [RustVariationRenderSpec; 2] = [
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
];

pub const DAMAGE_VARIATION_SPECS: [DamageVariationRenderSpec; 2] = [
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
];

fn energy_in_window(samples: &[f32], start: usize, len: usize) -> f32 {
    samples[start..start + len]
        .iter()
        .map(|sample| sample.abs())
        .sum::<f32>()
}

fn zero_crossings(samples: &[f32]) -> usize {
    samples
        .windows(2)
        .filter(|pair| {
            let first = pair[0];
            let second = pair[1];
            (first > 0.0 && second < 0.0) || (first < 0.0 && second > 0.0)
        })
        .count()
}

fn brightness_proxy(samples: &[f32]) -> f32 {
    let absolute_motion = samples
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .sum::<f32>();
    let absolute_level = samples.iter().map(|sample| sample.abs()).sum::<f32>();

    absolute_motion / absolute_level.max(f32::EPSILON)
}

fn roughness_proxy(samples: &[f32]) -> f32 {
    let crossing_positions = samples
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let first = pair[0];
            let second = pair[1];
            ((first > 0.0 && second < 0.0) || (first < 0.0 && second > 0.0)).then_some(index as f32)
        })
        .collect::<Vec<_>>();

    if crossing_positions.len() < 3 {
        return 0.0;
    }

    let intervals = crossing_positions
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .collect::<Vec<_>>();

    let mean_interval = intervals.iter().sum::<f32>() / intervals.len() as f32;

    intervals
        .iter()
        .map(|interval| (interval - mean_interval).abs())
        .sum::<f32>()
        / intervals.len() as f32
}

pub fn render_behavior_metrics(output: &[f32]) -> RenderBehaviorMetrics {
    let frame_count = output.len();
    let early_len = (frame_count / 4).max(1);
    let late_len = frame_count - early_len;

    RenderBehaviorMetrics {
        zero_crossings: zero_crossings(output),
        brightness_proxy: brightness_proxy(output),
        late_to_early_energy_ratio: energy_in_window(output, early_len, late_len)
            / energy_in_window(output, 0, early_len).max(f32::EPSILON),
        roughness_proxy: roughness_proxy(output),
    }
}

fn float_sample_to_pcm_i16(sample: f32) -> i16 {
    let clamped = crate::apply_output_limiter(sample, crate::LIMITER_THRESHOLD);
    (clamped * i16::MAX as f32) as i16
}

pub(crate) fn write_wav_i16(path: &Path, samples: &[f32], sample_rate: u32) -> io::Result<()> {
    let data_bytes: Vec<u8> = samples
        .iter()
        .flat_map(|sample| {
            let pcm = float_sample_to_pcm_i16(*sample);
            [(pcm & 0xFF) as u8, ((pcm >> 8) & 0xFF) as u8]
        })
        .collect();

    let data_len = data_bytes.len() as u32;
    let chunk_size = 36 + data_len;
    let byte_rate = sample_rate * 2;

    let mut wav = Vec::with_capacity(44 + data_bytes.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&chunk_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&data_bytes);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, wav)
}

fn path_display_string(path: &Path) -> String {
    path.display().to_string()
}

fn dft_magnitude_squared(samples: &[f32], bin: usize) -> f32 {
    let sample_count = samples.len() as f32;
    let angular_step = 2.0 * std::f32::consts::PI * bin as f32 / sample_count.max(1.0);
    let mut real = 0.0f32;
    let mut imag = 0.0f32;

    for (index, sample) in samples.iter().enumerate() {
        let phase = angular_step * index as f32;
        real += *sample * phase.cos();
        imag -= *sample * phase.sin();
    }

    real * real + imag * imag
}

pub fn analyze_post_chain_aliasing(sample_rate: u32, frame_count: usize) -> AliasingReport {
    analyze_post_chain_aliasing_at_quality(sample_rate, frame_count, PostQualityMode::Render)
}

/// Aliasing analysis at an explicit quality mode.
///
/// `analyze_post_chain_aliasing` calls this with `Render`. Pinning the quality
/// lets regression tests compare a low-oversample lane to a high-oversample one
/// and verify the clipper actually reduces alias energy as the factor rises.
pub fn analyze_post_chain_aliasing_at_quality(
    sample_rate: u32,
    frame_count: usize,
    quality_mode: PostQualityMode,
) -> AliasingReport {
    let sample_rate = sample_rate.max(1);
    let frame_count = frame_count.max(256);
    let nyquist_bin = frame_count / 2;
    let input_bin = (frame_count * 3 / 16).clamp(1, nyquist_bin.saturating_sub(1));
    let input_frequency_hz = input_bin as f32 * sample_rate as f32 / frame_count as f32;

    let mut chain = PostProcessingChain::new();
    chain.set_sample_rate(sample_rate as f32);
    chain.set_quality_mode(quality_mode);
    chain.set_filter_params(20_000.0, 0.0, 0.0);
    chain.set_drive_params(2.5, 0.35, 0.2);
    chain.set_body_params(0.0, 0.0);
    chain.set_spread_params(0.0, 0.0);
    chain.set_space_mode(SpaceMode::Off);
    chain.set_space_amount(0.0);
    chain.set_clipper_params(0.9661, 0.5);

    let warmup_frames = frame_count / 2;
    for frame in 0..warmup_frames {
        let phase =
            2.0 * std::f32::consts::PI * input_frequency_hz * frame as f32 / sample_rate as f32;
        let input = phase.sin() * 0.85;
        let _ = chain.process(input, input);
    }

    let mut output = Vec::with_capacity(frame_count);
    for frame in 0..frame_count {
        let phase =
            2.0 * std::f32::consts::PI * input_frequency_hz * frame as f32 / sample_rate as f32;
        let input = phase.sin() * 0.85;
        let (left, right) = chain.process(input, input);
        output.push((left + right) * 0.5);
    }

    let harmonic_bins = (1..)
        .map(|harmonic| input_bin * harmonic)
        .take_while(|bin| *bin <= nyquist_bin)
        .collect::<Vec<_>>();

    let dc_energy = dft_magnitude_squared(&output, 0);
    let harmonic_energy = harmonic_bins
        .iter()
        .map(|bin| dft_magnitude_squared(&output, *bin))
        .sum::<f32>();

    let mut alias_energy = 0.0f32;
    let mut strongest_alias_bin = 1usize;
    let mut strongest_alias_energy = 0.0f32;
    for bin in 1..=nyquist_bin {
        if harmonic_bins.contains(&bin) {
            continue;
        }

        let energy = dft_magnitude_squared(&output, bin);
        alias_energy += energy;
        if energy > strongest_alias_energy {
            strongest_alias_energy = energy;
            strongest_alias_bin = bin;
        }
    }

    let strongest_alias_frequency_hz =
        strongest_alias_bin as f32 * sample_rate as f32 / frame_count as f32;
    let alias_ratio_db = 10.0
        * (alias_energy / harmonic_energy.max(f32::EPSILON))
            .max(f32::EPSILON)
            .log10();

    AliasingReport {
        sample_rate,
        frame_count,
        input_frequency_hz,
        harmonic_bins,
        harmonic_energy,
        alias_energy,
        dc_energy,
        alias_ratio_db,
        strongest_alias_frequency_hz,
        strongest_alias_energy,
    }
}

pub struct OfflineRenderer {
    config: RenderConfig,
}

impl OfflineRenderer {
    pub fn new(config: RenderConfig) -> Self {
        Self { config }
    }

    pub fn render<R: crate::dsp::ResonatorCore>(
        &self,
        mut resonator: R,
    ) -> (Vec<f32>, RenderSummary) {
        let excitation = self.deterministic_excitation();

        self.render_with_excitation(&excitation, &mut resonator)
    }

    pub fn deterministic_excitation(&self) -> crate::dsp::ExcitationInput {
        crate::dsp::ExcitationInput::impulse(
            self.config.frame_count,
            self.config.excitation_frame,
            self.config.excitation_amplitude,
        )
    }

    pub fn render_with_excitation<R: crate::dsp::ResonatorCore>(
        &self,
        excitation: &crate::dsp::ExcitationInput,
        resonator: &mut R,
    ) -> (Vec<f32>, RenderSummary) {
        assert_eq!(
            excitation.frame_count(),
            self.config.frame_count,
            "excitation length must match render frame count"
        );

        let mut output = Vec::with_capacity(self.config.frame_count);
        let mut peak = 0.0_f32;
        let mut energy_sum = 0.0_f64;
        let mut checksum = 0_u64;

        for frame in 0..self.config.frame_count {
            let sample =
                resonator.process_sample(excitation.sample(frame), self.config.sample_rate);
            peak = peak.max(sample.abs());
            energy_sum += f64::from(sample) * f64::from(sample);
            checksum = checksum
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(u64::from(sample.to_bits()));
            output.push(sample);
        }

        let first_samples = output.iter().take(16).copied().collect::<Vec<_>>();
        let rms = (energy_sum / self.config.frame_count as f64).sqrt() as f32;

        let summary = RenderSummary {
            sample_rate: self.config.sample_rate,
            frame_count: self.config.frame_count,
            peak,
            rms,
            checksum,
            first_samples,
        };

        (output, summary)
    }

    #[allow(dead_code)]
    pub fn render_to_path<R: crate::dsp::ResonatorCore>(
        &self,
        resonator: R,
        path: &Path,
    ) -> Result<RenderSummary, Box<dyn std::error::Error>> {
        let (_output, summary) = self.render(resonator);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, summary.to_report())?;
        Ok(summary)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn render_family_comparisons_to_dir(
        &self,
        output_dir: &Path,
    ) -> Result<Vec<ComparisonRenderArtifact>, Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        let mut artifacts = Vec::with_capacity(FAMILY_COMPARISON_SPECS.len());

        for spec in FAMILY_COMPARISON_SPECS {
            let (output, summary) = self.render(ModalResonator::with_profile(spec.profile_id));
            let wav_path = output_dir.join(format!("{}_comparison.wav", spec.slug));
            let summary_path = output_dir.join(format!("{}_comparison_summary.txt", spec.slug));

            write_wav_i16(&wav_path, &output, self.config.sample_rate)?;

            let artifact = ComparisonRenderArtifact {
                profile_id: spec.profile_id,
                slug: spec.slug,
                paths: ComparisonArtifactPaths {
                    wav_path: path_display_string(&wav_path),
                    summary_path: path_display_string(&summary_path),
                },
                summary,
            };

            fs::write(&summary_path, artifact.summary_report())?;
            artifacts.push(artifact);
        }

        let manifest = artifacts
            .iter()
            .map(|artifact| {
                format!(
                    "family={} wav_path={} summary_path={} checksum={} peak={:.9} rms={:.9}",
                    artifact.slug,
                    artifact.paths.wav_path,
                    artifact.paths.summary_path,
                    artifact.summary.checksum,
                    artifact.summary.peak,
                    artifact.summary.rms,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(output_dir.join("family_comparison_manifest.txt"), manifest)?;

        Ok(artifacts)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn render_rust_variations_to_dir(
        &self,
        output_dir: &Path,
    ) -> Result<Vec<RustVariationRenderArtifact>, Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        let mut artifacts = Vec::with_capacity(RUST_VARIATION_SPECS.len());

        for spec in RUST_VARIATION_SPECS {
            let (output, summary) = self.render(ModalResonator::with_profile_size_and_rust(
                spec.profile_id,
                SizeScale::default(),
                RustAmount::new(spec.rust_amount),
            ));
            let behavior_metrics = render_behavior_metrics(&output);
            let wav_path =
                output_dir.join(format!("{}_{}.wav", spec.profile_slug, spec.variant_slug));
            let summary_path = output_dir.join(format!(
                "{}_{}_summary.txt",
                spec.profile_slug, spec.variant_slug
            ));

            write_wav_i16(&wav_path, &output, self.config.sample_rate)?;

            let artifact = RustVariationRenderArtifact {
                profile_id: spec.profile_id,
                profile_slug: spec.profile_slug,
                variant_slug: spec.variant_slug,
                rust_amount: spec.rust_amount,
                paths: ComparisonArtifactPaths {
                    wav_path: path_display_string(&wav_path),
                    summary_path: path_display_string(&summary_path),
                },
                summary,
                behavior_metrics,
            };

            fs::write(&summary_path, artifact.summary_report())?;
            artifacts.push(artifact);
        }

        let manifest = artifacts
            .iter()
            .map(|artifact| {
                format!(
                    "profile={} variant={} rust={:.2} wav_path={} summary_path={} checksum={} peak={:.9} rms={:.9}",
                    artifact.profile_slug,
                    artifact.variant_slug,
                    artifact.rust_amount,
                    artifact.paths.wav_path,
                    artifact.paths.summary_path,
                    artifact.summary.checksum,
                    artifact.summary.peak,
                    artifact.summary.rms,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(output_dir.join("rust_variation_manifest.txt"), manifest)?;

        Ok(artifacts)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn render_damage_variations_to_dir(
        &self,
        output_dir: &Path,
    ) -> Result<Vec<DamageVariationRenderArtifact>, Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;

        let mut artifacts = Vec::with_capacity(DAMAGE_VARIATION_SPECS.len());

        for spec in DAMAGE_VARIATION_SPECS {
            let (output, summary) = self.render(ModalResonator::with_profile_size_rust_and_damage(
                spec.profile_id,
                SizeScale::default(),
                RustAmount::default(),
                DamageAmount::new(spec.damage_amount),
            ));
            let behavior_metrics = render_behavior_metrics(&output);
            let wav_path =
                output_dir.join(format!("{}_{}.wav", spec.profile_slug, spec.variant_slug));
            let summary_path = output_dir.join(format!(
                "{}_{}_summary.txt",
                spec.profile_slug, spec.variant_slug
            ));

            write_wav_i16(&wav_path, &output, self.config.sample_rate)?;

            let artifact = DamageVariationRenderArtifact {
                profile_id: spec.profile_id,
                profile_slug: spec.profile_slug,
                variant_slug: spec.variant_slug,
                damage_amount: spec.damage_amount,
                paths: ComparisonArtifactPaths {
                    wav_path: path_display_string(&wav_path),
                    summary_path: path_display_string(&summary_path),
                },
                summary,
                behavior_metrics,
            };

            fs::write(&summary_path, artifact.summary_report())?;
            artifacts.push(artifact);
        }

        let manifest = artifacts
            .iter()
            .map(|artifact| {
                format!(
                    "profile={} variant={} damage={:.2} wav_path={} summary_path={} checksum={} peak={:.9} rms={:.9}",
                    artifact.profile_slug,
                    artifact.variant_slug,
                    artifact.damage_amount,
                    artifact.paths.wav_path,
                    artifact.paths.summary_path,
                    artifact.summary.checksum,
                    artifact.summary.peak,
                    artifact.summary.rms,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(output_dir.join("damage_variation_manifest.txt"), manifest)?;

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::{analyze_post_chain_aliasing, analyze_post_chain_aliasing_at_quality};
    use crate::dsp::PostQualityMode;

    #[test]
    fn aliasing_report_is_finite_and_populated() {
        let report = analyze_post_chain_aliasing(48_000, 2_048);

        assert!(report.input_frequency_hz.is_finite());
        assert!(!report.harmonic_bins.is_empty());
        assert!(report.harmonic_energy.is_finite());
        assert!(report.alias_energy.is_finite());
        assert!(report.dc_energy.is_finite());
        assert!(report.alias_ratio_db.is_finite());
        assert!(report.strongest_alias_frequency_hz > 0.0);
        assert!(report.strongest_alias_energy.is_finite());
    }

    /// Render-mode (16× oversample) aliasing must stay below a budget so that
    /// regressing the clipper or downstream stages fails CI rather than slipping
    /// out silently. The threshold is set above the current measured value with
    /// a margin and is the contract this test enforces; tighten it when the
    /// chain genuinely improves.
    #[test]
    fn render_mode_alias_ratio_stays_within_budget() {
        // Empirically the Render-mode ratio sits well below -10 dB; the budget
        // is the regression line, not the current measured value.
        const ALIAS_RATIO_BUDGET_DB: f32 = -10.0;

        let report = analyze_post_chain_aliasing_at_quality(48_000, 4_096, PostQualityMode::Render);
        assert!(
            report.alias_ratio_db < ALIAS_RATIO_BUDGET_DB,
            "Render alias_ratio_db {} exceeded budget {} dB — clipper or post chain regressed",
            report.alias_ratio_db,
            ALIAS_RATIO_BUDGET_DB
        );
    }

    /// Higher oversampling must produce strictly less alias energy than Eco
    /// (1×). This pairs the analyzer with the P0 oversampled-clipper fix: if
    /// the clipper ever silently goes back to a no-op, this fails.
    #[test]
    fn higher_quality_reduces_alias_ratio() {
        let eco = analyze_post_chain_aliasing_at_quality(48_000, 4_096, PostQualityMode::Eco);
        let render = analyze_post_chain_aliasing_at_quality(48_000, 4_096, PostQualityMode::Render);

        assert!(
            render.alias_ratio_db < eco.alias_ratio_db,
            "Render alias_ratio_db ({}) should be lower than Eco ({}) — clipper oversampling regressed",
            render.alias_ratio_db,
            eco.alias_ratio_db
        );
    }
}

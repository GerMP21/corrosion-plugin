//! Render every preset to a stereo WAV, plus a manifest of debug metrics.
//!
//! Each preset is loaded into a real plugin instance and rendered with
//! `corrosion::offline::render_preset`, the plugin's own output path (voices,
//! master drive, post chain, output gain, limiter), so the audition files
//! sound like the plugin (H13).

use clap::Parser;
use corrosion::offline::{render_behavior_metrics, render_preset, OfflineNote};
use corrosion::presets::Preset;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(about = "Render all presets to WAV with debug metrics")]
struct Args {
    #[arg(long, default_value = "presets/factory")]
    preset_dir: String,
    #[arg(long, default_value = "output")]
    out_dir: String,
    #[arg(long, default_value = "60")]
    note: u8,
    /// MIDI velocity, 0–127.
    #[arg(long, default_value = "127")]
    velocity: f32,
    /// Render length in seconds.
    #[arg(long, default_value = "2")]
    duration: f32,
    /// When the note is released, in seconds from the start (the rest of the
    /// render is the release and any Space tail).
    #[arg(long, default_value = "1")]
    note_off: f32,
    #[arg(long, default_value_t = 48_000)]
    sample_rate: u32,
    #[arg(long)]
    contains: Option<String>,
    #[arg(long)]
    limit: Option<usize>,
}

/// What to play: one note, held for `note_off` seconds of `duration`.
#[derive(Clone, Copy, Debug)]
struct RenderSpec {
    note: u8,
    /// MIDI velocity, 0–127.
    velocity: f32,
    duration: f32,
    note_off: f32,
    sample_rate: u32,
}

impl RenderSpec {
    fn notes(&self) -> [OfflineNote; 2] {
        let off_frame = (self.sample_rate as f32 * self.note_off) as usize;
        [
            OfflineNote::on(0, self.note, self.velocity / 127.0),
            OfflineNote::off(off_frame, self.note),
        ]
    }

    fn render(&self, preset: &Preset) -> Vec<(f32, f32)> {
        render_preset(preset, &self.notes(), self.sample_rate, self.duration)
    }
}

fn float_sample_to_pcm_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

/// Write interleaved stereo frames as a 16-bit PCM WAV.
fn write_wav_i16(path: &Path, frames: &[(f32, f32)], sample_rate: u32) -> std::io::Result<()> {
    const CHANNELS: u16 = 2;
    const BYTES_PER_SAMPLE: u16 = 2;
    let data_bytes: Vec<u8> = frames
        .iter()
        .flat_map(|&(l, r)| [l, r])
        .flat_map(|sample| float_sample_to_pcm_i16(sample).to_le_bytes())
        .collect();

    let data_len = data_bytes.len() as u32;
    let chunk_size = 36 + data_len;
    let block_align = CHANNELS * BYTES_PER_SAMPLE;
    let byte_rate = sample_rate * u32::from(block_align);

    let mut wav = Vec::with_capacity(44 + data_bytes.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&chunk_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&CHANNELS.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&(BYTES_PER_SAMPLE * 8).to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&data_bytes);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, wav)
}

#[derive(Clone, Debug)]
struct SampleSummary {
    peak: f32,
    rms: f32,
    checksum: u64,
}

/// Peak, RMS and a bit-level checksum over both channels.
fn summarize_frames(frames: &[(f32, f32)]) -> SampleSummary {
    let mut peak = 0.0_f32;
    let mut energy_sum = 0.0_f64;
    let mut checksum = 0_u64;

    for sample in frames.iter().flat_map(|&(l, r)| [l, r]) {
        peak = peak.max(sample.abs());
        energy_sum += f64::from(sample) * f64::from(sample);
        checksum = checksum
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(u64::from(sample.to_bits()));
    }

    let rms = (energy_sum / (2 * frames.len()) as f64).sqrt() as f32;
    SampleSummary {
        peak,
        rms,
        checksum,
    }
}

fn collect_preset_paths(preset_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut presets = Vec::new();

    for entry in fs::read_dir(preset_dir)? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "corrosion-preset")
        {
            presets.push(path);
        }
    }

    presets.sort();
    Ok(presets)
}

fn ensure_valid_args(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    if args.sample_rate == 0 {
        return Err("sample_rate must be greater than 0".into());
    }

    if !(args.duration.is_finite() && args.duration > 0.0) {
        return Err("duration must be finite and greater than 0".into());
    }

    if !(args.note_off.is_finite() && args.note_off >= 0.0) {
        return Err("note_off must be finite and not negative".into());
    }

    if !(args.velocity.is_finite() && (0.0..=127.0).contains(&args.velocity)) {
        return Err("velocity must be in [0.0, 127.0]".into());
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    ensure_valid_args(&args)?;
    let preset_dir = Path::new(&args.preset_dir);
    let out_dir = Path::new(&args.out_dir);

    let mut preset_paths = collect_preset_paths(preset_dir)?;

    if let Some(contains) = &args.contains {
        preset_paths.retain(|path| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(contains))
        });
    }

    if let Some(limit) = args.limit {
        preset_paths.truncate(limit);
    }

    if preset_paths.is_empty() {
        return Err(format!("No preset files found in {}", preset_dir.display()).into());
    }

    fs::create_dir_all(out_dir)?;
    let mut manifest_lines = Vec::with_capacity(preset_paths.len() + 1);

    let spec = RenderSpec {
        note: args.note,
        velocity: args.velocity,
        duration: args.duration,
        note_off: args.note_off,
        sample_rate: args.sample_rate,
    };
    let frame_count = (args.sample_rate as f32 * args.duration) as usize;

    manifest_lines.push(format!(
        "sample_rate={} duration={} note={} velocity={} note_off={} frame_count={} channels=2 preset_count={}",
        spec.sample_rate,
        spec.duration,
        spec.note,
        spec.velocity,
        spec.note_off,
        frame_count,
        preset_paths.len()
    ));

    for preset_path in preset_paths {
        let preset = Preset::load(&preset_path)?;
        let stem = preset_path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("Invalid preset filename: {}", preset_path.display()))?;

        let output_path = out_dir.join(format!("{stem}.wav"));
        let frames = spec.render(&preset);
        write_wav_i16(&output_path, &frames, spec.sample_rate)?;

        let summary = summarize_frames(&frames);
        let mono: Vec<f32> = frames.iter().map(|&(l, r)| 0.5 * (l + r)).collect();
        let behavior = render_behavior_metrics(&mono);
        let preset_json_name = preset_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| io::Error::other("Invalid preset file name"))?;

        manifest_lines.push(format!(
            concat!(
                "preset={} name=\"{}\" object={} exciter={} wav={} peak={:.6} rms={:.6} checksum={} ",
                "brightness={:.6} roughness={:.6} late_to_early={:.6} zero_crossings={}"
            ),
            preset_json_name,
            preset.name,
            preset.object.name(),
            preset.exciter,
            output_path.display(),
            summary.peak,
            summary.rms,
            summary.checksum,
            behavior.brightness_proxy,
            behavior.roughness_proxy,
            behavior.late_to_early_energy_ratio,
            behavior.zero_crossings,
        ));

        println!(
            "Wrote {} (peak={:.6} rms={:.6} brightness={:.6})",
            output_path.display(),
            summary.peak,
            summary.rms,
            behavior.brightness_proxy,
        );
    }

    let manifest_path = out_dir.join("preset_render_manifest.txt");
    fs::write(&manifest_path, manifest_lines.join("\n"))?;
    println!("Wrote {}", manifest_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec "Regenerated audition set": the WAV the tool writes holds the
    /// offline renderer's output for the preset, quantized to 16 bits.
    #[test]
    fn written_wav_equals_the_quantized_offline_render() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/presets/legacy/sheet_lightning.corrosion-preset");
        let preset = Preset::load(&path).expect("load preset");
        let spec = RenderSpec {
            note: 60,
            velocity: 127.0,
            duration: 0.5,
            note_off: 0.25,
            sample_rate: 48_000,
        };
        let frames = spec.render(&preset);
        let dir = std::env::temp_dir().join(format!("render_presets_test_{}", std::process::id()));
        let wav_path = dir.join("sheet_lightning.wav");
        write_wav_i16(&wav_path, &frames, spec.sample_rate).expect("write wav");
        let bytes = fs::read(&wav_path).expect("read wav");
        fs::remove_dir_all(&dir).ok();

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2, "channels");
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            48_000
        );
        assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 16, "bits");
        let samples: Vec<i16> = bytes[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&b| i16::from_le_bytes(b))
            .collect();
        let expected: Vec<i16> = render_preset(&preset, &spec.notes(), 48_000, 0.5)
            .iter()
            .flat_map(|&(l, r)| [l, r])
            .map(float_sample_to_pcm_i16)
            .collect();
        assert_eq!(samples.len(), 2 * 24_000);
        assert_eq!(samples, expected);
        assert!(
            samples.iter().any(|&s| s.unsigned_abs() > 1000),
            "silent render"
        );
    }
}

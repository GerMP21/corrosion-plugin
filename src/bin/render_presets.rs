use clap::Parser;
use corrosion::dsp::{BodyResonator, CharacterParams};
use corrosion::offline::{render_behavior_metrics, RenderConfig};
use corrosion::presets::Preset;
use corrosion::voice::{VoiceControls, VoiceManager};
use corrosion::{apply_drive, apply_output_limiter, Object, PlayMode, QualityMode};
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
    #[arg(long, default_value = "127")]
    velocity: f32,
    #[arg(long, default_value = "2")]
    duration: f32,
    #[arg(long, default_value_t = 48_000)]
    sample_rate: u32,
    #[arg(long)]
    contains: Option<String>,
    #[arg(long)]
    limit: Option<usize>,
}

fn float_sample_to_pcm_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

fn write_wav_i16(path: &Path, samples: &[f32], sample_rate: u32) -> std::io::Result<()> {
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

#[derive(Clone, Debug)]
struct SampleSummary {
    peak: f32,
    rms: f32,
    checksum: u64,
}

fn summarize_samples(samples: &[f32]) -> SampleSummary {
    let mut peak = 0.0_f32;
    let mut energy_sum = 0.0_f64;
    let mut checksum = 0_u64;

    for sample in samples {
        peak = peak.max(sample.abs());
        energy_sum += f64::from(*sample) * f64::from(*sample);
        checksum = checksum
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(u64::from(sample.to_bits()));
    }

    let rms = (energy_sum / samples.len() as f64).sqrt() as f32;
    SampleSummary {
        peak,
        rms,
        checksum,
    }
}

/// Map the active QualityMode to the resonator mode-count scale. Mirror of
/// `lib.rs::quality_mode_count_scale` — duplicated here so the renderer
/// doesn't depend on a private fn.
fn quality_mode_count_scale(mode: QualityMode) -> f32 {
    match mode {
        QualityMode::Eco => 0.5,
        QualityMode::Normal => 1.0,
        QualityMode::High => 1.5,
        QualityMode::Render => 2.0,
    }
}

/// Build a full `VoiceControls` snapshot from a loaded preset so the renderer
/// triggers exciters with the same per-knob configuration the plugin would
/// at run time. Without this the renderer falls back to `VoiceControls::default()`,
/// which silently disables exciter-specific behavior (TensionSnap thresholds,
/// HeavyGrinding pressure, MSEG loop in Drone mode, character params, …).
fn voice_controls_from_preset(preset: &Preset) -> VoiceControls {
    let extra = &preset.extra;
    let mut controls = VoiceControls {
        env_attack: extra.env_attack,
        env_decay: extra.env_decay,
        env_sustain: extra.env_sustain,
        env_release: extra.env_release,
        mseg_onset: extra.mseg_onset,
        mseg_attack: extra.mseg_attack,
        mseg_hold: extra.mseg_hold,
        mseg_decay: extra.mseg_decay,
        mseg_sustain: extra.mseg_sustain,
        mseg_release: extra.mseg_release,
        env_amount: extra.env_amount,
        velocity_to_peak: extra.velocity_to_peak,
        loop_mode: extra.loop_mode,
        loop_start_stage: extra.loop_start_stage,
        loop_end_stage: extra.loop_end_stage,
        global_time_scale: extra.global_time_scale,
        velocity_to_level: extra.velocity_to_level,
        velocity_to_time: extra.velocity_to_time,
        curve_tension: extra.curve_tension,
        exciter_pressure: extra.exciter_pressure,
        exciter_speed: extra.exciter_speed,
        exciter_roughness: extra.exciter_roughness,
        hand_mass: extra.hand_mass,
        flesh_stiffness: extra.flesh_stiffness,
        flesh_damping: extra.flesh_damping,
        mute_decay: extra.mute_decay,
        mallet_mass: extra.mallet_mass,
        felt_softness: extra.felt_softness,
        core_hardness: extra.core_hardness,
        compression_curve: extra.compression_curve,
        material_stiffness: extra.material_stiffness,
        impact_damping: extra.impact_damping,
        stick_mass: extra.stick_mass,
        tip_stiffness: extra.tip_stiffness,
        restitution_bounciness: extra.restitution_bounciness,
        micro_bounce_limit: extra.micro_bounce_limit,
        wire_density: extra.wire_density,
        spread_duration: extra.spread_duration,
        brush_wire_stiffness: extra.brush_wire_stiffness,
        amplitude_randomization: extra.amplitude_randomization,
        pipe_mass: extra.pipe_mass,
        metal_stiffness: extra.metal_stiffness,
        pipe_pitch: extra.pipe_pitch,
        pipe_ring_decay: extra.pipe_ring_decay,
        link_count: extra.link_count,
        chain_mass: extra.chain_mass,
        drop_envelope_spread: extra.drop_envelope_spread,
        internal_rattle: extra.internal_rattle,
        rattle_color: extra.rattle_color,
        bow_pressure: extra.bow_pressure,
        bow_speed: extra.bow_speed,
        rosin_grip: extra.rosin_grip,
        slip_curve: extra.slip_curve,
        scrape_speed: extra.scrape_speed,
        point_pressure: extra.point_pressure,
        chatter_pitch: extra.chatter_pitch,
        chatter_damping: extra.chatter_damping,
        grind_speed: extra.grind_speed,
        grind_pressure: extra.grind_pressure,
        surface_grit: extra.surface_grit,
        grit_color: extra.grit_color,
        drag_speed: extra.drag_speed,
        ridge_spacing: extra.ridge_spacing,
        ridge_depth: extra.ridge_depth,
        drag_exciter_mass: extra.drag_exciter_mass,
        pull_speed: extra.pull_speed,
        break_threshold: extra.break_threshold,
        slip_stochasticity: extra.slip_stochasticity,
        creak_sharpness: extra.creak_sharpness,
        air_pressure: extra.air_pressure,
        nozzle_width: extra.nozzle_width,
        turbulence_chaos: extra.turbulence_chaos,
        mains_frequency: extra.mains_frequency,
        coil_proximity: extra.coil_proximity,
        voltage_sag: extra.voltage_sag,
        pull_distance: extra.pull_distance,
        hook_stiffness: extra.hook_stiffness,
        snap_force: extra.snap_force,
        flow_rate: extra.flow_rate,
        particle_mass: extra.particle_mass,
        mass_variance: extra.mass_variance,
        strike_position: extra.strike_position,
        coupling_stiffness: extra.coupling_stiffness,
        position_wander: extra.position_wander,
        position_envelope: extra.position_envelope,
        fundamental_anchor: extra.fundamental_anchor,
        res_damping: extra.res_damping,
        res_brightness: extra.res_brightness,
        thickness: extra.thickness,
        heat: extra.heat,
        sludge: extra.sludge,
        character: CharacterParams {
            pipe_diameter: extra.pipe_diameter,
            plate_aspect: extra.plate_aspect,
            plate_stiffness: extra.plate_stiffness,
            tank_volume: extra.tank_volume,
            tank_cavity_mix: extra.tank_cavity_mix,
            chain_link_mass: extra.chain_link_mass,
            chain_instability: extra.chain_instability,
            beam_shear: extra.beam_shear,
            cable_braid: extra.cable_braid,
            cable_tension_drop: extra.cable_tension_drop,
            spring_dispersion: extra.spring_dispersion,
            spring_slosh: extra.spring_slosh,
            sheet_thinness: extra.sheet_thinness,
            cog_dissonance: extra.cog_dissonance,
        },
        mode_count_scale: quality_mode_count_scale(QualityMode::from_int(preset.quality_mode)),
    };

    // Drone mode forces the friction MSEG into a sustained loop (matches the
    // `handle_note_event` Drone branch in `lib.rs`).
    if matches!(PlayMode::from_int(preset.play_mode), PlayMode::Drone) {
        controls.loop_mode = 1; // Forward
        controls.loop_start_stage = 3;
        controls.loop_end_stage = 4;
    }

    controls
}

fn render_preset(
    preset: &Preset,
    note: u8,
    velocity: f32,
    duration: f32,
    sample_rate: u32,
) -> Vec<f32> {
    let frame_count = (sample_rate as f32 * duration) as usize;
    let mut voice_manager = VoiceManager::new();
    let mut body_resonator = BodyResonator::new();
    let profile = match preset.object {
        Object::Pipe => corrosion::dsp::ModalProfileId::Pipe,
        Object::Plate => corrosion::dsp::ModalProfileId::Plate,
        Object::Tank => corrosion::dsp::ModalProfileId::Tank,
        Object::Chain => corrosion::dsp::ModalProfileId::Chain,
        Object::IBeam => corrosion::dsp::ModalProfileId::IBeam,
        Object::TautCable => corrosion::dsp::ModalProfileId::TautCable,
        Object::CoilSpring => corrosion::dsp::ModalProfileId::CoilSpring,
        Object::SheetMetal => corrosion::dsp::ModalProfileId::SheetMetal,
        Object::IndustrialCog => corrosion::dsp::ModalProfileId::IndustrialCog,
    };

    voice_manager.note_on_with_controls(
        note,
        velocity,
        profile,
        preset.size,
        preset.rust,
        preset.damage,
        preset.exciter,
        voice_controls_from_preset(preset),
    );

    let mut output = Vec::with_capacity(frame_count);
    for _ in 0..frame_count {
        let (left_sample, right_sample) =
            voice_manager.process_sample_stereo(sample_rate, preset.width);

        let mut left = apply_drive(left_sample, preset.drive);
        let mut right = apply_drive(right_sample, preset.drive);

        let mono_for_body = (left + right) * 0.5;
        let body_out = body_resonator.process_sample(mono_for_body, sample_rate, preset.body);
        let body_diff = body_out - mono_for_body;
        left += body_diff;
        right += body_diff;

        left *= preset.output;
        right *= preset.output;
        // The preset's Analog Ceiling, as the plugin's Hard limiter uses it.
        let ceiling = preset.extra.analog_ceiling.clamp(0.1, 1.0);
        left = apply_output_limiter(left, ceiling);
        right = apply_output_limiter(right, ceiling);

        let mono = (left + right) * 0.5;
        output.push(mono.clamp(-1.0, 1.0));
    }

    output
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

    let render_config = RenderConfig {
        sample_rate: args.sample_rate,
        frame_count: (args.sample_rate as f32 * args.duration) as usize,
        excitation_frame: 0,
        excitation_amplitude: 1.0,
    };

    manifest_lines.push(format!(
        "sample_rate={} duration={} note={} velocity={} frame_count={} preset_count={}",
        render_config.sample_rate,
        args.duration,
        args.note,
        args.velocity,
        render_config.frame_count,
        preset_paths.len()
    ));

    for preset_path in preset_paths {
        let preset = Preset::load(&preset_path)?;
        let stem = preset_path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("Invalid preset filename: {}", preset_path.display()))?;

        let output_path = out_dir.join(format!("{stem}.wav"));
        let output = render_preset(
            &preset,
            args.note,
            args.velocity,
            args.duration,
            render_config.sample_rate,
        );
        write_wav_i16(&output_path, &output, render_config.sample_rate)?;

        let summary = summarize_samples(&output);
        let behavior = render_behavior_metrics(&output);
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

//! # Corrosion - Industrial Physical Modeling Synthesizer
//!
//! This is the root module of the Corrosion VST plugin, an industrial physical modeling
//! synthesizer built using the nih_plug framework. The plugin implements a modal synthesis
//! engine with 16 different exciter types, 9 resonator objects, and comprehensive
//! post-processing effects.
//!
//! ## Architecture Overview
//!
//! The plugin follows a modular architecture with clear separation of concerns:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        lib.rs (This File)                   │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │   params    │  │    voice    │  │         dsp         │  │
//! │  │  (Params)   │  │  (Voices)   │  │ (Exciters/Resonator)│  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
//! │         │                │                    │              │
//! │         ▼                ▼                    ▼              │
//! │  ┌────────────────────────────────────────────────────────┐ │
//! │  │              PostProcessingChain                       │ │
//! │  │  (Filter → Drive → Body → Spread → Space → Clipper)  │ │
//! │  └────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Module Relationships
//!
//! - **`dsp`** - Digital Signal Processing core: exciters, resonators, transforms, effects
//! - **`voice`** - Voice management: polyphony, voice stealing, per-note state
//! - **`params`** - Parameter definitions and host communication
//! - **`gui`** - Optional egui-based editor (feature-gated)
//! - **`presets`** - Preset serialization/deserialization
//! - **`offline`** - Offline rendering and batch processing utilities
//! - **`randomizer`** - Parameter randomization utilities
//!
//! ## Audio Signal Flow
//!
//! 1. **MIDI Input** → Note events trigger voice allocation in `VoiceManager`
//! 2. **Excitation** → Selected exciter model generates force
//! 3. **Resonator** → Modal resonator filters excitation into pitched sound
//! 4. **Voice Mixing** → Active voices are summed with polyphony limit
//! 5. **Post-Processing** → Global effects chain processes mixed output
//! 6. **Output** → Final stereo pair with limiting
//!
//! ## Real-Time Constraints
//!
//! This plugin is designed for real-time audio processing:
//! - No heap allocations in the audio callback (`process()`)
//! - Lock-free voice management
//! - Sample-accurate MIDI event handling
//! - Denormal number protection

/// Digital Signal Processing module - contains all audio algorithms
pub mod dsp;

/// GUI module - optional egui-based editor interface
pub mod gui;

/// Offline rendering module - batch processing and export utilities
pub mod offline;

/// Parameter definitions - host-facing parameter interface
mod params;

/// Preset management - serialization and preset bank handling
pub mod presets;

/// Parameter randomization - creative variation generation
pub mod randomizer;

/// Voice management - polyphony, voice allocation, per-note processing
pub mod voice;

// External dependencies
use std::num::NonZeroU32;
use std::sync::Arc;

// nih_plug framework for VST/CLAP plugin hosting
use nih_plug::prelude::*;

// Internal imports
use voice::{VoiceControls, VoiceManager};

// Public re-exports for convenience
pub use params::{CorrosionParams, Object, PlayMode, QualityMode};
pub use presets::Preset;

/// Map an incoming MIDI note to a kit-slot object when `PlayMode::Kit` is
/// active. Each object family gets a contiguous block of ~10 MIDI notes
/// across the playable range; outside the C1..D9 block the mapping wraps so
/// the player can still trigger every family at extreme registers.
fn note_to_kit_object(note: u8) -> Object {
    let slot = ((note as i32 - 24).max(0) / 10).rem_euclid(9);
    Object::from_int(slot)
}

/// Map the `sync_rate` knob and host tempo into a normalized echo
/// `delay_time` value (the 0..1 form the `FactoryEcho` expects).
///
/// Below `0.05` the sync is treated as Off and the caller's free-running
/// delay value is returned unchanged. Above that, `sync_rate` selects a
/// musical division and the resulting delay in seconds is converted back
/// into the echo's normalized space `(0.02 + t * 0.78)` so the existing
/// post-chain code path stays unchanged. If the host has not reported a
/// tempo the function falls back to the free-running value (no sync).
fn sync_rate_to_delay_time(
    sync_rate: f32,
    tempo_bpm: Option<f32>,
    free_running_delay_time: f32,
) -> f32 {
    // Off region: passthrough.
    if sync_rate < 0.05 {
        return free_running_delay_time;
    }
    let Some(tempo) = tempo_bpm.filter(|t| t.is_finite() && *t > 0.0) else {
        return free_running_delay_time;
    };

    // 6 discrete musical divisions in BEATS (1/4 note = 1 beat). Ascending
    // so a higher sync_rate yields a longer delay.
    const DIVISIONS: [f32; 6] = [
        0.25, // 1/16 note (0.25 beats)
        0.5,  // 1/8 note
        1.0,  // 1/4 note (quarter)
        2.0,  // 1/2 note (half)
        4.0,  // 1/1 note (whole)
        8.0,  // 2/1 note (double whole)
    ];
    let normalized = ((sync_rate - 0.05) / 0.95).clamp(0.0, 1.0);
    let idx = (normalized * DIVISIONS.len() as f32) as usize;
    let beats = DIVISIONS[idx.min(DIVISIONS.len() - 1)];
    let beat_seconds = 60.0 / tempo;
    let target_seconds = beats * beat_seconds;
    // Echo maps delay_time 0..1 → (0.02 + t * 0.78) seconds.
    ((target_seconds - 0.02) / 0.78).clamp(0.0, 1.0)
}

/// Map the active quality mode to the resonator mode-count multiplier captured
/// at note-on. Eco runs a leaner bank to save CPU; Render trades CPU for modal
/// density. Normal is the unscaled baseline.
#[inline]
fn quality_mode_count_scale(mode: QualityMode) -> f32 {
    match mode {
        QualityMode::Eco => 0.5,
        QualityMode::Normal => 1.0,
        QualityMode::High => 1.5,
        QualityMode::Render => 2.0,
    }
}

/// Bias values derived from the four macro params (Mass, Corrosion, Violence,
/// Brightness). Macros default to `0.5` which produces zero bias; deviation
/// from `0.5` layers the corresponding cluster of effects on top of the
/// per-parameter knobs. The structure is computed once per buffer in
/// `process()` so a single twist of a macro shows up at every downstream
/// destination consistently. See PRD §12 for the routing intent.
#[derive(Clone, Copy, Debug)]
struct MacroBias {
    /// Multiplicative scale applied to mass-like exciter controls and
    /// resonator size.
    mass_factor: f32,
    /// Multiplicative scale on the resonator size param. Smaller than
    /// `mass_factor` so a maxed Mass macro doesn't push the modal bank out
    /// of the calibrated frequency range.
    size_factor: f32,
    /// Additive offset applied to `res_damping` (Corrosion → more damping).
    res_damping_offset: f32,
    /// Additive offset applied to `res_brightness` (Corrosion darkens;
    /// Brightness opens).
    res_brightness_offset: f32,
    /// Additive offset applied to the global `rust` control.
    rust_offset: f32,
    /// Additive offset applied to the global `damage` control.
    damage_offset: f32,
    /// Additive offset applied to the master `drive`.
    drive_offset: f32,
    /// Additive offset applied to the post-chain `drive_amount`.
    drive_amount_offset: f32,
    /// Additive offset applied to the post-chain `chaos_depth`.
    chaos_depth_offset: f32,
    /// Multiplicative scale on the WDF filter cutoff. Brightness ±2 octaves.
    filter_cutoff_factor: f32,
}

impl MacroBias {
    /// Neutral bias — all multipliers `1.0`, all offsets `0.0`. Used by tests
    /// to verify that midpoint macros produce no DSP change.
    #[cfg(test)]
    const NEUTRAL: Self = Self {
        mass_factor: 1.0,
        size_factor: 1.0,
        res_damping_offset: 0.0,
        res_brightness_offset: 0.0,
        rust_offset: 0.0,
        damage_offset: 0.0,
        drive_offset: 0.0,
        drive_amount_offset: 0.0,
        chaos_depth_offset: 0.0,
        filter_cutoff_factor: 1.0,
    };
}

/// Body mix (0–1) for the post chain from the BODY control (0–5).
fn body_mix(body: f32) -> f32 {
    (body / 5.0).clamp(0.0, 1.0)
}

/// Derive the macro bias from the current parameter values.
fn resolve_macro_bias(params: &CorrosionParams) -> MacroBias {
    // Each macro is `0.0..=1.0`; subtract the neutral midpoint so the bias is
    // centered on zero. Below `0.5` reduces, above increases.
    let m = params.macro_mass.value() - 0.5;
    let c = params.macro_corrosion.value() - 0.5;
    let v = params.macro_violence.value() - 0.5;
    let b = params.macro_brightness.value() - 0.5;

    MacroBias {
        // Mass: ±50% on exciter mass cluster, ±25% on resonator size.
        mass_factor: (1.0 + m * 1.0).max(0.1),
        size_factor: (1.0 + m * 0.5).max(0.1),
        // Corrosion adds damping, removes brightness, adds rust.
        res_damping_offset: c * 0.4,
        // Brightness routes positively to brightness; corrosion subtracts.
        res_brightness_offset: -c * 0.4 + b * 0.6,
        rust_offset: c * 2.0,
        // Violence amplifies drive/chaos/damage.
        damage_offset: v * 4.0,
        drive_offset: v * 1.0,
        drive_amount_offset: v * 0.4,
        chaos_depth_offset: v * 0.4,
        // Brightness opens the filter exponentially (±2 octaves at the rails).
        filter_cutoff_factor: 2.0_f32.powf(b * 2.0),
    }
}

/// Apply the macro bias in-place to the `VoiceControls` snapshot used at
/// note-on. Voice-side fields (mass cluster, damping/brightness) get the
/// per-voice layer; everything else (drive, filter, rust/damage/size) is
/// handled at the host call site so the macro hits a single point in each
/// pipeline.
fn apply_macro_bias_to_controls(controls: &mut VoiceControls, bias: &MacroBias) {
    controls.hand_mass *= bias.mass_factor;
    controls.mallet_mass *= bias.mass_factor;
    controls.stick_mass *= bias.mass_factor;
    controls.pipe_mass *= bias.mass_factor;
    controls.chain_mass *= bias.mass_factor;
    controls.drag_exciter_mass *= bias.mass_factor;
    controls.particle_mass *= bias.mass_factor;

    controls.res_damping = (controls.res_damping + bias.res_damping_offset).clamp(0.0, 1.0);
    controls.res_brightness =
        (controls.res_brightness + bias.res_brightness_offset).clamp(0.0, 1.0);
}

/// One-pole smoother used to de-zipper per-sample parameter automation.
///
/// `current += (target - current) * coeff` each sample. Cheap, allocation-free,
/// and bit-exact when the target stops moving.
#[derive(Clone, Copy, Debug)]
struct OnePoleSmoother {
    current: f32,
    target: f32,
    coeff: f32,
}

impl OnePoleSmoother {
    const TAU_SECONDS: f32 = 0.02;

    fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            coeff: 0.0,
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let sr = sample_rate.max(1.0);
        self.coeff = 1.0 - (-1.0 / (Self::TAU_SECONDS * sr)).exp();
    }

    #[inline]
    fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    #[inline]
    fn next(&mut self) -> f32 {
        self.current += (self.target - self.current) * self.coeff;
        self.current
    }
}

/// Output limiter threshold to prevent clipping and ensure headroom.
///
/// Set slightly below 1.0 (0.9661) to allow for inter-sample peaks
/// and provide safety margin for downstream processing.
pub const LIMITER_THRESHOLD: f32 = 0.9661;

/// Applies a hard limiter to a single sample to prevent output clipping.
///
/// This function clamps the sample to the range [-LIMITER_THRESHOLD, LIMITER_THRESHOLD].
/// It's applied as the final stage of the signal chain before output.
///
/// # Arguments
/// * `sample` - Input sample value
///
/// # Returns
/// Limited sample value within safe output range
///
/// # Example
/// ```rust
/// let limited = corrosion::apply_output_limiter(1.5);
/// assert_eq!(limited, corrosion::LIMITER_THRESHOLD);
/// ```
#[inline]
pub fn apply_output_limiter(sample: f32) -> f32 {
    sample.clamp(-LIMITER_THRESHOLD, LIMITER_THRESHOLD)
}

/// Maximum drive-amount-to-gain multiplier in `apply_drive`. A drive of `1.0`
/// brings the input up by `1 + DRIVE_GAIN_SCALE` before the waveshaper.
const DRIVE_GAIN_SCALE: f32 = 4.0;
/// Start of the soft-knee region on the positive half (tube-like curve).
const DRIVE_POS_SOFT_THRESHOLD: f32 = 0.3;
/// Start of the hard-limit region on the positive half.
const DRIVE_POS_HARD_THRESHOLD: f32 = 0.8;
/// Upper bound for the compressed positive output (allows >1.0 to preserve
/// transient harmonics that the downstream limiter handles).
const DRIVE_POS_CEILING: f32 = 1.2;
/// Start of the soft-knee region on the negative half (sharper than positive
/// to emphasize asymmetric harmonics).
const DRIVE_NEG_SOFT_THRESHOLD: f32 = 0.25;
/// Start of the hard-limit region on the negative half.
const DRIVE_NEG_HARD_THRESHOLD: f32 = 0.7;
/// Upper bound for the compressed negative output. Tighter than the positive
/// side so the asymmetry is audible.
const DRIVE_NEG_CEILING: f32 = 1.1;
/// Outer clamp applied to the dry/wet mix; sized so downstream limiters
/// still have headroom even at maximum drive.
const DRIVE_OUTPUT_CLAMP: f32 = 1.5;

/// Applies asymmetric drive/saturation to a sample.
///
/// This implements a multi-stage waveshaper with:
/// - Linear pass-through for low amplitudes
/// - Smooth knee transition using cubic Hermite interpolation
/// - Asymmetric saturation (different curves for positive/negative)
/// - Dry/wet mixing
///
/// The asymmetry creates harmonic complexity typical of industrial textures.
///
/// # Arguments
/// * `sample` - Input sample value
/// * `drive` - Drive amount (0.0 = clean, higher = more saturation)
///
/// # Returns
/// Processed sample with applied drive and saturation
#[inline]
pub fn apply_drive(sample: f32, drive: f32) -> f32 {
    // Early exit if drive is disabled for efficiency
    if drive <= 0.0 {
        return sample;
    }

    // Drive gain scales the input before the saturation curve.
    let drive_gain = 1.0 + drive * DRIVE_GAIN_SCALE;
    let amplified = sample * drive_gain;

    // Extract magnitude and sign for asymmetric processing
    let abs_sample = amplified.abs();
    let sign = amplified.signum();

    // Apply asymmetric waveshaping based on signal polarity
    let shaped = if sign > 0.0 {
        // Positive half: gentler curve, more "tube-like"
        if abs_sample < DRIVE_POS_SOFT_THRESHOLD {
            // Linear region - preserve transients
            amplified
        } else if abs_sample < DRIVE_POS_HARD_THRESHOLD {
            // Soft knee using smoothstep (3t² - 2t³) for musical saturation
            let t = (abs_sample - DRIVE_POS_SOFT_THRESHOLD)
                / (DRIVE_POS_HARD_THRESHOLD - DRIVE_POS_SOFT_THRESHOLD);
            let eased = DRIVE_POS_SOFT_THRESHOLD
                + (DRIVE_POS_HARD_THRESHOLD - DRIVE_POS_SOFT_THRESHOLD) * (t * t * (3.0 - 2.0 * t));
            sign * eased
        } else {
            // Hard limiting with exponential decay tail
            let t = (abs_sample - DRIVE_POS_HARD_THRESHOLD) / (1.0 + drive_gain);
            let compressed =
                DRIVE_POS_HARD_THRESHOLD + (1.0 - DRIVE_POS_HARD_THRESHOLD) * (1.0 - (-t).exp());
            sign * compressed.min(DRIVE_POS_CEILING)
        }
    } else {
        // Negative half: sharper curve, more aggressive limiting
        if abs_sample < DRIVE_NEG_SOFT_THRESHOLD {
            // Linear region
            amplified
        } else if abs_sample < DRIVE_NEG_HARD_THRESHOLD {
            // Soft knee
            let t = (abs_sample - DRIVE_NEG_SOFT_THRESHOLD)
                / (DRIVE_NEG_HARD_THRESHOLD - DRIVE_NEG_SOFT_THRESHOLD);
            let eased = DRIVE_NEG_SOFT_THRESHOLD
                + (DRIVE_NEG_HARD_THRESHOLD - DRIVE_NEG_SOFT_THRESHOLD) * (t * t * (3.0 - 2.0 * t));
            sign * eased
        } else {
            // Hard limiting with tighter ceiling for negative excursions
            let t = (abs_sample - DRIVE_NEG_HARD_THRESHOLD) / (1.0 + drive_gain);
            let compressed =
                DRIVE_NEG_HARD_THRESHOLD + (1.0 - DRIVE_NEG_HARD_THRESHOLD) * (1.0 - (-t).exp());
            sign * compressed.min(DRIVE_NEG_CEILING)
        }
    };

    // Dry/wet mix: more drive = more wet signal
    let dry_wet = shaped * drive + sample * (1.0 - drive * 0.5);
    dry_wet.clamp(-DRIVE_OUTPUT_CLAMP, DRIVE_OUTPUT_CLAMP)
}

/// Handles incoming MIDI note events and routes them to voice management.
///
/// This function is called from the audio thread when MIDI events are received.
/// It:
/// 1. Maps Object parameter to ModalProfileId for resonator selection
/// 2. Collects all current parameter values into VoiceControls
/// 3. Triggers note_on or note_off in the voice manager
///
/// # Arguments
/// * `plugin` - Mutable reference to the plugin instance
/// * `event` - The MIDI note event to process
///
/// # Real-Time Safety
/// This function is called from the audio callback and must not allocate
/// or perform blocking operations.
fn handle_note_event(plugin: &mut Corrosion, event: NoteEvent<()>) {
    match event {
        NoteEvent::NoteOn { note, velocity, .. } => {
            // Decide which object family this note triggers. In Kit mode the
            // note range picks the family; in Tonal/Drone the global object
            // param wins. PRD §9.2 / §9.3.
            let play_mode = PlayMode::from_int(plugin.params.play_mode.value());
            let object = match play_mode {
                PlayMode::Kit => note_to_kit_object(note),
                PlayMode::Tonal | PlayMode::Drone => Object::from_int(plugin.params.object.value()),
            };

            // Map the Object parameter to a modal profile for the resonator
            // Each object has distinct modal characteristics (pipe vs plate vs tank, etc.)
            let profile = match object {
                Object::Pipe => dsp::ModalProfileId::Pipe,
                Object::Plate => dsp::ModalProfileId::Plate,
                Object::Tank => dsp::ModalProfileId::Tank,
                Object::Chain => dsp::ModalProfileId::Chain,
                Object::IBeam => dsp::ModalProfileId::IBeam,
                Object::TautCable => dsp::ModalProfileId::TautCable,
                Object::CoilSpring => dsp::ModalProfileId::CoilSpring,
                Object::SheetMetal => dsp::ModalProfileId::SheetMetal,
                Object::IndustrialCog => dsp::ModalProfileId::IndustrialCog,
            };

            // Resolve the macro bias once for this note-on. We apply the
            // voice-side fields via apply_macro_bias_to_controls below, and
            // bias size/rust/damage at the note_on_with_controls call site.
            let bias = resolve_macro_bias(&plugin.params);

            // Collect all current parameter values into VoiceControls struct
            // This bundles parameters that need to be passed to the voice
            let mut controls = VoiceControls {
                env_attack: plugin.params.env_attack.value(),
                env_decay: plugin.params.env_decay.value(),
                env_sustain: plugin.params.env_sustain.value(),
                env_release: plugin.params.env_release.value(),
                mseg_onset: plugin.params.mseg_onset.value(),
                mseg_attack: plugin.params.mseg_attack.value(),
                mseg_hold: plugin.params.mseg_hold.value(),
                mseg_decay: plugin.params.mseg_decay.value(),
                mseg_sustain: plugin.params.mseg_sustain.value(),
                mseg_release: plugin.params.mseg_release.value(),
                env_amount: plugin.params.env_amount.value(),
                velocity_to_peak: plugin.params.velocity_to_peak.value(),
                loop_mode: plugin.params.loop_mode.value(),
                loop_start_stage: plugin.params.loop_start_stage.value(),
                loop_end_stage: plugin.params.loop_end_stage.value(),
                global_time_scale: plugin.params.global_time_scale.value(),
                velocity_to_level: plugin.params.velocity_to_level.value(),
                velocity_to_time: plugin.params.velocity_to_time.value(),
                curve_tension: plugin.params.curve_tension.value(),
                exciter_pressure: plugin.params.exciter_pressure.value(),
                exciter_speed: plugin.params.exciter_speed.value(),
                exciter_roughness: plugin.params.exciter_roughness.value(),
                hand_mass: plugin.params.hand_mass.value(),
                flesh_stiffness: plugin.params.flesh_stiffness.value(),
                flesh_damping: plugin.params.flesh_damping.value(),
                mute_decay: plugin.params.mute_decay.value(),
                mallet_mass: plugin.params.mallet_mass.value(),
                felt_softness: plugin.params.felt_softness.value(),
                core_hardness: plugin.params.core_hardness.value(),
                compression_curve: plugin.params.compression_curve.value(),
                material_stiffness: plugin.params.material_stiffness.value(),
                impact_damping: plugin.params.impact_damping.value(),
                stick_mass: plugin.params.stick_mass.value(),
                tip_stiffness: plugin.params.tip_stiffness.value(),
                restitution_bounciness: plugin.params.restitution_bounciness.value(),
                micro_bounce_limit: plugin.params.micro_bounce_limit.value(),
                wire_density: plugin.params.wire_density.value(),
                spread_duration: plugin.params.spread_duration.value(),
                brush_wire_stiffness: plugin.params.brush_wire_stiffness.value(),
                amplitude_randomization: plugin.params.amplitude_randomization.value(),
                pipe_mass: plugin.params.pipe_mass.value(),
                metal_stiffness: plugin.params.metal_stiffness.value(),
                pipe_pitch: plugin.params.pipe_pitch.value(),
                pipe_ring_decay: plugin.params.pipe_ring_decay.value(),
                link_count: plugin.params.link_count.value(),
                chain_mass: plugin.params.chain_mass.value(),
                drop_envelope_spread: plugin.params.drop_envelope_spread.value(),
                internal_rattle: plugin.params.internal_rattle.value(),
                rattle_color: plugin.params.rattle_color.value(),
                bow_pressure: plugin.params.bow_pressure.value(),
                bow_speed: plugin.params.bow_speed.value(),
                rosin_grip: plugin.params.rosin_grip.value(),
                slip_curve: plugin.params.slip_curve.value(),
                scrape_speed: plugin.params.scrape_speed.value(),
                point_pressure: plugin.params.point_pressure.value(),
                chatter_pitch: plugin.params.chatter_pitch.value(),
                chatter_damping: plugin.params.chatter_damping.value(),
                grind_speed: plugin.params.grind_speed.value(),
                grind_pressure: plugin.params.grind_pressure.value(),
                surface_grit: plugin.params.surface_grit.value(),
                grit_color: plugin.params.grit_color.value(),
                drag_speed: plugin.params.drag_speed.value(),
                ridge_spacing: plugin.params.ridge_spacing.value(),
                ridge_depth: plugin.params.ridge_depth.value(),
                drag_exciter_mass: plugin.params.drag_exciter_mass.value(),
                pull_speed: plugin.params.pull_speed.value(),
                break_threshold: plugin.params.break_threshold.value(),
                slip_stochasticity: plugin.params.slip_stochasticity.value(),
                creak_sharpness: plugin.params.creak_sharpness.value(),
                air_pressure: plugin.params.air_pressure.value(),
                nozzle_width: plugin.params.nozzle_width.value(),
                turbulence_chaos: plugin.params.turbulence_chaos.value(),
                mains_frequency: plugin.params.mains_frequency.value(),
                coil_proximity: plugin.params.coil_proximity.value(),
                voltage_sag: plugin.params.voltage_sag.value(),
                pull_distance: plugin.params.pull_distance.value(),
                hook_stiffness: plugin.params.hook_stiffness.value(),
                snap_force: plugin.params.snap_force.value(),
                flow_rate: plugin.params.flow_rate.value(),
                particle_mass: plugin.params.particle_mass.value(),
                mass_variance: plugin.params.mass_variance.value(),
                strike_position: plugin.params.strike_position.value(),
                coupling_stiffness: plugin.params.coupling_stiffness.value(),
                position_wander: plugin.params.position_wander.value(),
                position_envelope: plugin.params.position_envelope.value(),
                fundamental_anchor: plugin.params.fundamental_anchor.value(),
                res_damping: plugin.params.res_damping.value(),
                res_brightness: plugin.params.res_brightness.value(),
                thickness: plugin.params.thickness.value(),
                heat: plugin.params.heat.value(),
                sludge: plugin.params.sludge.value(),
                character: dsp::CharacterParams {
                    pipe_diameter: plugin.params.pipe_diameter.value(),
                    plate_aspect: plugin.params.plate_aspect.value(),
                    plate_stiffness: plugin.params.plate_stiffness.value(),
                    tank_volume: plugin.params.tank_volume.value(),
                    tank_cavity_mix: plugin.params.tank_cavity_mix.value(),
                    chain_link_mass: plugin.params.chain_link_mass.value(),
                    chain_instability: plugin.params.chain_instability.value(),
                    beam_shear: plugin.params.beam_shear.value(),
                    cable_braid: plugin.params.cable_braid.value(),
                    cable_tension_drop: plugin.params.cable_tension_drop.value(),
                    spring_dispersion: plugin.params.spring_dispersion.value(),
                    spring_slosh: plugin.params.spring_slosh.value(),
                    sheet_thinness: plugin.params.sheet_thinness.value(),
                    cog_dissonance: plugin.params.cog_dissonance.value(),
                },
                mode_count_scale: quality_mode_count_scale(QualityMode::from_int(
                    plugin.params.quality_mode.value(),
                )),
            };

            // Apply Mass / Corrosion / Brightness macro biases to the voice
            // controls. Violence routes through size/rust/damage and the
            // post-chain (drive, filter), not through VoiceControls.
            apply_macro_bias_to_controls(&mut controls, &bias);

            // Drone mode forces the MSEG loop ON between decay (stage 3) and
            // sustain (stage 4) so friction voices ring indefinitely until
            // note-off. Hit-style families retain their one-shot envelope.
            if matches!(play_mode, PlayMode::Drone) {
                controls.loop_mode = 1; // Forward loop
                controls.loop_start_stage = 3;
                controls.loop_end_stage = 4;
            }

            // Get the selected exciter type (0-16 for different exciters)
            let exciter_type = plugin.params.exciter.value();

            // Macro-biased note-on inputs.
            let biased_size = (plugin.params.size.value() * bias.size_factor).clamp(0.05, 10.0);
            let biased_rust = (plugin.params.rust.value() + bias.rust_offset).clamp(0.0, 5.0);
            let biased_damage =
                (plugin.params.damage.value() + bias.damage_offset).clamp(0.0, 10.0);

            // Trigger note-on in voice manager with all parameters
            // The voice manager handles polyphony and voice allocation
            plugin.voice_manager.note_on_with_controls(
                note,
                note_event_velocity_to_voice_velocity(velocity),
                profile,
                biased_size,
                biased_rust,
                biased_damage,
                exciter_type,
                controls,
            );
        }
        NoteEvent::NoteOff { note, .. } => {
            // Release the voice playing this note
            plugin.voice_manager.note_off(note);
        }
        // Channel pitch bend — host delivers normalized 0.0..=1.0 (0.5 neutral).
        // Convert to semitones using the standard ±2 default range so a plain
        // MIDI controller bends an expected interval.
        NoteEvent::MidiPitchBend { value, .. } => {
            let semitones = (value - 0.5) * 2.0 * voice::PITCH_BEND_RANGE_SEMITONES;
            plugin.voice_manager.set_pitch_bend_semitones(semitones);
        }
        // Channel aftertouch — broadcasts to every voice on the channel.
        NoteEvent::MidiChannelPressure { pressure, .. } => {
            plugin.voice_manager.set_channel_pressure(pressure);
        }
        // Per-note aftertouch — routes only to the matching voice.
        NoteEvent::PolyPressure { note, pressure, .. } => {
            plugin.voice_manager.set_poly_pressure(note, pressure);
        }
        // CC1 mod wheel is the only CC we currently route. Other CCs are
        // ignored but reach the plugin so a future expansion can pick them up
        // without changing MidiConfig.
        NoteEvent::MidiCC { cc: 1, value, .. } => {
            plugin.voice_manager.set_mod_wheel(value);
        }
        _ => {}
    }
}

/// Convert NIH-plug's normalized note velocity into the MIDI-scale velocity
/// expected by the current voice layer.
///
/// `NoteEvent::NoteOn::velocity` is documented by NIH-plug as `[0, 1]`, while
/// `Voice::note_on()` and the existing voice/renderer tests use the traditional
/// MIDI `0..127` scale. Keeping the conversion at the host boundary prevents
/// hosted notes from being attenuated twice before they reach the exciters.
#[inline]
fn note_event_velocity_to_voice_velocity(velocity: f32) -> f32 {
    velocity.clamp(0.0, 1.0) * 127.0
}

/// Processes pending MIDI events that are scheduled for the current sample.
///
/// This function implements sample-accurate MIDI timing by checking if events
/// should be processed at the current sample position. Events with timing
/// less than or equal to the current sample_id are processed immediately.
///
/// # Arguments
/// * `plugin` - Mutable reference to the plugin instance
/// * `sample_id` - Current sample index within the buffer
/// * `next_event` - Pointer to the next pending event (updated as events are consumed)
/// * `fetch_next` - Closure that retrieves the next event from the host
///
/// # Type Parameters
/// * `F` - Closure type for fetching next event
fn process_pending_events<F>(
    plugin: &mut Corrosion,
    sample_id: u32,
    next_event: &mut Option<NoteEvent<()>>,
    mut fetch_next: F,
) where
    F: FnMut() -> Option<NoteEvent<()>>,
{
    // Process all events scheduled for or before the current sample
    while let Some(event) = *next_event {
        if event.timing() > sample_id {
            // Event is in the future, stop processing
            break;
        }

        // Handle the current event
        handle_note_event(plugin, event);

        // Fetch the next event from the host
        *next_event = fetch_next();
    }
}

/// Returns the plugin version from Cargo.toml at compile time.
///
/// # Returns
/// Plugin version string (e.g., "1.0.0")
pub fn corrosion_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Main plugin struct containing all state.
///
/// This struct implements the nih_plug Plugin trait and contains:
/// - Parameter storage (`Arc<CorrosionParams>`)
/// - Voice manager for polyphony
/// - Post-processing effects chain
///
/// # Thread Safety
/// The plugin is designed for single-threaded audio callback usage.
/// Parameters are shared via Arc but are read-only during processing.
pub struct Corrosion {
    /// Shared parameter storage - Arc allows sharing with GUI thread
    params: Arc<CorrosionParams>,

    /// Voice manager handles polyphony, voice allocation, and voice stealing
    voice_manager: VoiceManager,

    /// Post-processing chain applies global effects to mixed output
    post_chain: dsp::PostProcessingChain,

    /// Per-sample smoother for master drive so rapid automation doesn't zipper.
    drive_smoother: OnePoleSmoother,
    /// Per-sample smoother for master output gain.
    output_gain_smoother: OnePoleSmoother,
    /// Optional lookahead peak limiter for the left output channel.
    /// Engaged only when `limiter_mode == 1`; otherwise the master path
    /// falls through to `apply_output_limiter`.
    lookahead_left: dsp::LookaheadLimiter,
    /// Optional lookahead peak limiter for the right output channel.
    lookahead_right: dsp::LookaheadLimiter,
    /// Last reported plugin latency in samples. Tracked so we only call
    /// `context.set_latency_samples` when the value actually changes (toggling
    /// the limiter mode).
    reported_latency_samples: u32,
}

impl Default for Corrosion {
    /// Creates a default plugin instance with initialized subsystems.
    fn default() -> Self {
        let params = Arc::new(CorrosionParams::default());
        let initial_drive = params.drive.value();
        let initial_output = params.output.value();

        Self {
            params,
            voice_manager: VoiceManager::new(),
            post_chain: dsp::PostProcessingChain::new(),
            drive_smoother: OnePoleSmoother::new(initial_drive),
            output_gain_smoother: OnePoleSmoother::new(initial_output),
            lookahead_left: dsp::LookaheadLimiter::new(LIMITER_THRESHOLD),
            lookahead_right: dsp::LookaheadLimiter::new(LIMITER_THRESHOLD),
            reported_latency_samples: 0,
        }
    }
}

impl Corrosion {
    /// Serializes current parameter state to bytes for host state save.
    ///
    /// # Returns
    /// JSON-encoded preset bytes, or empty vector on serialization failure
    pub fn get_state(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(&Preset::from_params(Self::NAME, &self.params))
            .unwrap_or_default()
    }

    /// Restores parameter state from serialized bytes.
    ///
    /// # Arguments
    /// * `state` - Serialized preset bytes from previous get_state() call
    ///
    /// The state goes through `Preset::from_json_str` so legacy versions get
    /// migrated to the current schema before deserialization — same path as
    /// `Preset::load`, ensuring host state-restore and on-disk presets behave
    /// identically.
    pub fn load_state(&mut self, state: &[u8]) {
        let Ok(json) = std::str::from_utf8(state) else {
            return;
        };
        if let Ok(preset) = Preset::from_json_str(json) {
            self.params = Arc::new(preset.into_params());
        }
    }
}

impl Corrosion {
    /// Push the current parameter values (plus macro biases) into the post
    /// chain. Called once per buffer from `process()`; shared with offline
    /// measurement tests so they configure the chain exactly as the plugin does.
    fn apply_post_chain_params(
        &mut self,
        sample_rate: u32,
        host_tempo: Option<f32>,
        bias: &MacroBias,
        body_amount: f32,
    ) {
        self.post_chain.set_sample_rate(sample_rate as f32);
        let biased_cutoff =
            (self.params.filter_cutoff.value() * bias.filter_cutoff_factor).clamp(20.0, 20_000.0);
        self.post_chain.set_filter_params(
            biased_cutoff,
            self.params.filter_resonance.value(),
            self.params.component_tolerance.value(),
        );
        self.post_chain.set_drive_params(
            (self.params.drive_amount.value() + bias.drive_amount_offset).clamp(0.0, 1.0),
            self.params.bias_starvation.value(),
            (self.params.chaos_depth.value() + bias.chaos_depth_offset).clamp(0.0, 1.0),
        );
        // Chassis Volume only tunes the body; BODY alone sets its mix (C5).
        self.post_chain.set_body_params(
            self.params.chassis_material.value(),
            self.params.chassis_volume.value(),
        );
        self.post_chain.set_body_amount(body_amount);
        self.post_chain.set_spread_params(
            self.params.spread_width.value(),
            self.params.listener_proximity.value(),
        );

        // Map space mode integer to enum
        let space_mode = match self.params.space_mode.value() {
            0 => dsp::SpaceMode::Off,
            1 => dsp::SpaceMode::Factory,
            2 => dsp::SpaceMode::Spring,
            3 => dsp::SpaceMode::Echo,
            _ => dsp::SpaceMode::Off,
        };
        self.post_chain.set_space_mode(space_mode);

        let post_quality = match self.params.quality_mode.value() {
            0 => dsp::PostQualityMode::Eco,
            1 => dsp::PostQualityMode::Normal,
            2 => dsp::PostQualityMode::High,
            3 => dsp::PostQualityMode::Render,
            _ => dsp::PostQualityMode::Normal,
        };
        self.post_chain.set_quality_mode(post_quality);
        self.post_chain
            .set_space_amount(self.params.space_amount.value());
        self.post_chain.set_factory_params(
            self.params.factory_size.value(),
            self.params.machinery_clutter.value(),
            self.params.wall_impedance.value(),
        );
        self.post_chain.set_spring_params(
            self.params.spring_tension.value(),
            self.params.wire_stiffness.value(),
            self.params.spring_tank_size.value(),
        );
        // Tempo-synced echo delay (P1.6). When sync_rate is engaged and the
        // host reports a BPM, the sync_rate selects a musical division which
        // overrides the free-running delay_time knob. Otherwise the knob
        // value passes through unchanged. The remap inside
        // `sync_rate_to_delay_time` keeps the echo's existing 0..1 contract.
        let effective_delay_time = sync_rate_to_delay_time(
            self.params.sync_rate.value(),
            host_tempo,
            self.params.delay_time.value(),
        );
        self.post_chain.set_echo_params(
            effective_delay_time,
            self.params.machinery_movement.value(),
            self.params.high_frequency_damping.value(),
        );
        self.post_chain.set_clipper_params(
            self.params.analog_ceiling.value(),
            self.params.diode_softness.value(),
        );
    }
}

impl Plugin for Corrosion {
    // Plugin metadata constants
    const NAME: &'static str = "Corrosion";
    const VENDOR: &'static str = "Corrosion Audio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    /// Audio I/O configuration - stereo output, no input
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,                // No audio input (instrument)
        main_output_channels: NonZeroU32::new(2), // Stereo output
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    /// MIDI input configuration - full CC support so the plugin receives
    /// pitch bend, channel/poly pressure, and mod wheel in addition to notes.
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;

    /// No SysEx message support
    type SysExMessage = ();

    /// No background tasks
    type BackgroundTask = ();

    /// Returns parameter interface for host automation
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    /// Called once when plugin is instantiated.
    ///
    /// # Returns
    /// true if initialization succeeded
    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    /// Resets all plugin state (called when playback starts/stops).
    ///
    /// Clears voice and post-processing state in place. Hosts may call this
    /// on the audio thread, so it must not allocate (review C3).
    fn reset(&mut self) {
        self.voice_manager.reset();
        self.post_chain.reset();
        self.drive_smoother = OnePoleSmoother::new(self.params.drive.value());
        self.output_gain_smoother = OnePoleSmoother::new(self.params.output.value());
        self.lookahead_left.reset();
        self.lookahead_right.reset();
    }

    /// Main audio processing callback - called for each buffer.
    ///
    /// This is the core real-time processing loop. It:
    /// 1. Processes MIDI events with sample-accurate timing
    /// 2. Generates audio from all active voices
    /// 3. Applies post-processing effects
    /// 4. Outputs final stereo signal
    ///
    /// # Real-Time Constraints
    /// - No heap allocations
    /// - No blocking operations
    /// - Must complete within buffer duration
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Get first MIDI event and sample rate from host
        let mut next_event = context.next_event();
        let sample_rate = context.transport().sample_rate as u32;

        // Read parameter values and configure the post-processing chain once per
        // buffer. These setters recompute coefficients (filter tan(), FEM freqs,
        // reverb delays) that must not run at audio rate, and the per-sample
        // version compounded the FactoryReverb comb delays. Host automation is
        // applied at buffer granularity, so per-buffer updates are sufficient.
        let width = self.params.width.value();
        let body_amount = body_mix(self.params.body.value());

        // Resolve the macro bias once for the whole buffer so per-buffer DSP
        // setters and per-buffer held-note automation see the same numbers.
        let bias = resolve_macro_bias(&self.params);

        self.drive_smoother.set_sample_rate(sample_rate as f32);
        self.drive_smoother
            .set_target((self.params.drive.value() + bias.drive_offset).clamp(0.0, 5.0));
        self.output_gain_smoother
            .set_sample_rate(sample_rate as f32);
        self.output_gain_smoother
            .set_target(self.params.output.value());

        // Per-buffer setup for the lookahead limiter (sample rate + threshold).
        // Threshold tracks the analog ceiling so both limiter modes target the
        // same effective peak. The choice between Hard and Lookahead happens
        // per-sample below.
        let limiter_mode = self.params.limiter_mode.value();
        let limiter_threshold = self.params.analog_ceiling.value().clamp(0.1, 1.0);
        self.lookahead_left.set_sample_rate(sample_rate as f32);
        self.lookahead_right.set_sample_rate(sample_rate as f32);
        self.lookahead_left.set_threshold(limiter_threshold);
        self.lookahead_right.set_threshold(limiter_threshold);

        // Report plugin latency to the host. Only update when it changes —
        // hosts may re-negotiate buses on every set_latency_samples call.
        let target_latency = if limiter_mode == 1 {
            dsp::LOOKAHEAD_LIMITER_SAMPLES as u32
        } else {
            0
        };
        if target_latency != self.reported_latency_samples {
            context.set_latency_samples(target_latency);
            self.reported_latency_samples = target_latency;
        }

        let host_tempo = context.transport().tempo.map(|t| t as f32);
        self.apply_post_chain_params(sample_rate, host_tempo, &bias, body_amount);

        // Push subset of controls into still-held voices so sustaining notes
        // respond to damping / brightness / strike-position automation. Drive
        // is already live above (captured each buffer into `drive`). Macros
        // bias damping/brightness here so a Corrosion or Brightness twist also
        // tracks held notes, not just new ones.
        let live_damping =
            (self.params.res_damping.value() + bias.res_damping_offset).clamp(0.0, 1.0);
        let live_brightness =
            (self.params.res_brightness.value() + bias.res_brightness_offset).clamp(0.0, 1.0);
        self.voice_manager.update_live_controls(
            live_damping,
            live_brightness,
            self.params.strike_position.value(),
            self.params.coupling_stiffness.value(),
            self.params.position_wander.value(),
            self.params.position_envelope.value(),
            self.params.fundamental_anchor.value(),
            sample_rate,
        );

        // Process each sample in the buffer
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            // Handle any MIDI events scheduled for this sample
            process_pending_events(self, sample_id as u32, &mut next_event, || {
                context.next_event()
            });

            // Generate stereo audio from all active voices
            let (left_sample, right_sample) =
                self.voice_manager.process_sample_stereo(sample_rate, width);

            let mut left = left_sample;
            let mut right = right_sample;

            // Apply drive/saturation with per-sample smoothed master drive.
            let drive_smoothed = self.drive_smoother.next();
            left = apply_drive(left, drive_smoothed);
            right = apply_drive(right, drive_smoothed);

            // Apply post-processing chain
            let (post_left, post_right) = self.post_chain.process(left, right);
            left = post_left;
            right = post_right;

            // Apply output gain. The limiter choice is per-buffer: Hard
            // (default) clamps each sample; Lookahead introduces ~1 ms of
            // delay and pulls peaks back via the smoothed gain reduction in
            // `LookaheadLimiter`. Either way the master signal stays bounded
            // below the analog ceiling.
            let output_gain_smoothed = self.output_gain_smoother.next();
            left *= output_gain_smoothed;
            right *= output_gain_smoothed;
            if limiter_mode == 1 {
                left = self.lookahead_left.process(left);
                right = self.lookahead_right.process(right);
            } else {
                left = apply_output_limiter(left);
                right = apply_output_limiter(right);
            }

            // Write output to buffer channels
            for (idx, channel_sample) in channel_samples.into_iter().enumerate() {
                if idx == 0 {
                    *channel_sample = left;
                } else if idx == 1 {
                    *channel_sample = right;
                }
            }
        }

        ProcessStatus::Normal
    }

    /// Creates the plugin editor GUI (if gui feature is enabled).
    ///
    /// # Returns
    /// Some(Editor) if GUI is available, None otherwise
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        #[cfg(feature = "gui")]
        {
            gui::create_editor(self.params.clone(), self.params.editor_state.clone())
        }
        #[cfg(not(feature = "gui"))]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_drive, apply_output_limiter, handle_note_event,
        note_event_velocity_to_voice_velocity, process_pending_events, Corrosion, Object,
    };
    use crate::params::object_param;
    use nih_plug::prelude::NoteEvent;
    use std::sync::Arc;

    /// C3 follow-up: `Plugin::reset` with notes ringing through Echo and
    /// Spring must not allocate (checked in debug builds, where the checking
    /// allocator is installed) and must leave the whole chain silent.
    #[test]
    fn body_maps_to_a_zero_to_one_mix() {
        assert_eq!(super::body_mix(0.0), 0.0);
        assert!((super::body_mix(2.62) - 0.524).abs() < 1e-6);
        assert_eq!(super::body_mix(5.0), 1.0);
        assert_eq!(super::body_mix(7.0), 1.0);
        assert_eq!(super::body_mix(-1.0), 0.0);
    }

    #[test]
    fn plugin_reset_with_sounding_space_is_alloc_free_and_silent() {
        use nih_plug::prelude::Plugin;
        for file in ["hanging_chain_echo", "spring_reverb_tap"] {
            let path = format!(
                "{}/presets/factory/{file}.corrosion-preset",
                env!("CARGO_MANIFEST_DIR")
            );
            let preset = crate::presets::Preset::load(std::path::Path::new(&path)).unwrap();
            let mut plugin = Corrosion {
                params: Arc::new(preset.into_params()),
                ..Corrosion::default()
            };
            let params = plugin.params.clone();
            let bias = super::resolve_macro_bias(&params);
            let body_amount = super::body_mix(params.body.value());
            let width = params.width.value();
            plugin.apply_post_chain_params(48_000, None, &bias, body_amount);
            for note in [48, 55, 60, 67] {
                handle_note_event(
                    &mut plugin,
                    NoteEvent::NoteOn {
                        timing: 0,
                        voice_id: None,
                        channel: 0,
                        note,
                        velocity: 1.0,
                    },
                );
            }
            let render = |plugin: &mut Corrosion| {
                let (l, r) = plugin.voice_manager.process_sample_stereo(48_000, width);
                plugin.post_chain.process(l, r)
            };
            let mut loudest = 0.0_f32;
            for _ in 0..24_000 {
                let (l, r) = render(&mut plugin);
                loudest = loudest.max(l.abs()).max(r.abs());
            }
            assert!(loudest > 0.01, "{file}: nothing sounding before reset");

            assert_no_alloc::assert_no_alloc(|| plugin.reset());
            plugin.apply_post_chain_params(48_000, None, &bias, body_amount);
            for i in 0..48_000 {
                let (l, r) = render(&mut plugin);
                assert!(
                    l == 0.0 && r == 0.0,
                    "{file}: sample {i} after reset = ({l}, {r})"
                );
            }
        }
    }

    /// C5: with BODY at 0, Chassis Volume must not bring the body back in
    /// (it used to win a `max()` against BODY).
    #[test]
    fn chassis_volume_does_not_override_body_zero() {
        let render = |chassis_volume: f32| {
            let mut params = crate::params::CorrosionParams::default();
            params.body = crate::params::float_param_for_test("Body", 0.0, 0.0, 5.0);
            params.chassis_volume =
                crate::params::float_param_for_test("Chassis Volume", chassis_volume, 0.0, 1.0);
            let mut plugin = Corrosion {
                params: Arc::new(params),
                ..Corrosion::default()
            };
            let bias = super::resolve_macro_bias(&plugin.params);
            plugin.apply_post_chain_params(48_000, None, &bias, 0.0);
            (0..4_800)
                .map(|i| {
                    let x = 0.1 * (i as f32 * 0.03).sin();
                    plugin.post_chain.process(x, x).0
                })
                .collect::<Vec<_>>()
        };
        let reference = render(0.0);
        for volume in [0.5, 1.0] {
            assert_eq!(render(volume), reference, "chassis volume {volume}");
        }
    }

    /// Tests that multiple note events are processed correctly across a buffer.
    ///
    /// Verifies sample-accurate event timing and energy accumulation.
    #[test]
    fn processes_multiple_note_events_across_buffer() {
        let mut plugin = Corrosion::default();
        let mut single_note_plugin = Corrosion::default();
        let mut events = vec![
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 48,
                velocity: 1.0,
            },
            NoteEvent::NoteOn {
                timing: 3,
                voice_id: None,
                channel: 0,
                note: 52,
                velocity: 1.0,
            },
            NoteEvent::NoteOn {
                timing: 7,
                voice_id: None,
                channel: 0,
                note: 55,
                velocity: 1.0,
            },
        ]
        .into_iter();

        let mut next_event = events.next();

        for sample_id in 0..8 {
            process_pending_events(&mut plugin, sample_id, &mut next_event, || events.next());
        }

        handle_note_event(
            &mut single_note_plugin,
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 48,
                velocity: 1.0,
            },
        );

        // Compare energy of stacked notes vs single note
        let mut stacked_energy = 0.0f32;
        let mut single_note_energy = 0.0f32;
        for _ in 0..48_000 {
            stacked_energy += plugin.voice_manager.process_sample(48_000).abs();
            single_note_energy += single_note_plugin
                .voice_manager
                .process_sample(48_000)
                .abs();
        }

        assert!(
            stacked_energy > single_note_energy * 1.15,
            "stacked notes should accumulate energy, stacked={stacked_energy} single={single_note_energy}"
        );
        assert!(next_event.is_none(), "all queued events should be consumed");
    }

    #[test]
    fn note_event_velocity_is_converted_for_voice_layer() {
        assert_eq!(note_event_velocity_to_voice_velocity(0.0), 0.0);
        assert_eq!(note_event_velocity_to_voice_velocity(0.5), 63.5);
        assert_eq!(note_event_velocity_to_voice_velocity(1.0), 127.0);
    }

    #[test]
    fn hosted_normalized_note_velocity_reaches_output_chain() {
        let mut plugin = Corrosion::default();
        handle_note_event(
            &mut plugin,
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 1.0,
            },
        );

        let sample_rate = 48_000u32;
        // Same parameter → post-chain wiring as `process()` (space mode is
        // already Off by default).
        let bias = super::resolve_macro_bias(&plugin.params);
        let body_amount = super::body_mix(plugin.params.body.value());
        plugin.apply_post_chain_params(sample_rate, None, &bias, body_amount);
        let mut peak = 0.0f32;
        for _ in 0..4096 {
            let (left_sample, right_sample) = plugin
                .voice_manager
                .process_sample_stereo(sample_rate, plugin.params.width.value());

            let mut left = apply_drive(left_sample, plugin.params.drive.value());
            let mut right = apply_drive(right_sample, plugin.params.drive.value());

            (left, right) = plugin.post_chain.process(left, right);
            left = apply_output_limiter(left * plugin.params.output.value());
            right = apply_output_limiter(right * plugin.params.output.value());
            peak = peak.max(left.abs()).max(right.abs());
        }

        eprintln!("hosted normalized note peak={peak:.6}");
        // Level guard: within ±6 dB of the 0.452 reference. It was 0.118
        // (measured when the default BODY, 2.62, was chosen to keep the
        // pre-C5 level); `resolve-exciter-tradeoffs` sets every exciter's
        // drive for a common level instead of its worst note, which raised
        // the default patch about 12 dB.
        assert!(
            (0.226..=0.902).contains(&peak),
            "default note peak {peak} is more than 6 dB from 0.452"
        );
    }

    /// Tests that Object parameter displays correct names.
    #[test]
    fn object_param_displays_names() {
        let params = crate::CorrosionParams::default();

        assert_eq!(params.object.to_string(), Object::Pipe.name());
    }

    /// Tests that changing object parameter affects runtime output.
    #[test]
    fn direct_object_param_changes_runtime_profile() {
        let mut pipe_plugin = Corrosion::default();
        let mut tank_plugin = Corrosion::default();

        Arc::get_mut(&mut pipe_plugin.params).unwrap().object = object_param(Object::Pipe.to_int());
        Arc::get_mut(&mut tank_plugin.params).unwrap().object = object_param(Object::Tank.to_int());

        let event = NoteEvent::NoteOn {
            timing: 0,
            voice_id: None,
            channel: 0,
            note: 60,
            velocity: 1.0,
        };
        handle_note_event(&mut pipe_plugin, event);
        handle_note_event(&mut tank_plugin, event);

        let sample_rate = 48_000u32;
        let pipe_energy: f32 = (0..4096)
            .map(|_| pipe_plugin.voice_manager.process_sample(sample_rate).abs())
            .sum();
        let tank_energy: f32 = (0..4096)
            .map(|_| tank_plugin.voice_manager.process_sample(sample_rate).abs())
            .sum();

        assert_ne!(
            pipe_energy, tank_energy,
            "expected direct object param to change runtime profile output"
        );
    }

    #[test]
    fn neutral_macros_produce_neutral_bias() {
        // Every macro at exactly the default 0.5 must produce the NEUTRAL
        // bias — otherwise existing presets would silently drift.
        let params = crate::CorrosionParams::default();
        let bias = super::resolve_macro_bias(&params);
        let neutral = super::MacroBias::NEUTRAL;
        assert!((bias.mass_factor - neutral.mass_factor).abs() < 1e-6);
        assert!((bias.size_factor - neutral.size_factor).abs() < 1e-6);
        assert!((bias.res_damping_offset - neutral.res_damping_offset).abs() < 1e-6);
        assert!((bias.res_brightness_offset - neutral.res_brightness_offset).abs() < 1e-6);
        assert!((bias.rust_offset - neutral.rust_offset).abs() < 1e-6);
        assert!((bias.damage_offset - neutral.damage_offset).abs() < 1e-6);
        assert!((bias.drive_offset - neutral.drive_offset).abs() < 1e-6);
        assert!((bias.drive_amount_offset - neutral.drive_amount_offset).abs() < 1e-6);
        assert!((bias.chaos_depth_offset - neutral.chaos_depth_offset).abs() < 1e-6);
        assert!((bias.filter_cutoff_factor - neutral.filter_cutoff_factor).abs() < 1e-6);
    }

    #[test]
    fn corrosion_macro_darkens_and_dampens_voice() {
        // Crank Corrosion to the max and confirm the voice gets dimmer and
        // shorter than a neutral copy. We compare two plugin instances —
        // identical except for macro_corrosion — and integrate output energy.
        use crate::params::float_param_for_test;

        let mut bright_plugin = Corrosion::default();
        let mut corroded_plugin = Corrosion::default();
        Arc::get_mut(&mut corroded_plugin.params)
            .unwrap()
            .macro_corrosion = float_param_for_test("Corrosion", 1.0, 0.0, 1.0);

        let event = NoteEvent::NoteOn {
            timing: 0,
            voice_id: None,
            channel: 0,
            note: 60,
            velocity: 1.0,
        };
        handle_note_event(&mut bright_plugin, event);
        handle_note_event(&mut corroded_plugin, event);

        let sample_rate = 48_000u32;
        let bright_energy: f32 = (0..8_192)
            .map(|_| {
                bright_plugin
                    .voice_manager
                    .process_sample(sample_rate)
                    .abs()
            })
            .sum();
        let corroded_energy: f32 = (0..8_192)
            .map(|_| {
                corroded_plugin
                    .voice_manager
                    .process_sample(sample_rate)
                    .abs()
            })
            .sum();

        assert!(
            corroded_energy < bright_energy,
            "max Corrosion should reduce total voice energy: bright={bright_energy}, corroded={corroded_energy}"
        );
    }

    #[test]
    fn violence_macro_drives_post_chain_drive() {
        // Violence routes to the post-chain `drive_amount` smoother target.
        // We invoke the per-buffer setup path indirectly by reading the
        // drive_smoother's target after process_buffer initialization.
        use crate::params::float_param_for_test;

        let mut violent = Corrosion::default();
        Arc::get_mut(&mut violent.params).unwrap().macro_violence =
            float_param_for_test("Violence", 1.0, 0.0, 1.0);

        let bias = super::resolve_macro_bias(&violent.params);
        let neutral_bias = super::resolve_macro_bias(&Corrosion::default().params);
        assert!(
            bias.drive_offset > neutral_bias.drive_offset + 0.4,
            "Violence at 1.0 should push drive offset above the neutral midpoint"
        );
        assert!(bias.damage_offset > 1.5);
        assert!(bias.chaos_depth_offset > 0.15);
    }

    #[test]
    fn brightness_macro_opens_filter() {
        use crate::params::float_param_for_test;

        let mut bright = Corrosion::default();
        Arc::get_mut(&mut bright.params).unwrap().macro_brightness =
            float_param_for_test("Brightness", 1.0, 0.0, 1.0);
        let bias = super::resolve_macro_bias(&bright.params);
        assert!(
            bias.filter_cutoff_factor > 1.5,
            "Brightness at 1.0 should open the filter by > +1 octave: factor={}",
            bias.filter_cutoff_factor
        );
    }

    #[test]
    fn kit_note_to_object_covers_full_range() {
        // Every kit slot must be reachable across the playable keyboard, and
        // notes inside the same slot must resolve to the same object.
        for slot in 0..9 {
            let center_note = 24 + slot * 10 + 5;
            let object = super::note_to_kit_object(center_note as u8);
            assert_eq!(
                object.to_int(),
                slot,
                "slot {slot} should resolve to object {slot} at note {center_note}"
            );
        }

        // Two adjacent notes inside the same slot map identically.
        assert_eq!(
            super::note_to_kit_object(36),
            super::note_to_kit_object(40),
            "notes inside the same kit slot should map to the same object"
        );
        // Adjacent slots map to different objects.
        assert_ne!(
            super::note_to_kit_object(33),
            super::note_to_kit_object(34),
            "notes that cross a slot boundary should map to different objects"
        );
    }

    #[test]
    fn kit_mode_overrides_object_param() {
        // In Kit mode, the note picks the object; the global `object` param
        // is ignored. We verify by setting the param to Pipe (0) and triggering
        // a note in the Tank slot (52-61) — the voice's resonator must be Tank.
        use crate::params::{float_param_for_test, play_mode_param, PlayMode};

        let mut plugin = Corrosion::default();
        Arc::get_mut(&mut plugin.params).unwrap().play_mode =
            play_mode_param(PlayMode::Kit.to_int());
        // Make sure macros don't affect the test.
        let _ = float_param_for_test("Mass", 0.5, 0.0, 1.0);

        // Trigger a note inside the Tank slot (44-53 = `(note-24)/10 == 2`).
        handle_note_event(
            &mut plugin,
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 50,
                velocity: 1.0,
            },
        );

        let active_profile = plugin
            .voice_manager
            .voices_for_test()
            .iter()
            .find(|v| v.is_rendering())
            .map(|v| v.resonator_profile_for_test())
            .expect("a voice should be rendering after note-on");

        assert_eq!(
            active_profile,
            crate::dsp::ModalProfileId::Tank,
            "kit mode should select Tank for MIDI note 50 (Tank slot)"
        );
    }

    #[test]
    fn sync_rate_below_threshold_returns_free_running_value() {
        // sync_rate ≈ 0 must pass through the knob unchanged regardless of
        // whether the host reports a tempo.
        let knob = 0.42;
        assert_eq!(
            super::sync_rate_to_delay_time(0.0, Some(120.0), knob),
            knob,
            "sync off should ignore the host tempo"
        );
        assert_eq!(
            super::sync_rate_to_delay_time(0.0, None, knob),
            knob,
            "sync off without tempo also passes through"
        );
    }

    #[test]
    fn sync_rate_without_tempo_falls_back_to_knob() {
        // Even if sync_rate is engaged, hosts that don't report BPM must not
        // silently change the delay.
        let knob = 0.3;
        assert_eq!(
            super::sync_rate_to_delay_time(0.9, None, knob),
            knob,
            "no tempo should fall back to the free-running value"
        );
    }

    #[test]
    fn sync_rate_quarter_note_at_120bpm() {
        // At 120 BPM a quarter note = 0.5 s. The middle of the sync_rate
        // range lands inside the 1/4 band. Normalized delay_time = (0.5 - 0.02) / 0.78 ≈ 0.615.
        let tempo = Some(120.0);
        let synced = super::sync_rate_to_delay_time(0.5, tempo, 0.25);
        let expected = (0.5 - 0.02) / 0.78;
        assert!(
            (synced - expected).abs() < 0.02,
            "1/4 note @ 120 BPM should give normalized delay ≈ {expected}, got {synced}"
        );
    }

    #[test]
    fn higher_sync_rate_gives_longer_delay() {
        // The mapping is monotone: a larger sync_rate always picks a longer
        // (or equal) musical division.
        let tempo = Some(120.0);
        let mut prev = 0.0_f32;
        for step in 1..=10 {
            let sync = step as f32 / 10.0;
            let delay = super::sync_rate_to_delay_time(sync, tempo, 0.25);
            assert!(
                delay + 1e-4 >= prev,
                "sync_rate->delay must be monotone non-decreasing: sync={sync}, prev={prev}, delay={delay}"
            );
            prev = delay;
        }
    }

    #[test]
    fn drone_mode_forces_mseg_loop_on() {
        // In Drone mode, the friction voice's MSEG must come up with the loop
        // enabled — even if the preset stored loop_mode = 0 (Off).
        use crate::params::{play_mode_param, PlayMode};

        let mut plugin = Corrosion::default();
        Arc::get_mut(&mut plugin.params).unwrap().play_mode =
            play_mode_param(PlayMode::Drone.to_int());
        // Pick a friction exciter (Bow = 1).
        Arc::get_mut(&mut plugin.params).unwrap().exciter = crate::params::exciter_param(1);

        handle_note_event(
            &mut plugin,
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 1.0,
            },
        );

        let voice = plugin
            .voice_manager
            .voices_for_test()
            .iter()
            .find(|v| v.is_rendering())
            .expect("voice should be active");
        assert!(
            voice.mseg_loop_enabled_for_test(),
            "Drone mode should force MSEG loop on for friction voices"
        );
    }
}

/// CLAP plugin format implementation
impl ClapPlugin for Corrosion {
    const CLAP_ID: &'static str = "com.corrosion.corrosion";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Industrial physical modeling synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Instrument, ClapFeature::Synthesizer];
}

/// VST3 plugin format implementation
impl Vst3Plugin for Corrosion {
    const VST3_CLASS_ID: [u8; 16] = *b"CorrosionAudio01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

// Export plugin entry points for CLAP and VST3 hosts.
nih_export_clap!(Corrosion);
// nih-plug's VST3 export expands `vst3_com::vtable`, which trips the
// `semicolon_in_expressions_from_non_local_macros` future-incompat lint on
// newer toolchains (Rust 1.99+). The code is upstream's, so the lint is
// allowed on a wrapper module (a lint attribute on the macro call itself is
// ignored). The exported entry point is `#[no_mangle]`, so the module path
// doesn't change the symbol hosts load.
#[allow(semicolon_in_expressions_from_non_local_macros)]
mod vst3_export {
    use super::Corrosion;
    use nih_plug::prelude::*;

    nih_export_vst3!(Corrosion);
}

/// Offline measurement harness for the C5 post-chain re-routing
/// (OpenSpec change `fix-post-chain-body-routing`). Ignored by default; run
/// with `cargo test --release --no-default-features --lib c5_preset_measurements -- --ignored --nocapture`.
#[cfg(test)]
mod c5_measurements {
    use super::{apply_drive, handle_note_event, resolve_macro_bias, Corrosion, LIMITER_THRESHOLD};
    use crate::dsp::{FemBodyResonator, LorenzDrive, WdfLadderFilter};
    use crate::presets::Preset;
    use nih_plug::prelude::NoteEvent;
    use std::sync::Arc;

    const SAMPLE_RATE: u32 = 48_000;

    /// One preset's measurements: post-chain output peak/RMS and limiter hits
    /// (through the real chain), plus body-vs-dry energies from a replica of
    /// the pre-C5 mono routing.
    pub(crate) struct PresetMeasurement {
        pub name: String,
        pub peak: f32,
        pub rms: f32,
        pub limiter_hits: usize,
        /// Old routing: E(0.7·body) / E(0.3·dry).
        pub old_body_to_dry: f32,
        /// New parallel routing: BODY (0–5) whose body/dry energy ratio
        /// matches `old_body_to_dry`, before clamping to the BODY range.
        pub implied_body: f32,
    }

    pub(crate) fn measure_preset(path: &std::path::Path) -> PresetMeasurement {
        let preset = Preset::load(path).expect("load preset");
        let name = preset.name.clone();
        let mut plugin = Corrosion {
            params: Arc::new(preset.into_params()),
            ..Corrosion::default()
        };
        let params = plugin.params.clone();
        let bias = resolve_macro_bias(&params);
        let body_amount = super::body_mix(params.body.value());
        plugin.apply_post_chain_params(SAMPLE_RATE, None, &bias, body_amount);

        // Replica of the pre-C5 mono filter → drive → body path.
        let sr = SAMPLE_RATE as f32;
        let biased_cutoff =
            (params.filter_cutoff.value() * bias.filter_cutoff_factor).clamp(20.0, 20_000.0);
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(sr);
        filter.set_parameters(
            biased_cutoff,
            params.filter_resonance.value(),
            params.component_tolerance.value(),
        );
        let mut drive = LorenzDrive::new();
        drive.set_sample_rate(sr);
        drive.set_parameters(
            (params.drive_amount.value() + bias.drive_amount_offset).clamp(0.0, 1.0),
            params.bias_starvation.value(),
            (params.chaos_depth.value() + bias.chaos_depth_offset).clamp(0.0, 1.0),
        );
        let mut body = FemBodyResonator::new();
        body.set_sample_rate(sr);
        body.set_parameters(
            params.chassis_material.value(),
            params.chassis_volume.value().max(body_amount),
        );
        // Old body output carried a (0.5 + 0.5·volume) gain the new routing drops.
        let old_body_gain = 0.5 + 0.5 * params.chassis_volume.value().max(body_amount);

        handle_note_event(
            &mut plugin,
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 1.0,
            },
        );

        let master_drive = (params.drive.value() + bias.drive_offset).clamp(0.0, 5.0);
        let width = params.width.value();
        let output = params.output.value();
        let total = SAMPLE_RATE as usize * 2;
        let (mut peak, mut sum_sq, mut hits) = (0.0_f32, 0.0_f64, 0usize);
        let (mut e_dry, mut e_old_body, mut e_driven, mut e_new_body) =
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
        for i in 0..total {
            if i == total / 2 {
                handle_note_event(
                    &mut plugin,
                    NoteEvent::NoteOff {
                        timing: 0,
                        voice_id: None,
                        channel: 0,
                        note: 60,
                        velocity: 0.0,
                    },
                );
            }
            let (l, r) = plugin
                .voice_manager
                .process_sample_stereo(SAMPLE_RATE, width);
            let l = apply_drive(l, master_drive);
            let r = apply_drive(r, master_drive);

            let mono = (l + r) * 0.5;
            let driven = drive.process(filter.process(mono));
            let bodied = body.process(driven);
            e_dry += 0.09 * ((l * l + r * r) as f64);
            e_old_body += 0.49 * 2.0 * (bodied * bodied) as f64;
            e_driven += 2.0 * (driven * driven) as f64;
            let unity_body = bodied / old_body_gain;
            e_new_body += 2.0 * (unity_body * unity_body) as f64;

            let (pl, pr) = plugin.post_chain.process(l, r);
            for x in [pl * output, pr * output] {
                if x.abs() > LIMITER_THRESHOLD {
                    hits += 1;
                }
                let x = x.clamp(-LIMITER_THRESHOLD, LIMITER_THRESHOLD);
                peak = peak.max(x.abs());
                sum_sq += (x * x) as f64;
            }
        }
        let old_body_to_dry = (e_old_body / e_dry.max(1e-30)) as f32;
        // amount² · E(new body) / E(driven) = old ratio → BODY = 5 · amount.
        let amount = ((old_body_to_dry as f64) * e_driven / e_new_body.max(1e-30)).sqrt();
        PresetMeasurement {
            name,
            peak,
            rms: (sum_sq / (2 * total) as f64).sqrt() as f32,
            limiter_hits: hits,
            old_body_to_dry,
            implied_body: (5.0 * amount) as f32,
        }
    }

    /// Render one preset through the current chain with BODY forced to
    /// `body_amount` (0–1), the same note pattern as `measure_preset`.
    /// Returns (peak, rms, limiter hits).
    pub(crate) fn render_preset(
        path: &std::path::Path,
        body_amount: f32,
        output_scale: f32,
    ) -> (f32, f32, usize) {
        frame_stats(&render_preset_frames(path, body_amount, output_scale))
    }

    /// (peak, rms, limiter hits) of rendered frames, after the hard limiter.
    fn frame_stats(frames: &[(f32, f32)]) -> (f32, f32, usize) {
        let (mut peak, mut sum_sq, mut hits) = (0.0_f32, 0.0_f64, 0usize);
        for x in frames.iter().flat_map(|&(l, r)| [l, r]) {
            if x.abs() > LIMITER_THRESHOLD {
                hits += 1;
            }
            let x = x.clamp(-LIMITER_THRESHOLD, LIMITER_THRESHOLD);
            peak = peak.max(x.abs());
            sum_sq += (x * x) as f64;
        }
        (
            peak,
            (sum_sq / (2 * frames.len()) as f64).sqrt() as f32,
            hits,
        )
    }

    /// The stereo output frames (after `output`, before the limiter) of the
    /// note pattern `render_preset` measures: note on, note off at 1 s, 2 s total.
    pub(crate) fn render_preset_frames(
        path: &std::path::Path,
        body_amount: f32,
        output_scale: f32,
    ) -> Vec<(f32, f32)> {
        render_frames(
            Preset::load(path).expect("load preset"),
            body_amount,
            output_scale,
        )
    }

    fn render_frames(preset: Preset, body_amount: f32, output_scale: f32) -> Vec<(f32, f32)> {
        let mut plugin = Corrosion {
            params: Arc::new(preset.into_params()),
            ..Corrosion::default()
        };
        let params = plugin.params.clone();
        let bias = resolve_macro_bias(&params);
        plugin.apply_post_chain_params(SAMPLE_RATE, None, &bias, body_amount);
        handle_note_event(
            &mut plugin,
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 1.0,
            },
        );
        let master_drive = (params.drive.value() + bias.drive_offset).clamp(0.0, 5.0);
        let (width, output) = (params.width.value(), params.output.value() * output_scale);
        let total = SAMPLE_RATE as usize * 2;
        (0..total)
            .map(|i| {
                if i == total / 2 {
                    handle_note_event(
                        &mut plugin,
                        NoteEvent::NoteOff {
                            timing: 0,
                            voice_id: None,
                            channel: 0,
                            note: 60,
                            velocity: 0.0,
                        },
                    );
                }
                let (l, r) = plugin
                    .voice_manager
                    .process_sample_stereo(SAMPLE_RATE, width);
                let (pl, pr) = plugin
                    .post_chain
                    .process(apply_drive(l, master_drive), apply_drive(r, master_drive));
                (pl * output, pr * output)
            })
            .collect()
    }

    /// The default patch: BODY whose default-note peak (the
    /// `hosted_normalized_note_velocity_reaches_output_chain` setup: note 60,
    /// velocity 1, 4096 samples) matches the pre-C5 peak of 0.118.
    #[test]
    #[ignore]
    fn c5_default_body() {
        let peak_at = |amount: f32| {
            let mut plugin = Corrosion::default();
            let params = plugin.params.clone();
            let bias = resolve_macro_bias(&params);
            plugin.apply_post_chain_params(SAMPLE_RATE, None, &bias, amount);
            handle_note_event(
                &mut plugin,
                NoteEvent::NoteOn {
                    timing: 0,
                    voice_id: None,
                    channel: 0,
                    note: 60,
                    velocity: 1.0,
                },
            );
            let md = (params.drive.value() + bias.drive_offset).clamp(0.0, 5.0);
            (0..4_096)
                .map(|_| {
                    let (l, r) = plugin
                        .voice_manager
                        .process_sample_stereo(SAMPLE_RATE, params.width.value());
                    let (pl, pr) = plugin
                        .post_chain
                        .process(apply_drive(l, md), apply_drive(r, md));
                    pl.abs().max(pr.abs())
                })
                .fold(0.0_f32, f32::max)
        };
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for _ in 0..16 {
            let mid = 0.5 * (lo + hi);
            if peak_at(mid) < 0.118 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let amount = 0.5 * (lo + hi);
        println!(
            "DEFAULT body {:.2} → peak {:.4} (BODY 0 peak {:.4}, BODY 5 peak {:.4})",
            amount * 5.0,
            peak_at(amount),
            peak_at(0.0),
            peak_at(1.0)
        );
    }

    /// Each factory preset rendered with its stored BODY and output:
    /// (name, BODY 0–5, stored output, peak, rms, limiter hits).
    fn preset_levels() -> Vec<(String, f32, f32, f32, f32, usize)> {
        factory_preset_paths()
            .into_iter()
            .map(|path| {
                let preset = Preset::load(&path).unwrap();
                let (peak, rms, hits) = render_preset(&path, preset.body / 5.0, 1.0);
                (preset.name, preset.body, preset.output, peak, rms, hits)
            })
            .collect()
    }

    fn median(mut xs: Vec<f32>) -> f32 {
        xs.sort_by(f32::total_cmp);
        xs[xs.len() / 2]
    }

    /// The light level pass for the placeholder presets
    /// (`normalize-object-loudness`, design Decision 4). Keeps each preset's
    /// output unless it hits the limiter (trimmed in 5 % steps until it
    /// doesn't) or its RMS is more than 12 dB below the median (raised to
    /// 12 dB below). Prints the seeder's `PRESET_LEVELS` table.
    #[test]
    #[ignore]
    fn preset_levels_table() {
        let levels = preset_levels();
        let floor = median(levels.iter().map(|l| l.4).collect()) * 10f32.powf(-12.0 / 20.0);
        println!("LEVELS // median floor {floor:.5}");
        for (name, body, output, peak0, rms0, hits0) in levels {
            let path = factory_preset_paths()
                .into_iter()
                .find(|p| Preset::load(p).unwrap().name == name)
                .unwrap();
            let mut scale = 1.0_f32;
            if rms0 < floor {
                scale = floor / rms0.max(1e-9);
            }
            let mut result = render_preset(&path, body / 5.0, scale);
            let mut steps = 0;
            while result.2 > 0 && steps < 200 {
                scale *= 0.95;
                result = render_preset(&path, body / 5.0, scale);
                steps += 1;
            }
            let out = ((output * scale) * 1000.0).round() / 1000.0;
            println!(
                "LEVELS (\"{name}\", {body:.2}, {out:.3}), // rms {:.5} (was {rms0:.5}), peak {:.4} (was {peak0:.4}), hits {} (was {hits0})",
                result.1, result.0, result.2
            );
        }
    }

    /// The factory presets' levels are sane (design Decision 4): none hits
    /// the limiter, and none sits more than 12 dB below the median RMS.
    /// (Replaces the C5 check that pinned every preset within 0.5 dB of its
    /// pre-C5 level, a level that included heavy clipping.)
    #[test]
    #[ignore]
    fn preset_levels_are_sane() {
        let levels = preset_levels();
        let median_rms = median(levels.iter().map(|l| l.4).collect());
        let mut failures = Vec::new();
        for (name, _, _, peak, rms, hits) in &levels {
            let below = 20.0 * (rms / median_rms).log10();
            println!(
                "SANE {name}: rms {rms:.5} ({below:+.1} dB vs median), peak {peak:.4}, hits {hits}"
            );
            if *hits > 0 {
                failures.push(format!("{name}: {hits} limiter hits"));
            }
            if below < -12.0 - 0.1 {
                failures.push(format!("{name}: {below:+.1} dB below the median"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    pub(crate) fn factory_preset_paths() -> Vec<std::path::PathBuf> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/presets/factory");
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .expect("read presets/factory")
            .map(|e| e.expect("dir entry").path())
            .filter(|p| p.extension().is_some_and(|e| e == "corrosion-preset"))
            .collect();
        paths.sort();
        paths
    }

    #[test]
    #[ignore]
    fn c5_preset_measurements() {
        println!("| preset | peak | rms | limiter hits | old body/dry | implied BODY |");
        println!("|---|---|---|---|---|---|");
        for path in factory_preset_paths() {
            let m = measure_preset(&path);
            println!(
                "| {} | {:.4} | {:.5} | {} | {:.3} | {:.2} |",
                m.name, m.peak, m.rms, m.limiter_hits, m.old_body_to_dry, m.implied_body
            );
        }
    }

    /// C6 audition numbers for the factory presets that use Space = Spring:
    /// level, and for the spring's own contribution (render minus the same
    /// render with space amount 0) its L/R balance and correlation and its
    /// share of the output, plus the spring's first-echo delay.
    #[test]
    #[ignore]
    fn c6_spring_preset_measurements() {
        let spring_presets: Vec<_> = factory_preset_paths()
            .into_iter()
            .filter(|p| Preset::load(p).unwrap().into_params().space_mode.value() == 2)
            .collect();
        assert_eq!(spring_presets.len(), 2, "expected two Spring presets");
        println!("| preset | peak | rms | limiter hits | spring/output (dB) | spring side/mid (dB) | spring L/R corr | first echo (ms) |");
        println!("|---|---|---|---|---|---|---|---|");
        for path in spring_presets {
            let preset = Preset::load(&path).unwrap();
            let amount = preset.body / 5.0;
            let frames = render_preset_frames(&path, amount, 1.0);
            let (peak, rms, hits) = frame_stats(&frames);
            let mut dry_preset = Preset::load(&path).unwrap();
            dry_preset.extra.space_amount = 0.0;
            let dry = render_frames(dry_preset, amount, 1.0);
            let wet: Vec<(f32, f32)> = frames
                .iter()
                .zip(&dry)
                .map(|(&(l, r), &(dl, dr))| (l - dl, r - dr))
                .collect();
            let energy = |xs: &[(f32, f32)]| -> f64 {
                xs.iter().map(|&(l, r)| (l * l + r * r) as f64).sum()
            };
            let spring_share_db = 10.0 * (energy(&wet) / energy(&frames).max(1e-30)).log10();

            let (mut e_mid, mut e_side, mut lr, mut ll, mut rr) = (0.0f64, 0.0, 0.0, 0.0, 0.0);
            for &(l, r) in &wet {
                let (l, r) = (l as f64, r as f64);
                e_mid += (0.5 * (l + r)).powi(2);
                e_side += (0.5 * (l - r)).powi(2);
                lr += l * r;
                ll += l * l;
                rr += r * r;
            }
            let side_mid_db = 10.0 * (e_side / e_mid.max(1e-30)).log10();
            let corr = lr / (ll * rr).sqrt().max(1e-30);

            // First-echo delay: an impulse through this preset's post chain,
            // with the spring on minus off. The two are identical until the
            // spring's first echo returns.
            let impulse_mid = |space_amount: f32| -> Vec<f32> {
                let mut p = Preset::load(&path).unwrap();
                p.extra.space_amount = space_amount;
                let mut plugin = Corrosion {
                    params: Arc::new(p.into_params()),
                    ..Corrosion::default()
                };
                let params = plugin.params.clone();
                let bias = resolve_macro_bias(&params);
                plugin.apply_post_chain_params(SAMPLE_RATE, None, &bias, amount);
                (0..SAMPLE_RATE as usize / 4)
                    .map(|i| {
                        let x = if i == 0 { 0.1 } else { 0.0 };
                        let (l, r) = plugin.post_chain.process(x, x);
                        0.5 * (l + r)
                    })
                    .collect()
            };
            let space_amount = preset.extra.space_amount;
            let first_echo = impulse_mid(space_amount)
                .iter()
                .zip(&impulse_mid(0.0))
                .position(|(a, b)| a != b)
                .expect("spring never echoed");
            let echo_ms = first_echo as f32 * 1_000.0 / SAMPLE_RATE as f32;
            println!(
                "| {} | {peak:.4} | {rms:.5} | {hits} | {spring_share_db:.1} | {side_mid_db:.1} | {corr:.3} | {echo_ms:.1} |",
                preset.name
            );
        }
    }
}

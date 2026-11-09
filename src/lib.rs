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
/// active. Each object gets a 10-note slot, in object order, starting at
/// MIDI 24 (C1): 24–33 is object 0, 34–43 object 1, … 104–113 object 8.
/// Outside that block the slots repeat in both directions, so every object
/// can be played at extreme registers: 114–123 is object 0 again, and below
/// the block 14–23 is object 8, 4–13 object 7 and 0–3 object 6 (H14).
fn note_to_kit_object(note: u8) -> Object {
    // `div_euclid` floors toward negative infinity, so the notes below 24
    // continue the slot pattern downward in full 10-note slots.
    let slot = (note as i32 - 24).div_euclid(10).rem_euclid(9);
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
    let Some(division) = params::sync_division(sync_rate) else {
        return free_running_delay_time;
    };
    let Some(tempo) = tempo_bpm.filter(|t| t.is_finite() && *t > 0.0) else {
        return free_running_delay_time;
    };
    let beats = params::SYNC_DIVISIONS[division].0;
    let beat_seconds = 60.0 / tempo;
    let target_seconds = fit_on_the_beat(beats * beat_seconds);
    // Echo maps delay_time 0..1 → (0.02 + t * 0.78) seconds.
    ((target_seconds - 0.02) / 0.78).clamp(0.0, 1.0)
}

/// The echo's longest delay, in seconds (Delay Time 1).
const ECHO_MAX_SECONDS: f32 = 0.8;

/// A synced division longer than the echo can hold is halved until it fits,
/// so the echo stays on the beat (1/1 at 120 BPM plays as 1/4, 0.5 s)
/// rather than stopping at an off-tempo 0.8 s
/// (`h10-scale-space-effects-with-sample-rate`).
fn fit_on_the_beat(seconds: f32) -> f32 {
    let mut seconds = seconds;
    // A tempo is at least a few BPM, so a few dozen halvings always suffice.
    for _ in 0..32 {
        if seconds <= ECHO_MAX_SECONDS {
            break;
        }
        seconds *= 0.5;
    }
    seconds
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

/// The default output ceiling: the Analog Ceiling parameter's default.
///
/// Set slightly below 1.0 (0.9661, about −0.3 dBFS) to allow for inter-sample
/// peaks and provide safety margin for downstream processing. Offline tools
/// and tests that render without the parameter use it as their ceiling.
pub const LIMITER_THRESHOLD: f32 = 0.9661;

/// How long All Sound Off (CC120) takes to fade the output to silence
/// (`m13-sustain-pedal-and-panic`).
const PANIC_FADE_SECONDS: f32 = 0.005;

/// Applies the Hard output limiter to a single sample: clamps it to
/// `[-ceiling, ceiling]`.
///
/// The plugin passes the Analog Ceiling, the same threshold the Lookahead
/// limiter uses, so both modes stop at one ceiling
/// (`h07-hard-limiter-honors-analog-ceiling`). Tools without the parameter
/// pass `LIMITER_THRESHOLD`.
///
/// # Arguments
/// * `sample` - Input sample value
/// * `ceiling` - Largest output magnitude
///
/// # Returns
/// Limited sample value within `±ceiling`
///
/// # Example
/// ```rust
/// let limited = corrosion::apply_output_limiter(1.5, corrosion::LIMITER_THRESHOLD);
/// assert_eq!(limited, corrosion::LIMITER_THRESHOLD);
/// assert_eq!(corrosion::apply_output_limiter(-0.8, 0.5), -0.5);
/// ```
#[inline]
pub fn apply_output_limiter(sample: f32, ceiling: f32) -> f32 {
    sample.clamp(-ceiling, ceiling)
}

/// Host state saved before Wander Rate existed has no `position_wander_rate`;
/// Position Envelope set the wander's rate then. Give such state that rate
/// and Position Envelope 0, the same rule as the preset migration, so an old
/// project reopens sounding as it did (`h09-route-envelope-to-strike-position`).
/// State that has the field is left alone.
fn migrate_host_wander_rate(
    params: &mut std::collections::BTreeMap<String, nih_plug::wrapper::state::ParamValue>,
) {
    use nih_plug::wrapper::state::ParamValue;
    if params.contains_key("position_wander_rate") {
        return;
    }
    let Some(&ParamValue::F32(envelope)) = params.get("position_envelope") else {
        return;
    };
    params.insert(
        "position_wander_rate".into(),
        ParamValue::F32(presets::legacy_wander_rate(envelope)),
    );
    params.insert("position_envelope".into(), ParamValue::F32(0.0));
}

/// Host state saved before Onset Level existed has no `mseg_onset_level`;
/// Onset set the friction MSEG's onset level too, so restore that level
/// (`m01-m04-m05-clarify-parameters`).
fn migrate_host_onset_level(
    params: &mut std::collections::BTreeMap<String, nih_plug::wrapper::state::ParamValue>,
) {
    use nih_plug::wrapper::state::ParamValue;
    if params.contains_key("mseg_onset_level") {
        return;
    }
    let Some(&ParamValue::F32(onset)) = params.get("mseg_onset") else {
        return;
    };
    params.insert(
        "mseg_onset_level".into(),
        ParamValue::F32(presets::legacy_onset_level(onset)),
    );
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
/// Outer clamp applied to the dry/wet mix. The crossfade itself stays
/// within the shaper's ceilings (1.2 / 1.1) for inputs up to ±1, so this
/// only guards larger inputs.
const DRIVE_OUTPUT_CLAMP: f32 = 1.5;

/// Applies asymmetric drive/saturation to a sample.
///
/// This implements a multi-stage waveshaper with:
/// - Linear pass-through for low amplitudes
/// - Smooth knee transition using cubic Hermite interpolation
/// - Asymmetric saturation (different curves for positive/negative)
/// - A dry/shaped crossfade: shaped share `min(drive, 1)`, dry share
///   `1 − min(drive, 1)`, so the weights always sum to 1. Above 1 only the
///   shaper's input gain (`1 + 4·drive`) rises: more saturation, not more
///   level (`h02-normalize-master-drive-mix`).
///
/// The asymmetry creates harmonic complexity typical of industrial textures.
///
/// # Arguments
/// * `sample` - Input sample value
/// * `drive` - Drive amount, 0–5 (0.0 = clean; 1.0 = fully shaped; higher =
///   more saturation)
///
/// # Returns
/// Processed sample with applied drive and saturation
#[inline]
pub fn apply_drive(sample: f32, drive: f32) -> f32 {
    // Early exit if drive is disabled for efficiency
    if drive <= 0.0 {
        return sample;
    }
    let shaped = drive_shape(sample, drive);

    // Crossfade over the first unit of the knob; above 1 the shaper's
    // input gain keeps rising, so more drive is more saturation, not more
    // level (`h02-normalize-master-drive-mix`).
    let wet = drive.min(1.0);
    let dry_wet = shaped * wet + sample * (1.0 - wet);
    dry_wet.clamp(-DRIVE_OUTPUT_CLAMP, DRIVE_OUTPUT_CLAMP)
}

/// The master drive's waveshaper alone: the input scaled by
/// `1 + drive · DRIVE_GAIN_SCALE`, then the asymmetric soft-knee curve.
/// `apply_drive` mixes it with the dry signal.
#[inline]
pub(crate) fn drive_shape(sample: f32, drive: f32) -> f32 {
    // Drive gain scales the input before the saturation curve.
    let drive_gain = 1.0 + drive * DRIVE_GAIN_SCALE;
    let amplified = sample * drive_gain;

    // Extract magnitude and sign for asymmetric processing
    let abs_sample = amplified.abs();
    let sign = amplified.signum();

    // Apply asymmetric waveshaping based on signal polarity
    if sign > 0.0 {
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
    }
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
                mseg_onset_level: plugin.params.mseg_onset_level.value(),
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
                position_wander_rate: plugin.params.position_wander_rate.value(),
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
        // CC1 mod wheel, CC64 sustain, and the channel-mode messages CC120,
        // CC121 and CC123 are routed; other CCs reach the plugin and are
        // ignored.
        NoteEvent::MidiCC { cc: 1, value, .. } => {
            plugin.voice_manager.set_mod_wheel(value);
        }
        // Sustain pedal: down at 64 or above (`m13-sustain-pedal-and-panic`).
        NoteEvent::MidiCC { cc: 64, value, .. } => {
            plugin.voice_manager.set_sustain(value >= 0.5);
        }
        // All Notes Off: every held key released; a down pedal still holds.
        NoteEvent::MidiCC { cc: 123, .. } => {
            plugin.voice_manager.all_notes_off();
        }
        // All Sound Off: a 5 ms fade, then every voice and tail is cleared.
        NoteEvent::MidiCC { cc: 120, .. } => {
            plugin.start_panic();
        }
        // Reset All Controllers: pedal up, mod wheel, bend and pressure at rest.
        NoteEvent::MidiCC { cc: 121, .. } => {
            plugin.voice_manager.set_sustain(false);
            plugin.voice_manager.set_mod_wheel(0.0);
            plugin.voice_manager.set_pitch_bend_semitones(0.0);
            plugin.voice_manager.set_channel_pressure(0.0);
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
    /// falls through to `apply_output_limiter`. Both use the Analog Ceiling.
    lookahead_left: dsp::LookaheadLimiter,
    /// Optional lookahead peak limiter for the right output channel.
    lookahead_right: dsp::LookaheadLimiter,
    /// Last reported plugin latency in samples. Tracked so we only call
    /// `context.set_latency_samples` when the value actually changes (toggling
    /// the limiter mode).
    reported_latency_samples: u32,
    /// All Sound Off fade: the gain applied to the output, and how much it
    /// falls per sample. A step of 0 means no panic is in progress
    /// (`m13-sustain-pedal-and-panic`).
    panic_gain: f32,
    panic_step: f32,
}

/// The per-block values `Corrosion::begin_block` resolves and each
/// output frame of that block reads.
#[derive(Clone, Copy, Debug)]
struct BlockSetup {
    sample_rate: u32,
    width: f32,
    /// 0 Hard, 1 Lookahead.
    limiter_mode: i32,
    /// The Analog Ceiling, which both limiter modes target.
    limiter_threshold: f32,
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
            panic_gain: 1.0,
            panic_step: 0.0,
        }
    }
}

impl Corrosion {
    /// Per-buffer setup of the output limiters: sample rate and threshold.
    /// The threshold is the Analog Ceiling, which both limiter modes target.
    /// Returns the limiter mode (0 Hard, 1 Lookahead) and that threshold.
    fn configure_output_limiters(&mut self, sample_rate: u32) -> (i32, f32) {
        let limiter_mode = self.params.limiter_mode.value();
        let limiter_threshold = self.params.analog_ceiling.value().clamp(0.1, 1.0);
        self.lookahead_left.set_sample_rate(sample_rate as f32);
        self.lookahead_right.set_sample_rate(sample_rate as f32);
        self.lookahead_left.set_threshold(limiter_threshold);
        self.lookahead_right.set_threshold(limiter_threshold);
        (limiter_mode, limiter_threshold)
    }

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
            (self.params.drive_amount.value() + bias.drive_amount_offset).clamp(0.0, 5.0),
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

    /// The latency to report to the host for `limiter_mode` (0 Hard, 1
    /// Lookahead), or `None` when it equals the last reported value. Hosts
    /// may re-negotiate buses on every `set_latency_samples` call, so only
    /// changes are reported.
    fn latency_change(&mut self, limiter_mode: i32) -> Option<u32> {
        let target_latency = if limiter_mode == 1 {
            dsp::LOOKAHEAD_LIMITER_SAMPLES as u32
        } else {
            0
        };
        if target_latency == self.reported_latency_samples {
            return None;
        }
        self.reported_latency_samples = target_latency;
        Some(target_latency)
    }

    /// Per-buffer setup, run once at the start of every block before any
    /// sample is rendered: drive and output-gain smoother targets, limiter
    /// configuration, post-chain parameters (with macro biases) and the live
    /// controls of held voices. These setters recompute coefficients (filter
    /// tan(), FEM freqs, reverb delays) that must not run at audio rate, and
    /// the per-sample version compounded the FactoryReverb comb delays. Host
    /// automation is applied at buffer granularity, so per-buffer updates are
    /// sufficient.
    ///
    /// `process()` and the offline renderer both call it through
    /// `process_block`, so tools and tests run the plugin's exact setup (H13).
    fn begin_block(&mut self, sample_rate: u32, host_tempo: Option<f32>) -> BlockSetup {
        // Every voice, idle ones included, follows a host rate change.
        if self.voice_manager.sample_rate() != sample_rate {
            self.voice_manager.set_sample_rate(sample_rate);
        }
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

        let (limiter_mode, limiter_threshold) = self.configure_output_limiters(sample_rate);

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
            self.params.position_wander_rate.value(),
            self.params.fundamental_anchor.value(),
            self.params.exciter_pressure.value(),
            sample_rate,
        );

        BlockSetup {
            sample_rate,
            width,
            limiter_mode,
            limiter_threshold,
        }
    }

    /// One output frame before the limiter: voices, master drive (smoothed
    /// per sample), the post chain, then the smoothed output gain.
    #[inline]
    fn render_frame_unlimited(&mut self, block: &BlockSetup) -> (f32, f32) {
        // Generate stereo audio from all active voices
        let (left, right) = self
            .voice_manager
            .process_sample_stereo(block.sample_rate, block.width);

        // Apply drive/saturation with per-sample smoothed master drive.
        let drive_smoothed = self.drive_smoother.next();
        let left = apply_drive(left, drive_smoothed);
        let right = apply_drive(right, drive_smoothed);

        // Apply post-processing chain
        let (left, right) = self.post_chain.process(left, right);

        let gain = self.output_gain_smoother.next();
        let (left, right) = (left * gain, right * gain);
        if self.panic_step > 0.0 {
            return self.panic_frame(left, right);
        }
        (left, right)
    }

    /// Start All Sound Off: fade the output to silence over
    /// `PANIC_FADE_SECONDS`, then clear every voice and the post chain.
    fn start_panic(&mut self) {
        let sample_rate = self.voice_manager.sample_rate().max(1) as f32;
        self.panic_step = 1.0 / (PANIC_FADE_SECONDS * sample_rate);
    }

    /// One frame of the All Sound Off fade. At the end of the fade the
    /// voices, post chain and lookahead limiters are cleared in place
    /// (allocation-free), so the output is exactly zero until a new note.
    #[cold]
    #[inline(never)]
    fn panic_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        self.panic_gain -= self.panic_step;
        if self.panic_gain > 0.0 {
            return (left * self.panic_gain, right * self.panic_gain);
        }
        self.voice_manager.silence();
        self.post_chain.reset();
        self.lookahead_left.reset();
        self.lookahead_right.reset();
        self.panic_gain = 1.0;
        self.panic_step = 0.0;
        (0.0, 0.0)
    }

    /// The selected output limiter. Hard (default) clamps each sample;
    /// Lookahead introduces ~1 ms of delay and pulls peaks back via the
    /// smoothed gain reduction in `LookaheadLimiter`. Both stop at the Analog
    /// Ceiling, so it is the output ceiling whatever the gain.
    #[inline]
    fn limit_frame(&mut self, left: f32, right: f32, block: &BlockSetup) -> (f32, f32) {
        if block.limiter_mode == 1 {
            (
                self.lookahead_left.process(left),
                self.lookahead_right.process(right),
            )
        } else {
            (
                apply_output_limiter(left, block.limiter_threshold),
                apply_output_limiter(right, block.limiter_threshold),
            )
        }
    }

    /// One host block: `begin_block`, then for each sample the MIDI events
    /// due at it and one output frame. `write` receives each sample's index
    /// in the block, its output frame (`render_frame_unlimited`, then
    /// `limit_frame`) and the same frame before the limiter
    /// (which level measurements use to count limiter hits).
    ///
    /// This is the plugin's whole audio path below the host API: `process()`
    /// calls it with the host's buffer and events, `offline::render_preset`
    /// with its own (H13). Returns the block's setup so `process()` can report
    /// the limiter latency.
    fn process_block<F, W>(
        &mut self,
        block_len: usize,
        sample_rate: u32,
        host_tempo: Option<f32>,
        next_event: &mut Option<NoteEvent<()>>,
        mut fetch_next: F,
        mut write: W,
    ) -> BlockSetup
    where
        F: FnMut() -> Option<NoteEvent<()>>,
        W: FnMut(usize, (f32, f32), (f32, f32)),
    {
        let block = self.begin_block(sample_rate, host_tempo);
        for sample_id in 0..block_len {
            // Handle any MIDI events scheduled for this sample
            process_pending_events(self, sample_id as u32, next_event, &mut fetch_next);
            let (left, right) = self.render_frame_unlimited(&block);
            let limited = self.limit_frame(left, right, &block);
            write(sample_id, limited, (left, right));
        }
        block
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

    /// Migrate host state (a saved DAW project) before nih-plug restores it.
    /// The host restores parameters by ID, never through the preset loader,
    /// so this applies the preset migrations that would otherwise be skipped.
    fn filter_state(state: &mut PluginState) {
        migrate_host_wander_rate(&mut state.params);
        migrate_host_onset_level(&mut state.params);
    }

    /// Called once when plugin is instantiated.
    ///
    /// # Returns
    /// true if initialization succeeded
    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        // Voices learn the host rate before they play
        // (`m03-m20-start-at-host-rate-and-report-latency`).
        self.voice_manager
            .set_sample_rate(buffer_config.sample_rate as u32);
        // Report the restored Limiter Mode's latency now, before the host's
        // delay compensation is settled; `process()` reports later changes.
        let limiter_mode = self.params.limiter_mode.value();
        let latency = if limiter_mode == 1 {
            dsp::LOOKAHEAD_LIMITER_SAMPLES as u32
        } else {
            0
        };
        context.set_latency_samples(latency);
        self.reported_latency_samples = latency;
        true
    }

    /// Resets all plugin state (called when playback starts/stops).
    ///
    /// Clears voice and post-processing state in place. Hosts may call this
    /// on the audio thread, so it must not allocate (review C3).
    fn reset(&mut self) {
        self.voice_manager.reset();
        self.post_chain.reset();
        // Start the master drive where `begin_block` aims it (Drive plus the
        // Violence bias), so the first note after a reset isn't driven by a
        // ramp from the unbiased Drive (H13).
        let bias = resolve_macro_bias(&self.params);
        self.drive_smoother =
            OnePoleSmoother::new((self.params.drive.value() + bias.drive_offset).clamp(0.0, 5.0));
        self.output_gain_smoother = OnePoleSmoother::new(self.params.output.value());
        self.lookahead_left.reset();
        self.lookahead_right.reset();
        self.panic_gain = 1.0;
        self.panic_step = 0.0;
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
        // Get first MIDI event, sample rate and tempo from the host
        let mut next_event = context.next_event();
        let sample_rate = context.transport().sample_rate as u32;
        let host_tempo = context.transport().tempo.map(|t| t as f32);

        // Everything from here to the output samples is `process_block`,
        // shared with the offline renderer (`offline::render_preset`, H13).
        let block_len = buffer.samples();
        let outputs = buffer.as_slice();
        let block = self.process_block(
            block_len,
            sample_rate,
            host_tempo,
            &mut next_event,
            || context.next_event(),
            |sample_id, (left, right), _unlimited| {
                for (idx, channel) in outputs.iter_mut().enumerate() {
                    if idx == 0 {
                        channel[sample_id] = left;
                    } else if idx == 1 {
                        channel[sample_id] = right;
                    }
                }
            },
        );

        // Report plugin latency to the host. Only update when it changes —
        // hosts may re-negotiate buses on every set_latency_samples call.
        if let Some(latency) = self.latency_change(block.limiter_mode) {
            context.set_latency_samples(latency);
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
        handle_note_event, note_event_velocity_to_voice_velocity, process_pending_events,
        Corrosion, Object,
    };
    use crate::params::object_param;
    use nih_plug::prelude::NoteEvent;
    use std::sync::Arc;

    /// H13: after `reset()` the master drive starts where `begin_block`
    /// aims it, Drive plus the Violence bias (clamped), so the first note
    /// after a host reset is driven like every later one instead of ramping
    /// from the unbiased Drive. The offline renderer starts from a reset, so
    /// this also keeps its renders equal to a settled plugin.
    #[test]
    fn reset_starts_the_drive_smoother_at_the_biased_drive() {
        use nih_plug::prelude::Plugin;
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/presets/factory/chain_whip_snap.corrosion-preset"
        );
        let base = crate::presets::Preset::load(std::path::Path::new(path)).expect("load preset");
        for (violence, drive) in [(0.75, base.drive), (0.2, base.drive), (1.0, 5.0)] {
            let mut preset = base.clone();
            preset.extra.macro_violence = violence;
            preset.drive = drive;
            let mut plugin = Corrosion {
                params: Arc::new(preset.into_params()),
                ..Corrosion::default()
            };
            plugin.reset();
            let bias = super::resolve_macro_bias(&plugin.params);
            assert!(
                bias.drive_offset != 0.0,
                "Violence {violence}: no drive bias"
            );
            let target = (plugin.params.drive.value() + bias.drive_offset).clamp(0.0, 5.0);
            plugin.begin_block(48_000, None);
            let first = plugin.drive_smoother.next();
            assert_eq!(
                first.to_bits(),
                target.to_bits(),
                "Violence {violence}, Drive {drive}: first drive {first}, target {target}"
            );
        }
    }

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
            // Through the plugin's own output path (`process_block`, H13).
            let mut renderer = crate::offline::PresetRenderer::new(&preset);
            let notes = [48, 55, 60, 67].map(|note| crate::offline::OfflineNote::on(0, note, 1.0));
            let before = renderer.render(&notes, 48_000, 24_000).output;
            let loudest = before
                .iter()
                .fold(0.0_f32, |m, &(l, r)| m.max(l.abs()).max(r.abs()));
            assert!(loudest > 0.01, "{file}: nothing sounding before reset");

            assert_no_alloc::assert_no_alloc(|| renderer.plugin_mut().reset());
            let after = renderer.render(&[], 48_000, 48_000).output;
            for (i, &(l, r)) in after.iter().enumerate() {
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
        // The default patch through the plugin's own output path
        // (`process_block`, H13): a host note at velocity 1.0.
        let mut renderer =
            crate::offline::PresetRenderer::from_params(crate::params::CorrosionParams::default());
        let peak = renderer
            .render(&[crate::offline::OfflineNote::on(0, 60, 1.0)], 48_000, 4096)
            .output
            .iter()
            .fold(0.0f32, |m, &(l, r)| m.max(l.abs()).max(r.abs()));

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

    /// RMS of a quiet 1 kHz tone through the post chain, set up the way
    /// `process()` sets it, with `drive_amount` and the Violence macro
    /// (neutral at 0.5).
    fn post_chain_drive_rms(drive_amount: f32, violence: f32) -> f32 {
        use crate::params::float_param_for_test;
        let mut plugin = Corrosion::default();
        {
            let params = Arc::get_mut(&mut plugin.params).unwrap();
            params.drive_amount = float_param_for_test("Drive Amount", drive_amount, 0.0, 5.0);
            params.macro_violence = float_param_for_test("Violence", violence, 0.0, 1.0);
        }
        let bias = super::resolve_macro_bias(&plugin.params);
        plugin.apply_post_chain_params(48_000, None, &bias, 0.0);
        let mut sum = 0.0f64;
        for i in 0..4_800 {
            let x = 0.05 * (std::f32::consts::TAU * 1_000.0 * i as f32 / 48_000.0).sin();
            let (l, _) = plugin.post_chain.process(x, x);
            sum += (l * l) as f64;
        }
        (sum / 4_800.0).sqrt() as f32
    }

    // --- Master drive mix (`h02-normalize-master-drive-mix`) ---

    #[test]
    fn master_drive_crossfades_below_one() {
        for x in [-0.6f32, -0.05, 0.01, 0.3, 0.9] {
            let expected = 0.5 * x + 0.5 * super::drive_shape(x, 0.5);
            let out = super::apply_drive(x, 0.5);
            assert!(
                (out - expected).abs() < 1e-6,
                "drive 0.5, input {x}: {out} vs crossfade {expected}"
            );
        }
    }

    #[test]
    fn master_drive_has_no_dry_from_one_up() {
        for drive in [1.0f32, 2.5, 5.0] {
            for x in [-0.6f32, -0.05, 0.01, 0.3, 0.9] {
                let (out, shaped) = (super::apply_drive(x, drive), super::drive_shape(x, drive));
                assert!(
                    (out - shaped).abs() < 1e-6,
                    "drive {drive}, input {x}: {out} has a dry part (shaped {shaped})"
                );
            }
        }
    }

    #[test]
    fn master_drive_never_inverts() {
        for drive in [0.0f32, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0] {
            for x in [-1.0f32, -0.3, -0.01, 0.0, 0.01, 0.3, 1.0] {
                let out = super::apply_drive(x, drive);
                assert!(
                    out.signum() == x.signum() || (x == 0.0 && out == 0.0),
                    "drive {drive}, input {x}: output {out} has the wrong sign"
                );
            }
        }
    }

    /// Share of output power outside the 1 kHz fundamental for a
    /// 0.25-amplitude 1 kHz sine through the master drive at 48 kHz.
    fn master_drive_harmonic_share(drive: f32) -> f64 {
        let n = 4_800; // 100 whole periods
        let out: Vec<f64> = (0..n)
            .map(|i| {
                let x = 0.25 * (std::f32::consts::TAU * 1_000.0 * i as f32 / 48_000.0).sin();
                super::apply_drive(x, drive) as f64
            })
            .collect();
        let total: f64 = out.iter().map(|y| y * y).sum::<f64>() / n as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, y) in out.iter().enumerate() {
            let phase = std::f64::consts::TAU * 1_000.0 * i as f64 / 48_000.0;
            re += y * phase.cos();
            im += y * phase.sin();
        }
        let fundamental = 2.0 * (re * re + im * im) / (n as f64 * n as f64);
        1.0 - fundamental / total
    }

    #[test]
    fn higher_master_drive_saturates_more() {
        let shares = [1.0f32, 2.0, 5.0].map(master_drive_harmonic_share);
        assert!(
            shares[0] < shares[1] && shares[1] < shares[2],
            "harmonic share at drive 1/2/5: {shares:?}"
        );
    }

    #[test]
    fn drive_amount_above_one_reaches_the_drive() {
        // `h01-unclamp-post-drive-amount`: the whole 0–5 range is audible.
        let (one, three, five) = (
            post_chain_drive_rms(1.0, 0.5),
            post_chain_drive_rms(3.0, 0.5),
            post_chain_drive_rms(5.0, 0.5),
        );
        assert!(
            three > one * 1.05 && five > three * 1.05,
            "drive amounts 1/3/5 give RMS {one:.5}/{three:.5}/{five:.5}"
        );
    }

    #[test]
    fn violence_pushes_drive_amount_past_one() {
        let (plain, violent) = (
            post_chain_drive_rms(1.0, 0.5),
            post_chain_drive_rms(1.0, 1.0),
        );
        assert!(
            violent > plain * 1.02,
            "Violence at drive amount 1: RMS {violent:.5} vs {plain:.5}"
        );
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

    /// Kit slots for (note, expected object), checked one by one.
    fn assert_kit_slots(cases: &[(u8, i32)]) {
        for &(note, object) in cases {
            assert_eq!(
                super::note_to_kit_object(note).to_int(),
                object,
                "MIDI {note} should play object {object}"
            );
        }
    }

    /// H14 spec "Inside the main block".
    #[test]
    fn kit_notes_inside_the_block_map_to_their_slot() {
        assert_kit_slots(&[(24, 0), (33, 0), (34, 1), (113, 8)]);
    }

    /// H14 spec "Above the block": the slots repeat upward.
    #[test]
    fn kit_notes_above_the_block_wrap() {
        assert_kit_slots(&[(114, 0), (127, 1)]);
    }

    /// H14 spec "Below the block": the slots repeat downward too, instead of
    /// every note under MIDI 24 playing Pipe.
    #[test]
    fn kit_notes_below_the_block_wrap() {
        assert_kit_slots(&[(23, 8), (14, 8), (13, 7), (4, 7), (3, 6), (0, 6)]);
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

    /// The sync parameter's displayed text at `value`.
    fn sync_display(params: &crate::params::CorrosionParams, value: f32) -> String {
        use nih_plug::prelude::Param;
        let normalized = params.sync_rate.preview_normalized(value);
        params
            .sync_rate
            .normalized_value_to_string(normalized, false)
    }

    /// The echo's delay in seconds for a normalized delay time.
    fn echo_seconds(delay_time: f32) -> f32 {
        0.02 + delay_time * 0.78
    }

    #[test]
    fn fresh_instance_has_sync_off() {
        // `h03-default-tempo-sync-off`: with the host reporting 120 BPM, the
        // default sync leaves Delay Time in charge.
        let plugin = Corrosion::default();
        let default = plugin.params.sync_rate.value();
        for knob in [0.1f32, 0.42, 0.9] {
            assert_eq!(
                super::sync_rate_to_delay_time(default, Some(120.0), knob),
                knob,
                "default sync_rate {default} should leave Delay Time {knob} free-running"
            );
        }
    }

    #[test]
    fn sync_display_names_the_division() {
        let plugin = Corrosion::default();
        let params = &plugin.params;
        for (value, text, seconds) in [
            (0.0f32, "Off", None),
            (0.3, "1/8", Some(0.25)),
            // 2/1 is 4 s at 120 BPM: halved to 0.5 s, on the beat.
            (1.0, "2/1", Some(0.5)),
        ] {
            assert_eq!(sync_display(params, value), text, "display at {value}");
            let delay = echo_seconds(super::sync_rate_to_delay_time(value, Some(120.0), 0.42));
            match seconds {
                None => assert!(
                    (delay - echo_seconds(0.42)).abs() < 1e-6,
                    "{value}: free-running"
                ),
                Some(s) => assert!(
                    (delay - s).abs() < 1e-4,
                    "{value}: delay {delay} s, expected {s} s"
                ),
            }
        }
    }

    #[test]
    fn typed_sync_division_round_trips() {
        use nih_plug::prelude::Param;
        let plugin = Corrosion::default();
        let param = &plugin.params.sync_rate;
        let names = std::iter::once("Off").chain(crate::params::SYNC_DIVISIONS.iter().map(|d| d.1));
        for name in names {
            let normalized = param
                .string_to_normalized_value(name)
                .unwrap_or_else(|| panic!("{name} should parse"));
            assert_eq!(param.normalized_value_to_string(normalized, false), name);
        }
    }

    #[test]
    fn sync_display_matches_behaviour_across_the_range() {
        let plugin = Corrosion::default();
        let params = &plugin.params;
        for step in 0..=100 {
            let value = step as f32 / 100.0;
            let shown = sync_display(params, value);
            let expected = match crate::params::sync_division(value) {
                None => "Off",
                Some(i) => crate::params::SYNC_DIVISIONS[i].1,
            };
            assert_eq!(shown, expected, "display at {value}");
            // The division the echo plays at 60 BPM, where one beat is 1 s.
            let delay = echo_seconds(super::sync_rate_to_delay_time(value, Some(60.0), 0.5));
            match crate::params::sync_division(value) {
                None => assert!((delay - echo_seconds(0.5)).abs() < 1e-6),
                Some(i) => {
                    // Halved until it fits in 0.8 s, so it stays on the beat.
                    let beats = crate::params::SYNC_DIVISIONS[i].0;
                    let mut expected = beats;
                    while expected > 0.8 {
                        expected *= 0.5;
                    }
                    assert!(
                        (delay - expected).abs() < 1e-4,
                        "{value}: {delay} s for {beats} beats"
                    );
                }
            }
        }
    }

    #[test]
    fn long_synced_divisions_stay_on_the_beat() {
        // At 120 BPM a beat is 0.5 s. Divisions that fit play as written;
        // longer ones are halved until they fit, landing on 1/4 here.
        let tempo = Some(120.0);
        for (division, beats, expected) in [
            ("1/8", 0.5f32, 0.25f32),
            ("1/4", 1.0, 0.5),
            ("1/2", 2.0, 0.5),
            ("1/1", 4.0, 0.5),
            ("2/1", 8.0, 0.5),
        ] {
            let index = crate::params::SYNC_DIVISIONS
                .iter()
                .position(|d| d.1 == division)
                .unwrap();
            assert_eq!(crate::params::SYNC_DIVISIONS[index].0, beats);
            // The middle of the division's band on the Sync control.
            let bands = crate::params::SYNC_DIVISIONS.len() as f32;
            let off = crate::params::SYNC_OFF_BELOW;
            let value = off + (index as f32 + 0.5) / bands * (1.0 - off);
            let delay = echo_seconds(super::sync_rate_to_delay_time(value, tempo, 0.42));
            assert!(
                (delay - expected).abs() < 1e-4,
                "{division} at 120 BPM: {delay} s, expected {expected} s"
            );
        }
        // At 90 BPM (0.667 s a beat) 1/1 is 2.667 s: halved twice to 0.667 s.
        let value = super::fit_on_the_beat(4.0 * 60.0 / 90.0);
        assert!((value - 60.0 / 90.0).abs() < 1e-5, "{value}");
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
            (params.drive_amount.value() + bias.drive_amount_offset).clamp(0.0, 5.0),
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

    /// Render one preset through the plugin's output path with BODY forced
    /// to `body` (0–5) and its output scaled by `output_scale`: note 60 at
    /// full velocity, released at 1 s, 2 s total. Returns (peak, rms, limiter
    /// hits).
    pub(crate) fn render_preset(
        path: &std::path::Path,
        body: f32,
        output_scale: f32,
    ) -> (f32, f32, usize) {
        frame_stats(&render_preset_frames(path, body, output_scale))
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
        body: f32,
        output_scale: f32,
    ) -> Vec<(f32, f32)> {
        render_frames(Preset::load(path).expect("load preset"), body, output_scale)
    }

    /// `preset` with BODY set to `body` (0–5) and its output scaled by
    /// `output_scale`, rendered by the offline renderer (the plugin's own
    /// output path, H13). Returns the frames before the output limiter, so
    /// the level checks can count limiter hits.
    fn render_frames(mut preset: Preset, body: f32, output_scale: f32) -> Vec<(f32, f32)> {
        preset.body = body;
        preset.output *= output_scale;
        let notes = [
            crate::offline::OfflineNote::on(0, 60, 1.0),
            crate::offline::OfflineNote::off(SAMPLE_RATE as usize, 60),
        ];
        crate::offline::render_preset_pre_limiter(&preset, &notes, SAMPLE_RATE, 2.0)
    }

    /// The default patch: BODY whose default-note peak (the
    /// `hosted_normalized_note_velocity_reaches_output_chain` setup: note 60,
    /// velocity 1, 4096 samples) matches the pre-C5 peak of 0.118.
    #[test]
    #[ignore]
    fn c5_default_body() {
        // Through the plugin's own output path (`process_block`, H13); the
        // default output is 1, so the pre-limiter frames are the post chain's.
        let peak_at = |amount: f32| {
            let mut params = crate::params::CorrosionParams::default();
            params.body = crate::params::float_param_for_test("Body", amount * 5.0, 0.0, 5.0);
            crate::offline::PresetRenderer::from_params(params)
                .render(
                    &[crate::offline::OfflineNote::on(0, 60, 1.0)],
                    SAMPLE_RATE,
                    4_096,
                )
                .pre_limiter
                .iter()
                .fold(0.0_f32, |m, &(l, r)| m.max(l.abs()).max(r.abs()))
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
                let (peak, rms, hits) = render_preset(&path, preset.body, 1.0);
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
    ///
    /// An output above 1 is a raise from an earlier pass, so it starts from
    /// 1 again and the floor rule re-derives it (`resolve-exciter-tradeoffs`:
    /// the raises compensated for exciters that are now at a common level).
    #[test]
    #[ignore]
    fn preset_levels_table() {
        let levels: Vec<_> = preset_levels()
            .into_iter()
            .map(|(name, body, output, peak, rms, hits)| {
                if output > 1.0 {
                    let path = factory_preset_paths()
                        .into_iter()
                        .find(|p| Preset::load(p).unwrap().name == name)
                        .unwrap();
                    let (peak, rms, hits) = render_preset(&path, body, 1.0 / output);
                    (name, body, 1.0, peak, rms, hits)
                } else {
                    (name, body, output, peak, rms, hits)
                }
            })
            .collect();
        let floor = median(levels.iter().map(|l| l.4).collect()) * 10f32.powf(-12.0 / 20.0);
        println!("LEVELS // median floor {floor:.5}");
        for (name, body, output, peak0, rms0, hits0) in levels {
            let path = factory_preset_paths()
                .into_iter()
                .find(|p| Preset::load(p).unwrap().name == name)
                .unwrap();
            let stored = Preset::load(&path).unwrap().output;
            // `render_preset` scales the stored output; start from `output`.
            let mut scale = output / stored;
            if rms0 < floor {
                scale *= floor / rms0.max(1e-9);
            }
            let mut result = render_preset(&path, body, scale);
            let mut steps = 0;
            while result.2 > 0 && steps < 200 {
                scale *= 0.95;
                result = render_preset(&path, body, scale);
                steps += 1;
            }
            let out = ((stored * scale) * 1000.0).round() / 1000.0;
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
            let frames = render_preset_frames(&path, preset.body, 1.0);
            let (peak, rms, hits) = frame_stats(&frames);
            let mut dry_preset = Preset::load(&path).unwrap();
            dry_preset.extra.space_amount = 0.0;
            let dry = render_frames(dry_preset, preset.body, 1.0);
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
                plugin.apply_post_chain_params(SAMPLE_RATE, None, &bias, preset.body / 5.0);
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

/// The output ceiling in both limiter modes (`h07-hard-limiter-honors-analog-ceiling`).
#[cfg(test)]
mod output_ceiling_tests {
    use super::{Corrosion, LIMITER_THRESHOLD};
    use crate::params::{limiter_mode_param, CorrosionParams};
    use nih_plug::prelude::{FloatParam, FloatRange};
    use std::sync::Arc;

    const SAMPLE_RATE: u32 = 48_000;

    /// A full-velocity C4 through the whole plugin path (voices, master
    /// drive, post chain, output gain, limiter; `process_block`) at `ceiling`,
    /// output `gain` and
    /// limiter `mode`: half a second, 48 kHz.
    fn render(ceiling: f32, gain: f32, mode: i32) -> Vec<(f32, f32)> {
        let params = CorrosionParams {
            analog_ceiling: FloatParam::new(
                "Analog Ceiling",
                ceiling,
                FloatRange::Linear { min: 0.5, max: 1.0 },
            ),
            limiter_mode: limiter_mode_param(mode),
            output: crate::params::float_param_for_test("Output", gain, 0.0, 100.0),
            ..CorrosionParams::default()
        };
        crate::offline::PresetRenderer::from_params(params)
            .render(
                &[crate::offline::OfflineNote::on(0, 60, 1.0)],
                SAMPLE_RATE,
                SAMPLE_RATE as usize / 2,
            )
            .output
    }

    fn peak(frames: &[(f32, f32)]) -> f32 {
        frames
            .iter()
            .fold(0.0f32, |m, &(l, r)| m.max(l.abs()).max(r.abs()))
    }

    #[test]
    fn hard_mode_stays_under_a_low_ceiling() {
        let peak = peak(&render(0.5, 4.0, 0));
        assert!(peak <= 0.5, "Hard, ceiling 0.5, gain 4: peak {peak:.4}");
    }

    #[test]
    fn both_limiter_modes_reach_the_same_ceiling() {
        for (mode, name) in [(0, "Hard"), (1, "Lookahead")] {
            let peak = peak(&render(0.7, 4.0, mode));
            assert!(
                (0.65..=0.7).contains(&peak),
                "{name}, ceiling 0.7: peak {peak:.4}, expected 0.65..=0.7"
            );
        }
    }

    #[test]
    fn factory_preset_is_bit_identical_at_the_default_ceiling() {
        let path = &super::c5_measurements::factory_preset_paths()[0];
        let preset = crate::presets::Preset::load(path).expect("load preset");
        assert_eq!(preset.extra.analog_ceiling, LIMITER_THRESHOLD);
        assert_eq!(preset.extra.limiter_mode, 0);
        let mut plugin = Corrosion {
            params: Arc::new(preset.into_params()),
            ..Corrosion::default()
        };
        let (mode, threshold) = plugin.configure_output_limiters(SAMPLE_RATE);
        let block = super::BlockSetup {
            sample_rate: SAMPLE_RATE,
            width: 1.0,
            limiter_mode: mode,
            limiter_threshold: threshold,
        };
        // Frames after output gain, before the limiter. Output ×4 so the
        // clamp is exercised, not just passed through.
        let frames = super::c5_measurements::render_preset_frames(path, 2.5, 4.0);
        let mut over = 0;
        for &(l, r) in &frames {
            let (ol, or) = plugin.limit_frame(l, r, &block);
            let fixed = (
                l.clamp(-LIMITER_THRESHOLD, LIMITER_THRESHOLD),
                r.clamp(-LIMITER_THRESHOLD, LIMITER_THRESHOLD),
            );
            assert_eq!(
                (ol.to_bits(), or.to_bits()),
                (fixed.0.to_bits(), fixed.1.to_bits()),
                "{}: output differs from a fixed {LIMITER_THRESHOLD} clamp",
                path.display()
            );
            if l.abs() > LIMITER_THRESHOLD || r.abs() > LIMITER_THRESHOLD {
                over += 1;
            }
        }
        assert!(over > 0, "{}: no sample reached the clamp", path.display());
    }
}

/// Host-state migration (`h09-route-envelope-to-strike-position`).
#[cfg(test)]
mod host_state_tests {
    use super::Corrosion;
    use nih_plug::prelude::{Param, Plugin, PluginState};
    use nih_plug::wrapper::state::ParamValue;

    fn state(params: &[(&str, f32)]) -> PluginState {
        PluginState {
            version: "0.2.0".into(),
            params: params
                .iter()
                .map(|&(id, v)| (id.to_string(), ParamValue::F32(v)))
                .collect(),
            fields: Default::default(),
        }
    }

    fn f32_param(state: &PluginState, id: &str) -> f32 {
        match state.params.get(id) {
            Some(&ParamValue::F32(v)) => v,
            other => panic!("{id}: {other:?}"),
        }
    }

    #[test]
    fn old_host_state_keeps_its_wander() {
        let mut saved = state(&[("position_envelope", 0.5), ("position_wander", 0.3)]);
        Corrosion::filter_state(&mut saved);
        assert!((f32_param(&saved, "position_wander_rate") - 2.55).abs() < 1e-5);
        assert_eq!(f32_param(&saved, "position_envelope"), 0.0);
        assert_eq!(f32_param(&saved, "position_wander"), 0.3);
    }

    #[test]
    fn current_host_state_is_left_alone() {
        let mut saved = state(&[("position_envelope", 0.5), ("position_wander_rate", 1.5)]);
        Corrosion::filter_state(&mut saved);
        assert_eq!(f32_param(&saved, "position_wander_rate"), 1.5);
        assert_eq!(f32_param(&saved, "position_envelope"), 0.5);
    }

    #[test]
    fn old_host_state_keeps_its_onset_level() {
        // `m01-m04-m05-clarify-parameters` (M1): Onset used to set the onset
        // stage's level too (clamped to 0–1).
        let mut saved = state(&[("mseg_onset", 0.4)]);
        Corrosion::filter_state(&mut saved);
        assert_eq!(f32_param(&saved, "mseg_onset"), 0.4);
        assert_eq!(f32_param(&saved, "mseg_onset_level"), 0.4);
    }

    #[test]
    fn current_host_onset_level_is_left_alone() {
        let mut saved = state(&[("mseg_onset", 0.4), ("mseg_onset_level", 0.2)]);
        Corrosion::filter_state(&mut saved);
        assert_eq!(f32_param(&saved, "mseg_onset_level"), 0.2);
    }

    #[test]
    fn old_project_keeps_its_output_gain() {
        // The host stores Output's plain value, the gain, so the taper change
        // keeps a saved project's level (`m01-m04-m05-clarify-parameters`).
        let mut saved = state(&[("output", 2.0)]);
        Corrosion::filter_state(&mut saved);
        assert_eq!(f32_param(&saved, "output"), 2.0);
        let params = crate::params::CorrosionParams::default();
        let restored = params
            .output
            .preview_plain(params.output.preview_normalized(2.0));
        assert!((restored - 2.0).abs() < 1e-4, "restored gain {restored}");
    }

    #[test]
    fn host_state_without_position_envelope_is_left_alone() {
        let mut saved = state(&[("position_wander", 0.3)]);
        Corrosion::filter_state(&mut saved);
        assert!(!saved.params.contains_key("position_wander_rate"));
    }
}

/// `m03-m20-start-at-host-rate-and-report-latency`: latency reported at
/// initialization, and voices rated before they play.
#[cfg(test)]
mod host_rate_and_latency_tests {
    use super::Corrosion;
    use crate::offline::{OfflineNote, PresetRenderer};
    use crate::params::{exciter_param, limiter_mode_param, CorrosionParams};
    use nih_plug::prelude::*;
    use std::cell::RefCell;
    use std::sync::Arc;

    /// An `InitContext` that records every latency the plugin reports.
    #[derive(Default)]
    struct RecordingInit {
        latencies: RefCell<Vec<u32>>,
    }

    impl InitContext<Corrosion> for RecordingInit {
        fn plugin_api(&self) -> PluginApi {
            PluginApi::Vst3
        }
        fn execute(&self, _task: ()) {}
        fn set_latency_samples(&self, samples: u32) {
            self.latencies.borrow_mut().push(samples);
        }
        fn set_current_voice_capacity(&self, _capacity: u32) {}
    }

    fn buffer_config(sample_rate: f32) -> BufferConfig {
        BufferConfig {
            sample_rate,
            min_buffer_size: None,
            max_buffer_size: 512,
            process_mode: ProcessMode::Realtime,
        }
    }

    fn initialized(params: CorrosionParams, sample_rate: f32) -> (Corrosion, Vec<u32>) {
        let mut plugin = Corrosion {
            params: Arc::new(params),
            ..Corrosion::default()
        };
        let mut init = RecordingInit::default();
        assert!(plugin.initialize(
            &Corrosion::AUDIO_IO_LAYOUTS[0],
            &buffer_config(sample_rate),
            &mut init
        ));
        let latencies = init.latencies.into_inner();
        (plugin, latencies)
    }

    fn with_limiter(mode: i32) -> CorrosionParams {
        CorrosionParams {
            limiter_mode: limiter_mode_param(mode),
            ..CorrosionParams::default()
        }
    }

    /// Spec "Session restored with Lookahead".
    #[test]
    fn lookahead_latency_is_reported_at_initialization() {
        let (mut plugin, latencies) = initialized(with_limiter(1), 48_000.0);
        assert_eq!(
            latencies,
            vec![crate::dsp::LOOKAHEAD_LIMITER_SAMPLES as u32]
        );
        // The first block has nothing further to report.
        assert_eq!(plugin.latency_change(1), None);
    }

    /// Spec "Session restored with Hard".
    #[test]
    fn hard_limiter_reports_no_latency_at_initialization() {
        let (_, latencies) = initialized(with_limiter(0), 48_000.0);
        assert!(latencies.iter().all(|&l| l == 0), "reported {latencies:?}");
    }

    /// Spec "Mode changed while playing": reported once, at the first block
    /// with the new mode.
    #[test]
    fn mode_change_while_playing_is_reported_once() {
        let (mut plugin, _) = initialized(with_limiter(0), 48_000.0);
        assert_eq!(plugin.latency_change(0), None);
        assert_eq!(
            plugin.latency_change(1),
            Some(crate::dsp::LOOKAHEAD_LIMITER_SAMPLES as u32)
        );
        assert_eq!(plugin.latency_change(1), None);
        assert_eq!(plugin.latency_change(0), Some(0));
    }

    /// A renderer for `params` whose plugin was initialized at `sample_rate`
    /// (as a host does before playback) and, when `primed`, has already
    /// played a note at that rate and been reset.
    fn renderer(params: CorrosionParams, sample_rate: u32, primed: bool) -> PresetRenderer {
        let mut renderer = PresetRenderer::from_params(params);
        let mut init = RecordingInit::default();
        assert!(renderer.plugin_mut().initialize(
            &Corrosion::AUDIO_IO_LAYOUTS[0],
            &buffer_config(sample_rate as f32),
            &mut init
        ));
        if primed {
            let _ = renderer.render(&[OfflineNote::on(0, 60, 1.0)], sample_rate, 1_024);
            renderer.plugin_mut().reset();
        }
        renderer
    }

    /// Spec "First note at 96 kHz": a note renders the same samples on a
    /// voice that has never played as on one that has played at that rate.
    #[test]
    fn first_note_does_not_depend_on_voice_history() {
        let mut differing = Vec::new();
        for exciter in 1..=16 {
            let params = || CorrosionParams {
                exciter: exciter_param(exciter),
                ..CorrosionParams::default()
            };
            let notes = [OfflineNote::on(0, 60, 1.0)];
            let fresh = renderer(params(), 96_000, false)
                .render(&notes, 96_000, 4_096)
                .pre_limiter;
            let primed = renderer(params(), 96_000, true)
                .render(&notes, 96_000, 4_096)
                .pre_limiter;
            if let Some(first) = fresh.iter().zip(&primed).position(|(a, b)| a != b) {
                differing.push(format!("exciter {exciter} from frame {first}"));
            }
        }
        assert!(
            differing.is_empty(),
            "fresh and primed renders differ: {differing:?}"
        );
    }

    /// Spec "Bend before the first sample", checked on the bank itself: after
    /// a note-on and a +2-semitone bend, before any sample is rendered, every
    /// mode's pole angle is its bent frequency's angle at 44.1 kHz.
    #[test]
    fn bend_before_first_sample_tunes_the_poles_for_the_host_rate() {
        let offsets = bent_pole_errors(true);
        assert!(
            offsets.iter().all(|&cents| cents <= 0.05),
            "pole angles off by up to {:.3} cents",
            offsets.iter().cloned().fold(0.0, f64::max)
        );
    }

    /// Pole-angle error, in cents, of every mode of a note bent +2 semitones
    /// before its first sample at 44.1 kHz, on a plugin that was (or wasn't)
    /// initialized at that rate.
    fn bent_pole_errors(initialize: bool) -> Vec<f64> {
        let mut renderer = if initialize {
            renderer(CorrosionParams::default(), 44_100, false)
        } else {
            PresetRenderer::from_params(CorrosionParams::default())
        };
        let plugin = renderer.plugin_mut();
        super::handle_note_event(
            plugin,
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 1.0,
            },
        );
        super::handle_note_event(
            plugin,
            NoteEvent::MidiPitchBend {
                timing: 0,
                channel: 0,
                value: 1.0,
            },
        );
        let voice = plugin
            .voice_manager
            .voices_for_test()
            .iter()
            .find(|v| v.is_active())
            .expect("a voice plays the note");
        voice
            .resonator_for_test()
            .mode_tuning_for_test()
            .filter_map(|(frequency, angle)| {
                let expected = std::f64::consts::TAU * frequency as f64 / 44_100.0;
                (expected < std::f64::consts::PI)
                    .then(|| (angle / expected).ln().abs() / std::f64::consts::LN_2 * 1200.0)
            })
            .collect()
    }

    /// Spec "Bend before the first sample": a bend in the same block as the
    /// note-on, before its first sample, is applied at the host's rate. A
    /// voice that has played at 44.1 kHz is the reference.
    #[test]
    fn bend_before_first_sample_uses_the_host_rate() {
        let render = |primed: bool| {
            let mut renderer = renderer(CorrosionParams::default(), 44_100, primed);
            let plugin = renderer.plugin_mut();
            super::handle_note_event(
                plugin,
                NoteEvent::NoteOn {
                    timing: 0,
                    voice_id: None,
                    channel: 0,
                    note: 60,
                    velocity: 1.0,
                },
            );
            super::handle_note_event(
                plugin,
                NoteEvent::MidiPitchBend {
                    timing: 0,
                    channel: 0,
                    value: 1.0,
                },
            );
            renderer.render(&[], 44_100, 4_096).pre_limiter
        };
        let (fresh, primed) = (render(false), render(true));
        let first = fresh.iter().zip(&primed).position(|(a, b)| a != b);
        assert_eq!(first, None, "the bent note differs from frame {first:?}");
    }
}

/// `m13-sustain-pedal-and-panic`: the sustain pedal and the channel-mode
/// messages (All Notes Off, All Sound Off, Reset All Controllers).
#[cfg(test)]
mod midi_control_tests {
    use crate::offline::PresetRenderer;
    use crate::params::CorrosionParams;
    use crate::presets::Preset;
    use nih_plug::prelude::NoteEvent;

    const SR: u32 = 48_000;
    /// Event frames are multiples of the offline block size, so events land
    /// at block starts exactly as a host would deliver them there.
    const BLOCK: usize = crate::offline::OFFLINE_BLOCK_SIZE;

    #[derive(Clone, Copy, Debug)]
    enum Act {
        On(u8),
        Off(u8),
        Cc(u8, f32),
    }

    fn apply(plugin: &mut crate::Corrosion, act: Act) {
        let event = match act {
            Act::On(note) => NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note,
                velocity: 1.0,
            },
            Act::Off(note) => NoteEvent::NoteOff {
                timing: 0,
                voice_id: None,
                channel: 0,
                note,
                velocity: 0.0,
            },
            Act::Cc(cc, value) => NoteEvent::MidiCC {
                timing: 0,
                channel: 0,
                cc,
                value,
            },
        };
        super::handle_note_event(plugin, event);
    }

    /// Render `blocks` blocks of `renderer`, applying each `(block, act)` at
    /// the start of its block. Returns the frames before the limiter.
    fn script(
        mut renderer: PresetRenderer,
        blocks: usize,
        acts: &[(usize, Act)],
    ) -> (Vec<(f32, f32)>, PresetRenderer) {
        let mut out = Vec::with_capacity(blocks * BLOCK);
        for block in 0..blocks {
            for &(at, act) in acts {
                if at == block {
                    apply(renderer.plugin_mut(), act);
                }
            }
            out.extend(renderer.render(&[], SR, BLOCK).pre_limiter);
        }
        (out, renderer)
    }

    fn default_renderer() -> PresetRenderer {
        PresetRenderer::from_params(CorrosionParams::default())
    }

    /// A bowed (sustaining) patch: unlike a strike, its sound changes when
    /// the key is released, so the pedal's effect is audible.
    fn bowed_renderer() -> PresetRenderer {
        PresetRenderer::from_params(CorrosionParams {
            exciter: crate::params::exciter_param(1),
            ..CorrosionParams::default()
        })
    }

    fn assert_identical(a: &[(f32, f32)], b: &[(f32, f32)], what: &str) {
        assert_eq!(a.len(), b.len());
        let first = a.iter().zip(b).position(|(x, y)| x != y);
        assert_eq!(first, None, "{what}: renders differ from frame {first:?}");
    }

    const PEDAL_DOWN: Act = Act::Cc(64, 1.0);
    const PEDAL_UP: Act = Act::Cc(64, 0.0);
    const CHORD: [u8; 3] = [60, 64, 67];

    /// Spec "Chord held by the pedal": keys released under the pedal sound
    /// as if held until the pedal lifts, then release.
    #[test]
    fn pedal_holds_a_released_chord() {
        let mut pedal = vec![(0, PEDAL_DOWN)];
        let mut held = Vec::new();
        for n in CHORD {
            pedal.push((1, Act::On(n)));
            pedal.push((10, Act::Off(n)));
            held.push((1, Act::On(n)));
            held.push((94, Act::Off(n)));
        }
        pedal.push((94, PEDAL_UP));
        let (with_pedal, _) = script(bowed_renderer(), 188, &pedal);
        let (keys_held, _) = script(bowed_renderer(), 188, &held);
        assert_identical(&with_pedal, &keys_held, "pedal-held chord vs keys held");
    }

    /// Spec "Key still down when the pedal lifts".
    #[test]
    fn key_held_through_pedal_up_keeps_sounding() {
        let acts = [
            (0, PEDAL_DOWN),
            (1, Act::On(60)),
            (40, PEDAL_UP),
            (90, Act::Off(60)),
        ];
        let (with_pedal, _) = script(bowed_renderer(), 140, &acts);
        let (no_pedal, _) = script(
            bowed_renderer(),
            140,
            &[(1, Act::On(60)), (90, Act::Off(60))],
        );
        assert_identical(&with_pedal, &no_pedal, "key held across pedal-up");
    }

    /// Spec "Pedal up without notes": a released tail is unaffected.
    #[test]
    fn pedal_without_held_notes_changes_nothing() {
        let tail = [(0, Act::On(60)), (2, Act::Off(60))];
        let mut with_pedal = tail.to_vec();
        with_pedal.extend([(10, PEDAL_DOWN), (30, PEDAL_UP)]);
        let (a, _) = script(bowed_renderer(), 60, &tail);
        let (b, _) = script(bowed_renderer(), 60, &with_pedal);
        assert_identical(&a, &b, "tail with and without an empty pedal");
    }

    /// Spec "Repeated note under the pedal": at most one pedal-held voice per
    /// note, and the last strike is still held by the pedal.
    #[test]
    fn repeated_note_under_the_pedal_keeps_one_voice() {
        let mut renderer = bowed_renderer();
        let pedal_held = |renderer: &mut PresetRenderer| {
            renderer
                .plugin_mut()
                .voice_manager
                .voices_for_test()
                .iter()
                .filter(|v| v.is_pedal_held() && v.note() == 60)
                .count()
        };
        apply(renderer.plugin_mut(), PEDAL_DOWN);
        for strike in 0..10 {
            for act in [Act::On(60), Act::Off(60)] {
                apply(renderer.plugin_mut(), act);
                let _ = renderer.render(&[], SR, BLOCK);
                let held = pedal_held(&mut renderer);
                assert!(
                    held <= 1,
                    "strike {strike}, {act:?}: {held} pedal-held voices"
                );
            }
        }
        let active = renderer
            .plugin_mut()
            .voice_manager
            .voices_for_test()
            .iter()
            .filter(|v| v.is_active() && v.note() == 60)
            .count();
        assert_eq!(active, 1, "the last strike is still held by the pedal");
    }

    /// `Plugin::reset` lifts the pedal (design decision 8): after a reset, a
    /// key-up releases its note even though the pedal went down before.
    #[test]
    fn plugin_reset_lifts_the_pedal() {
        use nih_plug::prelude::Plugin;
        let mut renderer = bowed_renderer();
        apply(renderer.plugin_mut(), PEDAL_DOWN);
        renderer.plugin_mut().reset();
        assert!(!renderer.plugin_mut().voice_manager.sustain_down());
        apply(renderer.plugin_mut(), Act::On(60));
        let _ = renderer.render(&[], SR, BLOCK);
        apply(renderer.plugin_mut(), Act::Off(60));
        let active = renderer
            .plugin_mut()
            .voice_manager
            .voices_for_test()
            .iter()
            .filter(|v| v.is_active())
            .count();
        assert_eq!(active, 0, "the note is held after a reset");
    }

    /// Spec "Stuck notes": All Notes Off releases every held key at once.
    #[test]
    fn all_notes_off_releases_held_keys() {
        let notes = [48u8, 55, 60, 64];
        let mut cc = Vec::new();
        let mut offs = Vec::new();
        for n in notes {
            cc.push((1, Act::On(n)));
            offs.push((1, Act::On(n)));
            offs.push((20, Act::Off(n)));
        }
        cc.push((20, Act::Cc(123, 0.0)));
        let (a, _) = script(bowed_renderer(), 60, &cc);
        let (b, _) = script(bowed_renderer(), 60, &offs);
        assert_identical(&a, &b, "All Notes Off vs four note-offs");
    }

    /// Spec "Pedal down": All Notes Off doesn't override the pedal.
    #[test]
    fn all_notes_off_respects_the_pedal() {
        let notes = [48u8, 55, 60, 64];
        let mut cc = vec![(0, PEDAL_DOWN)];
        let mut held = Vec::new();
        for n in notes {
            cc.push((1, Act::On(n)));
            held.push((1, Act::On(n)));
            held.push((60, Act::Off(n)));
        }
        cc.push((20, Act::Cc(123, 0.0)));
        cc.push((60, PEDAL_UP));
        let (a, _) = script(bowed_renderer(), 100, &cc);
        let (b, _) = script(bowed_renderer(), 100, &held);
        assert_identical(
            &a,
            &b,
            "All Notes Off under the pedal vs keys held to pedal-up",
        );
    }

    fn space_renderer() -> PresetRenderer {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/presets/factory/plate_resonance_bath.corrosion-preset"
        );
        PresetRenderer::new(&Preset::load(std::path::Path::new(path)).expect("load preset"))
    }

    /// Spec "Panic with a reverb tail": silence within 10 ms, by a fade, and
    /// silence until a new note.
    #[test]
    fn all_sound_off_fades_to_silence() {
        let panic_block = 40;
        let acts = [
            (0, Act::On(48)),
            (0, Act::On(60)),
            (10, Act::Off(48)),
            (10, Act::Off(60)),
            (panic_block, Act::Cc(120, 0.0)),
        ];
        let (out, _) = script(space_renderer(), 100, &acts);
        let start = panic_block * BLOCK;
        let ten_ms = SR as usize / 100;
        assert!(
            out[start - ten_ms..start]
                .iter()
                .any(|&(l, r)| l.abs() > 1e-3 || r.abs() > 1e-3),
            "nothing ringing before the panic"
        );
        let step = |w: &[(f32, f32)]| {
            w.windows(2).fold(0.0f32, |m, p| {
                m.max((p[1].0 - p[0].0).abs()).max((p[1].1 - p[0].1).abs())
            })
        };
        let before = step(&out[start - ten_ms..=start]);
        let during = step(&out[start..start + ten_ms]);
        assert!(
            during <= before,
            "fade step {during} > step before {before}"
        );
        let silent_from = out[start..]
            .iter()
            .position(|&(l, r)| l == 0.0 && r == 0.0)
            .map(|i| start + i)
            .expect("never silent");
        assert!(
            silent_from <= start + ten_ms,
            "silent only at {silent_from}"
        );
        assert!(
            out[silent_from..]
                .iter()
                .all(|&(l, r)| l == 0.0 && r == 0.0),
            "sound after the panic finished"
        );
    }

    /// Spec "Playing after a panic": a note after All Sound Off renders as
    /// on a freshly reset plugin.
    #[test]
    fn playing_after_a_panic_is_like_after_a_reset() {
        use nih_plug::prelude::Plugin;
        let (_, mut panicked) = script(
            space_renderer(),
            30,
            &[(0, Act::On(60)), (20, Act::Cc(120, 0.0))],
        );
        // The fade ended mid-block and restarted the voice manager's frame
        // counter, which seeds each note's noise. The reset plugin plays its
        // note at the same count, so only state left behind could differ.
        let since_silence = panicked.plugin_mut().voice_manager.frame_counter_for_test() as usize;
        assert!(since_silence > 0 && since_silence < 10 * BLOCK);
        apply(panicked.plugin_mut(), Act::On(62));
        let after_panic = panicked.render(&[], SR, 20 * BLOCK).pre_limiter;
        let mut fresh = space_renderer();
        fresh.plugin_mut().reset();
        let _ = fresh.render(&[], SR, since_silence);
        apply(fresh.plugin_mut(), Act::On(62));
        let after_reset = fresh.render(&[], SR, 20 * BLOCK).pre_limiter;
        assert_identical(
            &after_panic,
            &after_reset,
            "note after panic vs after reset",
        );
    }

    /// Spec "Controllers left set": Reset All Controllers lifts the pedal and
    /// returns mod wheel, bend and pressure to rest.
    #[test]
    fn reset_all_controllers_returns_controls_to_rest() {
        let acts = [
            (0, PEDAL_DOWN),
            (0, Act::Cc(1, 1.0)),
            (1, Act::On(60)),
            (2, Act::Off(60)),
            (5, Act::Cc(121, 0.0)),
        ];
        let (_, mut renderer) = script(default_renderer(), 6, &acts);
        let held = renderer
            .plugin_mut()
            .voice_manager
            .voices_for_test()
            .iter()
            .filter(|v| v.is_active())
            .count();
        assert_eq!(held, 0, "pedal-held notes not released");

        // Mod wheel, bend and channel pressure left set, then reset: the next
        // note renders as on a plugin whose controllers were never touched.
        let mut set = default_renderer();
        apply(set.plugin_mut(), Act::Cc(1, 1.0));
        for event in [
            NoteEvent::MidiPitchBend {
                timing: 0,
                channel: 0,
                value: 1.0,
            },
            NoteEvent::MidiChannelPressure {
                timing: 0,
                channel: 0,
                pressure: 1.0,
            },
        ] {
            super::handle_note_event(set.plugin_mut(), event);
        }
        apply(set.plugin_mut(), Act::Cc(121, 0.0));
        let (after, _) = script(set, 20, &[(0, Act::On(64))]);
        let (rest, _) = script(default_renderer(), 20, &[(0, Act::On(64))]);
        assert_identical(&after, &rest, "note after Reset All Controllers vs at rest");
    }

    /// Spec "Pedal and panic in a checked zone": handling the pedal, All
    /// Notes Off, All Sound Off and Reset All Controllers, and rendering
    /// through them, doesn't allocate (checked in debug builds).
    #[test]
    fn controller_messages_are_alloc_free() {
        let mut renderer = bowed_renderer();
        let plugin = renderer.plugin_mut();
        let mut buffer = vec![(0.0f32, 0.0f32); BLOCK];
        let mut block = |plugin: &mut crate::Corrosion| {
            let mut next = None;
            plugin.process_block(
                BLOCK,
                SR,
                None,
                &mut next,
                || None,
                |i, frame, _| {
                    buffer[i] = frame;
                },
            );
        };
        for n in [48u8, 55, 60, 64] {
            apply(plugin, Act::On(n));
        }
        block(plugin);
        assert_no_alloc::assert_no_alloc(|| {
            apply(plugin, PEDAL_DOWN);
            for n in [48u8, 55, 60, 64] {
                apply(plugin, Act::Off(n));
            }
            block(plugin);
            apply(plugin, Act::On(60));
            apply(plugin, Act::Off(60));
            apply(plugin, PEDAL_UP);
            block(plugin);
            for n in [50u8, 57] {
                apply(plugin, Act::On(n));
            }
            apply(plugin, Act::Cc(123, 0.0));
            block(plugin);
            apply(plugin, PEDAL_DOWN);
            apply(plugin, Act::Cc(1, 1.0));
            apply(plugin, Act::Cc(121, 0.0));
            block(plugin);
            apply(plugin, Act::On(62));
            apply(plugin, Act::Cc(120, 0.0));
            for _ in 0..4 {
                block(plugin);
            }
        });
    }

    /// Spec "Full pool under the pedal": a pedal-held note is stolen before a
    /// held key.
    #[test]
    fn pedal_held_notes_are_stolen_before_held_keys() {
        let mut acts = vec![(0, PEDAL_DOWN)];
        for n in 0..16u8 {
            acts.push((1, Act::On(40 + n)));
        }
        for n in 0..8u8 {
            acts.push((30, Act::Off(40 + n)));
        }
        acts.push((31, Act::On(80)));
        let (_, mut renderer) = script(bowed_renderer(), 32, &acts);
        let sounding: Vec<u8> = renderer
            .plugin_mut()
            .voice_manager
            .sounding_notes()
            .collect();
        let missing: Vec<u8> = (40..56).filter(|n| !sounding.contains(n)).collect();
        assert_eq!(missing.len(), 1, "sounding {sounding:?}");
        assert!(
            missing[0] < 48,
            "stole held key {} instead of a pedal-held note",
            missing[0]
        );
    }
}

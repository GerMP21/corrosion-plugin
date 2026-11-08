//! # Parameter Definitions Module
//!
//! This module defines all user-facing parameters for the Corrosion synthesizer.
//! It provides the bridge between the host DAW (via nih_plug) and the internal
//! DSP algorithms, handling parameter serialization, automation, and UI integration.
//!
//! ## Parameter Categories
//!
//! The 130+ parameters are organized into logical groups:
//!
//! ### Sound Generation
//! - **Exciter** (exciter) - Type of excitation model
//! - **Object** (object) - Resonator object type (Pipe, Plate, Tank, etc.)
//!
//! ### Object Transformations
//! - **Size** (size) - Overall scale of the resonator (0.05x to 10x)
//! - **Rust** (rust) - Corrosion amount affecting high frequencies and decay
//! - **Damage** (damage) - Structural damage introducing rattle and chaos
//!
//! ### Exciter Controls
//! Global controls that act on every exciter model; each is neutral at its
//! default.
//! - **Pressure** (exciter_pressure) - Force trim, −12 to +12 dB (neutral 0.5), live on held notes
//! - **Speed** (exciter_speed) - Motion-speed multiplier, ×0.5 to ×2 (neutral 0.5), at note-on
//! - **Roughness** (exciter_roughness) - Grit and randomness multiplier, ×0 to ×3.33 (neutral 0.3), at note-on
//!
//! ### Envelope Generators
//! Standard ADSR for impact/specialty exciters:
//! - **Attack** (env_attack) - Time to reach peak (0.001s - 2s)
//! - **Decay** (env_decay) - Time to reach sustain (0.01s - 5s)
//! - **Sustain** (env_sustain) - Hold level (0.0 - 1.0)
//! - **Release** (env_release) - Time to silence (0.01s - 5s)
//!
//! MSEG (Multistage Envelope) for friction exciters:
//! - **Onset, Attack, Hold, Decay, Sustain, Release** - 6-stage envelope
//! - **Loop Mode** - Off, Forward, PingPong (partial)
//!
//! ### Post-Processing Chain
//! - **Filter** (filter_cutoff, filter_resonance) - Lowpass resonant filter
//! - **Drive** (drive, drive_amount) - Saturation and waveshaping
//! - **Body** (body) - Physical body resonance simulation
//! - **Spread** (width, spread_width) - Stereo width enhancement
//! - **Space** (space_mode, space_amount) - Reverb/echo effects
//!
//! ### Interaction Parameters
//! - **Strike Position** (strike_position) - Where exciter contacts object
//! - **Coupling** (coupling_stiffness) - Bidirectional interaction strength
//! - **Wander** (position_wander) - Simulated hand unsteadiness
//! - **Wander Rate** (position_wander_rate) - How fast Wander moves (0.1 to 5 Hz)
//! - **Position Envelope** (position_envelope) - How far the force envelope moves the strike position
//!
//! ## Relationships
//!
//! This module connects to:
//! - `lib.rs` - Parameter values read during audio processing
//! - `voice/mod.rs` - VoiceControls struct bundles parameters for voices
//! - `gui/editor.rs` - Parameters displayed and edited in UI
//! - `presets/mod.rs` - Parameters serialized/deserialized as presets
//!
//! ## Usage
//!
//! Parameters are accessed via the `CorrosionParams` struct which implements
//! nih_plug's `Params` trait for host integration:
//!
//! ```ignore
//! let params = plugin.params();
//! let cutoff = params.filter_cutoff.value();
//! let exciter_type = params.exciter.value();
//! ```

use nih_plug::prelude::*;
#[cfg(feature = "gui")]
use nih_plug_egui::EguiState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Main parameter struct containing all user-facing controls.
///
/// This struct implements nih_plug's `Params` trait, allowing the host DAW
/// to automate and save parameter values. Each field corresponds to a
/// controllable parameter in the synthesizer.
///
/// The `#[derive(Params)]` macro generates the necessary trait implementations
/// for host integration, including parameter enumeration and value conversion.
#[derive(Params)]
pub struct CorrosionParams {
    /// GUI editor state persistence (window size, position)
    /// Only available when the "gui" feature is enabled
    #[cfg(feature = "gui")]
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    /// Exciter type selection (0-16 mapping to ExciterType enum)
    /// Controls the type of physical excitation (mallets, scrapes, specialty)
    #[id = "exciter"]
    pub exciter: IntParam,

    /// Resonator object type (0-8 mapping to Object enum)
    /// Determines the modal characteristics (pipe vs plate vs tank, etc.)
    #[id = "object"]
    pub object: IntParam,

    /// Quality mode (0=Eco, 1=Normal, 2=High, 3=Render)
    /// Controls CPU/quality tradeoff for post-processing and oversampling.
    #[id = "quality_mode"]
    pub quality_mode: IntParam,

    /// Play mode (0=Tonal, 1=Kit, 2=Drone). Routes the note → object/pitch
    /// mapping at note-on. See `params::PlayMode` for the semantics.
    #[id = "play_mode"]
    pub play_mode: IntParam,

    /// Limiter mode (0=Hard clip, 1=Lookahead). When set to Lookahead the
    /// master output passes through `LookaheadLimiter` which trades ~1 ms
    /// latency for transparent peak limiting. Default keeps the
    /// pre-existing behavior so loading old presets is a no-op.
    #[id = "limiter_mode"]
    pub limiter_mode: IntParam,

    /// Object size scale (0.05 to 10.0)
    /// Affects pitch (inverse) and decay time (direct)
    /// Higher values = larger object = lower pitch, longer decay
    #[id = "size"]
    pub size: FloatParam,

    /// Rust amount (0.0 to 5.0)
    /// Simulates corrosion that damps high frequencies
    /// Higher values = duller, more muted sound
    #[id = "rust"]
    pub rust: FloatParam,

    /// Damage amount (0.0 to 10.0)
    /// Structural damage introducing non-linear rattle and buzzing
    /// Higher values = more industrial chaos
    #[id = "damage"]
    pub damage: FloatParam,

    /// Output drive/saturation (0.0 to 5.0)
    /// Asymmetric waveshaping for industrial character
    #[id = "drive"]
    pub drive: FloatParam,

    /// Master output gain in dB (mapped to linear 0.0 to +40dB)
    /// Final stage before the limiter
    #[id = "output"]
    pub output: FloatParam,

    /// Stereo width/spread (-2.0 to 3.0)
    /// Controls stereo field expansion in post-processing
    #[id = "width"]
    pub width: FloatParam,

    /// Body resonance amount (0.0 to 5.0).
    ///
    /// Mix level of the chassis body resonance, added in parallel to the
    /// filtered/driven signal: 0 is no body, 5 is the full body level
    /// (`body / 5` in the post chain).
    #[id = "body"]
    pub body: FloatParam,

    /// UI scale factor (0-4 mapping to 50%, 75%, 100%, 125%, 150%).
    /// The 100% editor baseline is the compact 1080x768 logical window.
    /// Only affects the editor GUI size.
    #[id = "ui_scale"]
    pub ui_scale: IntParam,

    // ADSR Envelope Parameters (for impact and specialty exciters)
    /// Envelope attack time in seconds (0.001 to 2.0)
    /// Time from zero to peak level
    #[id = "env_attack"]
    pub env_attack: FloatParam,

    /// Envelope decay time in seconds (0.01 to 5.0)
    /// Time from peak to sustain level
    #[id = "env_decay"]
    pub env_decay: FloatParam,

    /// Envelope sustain level (0.0 to 1.0)
    /// Hold level while note is active
    #[id = "env_sustain"]
    pub env_sustain: FloatParam,

    /// Envelope release time in seconds (0.01 to 5.0)
    /// Time from sustain to silence after note-off
    #[id = "env_release"]
    pub env_release: FloatParam,

    // MSEG (Multistage Envelope) Parameters (for friction exciters)
    /// MSEG onset time - initial transient stage (0.001 to 1.0s)
    #[id = "mseg_onset"]
    pub mseg_onset: FloatParam,

    /// MSEG attack time - main rise stage (0.001 to 2.0s)
    #[id = "mseg_attack"]
    pub mseg_attack: FloatParam,

    /// MSEG hold time - peak hold stage (0.0 to 2.0s)
    #[id = "mseg_hold"]
    pub mseg_hold: FloatParam,

    /// MSEG decay time - fall to sustain (0.01 to 5.0s)
    #[id = "mseg_decay"]
    pub mseg_decay: FloatParam,

    /// MSEG sustain level (0.0 to 1.0)
    #[id = "mseg_sustain"]
    pub mseg_sustain: FloatParam,

    /// MSEG release time (0.01 to 5.0s)
    #[id = "mseg_release"]
    pub mseg_release: FloatParam,

    // Envelope Modulation
    /// Envelope amount - scales envelope influence on excitation (0.0 to 1.0)
    #[id = "env_amount"]
    pub env_amount: FloatParam,

    /// Friction envelope peak level (0.0 to 1.0): peak = 0.5 + 0.5·value.
    /// Despite the id it does not read velocity; velocity scales the whole
    /// envelope via velocity-to-level.
    #[id = "velocity_to_peak"]
    pub velocity_to_peak: FloatParam,

    /// Loop mode for MSEG (0=Off, 1=Forward, 2=PingPong).
    ///
    /// PingPong is partial: it ignores the start/end stages and always loops
    /// Decay → Sustain, like Forward 3 → 4.
    #[id = "loop_mode"]
    pub loop_mode: IntParam,

    /// Loop start stage (0-5, default 3=Decay).
    ///
    /// Stages: 0 Onset, 1 Attack, 2 Hold, 3 Decay, 4 Sustain, 5 Release.
    /// Onset and Release are never looped, so 0 acts as 1 and 5 as 4. With
    /// Forward looping and the note held, reaching the end stage restarts the
    /// start stage; if start ≥ end the loop is empty and the envelope holds
    /// sustain as if looping were off.
    #[id = "loop_start_stage"]
    pub loop_start_stage: IntParam,

    /// Loop end stage (0-5, default 4=Sustain); 0 acts as 1 and 5 as 4.
    /// See `loop_start_stage` for how the loop region is applied.
    #[id = "loop_end_stage"]
    pub loop_end_stage: IntParam,

    /// Sync rate for the tempo-synced echo (0.0 to 1.0).
    ///
    /// Below `0.05` the echo runs free with the `delay_time` knob. Above
    /// that threshold the value selects a discrete musical division
    /// (1/16 → 2/1 in six bands) and the echo's effective delay is computed
    /// from the host tempo. If the host does not report a BPM, the knob
    /// silently falls back to free-running mode. See
    /// `lib.rs::sync_rate_to_delay_time` for the routing table and
    /// `docs/BACKLOG.md` → P1.6 for the spec.
    #[id = "sync_rate"]
    pub sync_rate: FloatParam,

    /// Global time scale multiplier (0.1 to 10.0)
    /// Scales all envelope times proportionally
    #[id = "global_time_scale"]
    pub global_time_scale: FloatParam,

    /// Velocity to envelope level amount (0.0 to 1.0)
    #[id = "velocity_to_level"]
    pub velocity_to_level: FloatParam,

    /// Velocity to envelope time amount (0.0 to 1.0)
    /// Higher velocities = shorter times
    #[id = "velocity_to_time"]
    pub velocity_to_time: FloatParam,

    /// Curve tension for envelope stages (-1.0 to 1.0)
    /// Negative = exponential, 0 = linear, positive = logarithmic
    #[id = "curve_tension"]
    pub curve_tension: FloatParam,

    // Exciter Controls (global: they act on every exciter model;
    // `h06-wire-global-exciter-controls`)
    /// Pressure (0.0 to 1.0, neutral at 0.5): trims every exciter's force by
    /// `2^((p − 0.5)·4)`, −12 dB at 0 to +12 dB at 1. Live on held notes
    /// (smoothed over ~10 ms); the drive governor still keeps each voice
    /// under full scale.
    #[id = "exciter_pressure"]
    pub exciter_pressure: FloatParam,

    /// Speed (0.0 to 1.0, neutral at 0.5): multiplies each motion speed by
    /// `2^((s − 0.5)·2)`, ×0.5 to ×2, at note-on: Bow Speed, Scrape, Grind,
    /// Drag and Pull Speed, and ParticleRain Flow Rate. WireBrush Spread
    /// Duration and MetalChain Drop Envelope Spread are divided by it.
    /// Inert on HandStrike, FeltMallet, HardMallet, Drumstick, MetalPipe,
    /// PneumaticJet, ElectromagneticHum and TensionSnap.
    #[id = "exciter_speed"]
    pub exciter_speed: FloatParam,

    /// Roughness (0.0 to 1.0, neutral at 0.3): multiplies each grit or
    /// randomness amount by `r / 0.3`, ×0 to ×3.33, at note-on: Bow Rosin
    /// Grip, WireBrush Amplitude Randomization, MetalChain Internal Rattle,
    /// HeavyGrinding Surface Grit, TensionRise Slip Stochasticity,
    /// PneumaticJet Turbulence Chaos and ParticleRain Mass Variance. Inert on
    /// the other exciters.
    #[id = "exciter_roughness"]
    pub exciter_roughness: FloatParam,

    // Dedicated Exciter Algorithm Parameters
    /// Hand strike force multiplier.
    #[id = "hand_mass"]
    pub hand_mass: FloatParam,

    /// Hand strike spring stiffness for the initial contact transient.
    #[id = "flesh_stiffness"]
    pub flesh_stiffness: FloatParam,

    /// Hand strike damping that absorbs high-frequency motion.
    #[id = "flesh_damping"]
    pub flesh_damping: FloatParam,

    /// Hand strike decay back to rest after the impact.
    #[id = "mute_decay"]
    pub mute_decay: FloatParam,

    /// Shared mass control used by the felt and hard mallet models.
    #[id = "mallet_mass"]
    pub mallet_mass: FloatParam,

    /// Felt mallet low-velocity softness term.
    #[id = "felt_softness"]
    pub felt_softness: FloatParam,

    /// Felt mallet hard-core stiffness multiplier.
    #[id = "core_hardness"]
    pub core_hardness: FloatParam,

    /// Felt mallet polynomial compression exponent.
    #[id = "compression_curve"]
    pub compression_curve: FloatParam,

    /// Hard mallet stiffness term controlling strike brightness.
    #[id = "material_stiffness"]
    pub material_stiffness: FloatParam,

    /// Hard mallet damping that suppresses secondary bounces.
    #[id = "impact_damping"]
    pub impact_damping: FloatParam,

    /// Drumstick effective mass.
    #[id = "stick_mass"]
    pub stick_mass: FloatParam,

    /// Drumstick tip stiffness controlling bite.
    #[id = "tip_stiffness"]
    pub tip_stiffness: FloatParam,

    /// Drumstick rebound energy retained after contact.
    #[id = "restitution_bounciness"]
    pub restitution_bounciness: FloatParam,

    /// Drumstick maximum number of micro-bounces.
    #[id = "micro_bounce_limit"]
    pub micro_bounce_limit: FloatParam,

    /// Wire brush impulse density.
    #[id = "wire_density"]
    pub wire_density: FloatParam,

    /// Wire brush spread window in milliseconds.
    #[id = "spread_duration"]
    pub spread_duration: FloatParam,

    /// Wire brush stiffness filter control.
    #[id = "brush_wire_stiffness"]
    pub brush_wire_stiffness: FloatParam,

    /// Wire brush amplitude variance between individual wires.
    #[id = "amplitude_randomization"]
    pub amplitude_randomization: FloatParam,

    /// Metal pipe exciter mass.
    #[id = "pipe_mass"]
    pub pipe_mass: FloatParam,

    /// Metal pipe contact stiffness.
    #[id = "metal_stiffness"]
    pub metal_stiffness: FloatParam,

    /// Metal pipe internal ringing pitch shift.
    #[id = "pipe_pitch"]
    pub pipe_pitch: FloatParam,

    /// Metal pipe internal ring decay.
    #[id = "pipe_ring_decay"]
    pub pipe_ring_decay: FloatParam,

    /// Metal chain link count.
    #[id = "link_count"]
    pub link_count: FloatParam,

    /// Metal chain per-link mass.
    #[id = "chain_mass"]
    pub chain_mass: FloatParam,

    /// Metal chain impact spread in milliseconds.
    #[id = "drop_envelope_spread"]
    pub drop_envelope_spread: FloatParam,

    /// Metal chain internal rattle gain.
    #[id = "internal_rattle"]
    pub internal_rattle: FloatParam,

    /// Metal chain rattle color / high-pass emphasis.
    #[id = "rattle_color"]
    pub rattle_color: FloatParam,

    /// Bow friction force.
    #[id = "bow_pressure"]
    pub bow_pressure: FloatParam,

    /// Bow motion speed.
    #[id = "bow_speed"]
    pub bow_speed: FloatParam,

    /// Bow rosin grip / static friction amount.
    #[id = "rosin_grip"]
    pub rosin_grip: FloatParam,

    /// Bow slip curve shaping.
    #[id = "slip_curve"]
    pub slip_curve: FloatParam,

    /// Stiff point drag speed.
    #[id = "scrape_speed"]
    pub scrape_speed: FloatParam,

    /// Stiff point contact pressure.
    #[id = "point_pressure"]
    pub point_pressure: FloatParam,

    /// Stiff point chatter pitch / stiffness.
    #[id = "chatter_pitch"]
    pub chatter_pitch: FloatParam,

    /// Stiff point chatter damping.
    #[id = "chatter_damping"]
    pub chatter_damping: FloatParam,

    /// Grinding drag speed.
    #[id = "grind_speed"]
    pub grind_speed: FloatParam,

    /// Grinding baseline pressure.
    #[id = "grind_pressure"]
    pub grind_pressure: FloatParam,

    /// Grinding surface grit / tearing-noise ratio.
    #[id = "surface_grit"]
    pub surface_grit: FloatParam,

    /// Grinding noise color.
    #[id = "grit_color"]
    pub grit_color: FloatParam,

    /// Corrugated drag speed.
    #[id = "drag_speed"]
    pub drag_speed: FloatParam,

    /// Corrugated ridge spacing.
    #[id = "ridge_spacing"]
    pub ridge_spacing: FloatParam,

    /// Corrugated ridge depth.
    #[id = "ridge_depth"]
    pub ridge_depth: FloatParam,

    /// Corrugated drag effective exciter mass.
    #[id = "drag_exciter_mass"]
    pub drag_exciter_mass: FloatParam,

    /// Tension-rise pull speed.
    #[id = "pull_speed"]
    pub pull_speed: FloatParam,

    /// Tension-rise slip threshold.
    #[id = "break_threshold"]
    pub break_threshold: FloatParam,

    /// Tension-rise random threshold jitter.
    #[id = "slip_stochasticity"]
    pub slip_stochasticity: FloatParam,

    /// Tension-rise slip sharpness / filtering.
    #[id = "creak_sharpness"]
    pub creak_sharpness: FloatParam,

    /// Pneumatic jet pressure / drive.
    #[id = "air_pressure"]
    pub air_pressure: FloatParam,

    /// Pneumatic jet nozzle width / resonance focus.
    #[id = "nozzle_width"]
    pub nozzle_width: FloatParam,

    /// Pneumatic jet turbulence non-linearity.
    #[id = "turbulence_chaos"]
    pub turbulence_chaos: FloatParam,

    /// Electromagnetic hum mains frequency.
    #[id = "mains_frequency"]
    pub mains_frequency: FloatParam,

    /// Electromagnetic hum coupling / proximity.
    #[id = "coil_proximity"]
    pub coil_proximity: FloatParam,

    /// Electromagnetic hum harmonic sag / distortion.
    #[id = "voltage_sag"]
    pub voltage_sag: FloatParam,

    /// Tension snap pull distance.
    #[id = "pull_distance"]
    pub pull_distance: FloatParam,

    /// Tension snap hook stiffness.
    #[id = "hook_stiffness"]
    pub hook_stiffness: FloatParam,

    /// Tension snap release force threshold.
    #[id = "snap_force"]
    pub snap_force: FloatParam,

    /// Particle rain spawn density.
    #[id = "flow_rate"]
    pub flow_rate: FloatParam,

    /// Particle rain particle mass.
    #[id = "particle_mass"]
    pub particle_mass: FloatParam,

    /// Particle rain mass randomization.
    #[id = "mass_variance"]
    pub mass_variance: FloatParam,

    // Interaction Parameters
    /// Strike position on resonator (0.0 to 1.0)
    /// 0.0 = edge, 0.5 = center, 1.0 = other edge
    #[id = "strike_position"]
    pub strike_position: FloatParam,

    /// Coupling stiffness (0.0 to 1.0)
    /// Bidirectional interaction strength between exciter and resonator
    #[id = "coupling_stiffness"]
    pub coupling_stiffness: FloatParam,

    /// Position wander amount (0.0 to 1.0)
    /// Simulates hand unsteadiness during excitation: a slow sine around the
    /// strike position, at Wander Rate.
    #[id = "position_wander"]
    pub position_wander: FloatParam,

    /// Position wander rate (0.1 to 5 Hz, default 0.1): how fast Wander
    /// moves the strike position (`h09-route-envelope-to-strike-position`).
    #[id = "position_wander_rate"]
    pub position_wander_rate: FloatParam,

    /// Position envelope amount (0.0 to 1.0, default 0)
    /// How much the note's force envelope moves the strike position: the
    /// effective position is `strike_position + position_envelope ·
    /// envelope · 0.5`, clamped to 0..1, every sample. A strike or a bow
    /// swell moves the contact point, and so the timbre, as the envelope
    /// rises and falls. At 0 the position doesn't move.
    #[id = "position_envelope"]
    pub position_envelope: FloatParam,

    /// Fundamental frequency anchor (0.0 to 1.0)
    /// Minimum coefficient for fundamental mode to prevent nulls
    #[id = "fundamental_anchor"]
    pub fundamental_anchor: FloatParam,

    // Resonator Controls
    /// Resonator damping (0.0 to 1.0)
    /// Overall decay time modifier
    #[id = "res_damping"]
    pub res_damping: FloatParam,

    /// Resonator brightness (0.0 to 1.0)
    /// High frequency emphasis
    #[id = "res_brightness"]
    pub res_brightness: FloatParam,

    // Advanced Transformations
    /// Material thickness (0.0 to 1.0)
    /// Affects stiffness and frequency distribution
    #[id = "thickness"]
    pub thickness: FloatParam,

    /// Heat amount (0.0 to 1.0)
    /// Thermal effects - detuning and damping
    #[id = "heat"]
    pub heat: FloatParam,

    /// Sludge amount (0.0 to 1.0)
    /// Viscous damping effect
    #[id = "sludge"]
    pub sludge: FloatParam,

    // Per-object resonator "character" parameters.
    //
    // Each drives one algorithmic-resonator generator field for the matching
    // object; the others are ignored. Size/decay/brightness-like behavior stays
    // covered by the global Size/Damping/Brightness controls.
    /// Pipe: stiffness/inharmonicity (wider = more bell-like).
    #[id = "pipe_diameter"]
    pub pipe_diameter: FloatParam,

    /// Plate: aspect ratio Lx/Ly tuning the inharmonic cluster.
    #[id = "plate_aspect"]
    pub plate_aspect: FloatParam,

    /// Plate: material stiffness spreading the modal cluster.
    /// Ships under `plate_stiffness` since `metal_stiffness` is the Metal Pipe exciter id.
    #[id = "plate_stiffness"]
    pub plate_stiffness: FloatParam,

    /// Tank: cavity volume setting the sub-bass boom depth.
    #[id = "tank_volume"]
    pub tank_volume: FloatParam,

    /// Tank: balance between deep cavity mode and metallic shell modes.
    #[id = "tank_cavity_mix"]
    pub tank_cavity_mix: FloatParam,

    /// Chain: base frequency range of the chaotic cluster (heavier = lower).
    #[id = "chain_link_mass"]
    pub chain_link_mass: FloatParam,

    /// Chain: chaotic coupling coefficient destabilizing the pitch.
    #[id = "chain_instability"]
    pub chain_instability: FloatParam,

    /// I-Beam: shear coefficient compressing the high-frequency modes.
    #[id = "beam_shear"]
    pub beam_shear: FloatParam,

    /// Taut Cable: inharmonicity pushing upper partials sharp.
    #[id = "cable_braid"]
    pub cable_braid: FloatParam,

    /// Taut Cable: how violently the pitch envelopes down after a hard strike.
    #[id = "cable_tension_drop"]
    pub cable_tension_drop: FloatParam,

    /// Coil Spring: dispersion chirp severity (the laser-like "pew").
    #[id = "spring_dispersion"]
    pub spring_dispersion: FloatParam,

    /// Coil Spring: detunes the modal array for chaotic, beating slosh.
    #[id = "spring_slosh"]
    pub spring_slosh: FloatParam,

    /// Sheet Metal: buckling coefficient (higher = roars and wobbles when hit hard).
    #[id = "sheet_thinness"]
    pub sheet_thinness: FloatParam,

    /// Industrial Cog: mode-splitting imperfection causing metallic beating.
    #[id = "cog_dissonance"]
    pub cog_dissonance: FloatParam,

    // Post-Processing Filter
    /// Filter cutoff frequency in Hz (20 to 20000, skewed)
    /// Lowpass filter before drive stage
    #[id = "filter_cutoff"]
    pub filter_cutoff: FloatParam,

    /// Filter resonance (0.0 to 1.0)
    /// Peaking response at cutoff frequency
    #[id = "filter_resonance"]
    pub filter_resonance: FloatParam,

    /// Component tolerance (0.0 to 1.0)
    /// Analog component variation for vintage character
    #[id = "component_tolerance"]
    pub component_tolerance: FloatParam,

    // Drive/Post-Processing
    /// Drive amount in post-processing (0.0 to 5.0)
    /// Separate from exciter drive - affects final output. The drive
    /// crossfades from dry to fully saturated across the range (wet share
    /// `amount / 5`), and also raises its input gain (`1 + amount / 2`).
    /// The Violence macro adds −0.2..+0.2 before the 0–5 clamp.
    #[id = "drive_amount"]
    pub drive_amount: FloatParam,

    /// Bias starvation (0.0 to 1.0)
    /// Circuit bias reduction for asymmetric distortion
    #[id = "bias_starvation"]
    pub bias_starvation: FloatParam,

    /// Chaos depth (0.0 to 1.0)
    /// Lorenz attractor modulation for chaotic drive
    #[id = "chaos_depth"]
    pub chaos_depth: FloatParam,

    // Stereo/Spatial
    /// Spread width (0.0 to 1.0)
    /// Stereo width from HRTF-based processing
    #[id = "spread_width"]
    pub spread_width: FloatParam,

    /// Listener proximity (0.0 to 1.0)
    /// Near-field effect strength
    #[id = "listener_proximity"]
    pub listener_proximity: FloatParam,

    // Body Resonance
    /// Chassis material (0.0 to 1.0)
    /// Body resonance material selection
    #[id = "chassis_material"]
    pub chassis_material: FloatParam,

    /// Chassis volume (0.0 to 1.0).
    ///
    /// Size of the chassis body: larger volumes move its resonances lower.
    /// It applies no gain of its own (BODY sets the body's mix level), but
    /// lower modes ring with more resonant gain, so at the same BODY a larger
    /// chassis sounds somewhat louder.
    #[id = "chassis_volume"]
    pub chassis_volume: FloatParam,

    // Space/Reverb
    /// Space mode (0=Off, 1=Factory, 2=Spring, 3=Echo)
    #[id = "space_mode"]
    pub space_mode: IntParam,

    /// Space/reverb amount (0.0 to 1.0)
    /// Wet/dry mix for spatial effects
    #[id = "space_amount"]
    pub space_amount: FloatParam,

    /// Factory reverb size (0.0 to 1.0)
    /// Room size for factory reverb algorithm
    #[id = "factory_size"]
    pub factory_size: FloatParam,

    /// Machinery clutter (0.0 to 1.0)
    /// Industrial equipment density in reverb
    #[id = "machinery_clutter"]
    pub machinery_clutter: FloatParam,

    /// Wall impedance (0.0 to 1.0)
    /// Surface reflectivity for reverb
    #[id = "wall_impedance"]
    pub wall_impedance: FloatParam,

    /// Spring reverb tension (0.0 to 1.0)
    /// Spring stiffness in spring reverb
    #[id = "spring_tension"]
    pub spring_tension: FloatParam,

    /// Wire stiffness (0.0 to 1.0)
    /// Spring wire thickness in spring reverb
    #[id = "wire_stiffness"]
    pub wire_stiffness: FloatParam,

    /// Spring tank size (0.0 to 1.0)
    /// Physical spring tank dimensions
    #[id = "spring_tank_size"]
    pub spring_tank_size: FloatParam,

    /// Echo delay time (0.0 to 1.0)
    /// Normalized delay time for echo mode
    #[id = "delay_time"]
    pub delay_time: FloatParam,

    /// Machinery movement (0.0 to 1.0)
    /// Doppler/modulation for echo delays
    #[id = "machinery_movement"]
    pub machinery_movement: FloatParam,

    /// High frequency damping (0.0 to 1.0)
    /// Air absorption in spatial effects
    #[id = "high_frequency_damping"]
    pub high_frequency_damping: FloatParam,

    /// Analog ceiling (0.5 to 1.0, default 0.9661 ≈ −0.3 dBFS): the output
    /// ceiling. The post-chain clipper clips at it, and after output gain both
    /// limiter modes (Hard and Lookahead) stop at it, so no output sample
    /// exceeds it (`h07-hard-limiter-honors-analog-ceiling`).
    #[id = "analog_ceiling"]
    pub analog_ceiling: FloatParam,

    /// Diode softness (0.0 to 1.0)
    /// Transition hardness in output clipper
    #[id = "diode_softness"]
    pub diode_softness: FloatParam,

    // Macro Controls (PRD §12)
    //
    // Each macro is a normalized `0.0..=1.0` knob with `0.5` as the neutral
    // point (no DSP change). They layer multiplicative or additive biases on
    // top of the existing per-parameter controls so a single twist can hit a
    // related cluster of destinations — see `lib.rs::resolve_macro_voice_bias`
    // for the routing table.
    /// Mass macro — biases exciter/resonator weight without losing per-exciter granularity.
    #[id = "macro_mass"]
    pub macro_mass: FloatParam,

    /// Corrosion macro — biases rust + damping + brightness loss as a group.
    #[id = "macro_corrosion"]
    pub macro_corrosion: FloatParam,

    /// Violence macro — biases drive + chaos + damage together.
    #[id = "macro_violence"]
    pub macro_violence: FloatParam,

    /// Brightness macro — biases resonator brightness and filter cutoff together.
    #[id = "macro_brightness"]
    pub macro_brightness: FloatParam,
}

/// Exciter type enumeration for serialization and UI display.
///
/// Maps integer parameter values (1-16) to named exciter models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ExciterType {
    /// Bow smooth stick-slip friction model.
    Bow,
    /// Hand strike - flesh-on-object impact
    HandStrike,
    /// Felt mallet - soft impact with polynomial compression
    FeltMallet,
    /// Hard mallet - rigid impact with Hertzian contact
    HardMallet,
    /// Drumstick - wood tip with micro-bouncing
    Drumstick,
    /// Wire brush - stochastic impulse cluster
    WireBrush,
    /// Metal pipe - bi-directional metal-on-metal
    MetalPipe,
    /// Metal chain - cascading multi-mass impacts
    MetalChain,
    /// Stiff point - nail/awl chatter scrape
    StiffPoint,
    /// Heavy grinding - concrete/sandpaper friction
    HeavyGrinding,
    /// Corrugated drag - stick on ribbed surface
    CorrugatedDrag,
    /// Tension rise - creak/groan mechanics
    TensionRise,
    /// Pneumatic jet - steam/air valve
    PneumaticJet,
    /// Electromagnetic hum - transformer/motor
    ElectromagneticHum,
    /// Tension snap - wire break/gear catch
    TensionSnap,
    /// Particle rain - debris/sand/gravel stream
    ParticleRain,
}

/// Exciter family categorization for envelope selection.
///
/// Each family uses a different envelope type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExciterFamily {
    /// Impact family - uses one-shot AR envelope
    Hit,
    /// Friction family - uses MSEG with looping
    Friction,
    /// Non-traditional family - uses ADSR envelope
    Specialty,
}

impl ExciterType {
    /// Converts integer parameter value to ExciterType enum.
    ///
    /// # Arguments
    /// * `v` - Integer value (0-16)
    ///
    /// # Returns
    /// Corresponding ExciterType variant, defaulting to HandStrike for invalid values
    pub fn from_int(v: i32) -> Self {
        match v {
            1 => ExciterType::Bow,
            2 => ExciterType::HandStrike,
            3 => ExciterType::FeltMallet,
            4 => ExciterType::HardMallet,
            5 => ExciterType::Drumstick,
            6 => ExciterType::WireBrush,
            7 => ExciterType::MetalPipe,
            8 => ExciterType::MetalChain,
            9 => ExciterType::StiffPoint,
            10 => ExciterType::HeavyGrinding,
            11 => ExciterType::CorrugatedDrag,
            12 => ExciterType::TensionRise,
            13 => ExciterType::PneumaticJet,
            14 => ExciterType::ElectromagneticHum,
            15 => ExciterType::TensionSnap,
            16 => ExciterType::ParticleRain,
            _ => ExciterType::HandStrike,
        }
    }

    /// Returns human-readable name for UI display.
    ///
    /// # Returns
    /// Static string slice with display name
    pub fn name(self) -> &'static str {
        match self {
            ExciterType::Bow => "Bow",
            ExciterType::HandStrike => "Hand Strike",
            ExciterType::FeltMallet => "Felt Mallet",
            ExciterType::HardMallet => "Hard Mallet",
            ExciterType::Drumstick => "Drumstick",
            ExciterType::WireBrush => "Wire Brush",
            ExciterType::MetalPipe => "Metal Pipe",
            ExciterType::MetalChain => "Metal Chain",
            ExciterType::StiffPoint => "Stiff Point Scrape",
            ExciterType::HeavyGrinding => "Heavy Grinding",
            ExciterType::CorrugatedDrag => "Corrugated Drag",
            ExciterType::TensionRise => "Tension Rise",
            ExciterType::PneumaticJet => "Pneumatic Jet",
            ExciterType::ElectromagneticHum => "Electromagnetic Hum",
            ExciterType::TensionSnap => "Tension Snap",
            ExciterType::ParticleRain => "Particle Rain",
        }
    }

    /// Returns the family category for envelope selection.
    ///
    /// # Returns
    /// ExciterFamily enum determining which envelope type to use
    pub fn family(self) -> ExciterFamily {
        match self {
            ExciterType::HandStrike
            | ExciterType::FeltMallet
            | ExciterType::HardMallet
            | ExciterType::Drumstick
            | ExciterType::WireBrush
            | ExciterType::MetalPipe
            | ExciterType::MetalChain => ExciterFamily::Hit,
            ExciterType::Bow
            | ExciterType::StiffPoint
            | ExciterType::HeavyGrinding
            | ExciterType::CorrugatedDrag
            | ExciterType::TensionRise => ExciterFamily::Friction,
            ExciterType::PneumaticJet
            | ExciterType::ElectromagneticHum
            | ExciterType::TensionSnap
            | ExciterType::ParticleRain => ExciterFamily::Specialty,
        }
    }

    /// Converts ExciterType to integer for parameter storage.
    ///
    /// # Returns
    /// Integer value (0-16) for serialization
    pub fn to_int(self) -> i32 {
        match self {
            ExciterType::Bow => 1,
            ExciterType::HandStrike => 2,
            ExciterType::FeltMallet => 3,
            ExciterType::HardMallet => 4,
            ExciterType::Drumstick => 5,
            ExciterType::WireBrush => 6,
            ExciterType::MetalPipe => 7,
            ExciterType::MetalChain => 8,
            ExciterType::StiffPoint => 9,
            ExciterType::HeavyGrinding => 10,
            ExciterType::CorrugatedDrag => 11,
            ExciterType::TensionRise => 12,
            ExciterType::PneumaticJet => 13,
            ExciterType::ElectromagneticHum => 14,
            ExciterType::TensionSnap => 15,
            ExciterType::ParticleRain => 16,
        }
    }
}

/// Parse `text` as either a plain parameter value or, when it ends with
/// `suffix` (or starts with `prefix`), as the displayed factor converted back
/// by `from_factor`.
fn parse_factor_text(
    text: &str,
    prefix: &[&str],
    suffix: &str,
    from_factor: fn(f32) -> f32,
) -> Option<f32> {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    if !suffix.is_empty() && lower.ends_with(&suffix.to_ascii_lowercase()) {
        let number = text[..text.len() - suffix.len()].trim();
        return number.parse::<f32>().ok().map(from_factor);
    }
    for p in prefix {
        if let Some(number) = text.strip_prefix(p) {
            return number.trim().parse::<f32>().ok().map(from_factor);
        }
    }
    text.parse::<f32>().ok()
}

/// The global Pressure: a trim on every exciter's force, from −12 dB at 0
/// through 0 dB at its default 0.5 to +12 dB at 1, live on held notes. Its
/// value is shown in dB; typing "6 dB" or a plain 0–1 value both work
/// (`h06-wire-global-exciter-controls`).
pub fn exciter_pressure_param(default: f32) -> FloatParam {
    FloatParam::new(
        "Pressure",
        default,
        FloatRange::Linear { min: 0.0, max: 1.0 },
    )
    .with_value_to_string(Arc::new(|value| {
        let db = 20.0 * crate::voice::pressure_factor(value).log10();
        format!("{db:+.1} dB")
    }))
    .with_string_to_value(Arc::new(|string| {
        parse_factor_text(string, &[], "dB", |db| {
            (0.5 + (db / 20.0) * std::f32::consts::LOG2_10 / 4.0).clamp(0.0, 1.0)
        })
    }))
}

/// The global Speed: a multiplier on every motion speed (and divisor on
/// every spread), ×0.5 at 0, ×1 at its default 0.5 and ×2 at 1, applied at
/// note-on. Its value is shown as the multiplier; typing "×1.5", "x1.5" or a
/// plain 0–1 value all work.
pub fn exciter_speed_param(default: f32) -> FloatParam {
    FloatParam::new("Speed", default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_value_to_string(Arc::new(|value| {
            format!("×{:.2}", crate::voice::speed_factor(value))
        }))
        .with_string_to_value(Arc::new(|string| {
            parse_factor_text(string, &["×", "x", "X"], "", |factor| {
                (0.5 + factor.max(f32::MIN_POSITIVE).log2() / 2.0).clamp(0.0, 1.0)
            })
        }))
}

/// The global Roughness: a multiplier on every grit and randomness amount,
/// ×0 at 0, ×1 at its default 0.3 and ×3.33 at 1, applied at note-on. Its
/// value is shown as the multiplier; typing "×2", "x2" or a plain 0–1 value
/// all work.
pub fn exciter_roughness_param(default: f32) -> FloatParam {
    FloatParam::new(
        "Roughness",
        default,
        FloatRange::Linear { min: 0.0, max: 1.0 },
    )
    .with_value_to_string(Arc::new(|value| {
        format!("×{:.2}", crate::voice::roughness_factor(value))
    }))
    .with_string_to_value(Arc::new(|string| {
        parse_factor_text(string, &["×", "x", "X"], "", |factor| {
            (factor * 0.3).clamp(0.0, 1.0)
        })
    }))
}

/// The position wander's rate: 0.1–5 Hz, default 0.1, skewed so the slow
/// rates get more knob travel (`h09-route-envelope-to-strike-position`).
pub fn position_wander_rate_param(default: f32) -> FloatParam {
    FloatParam::new(
        "Wander Rate",
        default,
        FloatRange::Skewed {
            min: 0.1,
            max: 5.0,
            factor: FloatRange::skew_factor(-1.5),
        },
    )
    .with_unit(" Hz")
    .with_value_to_string(formatters::v2s_f32_rounded(2))
}

/// The tempo-sync parameter: Off by default, so Delay Time works out of the
/// box. Its value is shown as "Off" or the division the echo plays, from
/// `sync_division`, and typing a division name selects the middle of its
/// band (`h03-default-tempo-sync-off`).
pub fn sync_rate_param(default: f32) -> FloatParam {
    FloatParam::new(
        "Sync Rate",
        default,
        FloatRange::Linear { min: 0.0, max: 1.0 },
    )
    .with_value_to_string(Arc::new(|value| match sync_division(value) {
        None => "Off".to_string(),
        Some(index) => SYNC_DIVISIONS[index].1.to_string(),
    }))
    .with_string_to_value(Arc::new(|string| {
        let text = string.trim();
        if text.eq_ignore_ascii_case("off") {
            return Some(0.0);
        }
        let bands = SYNC_DIVISIONS.len() as f32;
        SYNC_DIVISIONS
            .iter()
            .position(|(_, name)| name.eq_ignore_ascii_case(text))
            .map(|index| SYNC_OFF_BELOW + (index as f32 + 0.5) / bands * (1.0 - SYNC_OFF_BELOW))
    }))
}

/// `sync_rate` values below this are Off: the echo free-runs on Delay Time.
pub const SYNC_OFF_BELOW: f32 = 0.05;

/// Tempo-sync divisions in beats (1/4 note = 1 beat) and their display
/// names, ascending, so a higher `sync_rate` picks a longer delay.
pub const SYNC_DIVISIONS: [(f32, &str); 6] = [
    (0.25, "1/16"),
    (0.5, "1/8"),
    (1.0, "1/4"),
    (2.0, "1/2"),
    (4.0, "1/1"),
    (8.0, "2/1"),
];

/// The `SYNC_DIVISIONS` index `sync_rate` selects, or `None` when sync is
/// Off. Above the Off region the range is split into six equal bands. The
/// echo's tempo mapping and the parameter's display both use this, so the
/// display always names the division the echo plays
/// (`h03-default-tempo-sync-off`).
pub fn sync_division(sync_rate: f32) -> Option<usize> {
    if sync_rate < SYNC_OFF_BELOW {
        return None;
    }
    let normalized = ((sync_rate - SYNC_OFF_BELOW) / (1.0 - SYNC_OFF_BELOW)).clamp(0.0, 1.0);
    let index = (normalized * SYNC_DIVISIONS.len() as f32) as usize;
    Some(index.min(SYNC_DIVISIONS.len() - 1))
}

/// Creates an IntParam for exciter selection with name display and parsing.
///
/// # Arguments
/// * `default` - Default exciter type index (1-16)
///
/// # Returns
/// Configured IntParam with value-to-string and string-to-value conversions
pub fn exciter_param(default: i32) -> IntParam {
    let default = ExciterType::from_int(default).to_int();

    IntParam::new("Exciter", default, IntRange::Linear { min: 1, max: 16 })
        .with_value_to_string(Arc::new(|value| {
            ExciterType::from_int(value).name().to_string()
        }))
        .with_string_to_value(Arc::new(|string| {
            let normalized = string.trim();
            exciter_model_items()
                .iter()
                .find(|(_, name)| name.eq_ignore_ascii_case(normalized))
                .map(|(value, _)| *value)
        }))
}

pub fn exciter_model_items() -> &'static [(i32, &'static str)] {
    &[
        (2, "Hand Strike"),
        (3, "Felt Mallet"),
        (4, "Hard Mallet"),
        (5, "Drumstick"),
        (6, "Wire Brush"),
        (7, "Metal Pipe"),
        (8, "Metal Chain"),
        (1, "Bow"),
        (9, "Stiff Point Scrape"),
        (10, "Heavy Grinding"),
        (11, "Corrugated Drag"),
        (12, "Tension Rise"),
        (13, "Pneumatic Jet"),
        (14, "Electromagnetic Hum"),
        (15, "Tension Snap"),
        (16, "Particle Rain"),
    ]
}

/// Quality mode enumeration for CPU/quality tradeoff.
///
/// Every mode runs the full post chain; modes change only the clipper's
/// oversampling (1×/4×/8×/16×) and the per-voice modal density
/// (0.5×/1×/1.5×/2× the object's base mode count).
///
/// - **Eco**: No oversampling, half modal density
/// - **Normal**: Balanced quality (default, matches pre-quality-mode behavior)
/// - **High**: 8× oversampling, 1.5× modal density
/// - **Render**: Maximum quality, may be too expensive for live use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum QualityMode {
    Eco,
    Normal,
    High,
    Render,
}

impl QualityMode {
    pub fn from_int(v: i32) -> Self {
        match v {
            0 => QualityMode::Eco,
            1 => QualityMode::Normal,
            2 => QualityMode::High,
            3 => QualityMode::Render,
            _ => QualityMode::Normal,
        }
    }

    pub fn to_int(self) -> i32 {
        match self {
            QualityMode::Eco => 0,
            QualityMode::Normal => 1,
            QualityMode::High => 2,
            QualityMode::Render => 3,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            QualityMode::Eco => "Eco",
            QualityMode::Normal => "Normal",
            QualityMode::High => "High",
            QualityMode::Render => "Render",
        }
    }
}

/// Play mode for the voice layer (PRD §9.2–9.3).
///
/// - **Tonal** — Default. The note picks the pitch; the global `object` param
///   picks the resonator family.
/// - **Kit** — Percussion-kit mapping (PRD §9.2). The MIDI note range picks
///   the object family (10 notes per object across the full keyboard); the
///   note still drives pitch within each kit slot for tonal variation.
/// - **Drone** — Sustained mode (PRD §9.3). Forces the MSEG loop on
///   (decay→sustain) at note-on so friction voices ring indefinitely until
///   note-off. Hit-style exciter families keep their one-shot envelope so
///   they aren't silently neutered; explicit drone of those families is
///   tracked as a future expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PlayMode {
    Tonal,
    Kit,
    Drone,
}

impl PlayMode {
    pub fn from_int(v: i32) -> Self {
        match v {
            0 => PlayMode::Tonal,
            1 => PlayMode::Kit,
            2 => PlayMode::Drone,
            _ => PlayMode::Tonal,
        }
    }

    pub fn to_int(self) -> i32 {
        match self {
            PlayMode::Tonal => 0,
            PlayMode::Kit => 1,
            PlayMode::Drone => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PlayMode::Tonal => "Tonal",
            PlayMode::Kit => "Kit",
            PlayMode::Drone => "Drone",
        }
    }
}

/// Resonator object type enumeration.
///
/// Each object has distinct modal characteristics based on physical geometry:
/// - **Pipe**: Cylindrical resonator with longitudinal modes
/// - **Plate**: 2D surface with complex modal density
/// - **Tank**: Volumetric resonator with helmholtz modes
/// - **Chain**: 1D linked segments
/// - **IBeam**: Structural beam with bending/torsional modes
/// - **TautCable**: String-like with harmonic overtones
/// - **CoilSpring**: Helical spring with non-harmonic modes
/// - **SheetMetal**: Thin plate with high modal density
/// - **IndustrialCog**: Rotating object with cyclic modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Object {
    /// Cylindrical pipe resonator
    Pipe,
    /// Rectangular plate resonator
    Plate,
    /// Enclosed tank/volume
    Tank,
    /// Hanging chain segments
    Chain,
    /// Structural I-beam
    IBeam,
    /// Tensioned cable/string
    TautCable,
    /// Helical coil spring
    CoilSpring,
    /// Thin sheet metal panel
    SheetMetal,
    /// Industrial gear/cog
    IndustrialCog,
}

impl Object {
    /// Converts integer parameter value to Object enum.
    ///
    /// # Arguments
    /// * `v` - Integer value (0-8)
    ///
    /// # Returns
    /// Corresponding Object variant, defaulting to Pipe for invalid values
    pub fn from_int(v: i32) -> Self {
        match v {
            0 => Object::Pipe,
            1 => Object::Plate,
            2 => Object::Tank,
            3 => Object::Chain,
            4 => Object::IBeam,
            5 => Object::TautCable,
            6 => Object::CoilSpring,
            7 => Object::SheetMetal,
            8 => Object::IndustrialCog,
            _ => Object::Pipe,
        }
    }

    /// Returns human-readable name for UI display.
    ///
    /// # Returns
    /// Static string slice with display name
    pub fn name(self) -> &'static str {
        match self {
            Object::Pipe => "Pipe",
            Object::Plate => "Plate",
            Object::Tank => "Tank",
            Object::Chain => "Chain",
            Object::IBeam => "I-Beam",
            Object::TautCable => "Taut Cable",
            Object::CoilSpring => "Coil Spring",
            Object::SheetMetal => "Sheet Metal",
            Object::IndustrialCog => "Industrial Cog",
        }
    }

    /// Converts Object to integer for parameter storage.
    ///
    /// # Returns
    /// Integer value (0-8) for serialization
    pub fn to_int(self) -> i32 {
        match self {
            Object::Pipe => 0,
            Object::Plate => 1,
            Object::Tank => 2,
            Object::Chain => 3,
            Object::IBeam => 4,
            Object::TautCable => 5,
            Object::CoilSpring => 6,
            Object::SheetMetal => 7,
            Object::IndustrialCog => 8,
        }
    }
}

/// Creates an IntParam for object selection with name display and parsing.
///
/// # Arguments
/// * `default` - Default object index (0-8)
///
/// # Returns
/// Configured IntParam with value-to-string and string-to-value conversions
pub fn object_param(default: i32) -> IntParam {
    IntParam::new("Object", default, IntRange::Linear { min: 0, max: 8 })
        .with_value_to_string(Arc::new(|value| Object::from_int(value).name().to_string()))
        .with_string_to_value(Arc::new(|string| {
            let normalized = string.trim();
            [
                Object::Pipe,
                Object::Plate,
                Object::Tank,
                Object::Chain,
                Object::IBeam,
                Object::TautCable,
                Object::CoilSpring,
                Object::SheetMetal,
                Object::IndustrialCog,
            ]
            .into_iter()
            .find(|object| object.name().eq_ignore_ascii_case(normalized))
            .map(Object::to_int)
        }))
}

/// Test-only helper that constructs a plain `FloatParam` with linear range.
///
/// Used by integration tests to swap an existing macro / scalar field on a
/// shared `Arc<CorrosionParams>` (`Arc::get_mut` + field replacement) without
/// needing to mint the entire struct from scratch.
#[cfg(test)]
pub fn float_param_for_test(name: &'static str, value: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, value, FloatRange::Linear { min, max })
}

/// Build the limiter-mode `IntParam` with mappings for Hard / Lookahead.
pub fn limiter_mode_param(default: i32) -> IntParam {
    IntParam::new("Limiter Mode", default, IntRange::Linear { min: 0, max: 1 })
        .with_value_to_string(Arc::new(|value| match value {
            0 => "Hard".to_string(),
            1 => "Lookahead".to_string(),
            _ => "Hard".to_string(),
        }))
        .with_string_to_value(Arc::new(|string| match string.trim() {
            s if s.eq_ignore_ascii_case("hard") => Some(0),
            s if s.eq_ignore_ascii_case("lookahead") => Some(1),
            _ => None,
        }))
}

/// Build the play-mode `IntParam` with mappings for Tonal/Kit/Drone.
pub fn play_mode_param(default: i32) -> IntParam {
    IntParam::new("Play Mode", default, IntRange::Linear { min: 0, max: 2 })
        .with_value_to_string(Arc::new(|value| {
            PlayMode::from_int(value).name().to_string()
        }))
        .with_string_to_value(Arc::new(|string| {
            let normalized = string.trim();
            [PlayMode::Tonal, PlayMode::Kit, PlayMode::Drone]
                .into_iter()
                .find(|mode| mode.name().eq_ignore_ascii_case(normalized))
                .map(PlayMode::to_int)
        }))
}

pub fn quality_mode_param(default: i32) -> IntParam {
    IntParam::new("Quality Mode", default, IntRange::Linear { min: 0, max: 3 })
        .with_value_to_string(Arc::new(|value| {
            QualityMode::from_int(value).name().to_string()
        }))
        .with_string_to_value(Arc::new(|string| {
            let normalized = string.trim();
            [
                QualityMode::Eco,
                QualityMode::Normal,
                QualityMode::High,
                QualityMode::Render,
            ]
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(normalized))
            .map(QualityMode::to_int)
        }))
}

/// Create a float parameter with deterministic string roundtrips.
///
/// CLAP hosts and validators are allowed to convert normalized values to text,
/// parse that text back, and compare the displayed result. Rust's default float
/// formatting can expose last-digit `f32` rounding differences in that path, so
/// parameters that need plain numeric display use a fixed decimal precision.
fn display_float_param(name: &'static str, value: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, value, FloatRange::Linear { min, max })
        .with_value_to_string(Arc::new(|value| format!("{value:.6}")))
        .with_string_to_value(Arc::new(|string| string.trim().parse().ok()))
}

impl Default for CorrosionParams {
    /// Creates default parameter values for plugin initialization.
    ///
    /// Sets sensible defaults for all 130+ parameters to ensure
    /// the plugin produces audible output immediately upon loading.
    fn default() -> Self {
        Self {
            #[cfg(feature = "gui")]
            // 75 % of the 1080 x 768 base: fits a host window on a 1080p screen.
            editor_state: EguiState::from_size(810, 576),

            // Sound generation defaults
            exciter: exciter_param(ExciterType::HandStrike.to_int()),
            object: object_param(0), // Pipe
            quality_mode: quality_mode_param(QualityMode::Normal.to_int()),
            play_mode: play_mode_param(PlayMode::Tonal.to_int()),
            limiter_mode: limiter_mode_param(0),

            // Object transformation defaults
            size: FloatParam::new(
                "Size",
                1.0,
                FloatRange::Linear {
                    min: 0.05,
                    max: 10.0,
                },
            ),
            rust: FloatParam::new("Rust", 0.0, FloatRange::Linear { min: 0.0, max: 5.0 }),
            damage: FloatParam::new(
                "Damage",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),

            // Master controls
            drive: FloatParam::new("Drive", 0.0, FloatRange::Linear { min: 0.0, max: 5.0 }),
            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Linear {
                    min: 0.0,
                    max: util::db_to_gain(40.0),
                },
            ),
            width: FloatParam::new(
                "Width",
                0.5,
                FloatRange::Linear {
                    min: -2.0,
                    max: 3.0,
                },
            ),
            // 2.62 reproduces the pre-C5 default-note level (peak 0.118) with
            // the parallel body routing; see `c5_measurements::c5_default_body`.
            body: FloatParam::new("Body", 2.62, FloatRange::Linear { min: 0.0, max: 5.0 }),

            // UI scale (percentage display)
            ui_scale: IntParam::new("UI Scale", 1, IntRange::Linear { min: 0, max: 4 })
                .with_value_to_string(Arc::new(|value| match value {
                    0 => "50%".to_string(),
                    1 => "75%".to_string(),
                    2 => "100%".to_string(),
                    3 => "125%".to_string(),
                    4 => "150%".to_string(),
                    _ => "100%".to_string(),
                }))
                .with_string_to_value(Arc::new(|string| match string.trim() {
                    "50%" => Some(0),
                    "75%" => Some(1),
                    "100%" => Some(2),
                    "125%" => Some(3),
                    "150%" => Some(4),
                    _ => None,
                })),

            // ADSR envelope defaults
            env_attack: FloatParam::new(
                "Attack",
                0.05,
                FloatRange::Linear {
                    min: 0.001,
                    max: 2.0,
                },
            ),
            env_decay: FloatParam::new(
                "Decay",
                0.3,
                FloatRange::Linear {
                    min: 0.01,
                    max: 5.0,
                },
            ),
            env_sustain: FloatParam::new("Sustain", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 }),
            env_release: FloatParam::new(
                "Release",
                0.5,
                FloatRange::Linear {
                    min: 0.01,
                    max: 5.0,
                },
            ),

            // MSEG envelope defaults
            mseg_onset: FloatParam::new(
                "Onset",
                0.01,
                FloatRange::Linear {
                    min: 0.001,
                    max: 1.0,
                },
            ),
            mseg_attack: FloatParam::new(
                "MSEG Attack",
                0.05,
                FloatRange::Linear {
                    min: 0.001,
                    max: 2.0,
                },
            ),
            mseg_hold: FloatParam::new("Hold", 0.02, FloatRange::Linear { min: 0.0, max: 2.0 }),
            mseg_decay: FloatParam::new(
                "MSEG Decay",
                0.3,
                FloatRange::Linear {
                    min: 0.01,
                    max: 5.0,
                },
            ),
            mseg_sustain: FloatParam::new(
                "MSEG Sustain",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            mseg_release: FloatParam::new(
                "MSEG Release",
                0.1,
                FloatRange::Linear {
                    min: 0.01,
                    max: 5.0,
                },
            ),

            // Envelope modulation defaults
            env_amount: FloatParam::new(
                "Env Amount",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            velocity_to_peak: FloatParam::new(
                "Velocity To Peak",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // MSEG loop defaults
            loop_mode: IntParam::new("Loop Mode", 0, IntRange::Linear { min: 0, max: 2 }),
            loop_start_stage: IntParam::new(
                "Loop Start Stage",
                3,
                IntRange::Linear { min: 0, max: 5 },
            ),
            loop_end_stage: IntParam::new("Loop End Stage", 4, IntRange::Linear { min: 0, max: 5 }),

            // Timing and sync
            sync_rate: sync_rate_param(0.0),
            global_time_scale: FloatParam::new(
                "Global Time Scale",
                1.0,
                FloatRange::Linear {
                    min: 0.1,
                    max: 10.0,
                },
            ),
            velocity_to_level: FloatParam::new(
                "Velocity To Level",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            velocity_to_time: FloatParam::new(
                "Velocity To Time",
                0.15,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            curve_tension: FloatParam::new(
                "Curve Tension",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),

            // Exciter control defaults
            exciter_pressure: exciter_pressure_param(0.5),
            exciter_speed: exciter_speed_param(0.5),
            exciter_roughness: exciter_roughness_param(0.3),

            // Dedicated exciter defaults derived from the previous generic mappings
            hand_mass: display_float_param("Hand Mass", 1.7, 0.4, 3.0),
            flesh_stiffness: display_float_param("Flesh Stiffness", 0.425, 0.05, 0.8),
            flesh_damping: display_float_param("Flesh Damping", 0.75, 0.3, 1.8),
            mute_decay: display_float_param("Mute Decay", 0.9245, 0.85, 0.999),
            mallet_mass: display_float_param("Mallet Mass", 1.5, 0.4, 3.5),
            felt_softness: display_float_param("Felt Softness", 0.94, 0.1, 1.3),
            core_hardness: display_float_param("Core Hardness", 2.5, 0.5, 4.5),
            compression_curve: display_float_param("Compression Curve", 2.9, 2.0, 5.0),
            material_stiffness: display_float_param("Material Stiffness", 2.75, 0.5, 5.0),
            impact_damping: display_float_param("Impact Damping", 0.46, 0.1, 1.3),
            stick_mass: display_float_param("Stick Mass", 0.65, 0.05, 1.25),
            tip_stiffness: display_float_param("Tip Stiffness", 3.8, 0.8, 6.8),
            restitution_bounciness: display_float_param("Restitution Bounciness", 0.41, 0.2, 0.9),
            micro_bounce_limit: display_float_param("Micro Bounce Limit", 4.0, 2.0, 8.0),
            wire_density: display_float_param("Wire Density", 46.0, 10.0, 130.0),
            spread_duration: display_float_param("Spread Duration", 130.0, 10.0, 250.0),
            brush_wire_stiffness: display_float_param("Wire Stiffness", 0.3, 0.0, 1.0),
            amplitude_randomization: display_float_param("Amplitude Randomization", 0.3, 0.0, 1.0),
            pipe_mass: display_float_param("Pipe Mass", 1.5, 0.4, 2.6),
            metal_stiffness: display_float_param("Metal Stiffness", 3.0, 0.5, 5.5),
            pipe_pitch: display_float_param("Pipe Pitch", 1.1, 0.5, 2.5),
            pipe_ring_decay: display_float_param("Pipe Ring Decay", 0.9795, 0.96, 0.999),
            link_count: display_float_param("Link Count", 9.0, 3.0, 15.0),
            chain_mass: display_float_param("Chain Mass", 0.8, 0.2, 1.4),
            drop_envelope_spread: display_float_param("Drop Envelope Spread", 220.0, 40.0, 400.0),
            internal_rattle: display_float_param("Internal Rattle", 0.3, 0.0, 1.0),
            rattle_color: display_float_param("Rattle Color", 0.3, 0.0, 1.0),
            bow_pressure: display_float_param("Bow Pressure", 1.1, 0.2, 2.0),
            bow_speed: display_float_param("Bow Speed", 1.05, 0.1, 2.0),
            rosin_grip: display_float_param("Rosin Grip", 0.485, 0.05, 1.5),
            slip_curve: display_float_param("Slip Curve", 0.485, 0.05, 1.5),
            scrape_speed: display_float_param("Scrape Speed", 0.82, 0.1, 2.5),
            point_pressure: display_float_param("Point Pressure", 0.8, 0.1, 1.5),
            chatter_pitch: display_float_param("Chatter Pitch", 0.52, 0.1, 1.5),
            chatter_damping: display_float_param("Chatter Damping", 0.66, 0.1, 0.9),
            grind_speed: display_float_param("Grind Speed", 0.82, 0.1, 2.5),
            grind_pressure: display_float_param("Grind Pressure", 1.0, 0.1, 1.9),
            surface_grit: display_float_param("Surface Grit", 0.3, 0.0, 1.0),
            grit_color: display_float_param("Grit Color", 0.3, 0.0, 1.0),
            drag_speed: display_float_param("Drag Speed", 0.82, 0.1, 2.5),
            ridge_spacing: display_float_param("Ridge Spacing", 0.143, 0.01, 0.2),
            ridge_depth: display_float_param("Ridge Depth", 0.6, 0.0, 2.0),
            drag_exciter_mass: display_float_param("Exciter Mass", 1.1, 0.2, 2.0),
            pull_speed: display_float_param("Pull Speed", 0.8, 0.05, 1.55),
            break_threshold: display_float_param("Break Threshold", 0.85, 0.1, 1.6),
            slip_stochasticity: display_float_param("Slip Stochasticity", 0.3, 0.0, 1.0),
            creak_sharpness: display_float_param("Creak Sharpness", 0.56, 0.2, 1.4),
            air_pressure: display_float_param("Air Pressure", 1.1, 0.1, 2.1),
            nozzle_width: display_float_param("Nozzle Width", 0.85, 0.1, 1.6),
            turbulence_chaos: display_float_param("Turbulence Chaos", 0.6, 0.0, 2.0),
            mains_frequency: display_float_param("Mains Frequency", 80.0, 40.0, 120.0),
            coil_proximity: display_float_param("Coil Proximity", 1.0, 0.0, 2.0),
            voltage_sag: display_float_param("Voltage Sag", 0.6, 0.0, 2.0),
            pull_distance: display_float_param("Pull Distance", 0.8, 0.1, 1.5),
            hook_stiffness: display_float_param("Hook Stiffness", 1.2, 0.2, 2.2),
            snap_force: display_float_param("Snap Force", 0.67, 0.1, 2.0),
            flow_rate: display_float_param("Flow Rate", 1.6, 0.1, 3.1),
            particle_mass: display_float_param("Particle Mass", 0.525, 0.05, 1.0),
            mass_variance: display_float_param("Mass Variance", 0.6, 0.0, 2.0),

            // Interaction defaults
            strike_position: FloatParam::new(
                "Strike Position",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            coupling_stiffness: FloatParam::new(
                "Coupling Stiffness",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            position_wander: FloatParam::new(
                "Position Wander",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            position_wander_rate: position_wander_rate_param(0.1),
            position_envelope: FloatParam::new(
                "Position Envelope",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            fundamental_anchor: FloatParam::new(
                "Fundamental Anchor",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Resonator control defaults
            res_damping: FloatParam::new("Damping", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            res_brightness: FloatParam::new(
                "Brightness",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            thickness: FloatParam::new("Thickness", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            heat: FloatParam::new("Heat", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            sludge: FloatParam::new("Sludge", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),

            // Per-object character defaults
            pipe_diameter: FloatParam::new(
                "Pipe Diameter",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            plate_aspect: FloatParam::new(
                "Plate Aspect",
                1.0,
                FloatRange::Linear { min: 0.1, max: 4.0 },
            ),
            plate_stiffness: FloatParam::new(
                "Plate Stiffness",
                1.0,
                FloatRange::Linear {
                    min: 0.25,
                    max: 3.0,
                },
            ),
            tank_volume: FloatParam::new(
                "Tank Volume",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            tank_cavity_mix: FloatParam::new(
                "Cavity Mix",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            chain_link_mass: FloatParam::new(
                "Link Mass",
                0.5,
                FloatRange::Linear { min: 0.1, max: 1.0 },
            ),
            chain_instability: FloatParam::new(
                "Instability",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            beam_shear: FloatParam::new(
                "Shear Density",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            cable_braid: FloatParam::new(
                "Braid Stiffness",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            cable_tension_drop: FloatParam::new(
                "Tension Drop",
                0.4,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            spring_dispersion: FloatParam::new(
                "Dispersion Chirp",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            spring_slosh: FloatParam::new(
                "Spring Slosh",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            sheet_thinness: FloatParam::new(
                "Metal Thinness",
                0.4,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            cog_dissonance: FloatParam::new(
                "Tooth Dissonance",
                0.1,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Post-processing filter defaults
            filter_cutoff: FloatParam::new(
                "Filter Cutoff",
                20000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: 0.5,
                },
            ),
            filter_resonance: FloatParam::new(
                "Filter Resonance",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            component_tolerance: FloatParam::new(
                "Component Tolerance",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Drive defaults
            drive_amount: FloatParam::new(
                "Drive Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 5.0 },
            ),
            bias_starvation: FloatParam::new(
                "Bias Starvation",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            chaos_depth: FloatParam::new(
                "Chaos Depth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Stereo/spatial defaults
            spread_width: FloatParam::new(
                "Spread Width",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            listener_proximity: FloatParam::new(
                "Listener Proximity",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Body resonance defaults
            chassis_material: FloatParam::new(
                "Chassis Material",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            chassis_volume: FloatParam::new(
                "Chassis Volume",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Space/reverb defaults
            space_mode: IntParam::new("Space Mode", 0, IntRange::Linear { min: 0, max: 3 }),
            space_amount: FloatParam::new(
                "Space Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            factory_size: FloatParam::new(
                "Factory Size",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            machinery_clutter: FloatParam::new(
                "Machinery Clutter",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            wall_impedance: FloatParam::new(
                "Wall Impedance",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            spring_tension: FloatParam::new(
                "Spring Tension",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            wire_stiffness: FloatParam::new(
                "Wire Stiffness",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            spring_tank_size: FloatParam::new(
                "Spring Tank Size",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            delay_time: FloatParam::new(
                "Delay Time",
                0.25,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            machinery_movement: FloatParam::new(
                "Machinery Movement",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            high_frequency_damping: FloatParam::new(
                "High Frequency Damping",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Output stage defaults
            analog_ceiling: FloatParam::new(
                "Analog Ceiling",
                0.9661,
                FloatRange::Linear { min: 0.5, max: 1.0 },
            ),
            diode_softness: FloatParam::new(
                "Diode Softness",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            // Macro defaults — 0.5 is the neutral midpoint so existing presets
            // sound unchanged until the user actively dials a macro.
            macro_mass: FloatParam::new("Mass", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro_corrosion: FloatParam::new(
                "Corrosion",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            macro_violence: FloatParam::new(
                "Violence",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            macro_brightness: FloatParam::new(
                "Brightness",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{exciter_pressure_param, exciter_roughness_param, exciter_speed_param};
    use nih_plug::prelude::Param;

    #[test]
    fn global_exciter_controls_show_their_factor() {
        let text = |param: &nih_plug::prelude::FloatParam, value: f32| {
            param.normalized_value_to_string(param.preview_normalized(value), false)
        };
        let pressure = exciter_pressure_param(0.5);
        let speed = exciter_speed_param(0.5);
        let roughness = exciter_roughness_param(0.3);
        assert_eq!(text(&pressure, 0.5), "+0.0 dB");
        assert_eq!(text(&pressure, 1.0), "+12.0 dB");
        assert_eq!(text(&speed, 0.5), "×1.00");
        assert_eq!(text(&speed, 1.0), "×2.00");
        assert_eq!(text(&roughness, 0.3), "×1.00");
        assert_eq!(text(&roughness, 0.0), "×0.00");
    }

    #[test]
    fn global_exciter_controls_parse_what_they_show() {
        let parse = |param: &nih_plug::prelude::FloatParam, text: &str| {
            param
                .string_to_normalized_value(text)
                .map(|n| param.preview_plain(n))
        };
        let close = |got: Option<f32>, want: f32| got.is_some_and(|v| (v - want).abs() < 1e-4);
        let pressure = exciter_pressure_param(0.5);
        let speed = exciter_speed_param(0.5);
        let roughness = exciter_roughness_param(0.3);
        // ×4 is +12.04 dB, so +12.0 dB lands just below the maximum.
        assert!(close(parse(&pressure, "+12.0 dB"), 0.998_289));
        assert!(close(parse(&pressure, "0 dB"), 0.5));
        assert!(close(parse(&pressure, "0.25"), 0.25));
        assert!(close(parse(&speed, "×2.00"), 1.0));
        assert!(close(parse(&speed, "x0.5"), 0.0));
        assert!(close(parse(&speed, "0.75"), 0.75));
        assert!(close(parse(&roughness, "×1.00"), 0.3));
        assert!(close(parse(&roughness, "x2"), 0.6));
        assert!(close(parse(&roughness, "0.9"), 0.9));
    }
}

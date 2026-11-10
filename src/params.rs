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

    /// The kit (`per-key-kits`): Kit mode's pads, saved with the host's
    /// project state. Touched only off the audio thread; `kit_table` is the
    /// audio thread's copy, rebuilt from it after a restore
    /// (`Plugin::initialize`) and written with it by the editor.
    #[persist = "kit"]
    pub kit: Arc<std::sync::RwLock<crate::kit::KitState>>,

    /// The pads as the audio thread reads them at note-on (not a parameter).
    pub kit_table: Arc<crate::kit::KitTable>,

    /// Exciter type selection, 1–16 (`ExciterType::to_int`); see
    /// `ExciterType::from_int` for out-of-range values.
    /// Controls the type of physical excitation (mallets, scrapes, specialty)
    #[id = "exciter"]
    pub exciter: IntParam,

    /// Resonator object type (0-8 mapping to Object enum)
    /// Determines the modal characteristics (pipe vs plate vs tank, etc.)
    #[id = "object"]
    pub object: IntParam,

    /// Quality mode (0=Eco, 1=Normal, 2=High, 3=Render): the clipper's
    /// oversampling (1×/4×/8×/16×) and each voice's mode count
    /// (0.5×/1×/1.5×/2×). The oversampling changes at once; the mode count is
    /// taken at note-on, so sounding notes keep theirs (M14).
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
    /// The 100% editor baseline is the 1080x840 logical window (768 of
    /// panels plus the keyboard strip).
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
    /// MSEG onset time - initial transient stage's length (0.001 to 1.0s)
    #[id = "mseg_onset"]
    pub mseg_onset: FloatParam,

    /// MSEG onset level - the level the onset stage rises to (0.0 to 1.0)
    #[id = "mseg_onset_level"]
    pub mseg_onset_level: FloatParam,

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

    /// Diode softness (0.0 to 1.0): the output clipper's knee width, from a
    /// hard knee at 0 to a knee starting 14 dB under the curve's limit at 1
    /// (`add-asymmetric-diode-stage`).
    #[id = "diode_softness"]
    pub diode_softness: FloatParam,

    /// Diode asymmetry (0.0 to 1.0): lowers the clipper's negative limit, to
    /// 6 dB under the positive one at 1, adding even harmonics
    /// (`add-asymmetric-diode-stage`).
    #[id = "diode_asymmetry"]
    pub diode_asymmetry: FloatParam,

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
    /// * `v` - Integer value, 1–16 for the sixteen exciters
    ///
    /// # Returns
    /// The matching variant. Any other value (0, 17, negative numbers, as in
    /// a hand-edited preset) gives `HandStrike`, the default exciter, so one
    /// bad field doesn't reject a whole preset; `out_of_range_exciter_loads_as_hand_strike`
    /// (`tests/preset_roundtrip.rs`) pins this.
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
    /// Integer value, 1–16, for serialization
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
        // Round first, so a value just under 0 dB shows "+0.0", not "-0.0".
        let db = (db * 10.0).round() / 10.0 + 0.0;
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

/// Default Onset Level: the level the old shared Onset knob gave at its
/// default (0.01 s, clamped to 0–1), so the default patch is unchanged
/// (`m01-m04-m05-clarify-parameters`).
pub const DEFAULT_MSEG_ONSET_LEVEL: f32 = 0.01;

/// Build the friction MSEG's Onset Level `FloatParam` (0–1), shared by the
/// live parameters and `Preset::into_params`.
pub fn mseg_onset_level_param(value: f32) -> FloatParam {
    FloatParam::new(
        "Onset Level",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    )
}

/// Build the Output `FloatParam`, shared by the live parameters and
/// `Preset::into_params`. The stored value is the linear gain (0 to +40 dB),
/// so presets and projects keep their level; it's shown and typed in dB, and
/// the gain-skewed taper puts −10 dB at the middle of the travel, 0 dB at
/// about 57 % and −20 to +6 dB across about 19 %
/// (`m01-m04-m05-clarify-parameters`, M5).
pub fn output_param(gain: f32) -> FloatParam {
    let max = util::db_to_gain(40.0);
    FloatParam::new(
        "Output",
        gain.clamp(0.0, max),
        FloatRange::Skewed {
            min: 0.0,
            max,
            factor: FloatRange::gain_skew_factor(-60.0, 40.0),
        },
    )
    .with_unit(" dB")
    .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
    .with_string_to_value(formatters::s2v_f32_gain_to_db())
}

/// Default Diode Asymmetry for new instances: the negative limit 1.5 dB
/// under the positive one (`add-asymmetric-diode-stage`).
pub const DEFAULT_DIODE_ASYMMETRY: f32 = 0.25;

/// Build the Diode Asymmetry `FloatParam` (0–1), shared by the live
/// parameters and `Preset::into_params`.
pub fn diode_asymmetry_param(value: f32) -> FloatParam {
    FloatParam::new(
        "Diode Asymmetry",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    )
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
    IntParam::new("Note Mode", default, IntRange::Linear { min: 0, max: 2 })
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
    significant_display(FloatParam::new(
        name,
        value,
        FloatRange::Linear { min, max },
    ))
}

/// Significant digits shown for a parameter without a formatter of its own.
/// An `f32` holds about seven; five leave room for the normalized-to-plain
/// conversion, so text read back shows the same text (clap-validator's
/// `param-conversions`, `extend-ci-windows-linux-macos`).
const SHOWN_SIGNIFICANT_DIGITS: i32 = 5;

/// `value` with [`SHOWN_SIGNIFICANT_DIGITS`] significant digits, without
/// trailing zeros: `1100.0438` shows as `1100`, `0.50075006` as `0.50075`.
pub(crate) fn significant_text(value: f32) -> String {
    if value == 0.0 || !value.is_finite() {
        return if value.is_finite() {
            "0".into()
        } else {
            value.to_string()
        };
    }
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (SHOWN_SIGNIFICANT_DIGITS - 1 - magnitude).max(0) as usize;
    let mut text = format!("{value:.decimals$}");
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if text == "-0" {
        text = "0".into();
    }
    text
}

/// Show `param` with [`significant_text`] and read plain numbers back.
fn significant_display(param: FloatParam) -> FloatParam {
    param
        .with_value_to_string(Arc::new(significant_text))
        .with_string_to_value(Arc::new(|string| string.trim().parse().ok()))
}

/// Build the "Size" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn size_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Size",
        value.clamp(0.05, 10.0),
        FloatRange::Linear {
            min: 0.05,
            max: 10.0,
        },
    ))
}

/// Build the "Rust" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn rust_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Rust",
        value.clamp(0.0, 5.0),
        FloatRange::Linear { min: 0.0, max: 5.0 },
    ))
}

/// Build the "Damage" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn damage_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Damage",
        value.clamp(0.0, 10.0),
        FloatRange::Linear {
            min: 0.0,
            max: 10.0,
        },
    ))
}

/// Build the "Drive" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn drive_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Drive",
        value.clamp(0.0, 5.0),
        FloatRange::Linear { min: 0.0, max: 5.0 },
    ))
}

/// Build the "Width" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn width_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Width",
        value.clamp(-2.0, 3.0),
        FloatRange::Linear {
            min: -2.0,
            max: 3.0,
        },
    ))
}

/// Build the "Body" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn body_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Body",
        value.clamp(0.0, 5.0),
        FloatRange::Linear { min: 0.0, max: 5.0 },
    ))
}

/// Build the "UI Scale" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn ui_scale_param(value: i32) -> IntParam {
    IntParam::new(
        "UI Scale",
        value.clamp(0, 4),
        IntRange::Linear { min: 0, max: 4 },
    )
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
    }))
}

/// Build the "Attack" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn env_attack_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Attack",
        value.clamp(0.001, 2.0),
        FloatRange::Linear {
            min: 0.001,
            max: 2.0,
        },
    ))
}

/// Build the "Decay" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn env_decay_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Decay",
        value.clamp(0.01, 5.0),
        FloatRange::Linear {
            min: 0.01,
            max: 5.0,
        },
    ))
}

/// Build the "Sustain" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn env_sustain_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Sustain",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Release" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn env_release_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Release",
        value.clamp(0.01, 5.0),
        FloatRange::Linear {
            min: 0.01,
            max: 5.0,
        },
    ))
}

/// Build the "Onset Time" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mseg_onset_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Onset Time",
        value.clamp(0.001, 1.0),
        FloatRange::Linear {
            min: 0.001,
            max: 1.0,
        },
    ))
}

/// Build the "MSEG Attack" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mseg_attack_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "MSEG Attack",
        value.clamp(0.001, 2.0),
        FloatRange::Linear {
            min: 0.001,
            max: 2.0,
        },
    ))
}

/// Build the "Hold" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mseg_hold_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Hold",
        value.clamp(0.0, 2.0),
        FloatRange::Linear { min: 0.0, max: 2.0 },
    ))
}

/// Build the "MSEG Decay" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mseg_decay_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "MSEG Decay",
        value.clamp(0.01, 5.0),
        FloatRange::Linear {
            min: 0.01,
            max: 5.0,
        },
    ))
}

/// Build the "MSEG Sustain" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mseg_sustain_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "MSEG Sustain",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "MSEG Release" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mseg_release_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "MSEG Release",
        value.clamp(0.01, 5.0),
        FloatRange::Linear {
            min: 0.01,
            max: 5.0,
        },
    ))
}

/// Build the "Env Amount" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn env_amount_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Env Amount",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Velocity To Peak" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn velocity_to_peak_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Velocity To Peak",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Loop Mode" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn loop_mode_param(value: i32) -> IntParam {
    IntParam::new(
        "Loop Mode",
        value.clamp(0, 2),
        IntRange::Linear { min: 0, max: 2 },
    )
}

/// Build the "Loop Start Stage" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn loop_start_stage_param(value: i32) -> IntParam {
    IntParam::new(
        "Loop Start Stage",
        value.clamp(0, 5),
        IntRange::Linear { min: 0, max: 5 },
    )
}

/// Build the "Loop End Stage" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn loop_end_stage_param(value: i32) -> IntParam {
    IntParam::new(
        "Loop End Stage",
        value.clamp(0, 5),
        IntRange::Linear { min: 0, max: 5 },
    )
}

/// Build the "Global Time Scale" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn global_time_scale_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Global Time Scale",
        value.clamp(0.1, 10.0),
        FloatRange::Linear {
            min: 0.1,
            max: 10.0,
        },
    ))
}

/// Build the "Velocity To Level" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn velocity_to_level_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Velocity To Level",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Velocity To Time" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn velocity_to_time_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Velocity To Time",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Curve Tension" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn curve_tension_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Curve Tension",
        value.clamp(-1.0, 1.0),
        FloatRange::Linear {
            min: -1.0,
            max: 1.0,
        },
    ))
}

/// Build the "Hand Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn hand_mass_param(value: f32) -> FloatParam {
    display_float_param("Hand Mass", value.clamp(0.4, 3.0), 0.4, 3.0)
}

/// Build the "Flesh Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn flesh_stiffness_param(value: f32) -> FloatParam {
    display_float_param("Flesh Stiffness", value.clamp(0.05, 0.8), 0.05, 0.8)
}

/// Build the "Flesh Damping" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn flesh_damping_param(value: f32) -> FloatParam {
    display_float_param("Flesh Damping", value.clamp(0.3, 1.8), 0.3, 1.8)
}

/// Build the "Mute Decay" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mute_decay_param(value: f32) -> FloatParam {
    display_float_param("Mute Decay", value.clamp(0.85, 0.999), 0.85, 0.999)
}

/// Build the "Mallet Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mallet_mass_param(value: f32) -> FloatParam {
    display_float_param("Mallet Mass", value.clamp(0.4, 3.5), 0.4, 3.5)
}

/// Build the "Felt Softness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn felt_softness_param(value: f32) -> FloatParam {
    display_float_param("Felt Softness", value.clamp(0.1, 1.3), 0.1, 1.3)
}

/// Build the "Core Hardness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn core_hardness_param(value: f32) -> FloatParam {
    display_float_param("Core Hardness", value.clamp(0.5, 4.5), 0.5, 4.5)
}

/// Build the "Compression Curve" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn compression_curve_param(value: f32) -> FloatParam {
    display_float_param("Compression Curve", value.clamp(2.0, 5.0), 2.0, 5.0)
}

/// Build the "Material Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn material_stiffness_param(value: f32) -> FloatParam {
    display_float_param("Material Stiffness", value.clamp(0.5, 5.0), 0.5, 5.0)
}

/// Build the "Impact Damping" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn impact_damping_param(value: f32) -> FloatParam {
    display_float_param("Impact Damping", value.clamp(0.1, 1.3), 0.1, 1.3)
}

/// Build the "Stick Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn stick_mass_param(value: f32) -> FloatParam {
    display_float_param("Stick Mass", value.clamp(0.05, 1.25), 0.05, 1.25)
}

/// Build the "Tip Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn tip_stiffness_param(value: f32) -> FloatParam {
    display_float_param("Tip Stiffness", value.clamp(0.8, 6.8), 0.8, 6.8)
}

/// Build the "Restitution Bounciness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn restitution_bounciness_param(value: f32) -> FloatParam {
    display_float_param("Restitution Bounciness", value.clamp(0.2, 0.9), 0.2, 0.9)
}

/// Build the "Micro Bounce Limit" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn micro_bounce_limit_param(value: f32) -> FloatParam {
    display_float_param("Micro Bounce Limit", value.clamp(2.0, 8.0), 2.0, 8.0)
}

/// Build the "Wire Density" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn wire_density_param(value: f32) -> FloatParam {
    display_float_param("Wire Density", value.clamp(10.0, 130.0), 10.0, 130.0)
}

/// Build the "Spread Duration" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn spread_duration_param(value: f32) -> FloatParam {
    display_float_param("Spread Duration", value.clamp(10.0, 250.0), 10.0, 250.0)
}

/// Build the "Brush Wire Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn brush_wire_stiffness_param(value: f32) -> FloatParam {
    display_float_param("Brush Wire Stiffness", value.clamp(0.0, 1.0), 0.0, 1.0)
}

/// Build the "Amplitude Randomization" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn amplitude_randomization_param(value: f32) -> FloatParam {
    display_float_param("Amplitude Randomization", value.clamp(0.0, 1.0), 0.0, 1.0)
}

/// Build the "Pipe Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn pipe_mass_param(value: f32) -> FloatParam {
    display_float_param("Pipe Mass", value.clamp(0.4, 2.6), 0.4, 2.6)
}

/// Build the "Metal Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn metal_stiffness_param(value: f32) -> FloatParam {
    display_float_param("Metal Stiffness", value.clamp(0.5, 5.5), 0.5, 5.5)
}

/// Build the "Pipe Pitch" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn pipe_pitch_param(value: f32) -> FloatParam {
    display_float_param("Pipe Pitch", value.clamp(0.5, 2.5), 0.5, 2.5)
}

/// Build the "Pipe Ring Decay" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn pipe_ring_decay_param(value: f32) -> FloatParam {
    display_float_param("Pipe Ring Decay", value.clamp(0.96, 0.999), 0.96, 0.999)
}

/// Build the "Link Count" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn link_count_param(value: f32) -> FloatParam {
    display_float_param("Link Count", value.clamp(3.0, 15.0), 3.0, 15.0)
}

/// Build the "Chain Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn chain_mass_param(value: f32) -> FloatParam {
    display_float_param("Chain Mass", value.clamp(0.2, 1.4), 0.2, 1.4)
}

/// Build the "Drop Envelope Spread" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn drop_envelope_spread_param(value: f32) -> FloatParam {
    display_float_param(
        "Drop Envelope Spread",
        value.clamp(40.0, 400.0),
        40.0,
        400.0,
    )
}

/// Build the "Internal Rattle" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn internal_rattle_param(value: f32) -> FloatParam {
    display_float_param("Internal Rattle", value.clamp(0.0, 1.0), 0.0, 1.0)
}

/// Build the "Rattle Color" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn rattle_color_param(value: f32) -> FloatParam {
    display_float_param("Rattle Color", value.clamp(0.0, 1.0), 0.0, 1.0)
}

/// Build the "Bow Pressure" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn bow_pressure_param(value: f32) -> FloatParam {
    display_float_param("Bow Pressure", value.clamp(0.2, 2.0), 0.2, 2.0)
}

/// Build the "Bow Speed" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn bow_speed_param(value: f32) -> FloatParam {
    display_float_param("Bow Speed", value.clamp(0.1, 2.0), 0.1, 2.0)
}

/// Build the "Rosin Grip" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn rosin_grip_param(value: f32) -> FloatParam {
    display_float_param("Rosin Grip", value.clamp(0.05, 1.5), 0.05, 1.5)
}

/// Build the "Slip Curve" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn slip_curve_param(value: f32) -> FloatParam {
    display_float_param("Slip Curve", value.clamp(0.05, 1.5), 0.05, 1.5)
}

/// Build the "Scrape Speed" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn scrape_speed_param(value: f32) -> FloatParam {
    display_float_param("Scrape Speed", value.clamp(0.1, 2.5), 0.1, 2.5)
}

/// Build the "Point Pressure" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn point_pressure_param(value: f32) -> FloatParam {
    display_float_param("Point Pressure", value.clamp(0.1, 1.5), 0.1, 1.5)
}

/// Build the "Chatter Pitch" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn chatter_pitch_param(value: f32) -> FloatParam {
    display_float_param("Chatter Pitch", value.clamp(0.1, 1.5), 0.1, 1.5)
}

/// Build the "Chatter Damping" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn chatter_damping_param(value: f32) -> FloatParam {
    display_float_param("Chatter Damping", value.clamp(0.1, 0.9), 0.1, 0.9)
}

/// Build the "Grind Speed" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn grind_speed_param(value: f32) -> FloatParam {
    display_float_param("Grind Speed", value.clamp(0.1, 2.5), 0.1, 2.5)
}

/// Build the "Grind Pressure" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn grind_pressure_param(value: f32) -> FloatParam {
    display_float_param("Grind Pressure", value.clamp(0.1, 1.9), 0.1, 1.9)
}

/// Build the "Surface Grit" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn surface_grit_param(value: f32) -> FloatParam {
    display_float_param("Surface Grit", value.clamp(0.0, 1.0), 0.0, 1.0)
}

/// Build the "Grit Color" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn grit_color_param(value: f32) -> FloatParam {
    display_float_param("Grit Color", value.clamp(0.0, 1.0), 0.0, 1.0)
}

/// Build the "Drag Speed" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn drag_speed_param(value: f32) -> FloatParam {
    display_float_param("Drag Speed", value.clamp(0.1, 2.5), 0.1, 2.5)
}

/// Build the "Ridge Spacing" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn ridge_spacing_param(value: f32) -> FloatParam {
    display_float_param("Ridge Spacing", value.clamp(0.01, 0.2), 0.01, 0.2)
}

/// Build the "Ridge Depth" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn ridge_depth_param(value: f32) -> FloatParam {
    display_float_param("Ridge Depth", value.clamp(0.0, 2.0), 0.0, 2.0)
}

/// Build the "Exciter Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn drag_exciter_mass_param(value: f32) -> FloatParam {
    display_float_param("Exciter Mass", value.clamp(0.2, 2.0), 0.2, 2.0)
}

/// Build the "Pull Speed" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn pull_speed_param(value: f32) -> FloatParam {
    display_float_param("Pull Speed", value.clamp(0.05, 1.55), 0.05, 1.55)
}

/// Build the "Break Threshold" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn break_threshold_param(value: f32) -> FloatParam {
    display_float_param("Break Threshold", value.clamp(0.1, 1.6), 0.1, 1.6)
}

/// Build the "Slip Stochasticity" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn slip_stochasticity_param(value: f32) -> FloatParam {
    display_float_param("Slip Stochasticity", value.clamp(0.0, 1.0), 0.0, 1.0)
}

/// Build the "Creak Sharpness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn creak_sharpness_param(value: f32) -> FloatParam {
    display_float_param("Creak Sharpness", value.clamp(0.2, 1.4), 0.2, 1.4)
}

/// Build the "Air Pressure" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn air_pressure_param(value: f32) -> FloatParam {
    display_float_param("Air Pressure", value.clamp(0.1, 2.1), 0.1, 2.1)
}

/// Build the "Nozzle Width" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn nozzle_width_param(value: f32) -> FloatParam {
    display_float_param("Nozzle Width", value.clamp(0.1, 1.6), 0.1, 1.6)
}

/// Build the "Turbulence Chaos" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn turbulence_chaos_param(value: f32) -> FloatParam {
    display_float_param("Turbulence Chaos", value.clamp(0.0, 2.0), 0.0, 2.0)
}

/// Build the "Mains Frequency" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mains_frequency_param(value: f32) -> FloatParam {
    display_float_param("Mains Frequency", value.clamp(40.0, 120.0), 40.0, 120.0)
}

/// Build the "Coil Proximity" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn coil_proximity_param(value: f32) -> FloatParam {
    display_float_param("Coil Proximity", value.clamp(0.0, 2.0), 0.0, 2.0)
}

/// Build the "Voltage Sag" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn voltage_sag_param(value: f32) -> FloatParam {
    display_float_param("Voltage Sag", value.clamp(0.0, 2.0), 0.0, 2.0)
}

/// Build the "Pull Distance" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn pull_distance_param(value: f32) -> FloatParam {
    display_float_param("Pull Distance", value.clamp(0.1, 1.5), 0.1, 1.5)
}

/// Build the "Hook Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn hook_stiffness_param(value: f32) -> FloatParam {
    display_float_param("Hook Stiffness", value.clamp(0.2, 2.2), 0.2, 2.2)
}

/// Build the "Snap Force" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn snap_force_param(value: f32) -> FloatParam {
    display_float_param("Snap Force", value.clamp(0.1, 2.0), 0.1, 2.0)
}

/// Build the "Flow Rate" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn flow_rate_param(value: f32) -> FloatParam {
    display_float_param("Flow Rate", value.clamp(0.1, 3.1), 0.1, 3.1)
}

/// Build the "Particle Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn particle_mass_param(value: f32) -> FloatParam {
    display_float_param("Particle Mass", value.clamp(0.05, 1.0), 0.05, 1.0)
}

/// Build the "Mass Variance" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn mass_variance_param(value: f32) -> FloatParam {
    display_float_param("Mass Variance", value.clamp(0.0, 2.0), 0.0, 2.0)
}

/// Build the "Strike Position" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn strike_position_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Strike Position",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Coupling Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn coupling_stiffness_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Coupling Stiffness",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Position Wander" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn position_wander_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Position Wander",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Position Envelope" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn position_envelope_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Position Envelope",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Fundamental Anchor" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn fundamental_anchor_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Fundamental Anchor",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Damping" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn res_damping_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Damping",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Resonator Brightness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn res_brightness_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Resonator Brightness",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Thickness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn thickness_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Thickness",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Heat" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn heat_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Heat",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Sludge" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn sludge_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Sludge",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Pipe Diameter" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn pipe_diameter_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Pipe Diameter",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Plate Aspect" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn plate_aspect_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Plate Aspect",
        value.clamp(0.1, 4.0),
        FloatRange::Linear { min: 0.1, max: 4.0 },
    ))
}

/// Build the "Plate Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn plate_stiffness_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Plate Stiffness",
        value.clamp(0.25, 3.0),
        FloatRange::Linear {
            min: 0.25,
            max: 3.0,
        },
    ))
}

/// Build the "Tank Volume" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn tank_volume_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Tank Volume",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Cavity Mix" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn tank_cavity_mix_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Cavity Mix",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Link Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn chain_link_mass_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Link Mass",
        value.clamp(0.1, 1.0),
        FloatRange::Linear { min: 0.1, max: 1.0 },
    ))
}

/// Build the "Instability" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn chain_instability_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Instability",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Shear Density" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn beam_shear_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Shear Density",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Braid Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn cable_braid_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Braid Stiffness",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Tension Drop" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn cable_tension_drop_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Tension Drop",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Dispersion Chirp" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn spring_dispersion_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Dispersion Chirp",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Spring Slosh" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn spring_slosh_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Spring Slosh",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Metal Thinness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn sheet_thinness_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Metal Thinness",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Tooth Dissonance" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn cog_dissonance_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Tooth Dissonance",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Filter Cutoff" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn filter_cutoff_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Filter Cutoff",
        value.clamp(20.0, 20000.0),
        FloatRange::Skewed {
            min: 20.0,
            max: 20000.0,
            factor: 0.5,
        },
    ))
}

/// Build the "Filter Resonance" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn filter_resonance_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Filter Resonance",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Component Tolerance" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn component_tolerance_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Component Tolerance",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Drive Amount" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn drive_amount_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Drive Amount",
        value.clamp(0.0, 5.0),
        FloatRange::Linear { min: 0.0, max: 5.0 },
    ))
}

/// Build the "Bias Starvation" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn bias_starvation_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Bias Starvation",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Chaos Depth" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn chaos_depth_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Chaos Depth",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Spread Width" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn spread_width_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Spread Width",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Listener Proximity" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn listener_proximity_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Listener Proximity",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Chassis Material" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn chassis_material_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Chassis Material",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Chassis Volume" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn chassis_volume_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Chassis Volume",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Space Mode" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn space_mode_param(value: i32) -> IntParam {
    IntParam::new(
        "Space Mode",
        value.clamp(0, 3),
        IntRange::Linear { min: 0, max: 3 },
    )
}

/// Build the "Space Amount" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn space_amount_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Space Amount",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Factory Size" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn factory_size_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Factory Size",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Machinery Clutter" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn machinery_clutter_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Machinery Clutter",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Wall Impedance" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn wall_impedance_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Wall Impedance",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Spring Tension" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn spring_tension_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Spring Tension",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Spring Wire Stiffness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn wire_stiffness_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Spring Wire Stiffness",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Spring Tank Size" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn spring_tank_size_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Spring Tank Size",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Delay Time" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn delay_time_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Delay Time",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Machinery Movement" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn machinery_movement_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Machinery Movement",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "High Frequency Damping" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn high_frequency_damping_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "High Frequency Damping",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Analog Ceiling" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn analog_ceiling_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Analog Ceiling",
        value.clamp(0.5, 1.0),
        FloatRange::Linear { min: 0.5, max: 1.0 },
    ))
}

/// Build the "Diode Softness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn diode_softness_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Diode Softness",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Mass" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn macro_mass_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Mass",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Corrosion" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn macro_corrosion_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Corrosion",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Violence" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn macro_violence_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Violence",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

/// Build the "Macro Brightness" parameter, shared by the live parameters and
/// `Preset::into_params` (`m10-m18-tidy-dead-code-and-docs`).
pub(crate) fn macro_brightness_param(value: f32) -> FloatParam {
    significant_display(FloatParam::new(
        "Macro Brightness",
        value.clamp(0.0, 1.0),
        FloatRange::Linear { min: 0.0, max: 1.0 },
    ))
}

impl Default for CorrosionParams {
    /// Creates default parameter values for plugin initialization.
    ///
    /// Sets sensible defaults for all 130+ parameters to ensure
    /// the plugin produces audible output immediately upon loading.
    fn default() -> Self {
        Self {
            #[cfg(feature = "gui")]
            // 75 % of the 1080 x 840 base: fits a host window on a 1080p screen.
            editor_state: EguiState::from_size(810, 630),
            kit: Arc::default(),
            kit_table: Arc::default(),

            // Sound generation defaults
            exciter: exciter_param(ExciterType::HandStrike.to_int()),
            object: object_param(0), // Pipe
            quality_mode: quality_mode_param(QualityMode::Normal.to_int()),
            play_mode: play_mode_param(PlayMode::Tonal.to_int()),
            limiter_mode: limiter_mode_param(0),

            // Object transformation defaults
            size: size_param(1.0),
            rust: rust_param(0.0),
            damage: damage_param(0.0),

            // Master controls
            drive: drive_param(0.0),
            output: output_param(util::db_to_gain(0.0)),
            width: width_param(0.5),
            // 2.62 reproduces the pre-C5 default-note level (peak 0.118) with
            // the parallel body routing; see `c5_measurements::c5_default_body`.
            body: body_param(2.62),

            // UI scale (percentage display)
            ui_scale: ui_scale_param(1),

            // ADSR envelope defaults
            env_attack: env_attack_param(0.05),
            env_decay: env_decay_param(0.3),
            env_sustain: env_sustain_param(0.7),
            env_release: env_release_param(0.5),

            // MSEG envelope defaults
            mseg_onset: mseg_onset_param(0.01),
            mseg_onset_level: mseg_onset_level_param(DEFAULT_MSEG_ONSET_LEVEL),
            mseg_attack: mseg_attack_param(0.05),
            mseg_hold: mseg_hold_param(0.02),
            mseg_decay: mseg_decay_param(0.3),
            mseg_sustain: mseg_sustain_param(0.5),
            mseg_release: mseg_release_param(0.1),

            // Envelope modulation defaults
            env_amount: env_amount_param(1.0),
            velocity_to_peak: velocity_to_peak_param(0.5),

            // MSEG loop defaults
            loop_mode: loop_mode_param(0),
            loop_start_stage: loop_start_stage_param(3),
            loop_end_stage: loop_end_stage_param(4),

            // Timing and sync
            sync_rate: sync_rate_param(0.0),
            global_time_scale: global_time_scale_param(1.0),
            velocity_to_level: velocity_to_level_param(1.0),
            velocity_to_time: velocity_to_time_param(0.15),
            curve_tension: curve_tension_param(0.0),

            // Exciter control defaults
            exciter_pressure: exciter_pressure_param(0.5),
            exciter_speed: exciter_speed_param(0.5),
            exciter_roughness: exciter_roughness_param(0.3),

            // Dedicated exciter defaults derived from the previous generic mappings
            hand_mass: hand_mass_param(1.7),
            flesh_stiffness: flesh_stiffness_param(0.425),
            flesh_damping: flesh_damping_param(0.75),
            mute_decay: mute_decay_param(0.9245),
            mallet_mass: mallet_mass_param(1.5),
            felt_softness: felt_softness_param(0.94),
            core_hardness: core_hardness_param(2.5),
            compression_curve: compression_curve_param(2.9),
            material_stiffness: material_stiffness_param(2.75),
            impact_damping: impact_damping_param(0.46),
            stick_mass: stick_mass_param(0.65),
            tip_stiffness: tip_stiffness_param(3.8),
            restitution_bounciness: restitution_bounciness_param(0.41),
            micro_bounce_limit: micro_bounce_limit_param(4.0),
            wire_density: wire_density_param(46.0),
            spread_duration: spread_duration_param(130.0),
            brush_wire_stiffness: brush_wire_stiffness_param(0.3),
            amplitude_randomization: amplitude_randomization_param(0.3),
            pipe_mass: pipe_mass_param(1.5),
            metal_stiffness: metal_stiffness_param(3.0),
            pipe_pitch: pipe_pitch_param(1.1),
            pipe_ring_decay: pipe_ring_decay_param(0.9795),
            link_count: link_count_param(9.0),
            chain_mass: chain_mass_param(0.8),
            drop_envelope_spread: drop_envelope_spread_param(220.0),
            internal_rattle: internal_rattle_param(0.3),
            rattle_color: rattle_color_param(0.3),
            bow_pressure: bow_pressure_param(1.1),
            bow_speed: bow_speed_param(1.05),
            rosin_grip: rosin_grip_param(0.485),
            slip_curve: slip_curve_param(0.485),
            scrape_speed: scrape_speed_param(0.82),
            point_pressure: point_pressure_param(0.8),
            chatter_pitch: chatter_pitch_param(0.52),
            chatter_damping: chatter_damping_param(0.66),
            grind_speed: grind_speed_param(0.82),
            grind_pressure: grind_pressure_param(1.0),
            surface_grit: surface_grit_param(0.3),
            grit_color: grit_color_param(0.3),
            drag_speed: drag_speed_param(0.82),
            ridge_spacing: ridge_spacing_param(0.143),
            ridge_depth: ridge_depth_param(0.6),
            drag_exciter_mass: drag_exciter_mass_param(1.1),
            pull_speed: pull_speed_param(0.8),
            break_threshold: break_threshold_param(0.85),
            slip_stochasticity: slip_stochasticity_param(0.3),
            creak_sharpness: creak_sharpness_param(0.56),
            air_pressure: air_pressure_param(1.1),
            nozzle_width: nozzle_width_param(0.85),
            turbulence_chaos: turbulence_chaos_param(0.6),
            mains_frequency: mains_frequency_param(80.0),
            coil_proximity: coil_proximity_param(1.0),
            voltage_sag: voltage_sag_param(0.6),
            pull_distance: pull_distance_param(0.8),
            hook_stiffness: hook_stiffness_param(1.2),
            snap_force: snap_force_param(0.67),
            flow_rate: flow_rate_param(1.6),
            particle_mass: particle_mass_param(0.525),
            mass_variance: mass_variance_param(0.6),

            // Interaction defaults
            strike_position: strike_position_param(0.5),
            coupling_stiffness: coupling_stiffness_param(1.0),
            position_wander: position_wander_param(0.0),
            position_wander_rate: position_wander_rate_param(0.1),
            position_envelope: position_envelope_param(0.0),
            fundamental_anchor: fundamental_anchor_param(0.3),

            // Resonator control defaults
            res_damping: res_damping_param(0.5),
            res_brightness: res_brightness_param(0.5),
            thickness: thickness_param(0.5),
            heat: heat_param(0.0),
            sludge: sludge_param(0.0),

            // Per-object character defaults
            pipe_diameter: pipe_diameter_param(0.5),
            plate_aspect: plate_aspect_param(1.0),
            plate_stiffness: plate_stiffness_param(1.0),
            tank_volume: tank_volume_param(0.5),
            tank_cavity_mix: tank_cavity_mix_param(0.6),
            chain_link_mass: chain_link_mass_param(0.5),
            chain_instability: chain_instability_param(0.3),
            beam_shear: beam_shear_param(0.5),
            cable_braid: cable_braid_param(0.3),
            cable_tension_drop: cable_tension_drop_param(0.4),
            spring_dispersion: spring_dispersion_param(0.5),
            spring_slosh: spring_slosh_param(0.3),
            sheet_thinness: sheet_thinness_param(0.4),
            cog_dissonance: cog_dissonance_param(0.1),

            // Post-processing filter defaults
            filter_cutoff: filter_cutoff_param(20000.0),
            filter_resonance: filter_resonance_param(0.0),
            component_tolerance: component_tolerance_param(0.0),

            // Drive defaults
            drive_amount: drive_amount_param(0.0),
            bias_starvation: bias_starvation_param(0.0),
            chaos_depth: chaos_depth_param(0.0),

            // Stereo/spatial defaults
            spread_width: spread_width_param(0.5),
            listener_proximity: listener_proximity_param(0.5),

            // Body resonance defaults
            chassis_material: chassis_material_param(0.5),
            chassis_volume: chassis_volume_param(0.5),

            // Space/reverb defaults
            space_mode: space_mode_param(0),
            space_amount: space_amount_param(0.0),
            factory_size: factory_size_param(0.5),
            machinery_clutter: machinery_clutter_param(0.3),
            wall_impedance: wall_impedance_param(0.5),
            spring_tension: spring_tension_param(0.5),
            wire_stiffness: wire_stiffness_param(0.5),
            spring_tank_size: spring_tank_size_param(0.5),
            delay_time: delay_time_param(0.25),
            machinery_movement: machinery_movement_param(0.0),
            high_frequency_damping: high_frequency_damping_param(0.5),

            // Output stage defaults
            analog_ceiling: analog_ceiling_param(0.9661),
            diode_softness: diode_softness_param(0.5),
            diode_asymmetry: diode_asymmetry_param(DEFAULT_DIODE_ASYMMETRY),

            // Macro defaults — 0.5 is the neutral midpoint so existing presets
            // sound unchanged until the user actively dials a macro.
            macro_mass: macro_mass_param(0.5),
            macro_corrosion: macro_corrosion_param(0.5),
            macro_violence: macro_violence_param(0.5),
            macro_brightness: macro_brightness_param(0.5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{exciter_pressure_param, exciter_roughness_param, exciter_speed_param};
    use nih_plug::prelude::Param;

    #[test]
    fn parameter_names_are_unique() {
        // `m01-m04-m05-clarify-parameters` (M4): hosts list parameters by
        // display name, so no two may share one.
        use nih_plug::prelude::Params;
        let params = super::CorrosionParams::default();
        let mut seen = std::collections::BTreeMap::new();
        let mut duplicates = Vec::new();
        for (id, ptr, _) in params.param_map() {
            // SAFETY: the pointers come from `params`, alive for this loop.
            let name = unsafe { ptr.name() }.to_string();
            if let Some(other) = seen.insert(name.clone(), id.clone()) {
                duplicates.push(format!("{name:?} ({other}, {id})"));
            }
        }
        assert!(duplicates.is_empty(), "shared names: {duplicates:?}");
    }

    /// clap-validator's `param-conversions` (`extend-ci-windows-linux-macos`):
    /// a value shown as text, read back and shown again must give the same
    /// text, for every parameter. The CLAP wrapper passes normalized values.
    #[test]
    fn parameter_text_round_trips() {
        use nih_plug::prelude::Params;
        let params = super::CorrosionParams::default();
        // A grid plus the value clap-validator reported, and a few odd ones.
        let mut values: Vec<f32> = (0..=400).map(|i| i as f32 / 400.0).collect();
        values.extend([
            0.117_164_69,
            0.333_333_34,
            0.666_666_7,
            0.123_456_79,
            0.987_654_3,
        ]);
        // And pseudo-random ones, as clap-validator draws.
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        values.extend((0..20_000).map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f32 / (1u64 << 24) as f32
        }));
        let mut failures = Vec::new();
        for (id, ptr, _) in params.param_map() {
            for &v in &values {
                // SAFETY: the pointers come from `params`, alive for this loop.
                let first = unsafe { ptr.normalized_value_to_string(v, true) };
                let Some(back) = (unsafe { ptr.string_to_normalized_value(&first) }) else {
                    failures.push(format!("{id}: {first:?} does not parse"));
                    break;
                };
                let second = unsafe { ptr.normalized_value_to_string(back, true) };
                if first != second {
                    failures.push(format!("{id}: {v} -> {first:?} -> {back} -> {second:?}"));
                    break;
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} parameters do not round-trip:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn output_is_shown_in_decibels() {
        // M5: Output displays dB, and −inf at zero gain.
        let output = super::CorrosionParams::default().output;
        let shown =
            |gain: f32| output.normalized_value_to_string(output.preview_normalized(gain), true);
        let db = |text: String| -> f32 { text.trim_end_matches(" dB").parse().unwrap() };
        assert!(shown(1.0).ends_with(" dB"), "{}", shown(1.0));
        assert!(db(shown(1.0)).abs() < 0.01, "{}", shown(1.0));
        assert!((db(shown(0.5)) + 6.02).abs() < 0.01, "{}", shown(0.5));
        assert!(shown(0.0).to_lowercase().contains("inf"), "{}", shown(0.0));
    }

    #[test]
    fn output_accepts_typed_decibels() {
        let output = super::CorrosionParams::default().output;
        let normalized = output.string_to_normalized_value("-6").expect("parses");
        let gain = output.preview_plain(normalized);
        assert!((gain - 0.501).abs() < 0.001, "gain {gain}");
    }

    #[test]
    fn output_taper_reaches_the_useful_range() {
        // M5: 0 dB between 50 % and 65 % of the travel, −20 to +6 dB over at
        // least 15 %, and +40 dB still the maximum.
        use nih_plug::util::db_to_gain;
        let output = super::CorrosionParams::default().output;
        let at = |db: f32| output.preview_normalized(db_to_gain(db));
        let unity = at(0.0);
        let span = at(6.0) - at(-20.0);
        println!(
            "0 dB at {unity:.3}, -20..+6 dB spans {span:.3}, -10 dB at {:.3}",
            at(-10.0)
        );
        assert!((0.50..=0.65).contains(&unity), "0 dB at {unity}");
        assert!(span >= 0.15, "-20..+6 dB spans {span}");
        assert!((output.preview_plain(1.0) - db_to_gain(40.0)).abs() < 1e-3);
    }

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

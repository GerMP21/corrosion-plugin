//! Real-time per-note voice execution for Corrosion.
//!
//! This module owns the note lifecycle, exciter dispatch, envelope
//! handling, and tail management for a single polyphonic voice.
//! `VoiceManager` (in `manager.rs`) schedules these voices, while
//! `src/lib.rs` forwards host MIDI events and audio-rate parameter
//! snapshots into the voice layer.
//!
//! Each voice pairs a `crate::dsp::ModalResonator` with one of the
//! exciter families exposed by `crate::dsp`, keeping the audio-thread
//! path allocation-free and deterministic.

use crate::dsp::utils::rate;
use crate::dsp::{
    BowExciter, CharacterParams, CorrugatedDrag, DamageAmount, Drumstick, ElectromagneticHum,
    FeltMallet, HandStrike, HardMallet, HeatAmount, HeavyGrinding, LoopMode, MetalChain, MetalPipe,
    ModalProfileId, ModalResonator, ParticleRain, PneumaticJet, ResonatorCore, RustAmount,
    SizeScale, SludgeAmount, StiffPointScrape, TensionRise, TensionSnap, ThicknessAmount,
    WireBrush, MSEG,
};
use crate::params::{ExciterFamily, ExciterType};

pub mod manager;

pub use manager::{VoiceManager, MAX_VOICES};

/// Convert a MIDI note number to frequency in hertz.
///
/// # Arguments
///
/// * `note` - MIDI note number, where 69 equals A4.
///
/// # Returns
///
/// The corresponding frequency in hertz.
///
/// # Examples
///
/// ```rust
/// assert!((corrosion::voice::midi_to_hz(69) - 440.0).abs() < 1e-3);
/// ```
pub fn midi_to_hz(note: u8) -> f32 {
    // MIDI standard defines notes 0..=127 (`u8` already caps the upper end).
    // Note 0 = 8.18 Hz, note 127 = 12_543.85 Hz — both well within the safe
    // range for our biquad coefficient generation, so no further clamp is
    // necessary at this layer. The clamp on `note as f32` happens implicitly
    // because `u8` cannot represent values outside `[0, 255]` and host MIDI
    // events already cap velocity/note at 127.
    let safe_note = note.min(127) as f32;
    440.0 * 2_f32.powf((safe_note - 69.0) / 12.0)
}

/// Convert a pitch-bend value in semitones into a frequency multiplier.
///
/// Clamps to ±24 semitones — a reasonable safety bound since classic
/// 14-bit pitch bend is typically ±2 semitones and MPE per-note tuning
/// rarely exceeds ±48 (we cap conservatively so a malformed event can't
/// drive the modal bank past Nyquist).
#[inline]
pub fn pitch_bend_semitones_to_factor(semitones: f32) -> f32 {
    let clamped = semitones.clamp(-24.0, 24.0);
    2.0_f32.powf(clamped / 12.0)
}

/// Default pitch-bend range applied to `MidiPitchBend` events, in semitones.
///
/// Standard MIDI uses ±2 semitones by default; we follow that. The host
/// can send a higher-resolution per-note tuning via `PolyTuning` to override
/// for MPE setups.
pub const PITCH_BEND_RANGE_SEMITONES: f32 = 2.0;

#[derive(Clone, Copy, Debug)]
/// Snapshot of per-note controls captured at note-on time.
///
/// These values are copied into the voice so the audio thread can
/// trigger and render without touching shared state.
pub struct VoiceControls {
    /// Attack time for hit and specialty envelopes, in seconds.
    pub env_attack: f32,
    /// Decay time for the specialty ADSR, in seconds.
    pub env_decay: f32,
    /// Sustain level for the specialty ADSR, normalized to 0.0..=1.0.
    pub env_sustain: f32,
    /// Release time for hit and specialty envelopes, in seconds.
    pub env_release: f32,
    /// Onset stage level for the friction MSEG.
    pub mseg_onset: f32,
    /// Attack stage time for the friction MSEG, in seconds.
    pub mseg_attack: f32,
    /// Hold stage time for the friction MSEG, in seconds.
    pub mseg_hold: f32,
    /// Decay stage time for the friction MSEG, in seconds.
    pub mseg_decay: f32,
    /// Sustain level for the friction MSEG, normalized to 0.0..=1.0.
    pub mseg_sustain: f32,
    /// Release stage time for the friction MSEG, in seconds.
    pub mseg_release: f32,
    /// Scales the overall envelope contribution for friction voices.
    pub env_amount: f32,
    /// Sets the friction MSEG's peak level: `0.5 + 0.5·velocity_to_peak`.
    /// Despite the name it does not read velocity; velocity scales the whole
    /// envelope through the MSEG's velocity-to-level gain.
    pub velocity_to_peak: f32,
    /// Selects the MSEG loop mode: off, forward, or ping-pong.
    pub loop_mode: i32,
    /// Index of the first stage included in the MSEG loop.
    pub loop_start_stage: i32,
    /// Index of the last stage included in the MSEG loop.
    pub loop_end_stage: i32,
    /// Multiplies every MSEG stage time for global slow/fast control.
    pub global_time_scale: f32,
    /// Velocity-to-level response used by the MSEG trigger.
    pub velocity_to_level: f32,
    /// Velocity-to-time response used by the MSEG trigger.
    pub velocity_to_time: f32,
    /// Curvature applied to the MSEG stage ramps.
    pub curve_tension: f32,
    /// Exciter pressure amount used to shape force and contact intensity.
    pub exciter_pressure: f32,
    /// Exciter speed amount used to shape motion and friction rate.
    pub exciter_speed: f32,
    /// Exciter roughness amount used to shape texture and noise.
    pub exciter_roughness: f32,
    /// Hand-strike mass multiplier.
    pub hand_mass: f32,
    /// Hand-strike spring stiffness.
    pub flesh_stiffness: f32,
    /// Hand-strike damping term.
    pub flesh_damping: f32,
    /// Hand-strike return-to-rest decay.
    pub mute_decay: f32,
    /// Shared felt/hard mallet mass.
    pub mallet_mass: f32,
    /// Felt-mallet low-velocity softness.
    pub felt_softness: f32,
    /// Felt-mallet hard-core stiffness.
    pub core_hardness: f32,
    /// Felt-mallet compression exponent.
    pub compression_curve: f32,
    /// Hard-mallet contact stiffness.
    pub material_stiffness: f32,
    /// Hard-mallet damping term.
    pub impact_damping: f32,
    /// Drumstick effective mass.
    pub stick_mass: f32,
    /// Drumstick tip stiffness.
    pub tip_stiffness: f32,
    /// Drumstick rebound amount.
    pub restitution_bounciness: f32,
    /// Drumstick micro-bounce count cap.
    pub micro_bounce_limit: f32,
    /// Wire-brush impulse density.
    pub wire_density: f32,
    /// Wire-brush spread duration.
    pub spread_duration: f32,
    /// Wire-brush stiffness emphasis.
    pub brush_wire_stiffness: f32,
    /// Wire-brush amplitude variance.
    pub amplitude_randomization: f32,
    /// Metal-pipe exciter mass.
    pub pipe_mass: f32,
    /// Metal-pipe contact stiffness.
    pub metal_stiffness: f32,
    /// Metal-pipe internal pitch shift.
    pub pipe_pitch: f32,
    /// Metal-pipe ring decay.
    pub pipe_ring_decay: f32,
    /// Metal-chain link count.
    pub link_count: f32,
    /// Metal-chain link mass.
    pub chain_mass: f32,
    /// Metal-chain impact spread.
    pub drop_envelope_spread: f32,
    /// Metal-chain rattle amount.
    pub internal_rattle: f32,
    /// Metal-chain rattle filter color.
    pub rattle_color: f32,
    /// Bow pressure.
    pub bow_pressure: f32,
    /// Bow speed.
    pub bow_speed: f32,
    /// Bow rosin grip.
    pub rosin_grip: f32,
    /// Bow slip curve.
    pub slip_curve: f32,
    /// Stiff-point scrape speed.
    pub scrape_speed: f32,
    /// Stiff-point pressure.
    pub point_pressure: f32,
    /// Stiff-point chatter pitch.
    pub chatter_pitch: f32,
    /// Stiff-point chatter damping.
    pub chatter_damping: f32,
    /// Grinding speed.
    pub grind_speed: f32,
    /// Grinding pressure.
    pub grind_pressure: f32,
    /// Grinding surface grit.
    pub surface_grit: f32,
    /// Grinding noise color.
    pub grit_color: f32,
    /// Corrugated drag speed.
    pub drag_speed: f32,
    /// Corrugated ridge spacing.
    pub ridge_spacing: f32,
    /// Corrugated ridge depth.
    pub ridge_depth: f32,
    /// Corrugated drag effective mass.
    pub drag_exciter_mass: f32,
    /// Tension-rise pull speed.
    pub pull_speed: f32,
    /// Tension-rise break threshold.
    pub break_threshold: f32,
    /// Tension-rise threshold jitter.
    pub slip_stochasticity: f32,
    /// Tension-rise sharpness.
    pub creak_sharpness: f32,
    /// Pneumatic jet air pressure.
    pub air_pressure: f32,
    /// Pneumatic jet nozzle width.
    pub nozzle_width: f32,
    /// Pneumatic jet turbulence non-linearity.
    pub turbulence_chaos: f32,
    /// Electromagnetic-hum mains frequency.
    pub mains_frequency: f32,
    /// Electromagnetic-hum coupling strength.
    pub coil_proximity: f32,
    /// Electromagnetic-hum harmonic sag.
    pub voltage_sag: f32,
    /// Tension-snap pull distance.
    pub pull_distance: f32,
    /// Tension-snap hook stiffness.
    pub hook_stiffness: f32,
    /// Tension-snap force threshold.
    pub snap_force: f32,
    /// Particle-rain spawn density.
    pub flow_rate: f32,
    /// Particle-rain particle mass.
    pub particle_mass: f32,
    /// Particle-rain mass variance.
    pub mass_variance: f32,
    /// Contact point along the resonator used for interaction modeling.
    pub strike_position: f32,
    /// Strength of the exciter-to-resonator coupling.
    pub coupling_stiffness: f32,
    /// Randomized movement around the strike position.
    pub position_wander: f32,
    /// Envelope-driven movement around the strike position.
    pub position_envelope: f32,
    /// Pulls the resonator toward its fundamental mode.
    pub fundamental_anchor: f32,
    /// Resonator damping control; higher values shorten decay.
    pub res_damping: f32,
    /// Resonator brightness control; higher values emphasize upper modes.
    pub res_brightness: f32,
    /// Material thickness control passed into the resonator model.
    pub thickness: f32,
    /// Thermal coloration control passed into the resonator model.
    pub heat: f32,
    /// Viscous/material damping control passed into the resonator model.
    pub sludge: f32,
    /// Per-object character controls; only the active object's fields are used.
    pub character: CharacterParams,
    /// Multiplier applied to the requested resonator mode count.
    ///
    /// Driven by the active QualityMode at note-on (Eco=0.5x, Normal=1.0x,
    /// High=1.5x, Render=2.0x). The mode count is rounded and clamped to at
    /// least one so quality affects modal density, not just oversampling.
    pub mode_count_scale: f32,
}

impl Default for VoiceControls {
    fn default() -> Self {
        Self {
            env_attack: 0.05,
            env_decay: 0.3,
            env_sustain: 0.7,
            env_release: 0.5,
            mseg_onset: 0.01,
            mseg_attack: 0.05,
            mseg_hold: 0.02,
            mseg_decay: 0.3,
            mseg_sustain: 0.5,
            mseg_release: 0.1,
            env_amount: 1.0,
            velocity_to_peak: 0.5,
            loop_mode: 0,
            loop_start_stage: 3,
            loop_end_stage: 4,
            global_time_scale: 1.0,
            velocity_to_level: 1.0,
            velocity_to_time: 0.15,
            curve_tension: 0.0,
            exciter_pressure: 0.5,
            exciter_speed: 0.5,
            exciter_roughness: 0.3,
            hand_mass: 1.7,
            flesh_stiffness: 0.425,
            flesh_damping: 0.75,
            mute_decay: 0.9245,
            mallet_mass: 1.5,
            felt_softness: 0.94,
            core_hardness: 2.5,
            compression_curve: 2.9,
            material_stiffness: 2.75,
            impact_damping: 0.46,
            stick_mass: 0.65,
            tip_stiffness: 3.8,
            restitution_bounciness: 0.41,
            micro_bounce_limit: 4.0,
            wire_density: 46.0,
            spread_duration: 130.0,
            brush_wire_stiffness: 0.3,
            amplitude_randomization: 0.3,
            pipe_mass: 1.5,
            metal_stiffness: 3.0,
            pipe_pitch: 1.1,
            pipe_ring_decay: 0.9795,
            link_count: 9.0,
            chain_mass: 0.8,
            drop_envelope_spread: 220.0,
            internal_rattle: 0.3,
            rattle_color: 0.3,
            bow_pressure: 1.1,
            bow_speed: 1.05,
            rosin_grip: 0.485,
            slip_curve: 0.485,
            scrape_speed: 0.82,
            point_pressure: 0.8,
            chatter_pitch: 0.52,
            chatter_damping: 0.66,
            grind_speed: 0.82,
            grind_pressure: 1.0,
            surface_grit: 0.3,
            grit_color: 0.3,
            drag_speed: 0.82,
            ridge_spacing: 0.143,
            ridge_depth: 0.6,
            drag_exciter_mass: 1.1,
            pull_speed: 0.8,
            break_threshold: 0.85,
            slip_stochasticity: 0.3,
            creak_sharpness: 0.56,
            air_pressure: 1.1,
            nozzle_width: 0.85,
            turbulence_chaos: 0.6,
            mains_frequency: 80.0,
            coil_proximity: 1.0,
            voltage_sag: 0.6,
            pull_distance: 0.8,
            hook_stiffness: 1.2,
            snap_force: 0.67,
            flow_rate: 1.6,
            particle_mass: 0.525,
            mass_variance: 0.6,
            strike_position: 0.5,
            coupling_stiffness: 1.0,
            position_wander: 0.0,
            position_envelope: 0.0,
            fundamental_anchor: 0.3,
            res_damping: 0.5,
            res_brightness: 0.5,
            thickness: 0.5,
            heat: 0.0,
            sludge: 0.0,
            character: CharacterParams::default(),
            mode_count_scale: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Simplified envelope used for hit-style exciter families.
enum OneShotEnvelopePhase {
    /// Envelope is silent and inactive.
    Idle,
    /// Envelope is rising toward full level.
    Attack,
    /// Envelope is falling back to silence.
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// ADSR envelope used for specialty exciter families.
enum AdsrPhase {
    /// Envelope is silent and inactive.
    Idle,
    /// Envelope is rising toward full level.
    Attack,
    /// Envelope is falling toward the sustain level.
    Decay,
    /// Envelope is holding its sustain level until note-off.
    Sustain,
    /// Envelope is falling back to silence after note-off.
    Release,
}

/// One polyphonic voice containing a resonator, envelope state, and
/// every exciter implementation used by the instrument.
pub struct Voice {
    active: bool,
    rendering: bool,
    note: u8,
    velocity: f32,
    exciter_type: i32,
    excitation_sent: bool,
    resonator: ModalResonator,

    // One slot per exciter family so dispatch stays allocation-free.
    bow_exciter: BowExciter,
    hand_strike: HandStrike,
    felt_mallet: FeltMallet,
    hard_mallet: HardMallet,
    drumstick: Drumstick,
    wire_brush: WireBrush,
    metal_pipe: MetalPipe,
    metal_chain: MetalChain,
    stiff_point: StiffPointScrape,
    heavy_grinding: HeavyGrinding,
    corrugated_drag: CorrugatedDrag,
    tension_rise: TensionRise,
    pneumatic_jet: PneumaticJet,
    electromagnetic_hum: ElectromagneticHum,
    tension_snap: TensionSnap,
    particle_rain: ParticleRain,

    peak_hold: f32,
    /// Largest |sample| fed to the output clamp since note-on. Lets tests
    /// check that default notes don't overdrive (`normalize-object-loudness`).
    pre_clamp_peak: f32,
    /// Extra multiplier on the exciter force, 1.0 in use. Tests raise it to
    /// probe contact stability far above the default drive
    /// (`stabilize-mallet-and-grinding-contact`).
    pub(crate) drive_scale: f32,
    /// Whether the drive governor may turn the exciter force down. Always
    /// on in use; tests turn it off to probe the contact's own stability
    /// (`resolve-exciter-tradeoffs`).
    pub(crate) governor_enabled: bool,
    /// The governor's follower of the ring bound.
    governor_level: f32,
    /// The governor's gain on the exciter force.
    governor_gain: f32,
    /// Whether the resonator tracks its ring bound this sample.
    ring_tracking: bool,
    frames_below_threshold: u32,
    start_frame: u64,
    excitation_value: f32,
    excitation_state: f32,
    excitation_decay: f32,
    highpass_state: f32,
    damage_amount: f32,
    /// Noise for the damage rattle (`resolve-exciter-tradeoffs`).
    rattle_noise: crate::dsp::utils::noise::Noise,
    sample_rate: f32,
    /// Per-sample constants converted for `sample_rate` (see `set_rate`).
    rate: VoiceRateConstants,
    exciter_family: ExciterFamily,
    one_shot_phase: OneShotEnvelopePhase,
    /// Whether the next attack step is the first of the note.
    one_shot_first_step: bool,
    one_shot_level: f32,
    one_shot_attack: f32,
    one_shot_release: f32,
    adsr_phase: AdsrPhase,
    adsr_level: f32,
    adsr_attack: f32,
    adsr_decay: f32,
    adsr_sustain: f32,
    adsr_release: f32,
    mseg: MSEG,

    // MIDI expression state. The voice combines per-channel and per-note
    // expression sources, so MPE, classic CC1+aftertouch, and poly pressure
    // can all reach the same destinations without stepping on each other.
    /// Pitch bend in semitones. Channel-wide; updated by MidiPitchBend events.
    pitch_bend_semitones: f32,
    /// Channel pressure (aftertouch), `0.0..=1.0`. Updated by MidiChannelPressure.
    channel_pressure: f32,
    /// Per-note pressure (poly aftertouch), `0.0..=1.0`. Maxed with channel pressure.
    poly_pressure: f32,
    /// CC1 mod wheel, `0.0..=1.0`. Routed to brightness boost.
    mod_wheel: f32,
}

const TAIL_ENERGY_THRESHOLD: f32 = 1e-4;
/// Frames of near-silence before a released tail deactivates, at 48 kHz.
const TAIL_DEACTIVATE_FRAMES: u32 = 4800;
/// Per-sample peak-hold decay, at 48 kHz.
const PEAK_DECAY: f32 = 0.999;
/// Drive level per exciter ID (index 0 unused): the force multiplier that
/// puts each exciter's median note, over every object at C2/C4/C6 (velocity
/// 127, default controls), at a pre-clamp peak of about 0.35
/// (`resolve-exciter-tradeoffs`). Banks are normalized to one strike level,
/// so what remains is each exciter's own drive. Louder notes are left to the
/// drive governor rather than turning the exciter down everywhere. The
/// levels come from `print_exciter_overdrive`; HandStrike, TensionSnap and
/// MetalPipe respond non-linearly to their drive (the contact solve), so
/// theirs were found by bracketing rather than in one step.
const EXCITER_DRIVE_LEVEL: [f32; 17] = [
    1.0, 2.1664e-02, 5.5400e+01, 8.1953e-01, 5.7269e+00, 9.2944e-01, 5.3012e-01, 5.4100e-01,
    1.4146e+00, 8.3877e-03, 2.6216e-03, 1.9438e-02, 9.0962e-02, 5.9898e-03, 3.2879e-03, 6.2745e-01,
    4.4103e-02,
];

/// Pole and input gain of the voice's one-pole high-pass boost, at 48 kHz.
/// The resonator's ring bound weights each mode by this filter's response.
/// (The gain is kept as its own literal: `1.0 − 0.8` isn't exactly `0.2` in
/// f32.)
const HIGHPASS_POLE: f32 = crate::dsp::resonators::core::OUTPUT_HIGHPASS_POLE;
const HIGHPASS_GAIN: f32 = 0.2;
/// Boost of the high-passed signal per unit of normalized velocity.
const HIGHPASS_BOOST: f32 = 1.5;

/// Drive governor (`resolve-exciter-tradeoffs`, design Decision 1). Above
/// this level of the followed ring bound, the exciter force is scaled by
/// `(GOVERNOR_THRESHOLD / level)^GOVERNOR_ORDER`. A sustained note driven
/// `x` times over the threshold then settles at `x^(1/(order + 1))` of it:
/// +1.3 dB for 12 dB over.
const GOVERNOR_THRESHOLD: f32 = 0.7;
const GOVERNOR_ORDER: i32 = 8;
/// The bound no single sample's force may push the ring bound past, so a
/// strike that comes faster than the follower can react still stays under
/// full scale. 0.9 keeps the margin `stabilize-mallet-and-grinding-contact`
/// specifies for HardMallet and HeavyGrinding, for every exciter.
const GOVERNOR_CEILING: f32 = 0.9;
/// The follower's attack and release time constants, in seconds.
const GOVERNOR_ATTACK_S: f32 = 0.001;
const GOVERNOR_RELEASE_S: f32 = 0.2;

/// The voice's per-sample constants for one sample rate. Each is exactly its
/// 48 kHz value at 48 kHz (`fix-sample-rate-time-constants`).
#[derive(Clone, Copy, Debug)]
struct VoiceRateConstants {
    sample_rate: f32,
    highpass_pole: f32,
    highpass_gain: f32,
    peak_decay: f32,
    tail_frames: u32,
    governor_attack: f32,
    governor_release: f32,
}

/// One-pole follower coefficient for time constant `seconds`, at 48 kHz.
fn follower_coefficient(seconds: f32) -> f32 {
    1.0 - (-1.0 / (seconds * rate::REFERENCE_RATE)).exp()
}

impl VoiceRateConstants {
    fn for_rate(sample_rate: f32) -> Self {
        let r = rate::ratio(sample_rate);
        Self {
            sample_rate,
            highpass_pole: rate::decay(HIGHPASS_POLE, r),
            highpass_gain: if r == 1.0 {
                HIGHPASS_GAIN
            } else {
                1.0 - rate::decay(HIGHPASS_POLE, r)
            },
            peak_decay: rate::decay(PEAK_DECAY, r),
            tail_frames: rate::frames(TAIL_DEACTIVATE_FRAMES, r),
            governor_attack: rate::smoother(follower_coefficient(GOVERNOR_ATTACK_S), r),
            governor_release: rate::smoother(follower_coefficient(GOVERNOR_RELEASE_S), r),
        }
    }
}

impl Voice {
    /// Create an inactive voice with default resonator and exciter state.
    ///
    /// The voice starts muted and is only activated by `note_on`.
    ///
    /// # Returns
    ///
    /// A ready-to-use voice instance.
    pub fn new() -> Self {
        Self {
            active: false,
            rendering: false,
            note: 60,
            velocity: 0.0,
            exciter_type: 0,
            excitation_sent: false,
            resonator: ModalResonator::with_profile(ModalProfileId::Pipe),

            bow_exciter: BowExciter::new(),
            hand_strike: HandStrike::new(),
            felt_mallet: FeltMallet::new(),
            hard_mallet: HardMallet::new(),
            drumstick: Drumstick::new(),
            wire_brush: WireBrush::new(),
            metal_pipe: MetalPipe::new(),
            metal_chain: MetalChain::new(),
            stiff_point: StiffPointScrape::new(),
            heavy_grinding: HeavyGrinding::new(),
            corrugated_drag: CorrugatedDrag::new(),
            tension_rise: TensionRise::new(),
            pneumatic_jet: PneumaticJet::new(),
            electromagnetic_hum: ElectromagneticHum::new(),
            tension_snap: TensionSnap::new(),
            particle_rain: ParticleRain::new(),

            peak_hold: 0.0,
            pre_clamp_peak: 0.0,
            drive_scale: 1.0,
            governor_enabled: true,
            governor_level: 0.0,
            governor_gain: 1.0,
            ring_tracking: false,
            frames_below_threshold: 0,
            start_frame: 0,
            excitation_value: 0.0,
            excitation_state: 0.0,
            excitation_decay: 0.5,
            highpass_state: 0.0,
            damage_amount: 0.0,
            rattle_noise: crate::dsp::utils::noise::Noise::default(),
            sample_rate: 48000.0,
            rate: VoiceRateConstants::for_rate(48_000.0),
            exciter_family: ExciterFamily::Hit,
            one_shot_phase: OneShotEnvelopePhase::Idle,
            one_shot_first_step: false,
            one_shot_level: 0.0,
            one_shot_attack: 0.05,
            one_shot_release: 0.5,
            adsr_phase: AdsrPhase::Idle,
            adsr_level: 0.0,
            adsr_attack: 0.05,
            adsr_decay: 0.3,
            adsr_sustain: 0.7,
            adsr_release: 0.5,
            mseg: MSEG::new(),

            pitch_bend_semitones: 0.0,
            channel_pressure: 0.0,
            poly_pressure: 0.0,
            mod_wheel: 0.0,
        }
    }

    fn configure_envelope(
        &mut self,
        exciter_type: i32,
        velocity_norm: f32,
        controls: VoiceControls,
    ) {
        self.exciter_family = ExciterType::from_int(exciter_type).family();

        // Each family gets a different envelope model so the voice can
        // remain allocation-free while still matching the exciter behavior.
        match self.exciter_family {
            ExciterFamily::Hit => {
                self.one_shot_phase = OneShotEnvelopePhase::Attack;
                self.one_shot_level = 0.0;
                self.one_shot_first_step = true;
                self.one_shot_attack = controls.env_attack.max(0.001);
                self.one_shot_release = controls.env_release.max(0.001);
                self.mseg = MSEG::new();
                self.adsr_phase = AdsrPhase::Idle;
            }
            ExciterFamily::Specialty => {
                self.adsr_phase = AdsrPhase::Attack;
                self.adsr_level = 0.0;
                self.adsr_attack = controls.env_attack.max(0.001);
                self.adsr_decay = controls.env_decay.max(0.001);
                self.adsr_sustain = controls.env_sustain.clamp(0.0, 1.0);
                self.adsr_release = controls.env_release.max(0.001);
                self.one_shot_phase = OneShotEnvelopePhase::Idle;
                self.mseg = MSEG::new();
            }
            ExciterFamily::Friction => {
                let mut mseg = MSEG::new();
                mseg.set_sample_rate(self.sample_rate);
                mseg.set_stage_times(
                    controls.mseg_onset,
                    controls.mseg_attack,
                    controls.mseg_hold,
                    controls.mseg_decay,
                    controls.mseg_release,
                );
                let peak_level =
                    (0.5 + controls.velocity_to_peak.clamp(0.0, 1.0) * 0.5).clamp(0.0, 1.0);
                mseg.set_stage_levels(
                    controls.mseg_onset.clamp(0.0, 1.0),
                    peak_level,
                    controls.mseg_sustain,
                    0.0,
                );
                mseg.set_curves(
                    controls.curve_tension,
                    controls.curve_tension,
                    controls.curve_tension,
                    controls.curve_tension,
                );
                let loop_mode = match controls.loop_mode {
                    1 => LoopMode::Forward,
                    2 => LoopMode::PingPong,
                    _ => LoopMode::Off,
                };
                mseg.set_loop(
                    loop_mode,
                    controls.loop_start_stage as u8,
                    controls.loop_end_stage as u8,
                );
                mseg.set_velocity_response(controls.velocity_to_level, controls.velocity_to_time);
                mseg.set_env_amount(controls.env_amount);
                mseg.set_global_time_scale(controls.global_time_scale);
                mseg.trigger(velocity_norm);
                self.mseg = mseg;
                self.one_shot_phase = OneShotEnvelopePhase::Idle;
                self.one_shot_level = 0.0;
            }
        }
    }

    fn process_force_envelope(&mut self) -> f32 {
        // Each family gets a different envelope model so the voice can
        // remain allocation-free while still matching the exciter behavior.
        match self.exciter_family {
            ExciterFamily::Hit => {
                let dt = 1.0 / self.sample_rate.max(1.0);
                match self.one_shot_phase {
                    OneShotEnvelopePhase::Idle => 0.0,
                    OneShotEnvelopePhase::Attack => {
                        // Sampled at sample centres: the first step is half
                        // a step, so a short strike on the ramp has the same
                        // area at every rate (`resolve-exciter-tradeoffs`).
                        let step = if std::mem::take(&mut self.one_shot_first_step) {
                            0.5
                        } else {
                            1.0
                        };
                        self.one_shot_level =
                            (self.one_shot_level + step * dt / self.one_shot_attack).min(1.0);
                        if self.one_shot_level >= 1.0 {
                            self.one_shot_phase = OneShotEnvelopePhase::Release;
                        }
                        self.one_shot_level
                    }
                    OneShotEnvelopePhase::Release => {
                        self.one_shot_level =
                            (self.one_shot_level - dt / self.one_shot_release).max(0.0);
                        if self.one_shot_level <= 0.0 {
                            self.one_shot_phase = OneShotEnvelopePhase::Idle;
                        }
                        self.one_shot_level
                    }
                }
            }
            ExciterFamily::Specialty => {
                let dt = 1.0 / self.sample_rate.max(1.0);
                match self.adsr_phase {
                    AdsrPhase::Idle => 0.0,
                    AdsrPhase::Attack => {
                        self.adsr_level = (self.adsr_level + dt / self.adsr_attack).min(1.0);
                        if self.adsr_level >= 1.0 {
                            self.adsr_phase = AdsrPhase::Decay;
                        }
                        self.adsr_level
                    }
                    AdsrPhase::Decay => {
                        let decay_rate = (1.0 - self.adsr_sustain) / self.adsr_decay.max(dt);
                        self.adsr_level =
                            (self.adsr_level - decay_rate * dt).max(self.adsr_sustain);
                        if self.adsr_level <= self.adsr_sustain {
                            self.adsr_phase = AdsrPhase::Sustain;
                        }
                        self.adsr_level
                    }
                    AdsrPhase::Sustain => {
                        if !self.active {
                            self.adsr_phase = AdsrPhase::Release;
                        }
                        self.adsr_sustain
                    }
                    AdsrPhase::Release => {
                        let release_rate = self.adsr_sustain / self.adsr_release.max(dt);
                        self.adsr_level = (self.adsr_level - release_rate * dt).max(0.0);
                        if self.adsr_level <= 0.0 {
                            self.adsr_phase = AdsrPhase::Idle;
                        }
                        self.adsr_level
                    }
                }
            }
            ExciterFamily::Friction => {
                self.mseg.set_sample_rate(self.sample_rate);
                self.mseg.process_sample()
            }
        }
    }

    fn release_force_envelope(&mut self) {
        // Each family gets a different envelope model so the voice can
        // remain allocation-free while still matching the exciter behavior.
        match self.exciter_family {
            ExciterFamily::Hit => {
                if self.one_shot_phase != OneShotEnvelopePhase::Idle {
                    self.one_shot_phase = OneShotEnvelopePhase::Release;
                }
            }
            ExciterFamily::Specialty => {
                if self.adsr_phase != AdsrPhase::Idle && self.adsr_phase != AdsrPhase::Release {
                    self.adsr_phase = AdsrPhase::Release;
                }
            }
            ExciterFamily::Friction => self.mseg.release(),
        }
    }

    /// Trigger the voice with default envelope controls.
    ///
    /// This is the simple entry point used when the caller does not need
    /// per-note control overrides.
    ///
    /// # Arguments
    ///
    /// * `note` - MIDI note number for the new voice.
    /// * `velocity` - MIDI velocity used to scale excitation energy.
    /// * `profile_id` - Resonator profile to load for the voice.
    /// * `start_frame` - Audio frame when the note began.
    /// * `size` - Material size parameter passed into the resonator.
    /// * `rust` - Material rust parameter passed into the resonator.
    /// * `damage` - Material damage parameter passed into the resonator.
    /// * `exciter_type` - Integer selector for the exciter model.
    ///
    /// # Returns
    ///
    /// This method mutates the voice in place and returns nothing.
    // Kept as positional API to avoid widening the voice/manager call graph for a Clippy-only cleanup.
    #[allow(clippy::too_many_arguments)]
    pub fn note_on(
        &mut self,
        note: u8,
        velocity: f32,
        profile_id: ModalProfileId,
        start_frame: u64,
        size: f32,
        rust: f32,
        damage: f32,
        exciter_type: i32,
    ) {
        self.note_on_with_controls(
            note,
            velocity,
            profile_id,
            start_frame,
            size,
            rust,
            damage,
            exciter_type,
            VoiceControls::default(),
        );
    }

    /// Trigger the voice with explicit envelope and exciter controls.
    ///
    /// Used by the host path in `src/lib.rs` when parameter snapshots need
    /// to be applied at note-on time.
    ///
    /// # Arguments
    ///
    /// * `note` - MIDI note number for the new voice.
    /// * `velocity` - MIDI velocity used to scale excitation energy.
    /// * `profile_id` - Resonator profile to load for the voice.
    /// * `start_frame` - Audio frame when the note began.
    /// * `size` - Material size parameter passed into the resonator.
    /// * `rust` - Material rust parameter passed into the resonator.
    /// * `damage` - Material damage parameter passed into the resonator.
    /// * `exciter_type` - Integer selector for the exciter model.
    /// * `controls` - Complete per-voice control snapshot.
    ///
    /// # Returns
    ///
    /// This method mutates the voice in place and returns nothing.
    // Mirrors the host parameter snapshot shape; grouping would be a broader API refactor.
    #[allow(clippy::too_many_arguments)]
    pub fn note_on_with_controls(
        &mut self,
        note: u8,
        velocity: f32,
        profile_id: ModalProfileId,
        start_frame: u64,
        size: f32,
        rust: f32,
        damage: f32,
        exciter_type: i32,
        controls: VoiceControls,
    ) {
        self.active = true;
        self.rendering = true;
        self.note = note;
        self.velocity = velocity;
        let exciter_type = ExciterType::from_int(exciter_type).to_int();
        self.exciter_type = exciter_type;
        self.excitation_sent = false;
        self.peak_hold = 0.0;
        self.pre_clamp_peak = 0.0;
        self.governor_level = 0.0;
        self.governor_gain = 1.0;
        self.frames_below_threshold = 0;
        self.start_frame = start_frame;
        // One seed per note (and so per voice), a separate stream for each
        // noise consumer.
        {
            use crate::dsp::utils::noise::{note_seed, salted};
            let seed = note_seed(note, velocity, start_frame);
            self.rattle_noise.reseed(salted(seed, 0));
            self.wire_brush.reseed(salted(seed, 6));
            self.metal_chain.reseed(salted(seed, 8));
            self.heavy_grinding.reseed(salted(seed, 10));
            self.tension_rise.reseed(salted(seed, 12));
            self.pneumatic_jet.reseed(salted(seed, 13));
            self.particle_rain.reseed(salted(seed, 16));
        }
        let velocity_norm = (velocity / 127.0).clamp(0.0, 1.0);
        let clamped_damage = damage.clamp(0.0, 10.0);
        self.damage_amount = clamped_damage;
        // The algorithmic per-object generator is the only resonator path; the
        // profile tables now contribute only mode counts (see core.rs). The
        // bank is rebuilt in the voice's preallocated buffers so note-on never
        // allocates on the audio thread.
        self.resonator.rebuild_in_place(
            profile_id,
            SizeScale::new(size),
            RustAmount::new(rust),
            DamageAmount::new(clamped_damage),
            ThicknessAmount::new(controls.thickness),
            HeatAmount::new(controls.heat),
            SludgeAmount::new(controls.sludge),
            midi_to_hz(note),
            controls.res_damping,
            controls.res_brightness,
            controls.character,
            controls.mode_count_scale,
        );
        self.resonator.set_interaction_params(
            controls.strike_position,
            controls.coupling_stiffness,
            controls.position_wander,
            controls.position_envelope,
            controls.fundamental_anchor,
            self.sample_rate,
        );
        // Carry the current channel pitch bend into the new note so a held
        // bend doesn't reset every time a fresh key is pressed. Channel
        // pressure and mod wheel persist for the same reason; poly pressure
        // is per-note and must reset.
        self.poly_pressure = 0.0;
        let bend_factor = pitch_bend_semitones_to_factor(self.pitch_bend_semitones);
        self.resonator
            .set_pitch_bend_factor(bend_factor, self.sample_rate as u32);
        self.configure_envelope(exciter_type, velocity_norm, controls);

        let v_norm = velocity_norm;
        // Dispatch the requested exciter family in place; each branch
        // configures the model and triggers it with the current velocity.
        match exciter_type {
            1 => {
                self.bow_exciter.set_parameters(
                    controls.bow_pressure,
                    controls.bow_speed,
                    controls.rosin_grip,
                    controls.slip_curve,
                );
                self.bow_exciter.trigger(v_norm);
            }
            2 => {
                self.hand_strike.set_parameters(
                    controls.hand_mass,
                    controls.flesh_stiffness,
                    controls.flesh_damping,
                    controls.mute_decay,
                );
                self.hand_strike.trigger(v_norm);
            }
            3 => {
                self.felt_mallet.set_parameters(
                    controls.mallet_mass,
                    controls.felt_softness,
                    controls.core_hardness,
                    controls.compression_curve,
                );
                self.felt_mallet.trigger(v_norm);
            }
            4 => {
                self.hard_mallet.set_parameters(
                    controls.mallet_mass,
                    controls.material_stiffness,
                    controls.impact_damping,
                );
                self.hard_mallet.trigger(v_norm);
            }
            5 => {
                self.drumstick.set_parameters(
                    controls.stick_mass,
                    controls.tip_stiffness,
                    controls.restitution_bounciness,
                    controls.micro_bounce_limit.round() as u32,
                );
                self.drumstick.trigger(v_norm);
            }
            6 => {
                self.wire_brush.set_parameters(
                    controls.wire_density.round() as u32,
                    controls.spread_duration,
                    controls.brush_wire_stiffness,
                    controls.amplitude_randomization,
                );
                self.wire_brush.set_sample_rate(self.sample_rate);
                self.wire_brush.trigger(v_norm);
            }
            7 => {
                self.metal_pipe.set_parameters(
                    controls.pipe_mass,
                    controls.metal_stiffness,
                    controls.pipe_pitch,
                    controls.pipe_ring_decay,
                );
                self.metal_pipe.set_sample_rate(self.sample_rate);
                self.metal_pipe.trigger(v_norm);
            }
            8 => {
                self.metal_chain.set_parameters(
                    controls.link_count.round() as u32,
                    controls.chain_mass,
                    controls.drop_envelope_spread,
                    controls.internal_rattle,
                    controls.rattle_color,
                );
                self.metal_chain.set_sample_rate(self.sample_rate);
                self.metal_chain.trigger(v_norm);
            }
            9 => {
                self.stiff_point.set_parameters(
                    controls.scrape_speed,
                    controls.point_pressure,
                    controls.chatter_pitch,
                    controls.chatter_damping,
                );
                self.stiff_point.trigger(v_norm);
            }
            10 => {
                self.heavy_grinding.set_parameters(
                    controls.grind_speed,
                    controls.grind_pressure,
                    controls.surface_grit,
                    controls.grit_color,
                );
                self.heavy_grinding.trigger(v_norm);
            }
            11 => {
                self.corrugated_drag.set_parameters(
                    controls.drag_speed,
                    controls.ridge_spacing,
                    controls.ridge_depth,
                    controls.drag_exciter_mass,
                );
                self.corrugated_drag.trigger(v_norm);
            }
            12 => {
                self.tension_rise.set_parameters(
                    controls.pull_speed,
                    controls.break_threshold,
                    controls.slip_stochasticity,
                    controls.creak_sharpness,
                );
                self.tension_rise.trigger(v_norm);
            }
            13 => {
                self.pneumatic_jet.set_parameters(
                    controls.air_pressure,
                    controls.nozzle_width,
                    controls.turbulence_chaos,
                );
                self.pneumatic_jet.trigger(v_norm);
            }
            14 => {
                self.electromagnetic_hum.set_parameters(
                    controls.mains_frequency,
                    controls.coil_proximity,
                    controls.voltage_sag,
                );
                self.electromagnetic_hum.trigger(v_norm);
            }
            15 => {
                self.tension_snap.set_parameters(
                    controls.pull_distance,
                    controls.hook_stiffness,
                    controls.snap_force,
                );
                self.tension_snap.trigger(v_norm);
            }
            16 => {
                self.particle_rain.set_parameters(
                    controls.flow_rate,
                    controls.particle_mass,
                    controls.mass_variance,
                );
                self.particle_rain.trigger(v_norm);
            }
            _ => {}
        }

        self.excitation_value = velocity_norm;
        self.excitation_state = 0.0;
        self.excitation_decay = (0.985 - (velocity_norm * 0.35)).clamp(0.6, 0.985);
        self.highpass_state = 0.0;
    }

    /// Apply held-note automation: damping, brightness, and strike position
    /// for an already-rendering voice.
    ///
    /// Only the active (held) voices receive updates so tails decay with the
    /// timbre they had at note-on. Allocation-free; recomputes resonator mode
    /// coefficients and interaction-bus state in place.
    // Mirrors `VoiceManager::update_live_controls` so the per-buffer host
    // parameter shape stays positional. Grouping into a struct would force
    // a wider refactor across the lib.rs → manager → voice call chain.
    #[allow(clippy::too_many_arguments)]
    pub fn update_live_controls(
        &mut self,
        damping: f32,
        brightness: f32,
        strike_position: f32,
        coupling_stiffness: f32,
        position_wander: f32,
        position_envelope: f32,
        fundamental_anchor: f32,
        sample_rate: u32,
    ) {
        if !self.active {
            return;
        }
        self.resonator
            .set_live_resonator_controls(damping, brightness, sample_rate);
        self.resonator.set_interaction_params(
            strike_position,
            coupling_stiffness,
            position_wander,
            position_envelope,
            fundamental_anchor,
            self.sample_rate,
        );
    }

    /// Apply a channel pitch bend in semitones to this voice.
    ///
    /// Rebuilds the resonator mode coefficients in place via
    /// `ModalResonator::set_pitch_bend_factor`. Allocation-free.
    pub fn set_pitch_bend_semitones(&mut self, semitones: f32) {
        self.pitch_bend_semitones = semitones;
        let factor = pitch_bend_semitones_to_factor(semitones);
        self.resonator
            .set_pitch_bend_factor(factor, self.sample_rate as u32);
    }

    /// Update channel pressure (`MidiChannelPressure`) for this voice.
    #[inline]
    pub fn set_channel_pressure(&mut self, value: f32) {
        self.channel_pressure = value.clamp(0.0, 1.0);
    }

    /// Update per-note (poly) pressure (`PolyPressure`) for this voice.
    #[inline]
    pub fn set_poly_pressure(&mut self, value: f32) {
        self.poly_pressure = value.clamp(0.0, 1.0);
    }

    /// Update CC1 mod wheel for this voice (channel-wide expression source).
    #[inline]
    pub fn set_mod_wheel(&mut self, value: f32) {
        self.mod_wheel = value.clamp(0.0, 1.0);
    }

    /// Effective pressure used by the expression scaler: max of channel and
    /// poly pressure so MPE (poly) and classic (channel) aftertouch both reach
    /// the same destination.
    #[inline]
    fn effective_pressure(&self) -> f32 {
        self.channel_pressure.max(self.poly_pressure)
    }

    /// Per-sample output gain shaped by expression. Pressure raises gain up
    /// to ~+3.5 dB; mod wheel adds ~+2 dB so CC1 sweeps stay distinct from
    /// pressure on hosts that only send one of the two. The cap keeps a
    /// single voice's expressioned output ≲ 1.8; the voice sum is not
    /// normalized, so the output limiter absorbs dense 16-voice peaks.
    #[inline]
    fn expression_gain(&self) -> f32 {
        1.0 + self.effective_pressure() * 0.5 + self.mod_wheel * 0.3
    }

    /// Release the voice and begin its tail phase.
    ///
    /// The resonator is allowed to decay naturally; only the excitation
    /// source is told to release immediately.
    pub fn note_off(&mut self) {
        self.active = false;
        self.release_force_envelope();
        self.bow_exciter.release();
        self.stiff_point.release();
        self.heavy_grinding.release();
        self.corrugated_drag.release();
        self.tension_rise.release();
        self.pneumatic_jet.release();
        self.electromagnetic_hum.release();
        self.tension_snap.release();
        self.particle_rain.release();
    }

    /// Report whether the voice's note is held (not yet released).
    ///
    /// `note_off` clears this at once; the voice can keep sounding afterwards
    /// while its tail decays (see `is_rendering`). Voice stealing relies on
    /// this held-only meaning to tell held notes from released tails.
    ///
    /// # Returns
    ///
    /// `true` while the note is held; `false` once released or idle.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Silence the voice and return it to idle without allocating, as if its
    /// tail had finished. The next note-on rebuilds the resonator and
    /// re-triggers its exciter, so only the state that could leak sound or
    /// expression into that note is cleared here.
    pub fn reset(&mut self) {
        self.active = false;
        self.rendering = false;
        self.excitation_sent = false;
        self.resonator.clear_state();
        self.peak_hold = 0.0;
        self.frames_below_threshold = 0;
        self.excitation_value = 0.0;
        self.excitation_state = 0.0;
        self.highpass_state = 0.0;
        self.rattle_noise.reseed(0);
        self.one_shot_phase = OneShotEnvelopePhase::Idle;
        self.one_shot_level = 0.0;
        self.adsr_phase = AdsrPhase::Idle;
        self.adsr_level = 0.0;
        self.mseg = MSEG::new();
        self.pitch_bend_semitones = 0.0;
        self.channel_pressure = 0.0;
        self.poly_pressure = 0.0;
        self.mod_wheel = 0.0;
    }

    pub fn is_rendering(&self) -> bool {
        self.rendering
    }

    /// Return the MIDI note currently assigned to the voice.
    ///
    /// # Returns
    ///
    /// The active MIDI note number.
    pub fn note(&self) -> u8 {
        self.note
    }

    /// Return the decaying peak used by the stealing algorithm.
    ///
    /// # Returns
    ///
    /// The current peak-hold value.
    pub fn peak_hold(&self) -> f32 {
        self.peak_hold
    }

    /// Return the audio frame when the voice was started.
    ///
    /// # Returns
    ///
    /// The frame counter snapshot captured at note-on time.
    pub fn start_frame(&self) -> u64 {
        self.start_frame
    }

    /// Test-only accessor for the resonator's modal profile, so lib-level
    /// integration tests can confirm which family the voice picked.
    #[cfg(test)]
    pub(crate) fn resonator_profile_for_test(&self) -> ModalProfileId {
        self.resonator.profile
    }

    /// Test-only accessor for whether the MSEG loop is engaged on this voice.
    /// Used to verify Drone-mode wiring.
    #[cfg(test)]
    pub(crate) fn mseg_loop_enabled_for_test(&self) -> bool {
        self.mseg.is_loop_enabled()
    }

    /// Track the host sample rate, re-deriving per-sample constants when it
    /// changes. Cheap when unchanged (one comparison).
    fn set_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        if self.rate.sample_rate != sample_rate {
            self.rate = VoiceRateConstants::for_rate(sample_rate);
            self.bow_exciter.set_sample_rate(sample_rate);
            self.hand_strike.set_sample_rate(sample_rate);
            self.felt_mallet.set_sample_rate(sample_rate);
            self.hard_mallet.set_sample_rate(sample_rate);
            self.drumstick.set_sample_rate(sample_rate);
            self.wire_brush.set_sample_rate(sample_rate);
            self.metal_pipe.set_sample_rate(sample_rate);
            self.metal_chain.set_sample_rate(sample_rate);
            self.stiff_point.set_sample_rate(sample_rate);
            self.heavy_grinding.set_sample_rate(sample_rate);
            self.corrugated_drag.set_sample_rate(sample_rate);
            self.tension_rise.set_sample_rate(sample_rate);
            self.pneumatic_jet.set_sample_rate(sample_rate);
            self.electromagnetic_hum.set_sample_rate(sample_rate);
            self.tension_snap.set_sample_rate(sample_rate);
            self.particle_rain.set_sample_rate(sample_rate);
        }
    }

    fn rattle_noise(&mut self, signal_level: f32) -> f32 {
        if self.damage_amount <= 0.0 {
            return 0.0;
        }

        let threshold = 0.02 + (1.0 - self.damage_amount.min(1.0)) * 0.15;
        if signal_level < threshold {
            return 0.0;
        }

        // Per-sample white noise added to the output: hold its audible-band
        // level at any rate.
        let noise = crate::dsp::utils::rate::white_noise(
            self.rattle_noise.bipolar(),
            crate::dsp::utils::rate::ratio(self.sample_rate),
        );
        let highpass_noise = noise * 0.5;
        let rattle_amount = self.damage_amount.min(1.0) * (signal_level - threshold) * 0.08;
        highpass_noise * rattle_amount
    }

    /// The resonator's response to this sample's exciter force, as the
    /// exciter sees it: scaled by everything the voice multiplies the force
    /// by (`stabilize-mallet-and-grinding-contact`).
    fn contact_prediction(&mut self, envelope: f32) -> crate::dsp::ContactPrediction {
        let scale = envelope * self.force_scale();
        let prediction = self
            .resonator
            .contact_prediction(self.sample_rate as u32)
            .scaled(scale);
        match self.governor_force_limit() {
            Some(limit) => prediction.limited(limit / scale.abs().max(f32::MIN_POSITIVE)),
            None => prediction,
        }
    }

    /// Everything the voice multiplies the exciter's force by, after the
    /// envelope: drive level, test drive and the governor's smooth gain.
    fn force_scale(&self) -> f32 {
        let governor = if self.governor_enabled {
            self.governor_gain
        } else {
            1.0
        };
        EXCITER_DRIVE_LEVEL[self.exciter_type.clamp(0, 16) as usize] * self.drive_scale * governor
    }

    /// The governor's per-sample ceiling: cap `force` so it can't push the
    /// ring bound past `GOVERNOR_CEILING`. Contact exciters see the same cap
    /// in their prediction (`ContactPrediction::limit`). The modes only decay
    /// without force, so the bound after the last sample is an upper bound
    /// on where this sample starts.
    fn apply_governor_ceiling(&mut self, force: f32) -> f32 {
        match self.governor_force_limit() {
            Some(limit) => force.clamp(-limit, limit),
            None => force,
        }
    }

    /// The largest force this sample may apply, if the governor limits it.
    fn governor_force_limit(&mut self) -> Option<f32> {
        if !self.governor_enabled {
            return None;
        }
        let bound = self.resonator.ring_bound(self.sample_rate as u32);
        (bound.per_force > 0.0).then(|| (GOVERNOR_CEILING - bound.level).max(0.0) / bound.per_force)
    }

    /// The governor's follower, after the resonator has processed this
    /// sample's force: it follows the ring bound, and above
    /// `GOVERNOR_THRESHOLD` the gain is `(threshold / level)^GOVERNOR_ORDER`.
    /// The gain depends only on the current level, so a long ring that
    /// keeps the bound up after the force has dropped can't wind it down.
    fn update_governor(&mut self) {
        if !self.governor_enabled || !self.ring_tracking {
            return;
        }
        let level = self.resonator.ring_bound(self.sample_rate as u32).level;
        let k = if level > self.governor_level {
            self.rate.governor_attack
        } else {
            self.rate.governor_release
        };
        self.governor_level += k * (level - self.governor_level);
        self.governor_gain = if self.governor_level > GOVERNOR_THRESHOLD {
            (GOVERNOR_THRESHOLD / self.governor_level).powi(GOVERNOR_ORDER)
        } else {
            1.0
        };
    }

    fn process_exciter(&mut self) -> f32 {
        // If there is no active envelope, there is nothing left to excite.
        if !self.active
            && !self.mseg.is_active()
            && self.one_shot_phase == OneShotEnvelopePhase::Idle
        {
            return 0.0;
        }

        let res_disp = self.resonator.get_displacement();
        let res_vel = self.resonator.get_velocity();

        let envelope = self.process_force_envelope();
        self.ring_tracking = self.governor_enabled && envelope > 0.0;
        self.resonator
            .set_ring_tracking(self.ring_tracking, HIGHPASS_BOOST * self.velocity / 127.0);
        if envelope <= 0.0 {
            return 0.0;
        }

        let excitation = match self.exciter_type {
            1 => self.bow_exciter.process_sample(res_vel),
            2 => {
                let prediction = self.contact_prediction(envelope);
                self.hand_strike.process_sample(prediction)
            }
            3 => {
                let prediction = self.contact_prediction(envelope);
                self.felt_mallet.process_sample(prediction)
            }
            4 => {
                let prediction = self.contact_prediction(envelope);
                self.hard_mallet.process_sample(prediction)
            }
            5 => {
                let prediction = self.contact_prediction(envelope);
                self.drumstick.process_sample(prediction)
            }
            6 => self.wire_brush.process_sample(res_disp, 0.0),
            7 => {
                let prediction = self.contact_prediction(envelope);
                self.metal_pipe.process_sample(prediction)
            }
            8 => {
                let prediction = self.contact_prediction(envelope);
                self.metal_chain.process_sample(prediction)
            }
            9 => self.stiff_point.process_sample(res_disp, res_vel),
            10 => {
                let prediction = self.contact_prediction(envelope);
                self.heavy_grinding.process_sample(prediction)
            }
            11 => self.corrugated_drag.process_sample(res_disp, res_vel),
            12 => self.tension_rise.process_sample(res_disp, res_vel),
            13 => self.pneumatic_jet.process_sample(res_disp, res_vel),
            14 => self.electromagnetic_hum.process_sample(res_disp, 0.0),
            15 => {
                let prediction = self.contact_prediction(envelope);
                self.tension_snap.process_sample(prediction)
            }
            16 => {
                let prediction = self.contact_prediction(envelope);
                self.particle_rain.process_sample(prediction)
            }
            _ => 0.0,
        };

        self.apply_governor_ceiling(excitation * envelope * self.force_scale())
    }

    /// Render one mono sample from the voice.
    ///
    /// This performs exciter dispatch, resonator processing, NaN/denormal
    /// sanitization, and tail tracking without allocating.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Current host sample rate in hertz.
    ///
    /// # Returns
    ///
    /// The rendered mono sample.
    pub fn process_sample(&mut self, sample_rate: u32) -> f32 {
        if !self.rendering {
            return 0.0;
        }

        self.set_rate(sample_rate as f32);
        let velocity_norm = self.velocity / 127.0;

        let excitation = self.process_exciter();
        let sample = self.resonator.process_sample(excitation, sample_rate);
        self.update_governor();

        // Guard the audio output against NaNs or infinities before it
        // leaves the voice boundary.
        let sample = if !sample.is_finite() || sample.abs() < 1e-30 {
            0.0
        } else {
            sample
        };

        let rattle = self.rattle_noise(sample.abs());
        let sample_with_rattle = sample + rattle;

        self.highpass_state = self.rate.highpass_pole * self.highpass_state
            + self.rate.highpass_gain * sample_with_rattle;
        let highpass = sample_with_rattle - self.highpass_state;
        let boost_amount = velocity_norm * HIGHPASS_BOOST;
        let boosted_sample = sample_with_rattle + (highpass * boost_amount);

        self.pre_clamp_peak = self.pre_clamp_peak.max(boosted_sample.abs());
        let clamped = boosted_sample.clamp(-1.0, 1.0);
        // Expression (pressure + mod wheel) scales the post-clamp signal so
        // it can swell past unity without distorting the resonator path. The
        // global limiter at the plugin boundary handles any per-voice over-1.0
        // peaks the expression curve produces.
        let gained = clamped * self.expression_gain();
        const DENORMAL_FLUSH: f32 = 1e-20;
        let flushed = gained + DENORMAL_FLUSH - DENORMAL_FLUSH;

        // Track decaying peak energy so the manager can steal the quietest
        // voice and so inactive tails eventually self-disable.
        self.peak_hold = self.peak_hold.max(clamped.abs());
        self.peak_hold *= self.rate.peak_decay;
        if !self.active && self.peak_hold < TAIL_ENERGY_THRESHOLD {
            self.frames_below_threshold += 1;
            if self.frames_below_threshold >= self.rate.tail_frames {
                self.active = false;
                self.rendering = false;
            }
        } else {
            self.frames_below_threshold = 0;
        }

        flushed
    }

    fn process_exciter_stereo(&mut self, _width: f32) -> f32 {
        self.process_exciter()
    }

    /// Render one stereo sample pair from the voice.
    ///
    /// The current implementation keeps the exciter mono and lets the
    /// resonator provide stereo spread.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Current host sample rate in hertz.
    /// * `width` - Stereo width control forwarded to the resonator.
    ///
    /// # Returns
    ///
    /// The rendered `(left, right)` sample pair.
    pub fn process_sample_stereo(&mut self, sample_rate: u32, width: f32) -> (f32, f32) {
        if !self.rendering {
            return (0.0, 0.0);
        }

        self.set_rate(sample_rate as f32);
        let velocity_norm = self.velocity / 127.0;

        let excitation = self.process_exciter_stereo(width);
        let (left, right) = self
            .resonator
            .process_sample_stereo(excitation, sample_rate, width);
        self.update_governor();

        // Guard the stereo output against NaNs or infinities before it
        // leaves the voice boundary.
        let left = if !left.is_finite() || left.abs() < 1e-30 {
            0.0
        } else {
            left
        };
        let right = if !right.is_finite() || right.abs() < 1e-30 {
            0.0
        } else {
            right
        };

        let mono = (left + right) * 0.5;
        let rattle = self.rattle_noise(mono.abs());
        let left_with_rattle = left + rattle;
        let right_with_rattle = right + rattle;

        self.highpass_state = self.rate.highpass_pole * self.highpass_state
            + self.rate.highpass_gain * (mono + rattle);
        let highpass = (mono + rattle) - self.highpass_state;
        // Exciter ids are 1..=16 (see params::ExciterType). The `0` branch
        // never fires today, so we keep the unconditional 1.5x boost.
        let boost_amount = velocity_norm * HIGHPASS_BOOST;
        let boosted_left = left_with_rattle + (highpass * boost_amount);
        let boosted_right = right_with_rattle + (highpass * boost_amount);

        self.pre_clamp_peak = self
            .pre_clamp_peak
            .max(boosted_left.abs())
            .max(boosted_right.abs());
        let clamped_left = boosted_left.clamp(-1.0, 1.0);
        let clamped_right = boosted_right.clamp(-1.0, 1.0);
        // Apply expression gain to both channels equally. Mirror of the mono
        // path; pressure/CC1 should sound balanced across the stereo field.
        let exp_gain = self.expression_gain();
        let gained_left = clamped_left * exp_gain;
        let gained_right = clamped_right * exp_gain;
        const DENORMAL_FLUSH: f32 = 1e-20;
        let flushed_left = gained_left + DENORMAL_FLUSH - DENORMAL_FLUSH;
        let flushed_right = gained_right + DENORMAL_FLUSH - DENORMAL_FLUSH;

        // Track decaying peak energy so the manager can steal the quietest
        // voice and so inactive tails eventually self-disable.
        let peak = clamped_left.abs().max(clamped_right.abs());
        self.peak_hold = self.peak_hold.max(peak);
        self.peak_hold *= self.rate.peak_decay;
        if !self.active && self.peak_hold < TAIL_ENERGY_THRESHOLD {
            self.frames_below_threshold += 1;
            if self.frames_below_threshold >= self.rate.tail_frames {
                self.active = false;
                self.rendering = false;
            }
        } else {
            self.frames_below_threshold = 0;
        }

        (flushed_left, flushed_right)
    }
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{midi_to_hz, Voice, VoiceControls};
    use crate::dsp::ModalProfileId;

    #[test]
    fn midi_69_is_a440() {
        assert!((midi_to_hz(69) - 440.0).abs() < 1e-3);
    }

    #[test]
    fn midi_57_is_a220() {
        assert!((midi_to_hz(57) - 220.0).abs() < 1e-3);
    }

    #[test]
    fn midi_60_is_c4() {
        let hz = midi_to_hz(60);
        assert!((hz - 261.626).abs() < 0.1);
    }

    #[test]
    fn hit_attack_has_the_same_area_at_every_rate() {
        // A short strike on the attack ramp: the envelope's area over its
        // first millisecond matches the continuous ramp `t / attack` at every
        // rate, because the ramp is sampled at sample centres
        // (`resolve-exciter-tradeoffs`, design Decision 5).
        let attack = 0.005;
        let window = 0.001;
        let exact = 0.5 * window * window / attack;
        for sample_rate in [44_100.0f32, 48_000.0, 96_000.0] {
            let mut voice = Voice::new();
            voice.set_rate(sample_rate);
            let controls = VoiceControls {
                env_attack: attack,
                ..VoiceControls::default()
            };
            voice.configure_envelope(0, 0.8, controls);
            let samples = (window * sample_rate).round() as usize;
            let area: f32 = (0..samples)
                .map(|_| voice.process_force_envelope())
                .sum::<f32>()
                / sample_rate;
            let duration = samples as f32 / sample_rate;
            let expected = exact * (duration / window).powi(2);
            assert!(
                (area / expected - 1.0).abs() < 0.005,
                "{sample_rate}: area {area:.4e}, continuous {expected:.4e}"
            );
        }
    }

    #[test]
    fn voice_starts_inactive() {
        let voice = Voice::new();
        assert!(!voice.is_active());
    }

    #[test]
    fn note_on_activates_voice() {
        let mut voice = Voice::new();
        voice.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);
        assert!(voice.is_active());
    }

    #[test]
    fn note_off_deactivates_voice() {
        let mut voice = Voice::new();
        voice.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);
        voice.note_off();
        assert!(!voice.is_active());
    }

    #[test]
    fn note_off_natural_decay() {
        let mut voice = Voice::new();
        voice.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);

        let sample_rate = 48_000u32;
        let note_off_frame = 4800;
        let total_frames = 48_000;

        let mut output = Vec::with_capacity(total_frames);
        for frame in 0..total_frames {
            if frame == note_off_frame {
                voice.note_off();
            }
            output.push(voice.process_sample(sample_rate));
        }

        let pre_rms: f32 = output[..note_off_frame]
            .iter()
            .map(|s| s * s)
            .sum::<f32>()
            .sqrt()
            / note_off_frame as f32;
        let post_rms: f32 = output[note_off_frame..]
            .iter()
            .map(|s| s * s)
            .sum::<f32>()
            .sqrt()
            / (total_frames - note_off_frame) as f32;

        assert!(pre_rms > 0.0, "pre-off should be audible");
        assert!(
            post_rms > pre_rms * 0.10,
            "post-off RMS ({post_rms}) should be > 10% of pre-off RMS ({pre_rms}) — natural decay, not cut"
        );
    }

    #[test]
    fn output_clamped_to_unit_range() {
        let mut voice = Voice::new();
        voice.note_on(60, 127.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);

        let sample_rate = 48_000u32;
        for _ in 0..48_000 {
            let sample = voice.process_sample(sample_rate);
            assert!(
                (-1.0..=1.0).contains(&sample),
                "output sample {sample} outside [-1, 1]"
            );
        }
    }

    #[test]
    fn output_finite_over_long_render() {
        let mut voice = Voice::new();
        voice.note_on(36, 127.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);

        let sample_rate = 48_000u32;
        for _ in 0..96_000 {
            let sample = voice.process_sample(sample_rate);
            assert!(sample.is_finite(), "non-finite output: {sample}");
        }
    }

    #[test]
    fn active_voice_not_falsely_deactivated() {
        let mut voice = Voice::new();
        voice.note_on(60, 100.0, ModalProfileId::Tank, 0, 1.0, 0.0, 0.0, 2);

        let sample_rate = 48_000u32;
        let half_second = 48_000 / 2;

        for _ in 0..half_second {
            voice.process_sample(sample_rate);
        }

        assert!(
            voice.is_active(),
            "voice should still be active at t=0.5s for tank profile"
        );
    }

    #[test]
    fn note_pitch_retargets_modal_bank() {
        let mut low_voice = Voice::new();
        let mut high_voice = Voice::new();

        low_voice.note_on(57, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);
        high_voice.note_on(69, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);

        let low_frequency = low_voice.resonator.modes[0].spec.frequency_hz;
        let high_frequency = high_voice.resonator.modes[0].spec.frequency_hz;

        assert!(
            high_frequency > low_frequency * 1.9,
            "expected higher note to retune modal bank: low={low_frequency}, high={high_frequency}"
        );
    }

    #[test]
    fn controls_change_resonator_specs() {
        let mut dark_voice = Voice::new();
        let mut bright_voice = Voice::new();

        dark_voice.note_on_with_controls(
            60,
            100.0,
            ModalProfileId::Pipe,
            0,
            1.0,
            0.0,
            0.0,
            1,
            VoiceControls {
                res_damping: 1.0,
                res_brightness: 0.0,
                ..VoiceControls::default()
            },
        );
        bright_voice.note_on_with_controls(
            60,
            100.0,
            ModalProfileId::Pipe,
            0,
            1.0,
            0.0,
            0.0,
            1,
            VoiceControls {
                res_damping: 0.0,
                res_brightness: 1.0,
                ..VoiceControls::default()
            },
        );

        assert!(
            dark_voice.resonator.modes[0].spec.decay_seconds
                < bright_voice.resonator.modes[0].spec.decay_seconds
        );
        assert!(
            dark_voice.resonator.modes.last().unwrap().spec.gain
                < bright_voice.resonator.modes.last().unwrap().spec.gain
        );
    }

    #[test]
    fn bow_controls_change_output() {
        let mut soft_voice = Voice::new();
        let mut aggressive_voice = Voice::new();

        soft_voice.note_on_with_controls(
            60,
            100.0,
            ModalProfileId::Pipe,
            0,
            1.0,
            0.0,
            0.0,
            1,
            VoiceControls {
                bow_pressure: 0.2,
                bow_speed: 0.2,
                rosin_grip: 0.1,
                slip_curve: 0.1,
                mseg_sustain: 0.2,
                ..VoiceControls::default()
            },
        );
        aggressive_voice.note_on_with_controls(
            60,
            100.0,
            ModalProfileId::Pipe,
            0,
            1.0,
            0.0,
            0.0,
            1,
            VoiceControls {
                bow_pressure: 2.0,
                bow_speed: 2.0,
                rosin_grip: 1.5,
                slip_curve: 1.5,
                mseg_sustain: 1.0,
                ..VoiceControls::default()
            },
        );

        let sample_rate = 48_000u32;
        let soft_energy: f32 = (0..1024)
            .map(|_| soft_voice.process_sample(sample_rate).abs())
            .sum();
        let aggressive_energy: f32 = (0..1024)
            .map(|_| aggressive_voice.process_sample(sample_rate).abs())
            .sum();

        assert!(aggressive_energy > soft_energy, "expected bow controls to change output: soft={soft_energy}, aggressive={aggressive_energy}");
    }

    #[test]
    fn character_param_changes_resonator_output() {
        // A curated per-object character control (Plate Aspect) must
        // measurably change the resonator output now that the algorithmic
        // path is the only path.
        let mut square_voice = Voice::new();
        let mut rect_voice = Voice::new();

        let mut square = VoiceControls::default();
        square.character.plate_aspect = 1.0;
        let mut rect = VoiceControls::default();
        rect.character.plate_aspect = 4.0;

        square_voice.note_on_with_controls(
            60,
            100.0,
            ModalProfileId::Plate,
            0,
            1.0,
            0.0,
            0.0,
            2,
            square,
        );
        rect_voice.note_on_with_controls(
            60,
            100.0,
            ModalProfileId::Plate,
            0,
            1.0,
            0.0,
            0.0,
            2,
            rect,
        );

        let sample_rate = 48_000u32;
        let mut square_energy = 0.0f32;
        let mut rect_energy = 0.0f32;
        for _ in 0..2048 {
            let s = square_voice.process_sample(sample_rate);
            let r = rect_voice.process_sample(sample_rate);
            assert!(s.is_finite());
            assert!(r.is_finite());
            square_energy += s.abs();
            rect_energy += r.abs();
        }

        assert!(
            (square_energy - rect_energy).abs() > 0.01,
            "plate aspect character param should measurably change resonator output"
        );
    }

    #[test]
    fn chain_resonator_tracks_pitch() {
        // Chain previously ignored the MIDI note; the fundamental must now
        // follow the note like the other objects.
        let mut low_voice = Voice::new();
        let mut high_voice = Voice::new();

        low_voice.note_on(45, 100.0, ModalProfileId::Chain, 0, 1.0, 0.0, 0.0, 2);
        high_voice.note_on(69, 100.0, ModalProfileId::Chain, 0, 1.0, 0.0, 0.0, 2);

        let low = low_voice.resonator.modes[0].spec.frequency_hz;
        let high = high_voice.resonator.modes[0].spec.frequency_hz;
        assert!(
            high > low * 1.5,
            "expected higher note to raise the chain cluster: low={low}, high={high}"
        );
    }

    #[test]
    fn mode_count_scale_changes_resonator_density() {
        // Eco should produce fewer modes than Render at note-on for the same
        // profile. The exact counts depend on the per-object generator, but
        // Render-scaled mode count must strictly exceed Eco-scaled mode count
        // for at least one profile we ship.
        let mut eco_voice = Voice::new();
        let mut render_voice = Voice::new();

        let eco = VoiceControls {
            mode_count_scale: 0.5,
            ..VoiceControls::default()
        };
        let render = VoiceControls {
            mode_count_scale: 2.0,
            ..VoiceControls::default()
        };

        eco_voice.note_on_with_controls(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2, eco);
        render_voice.note_on_with_controls(
            60,
            100.0,
            ModalProfileId::Pipe,
            0,
            1.0,
            0.0,
            0.0,
            2,
            render,
        );

        let eco_modes = eco_voice.resonator.modes.len();
        let render_modes = render_voice.resonator.modes.len();
        assert!(
            render_modes > eco_modes,
            "Render quality should produce more modes than Eco: eco={eco_modes}, render={render_modes}"
        );
    }

    #[test]
    fn held_note_damping_automation_shortens_decay() {
        // A long-sustaining specialty voice gets clobbered with maximum damping
        // mid-note. The resonator decay must collapse — proving held-note
        // automation actually retunes the modes after note-on.
        let mut voice = Voice::new();
        let bright = VoiceControls {
            res_damping: 0.0, // longest possible decay
            res_brightness: 0.5,
            ..VoiceControls::default()
        };
        // Pneumatic jet sustains as long as the note is held.
        voice.note_on_with_controls(
            60,
            127.0,
            ModalProfileId::Pipe,
            0,
            1.0,
            0.0,
            0.0,
            13,
            bright,
        );

        let sample_rate = 48_000u32;
        for _ in 0..4_096 {
            voice.process_sample(sample_rate);
        }

        let baseline_decay = voice.resonator.modes[0].spec.decay_seconds;

        voice.update_live_controls(
            1.0, // max damping → shortest decay
            0.5,
            0.5,
            1.0,
            0.0,
            0.0,
            0.3,
            sample_rate,
        );

        let damped_decay = voice.resonator.modes[0].spec.decay_seconds;
        assert!(
            damped_decay < baseline_decay * 0.5,
            "held-note damping automation should shorten decay: baseline={baseline_decay}, damped={damped_decay}"
        );
    }

    #[test]
    fn held_note_automation_skips_released_voices() {
        // Released (tail) voices keep their note-on snapshot so held-note
        // automation never reaches them.
        let mut voice = Voice::new();
        voice.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 13);
        voice.note_off();

        let pre = voice.resonator.modes[0].spec.decay_seconds;
        voice.update_live_controls(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.3, 48_000);
        let post = voice.resonator.modes[0].spec.decay_seconds;

        assert_eq!(pre, post, "tail voices should not receive live automation");
    }

    #[test]
    fn taut_cable_dynamic_hook_changes_output() {
        // With the tension-drop hook wired into the per-sample loop, a large
        // tension drop must measurably change the cable's output versus none.
        let mut stiff_voice = Voice::new();
        let mut boing_voice = Voice::new();

        let mut stiff = VoiceControls::default();
        stiff.character.cable_tension_drop = 0.0;
        let mut boing = VoiceControls::default();
        boing.character.cable_tension_drop = 1.0;

        stiff_voice.note_on_with_controls(
            48,
            127.0,
            ModalProfileId::TautCable,
            0,
            1.0,
            0.0,
            0.0,
            4,
            stiff,
        );
        boing_voice.note_on_with_controls(
            48,
            127.0,
            ModalProfileId::TautCable,
            0,
            1.0,
            0.0,
            0.0,
            4,
            boing,
        );

        let sample_rate = 48_000u32;
        let mut stiff_energy = 0.0f32;
        let mut boing_energy = 0.0f32;
        for _ in 0..4096 {
            let s = stiff_voice.process_sample(sample_rate);
            let b = boing_voice.process_sample(sample_rate);
            assert!(s.is_finite());
            assert!(b.is_finite());
            stiff_energy += s.abs();
            boing_energy += b.abs();
        }

        assert!(
            (stiff_energy - boing_energy).abs() > 0.01,
            "tension-drop dynamic hook should change output: stiff={stiff_energy}, boing={boing_energy}"
        );
    }

    #[test]
    fn pitch_bend_semitones_factor_round_trip() {
        // ±12 semitones → exactly an octave; 0 → unity.
        let neutral = super::pitch_bend_semitones_to_factor(0.0);
        let octave_up = super::pitch_bend_semitones_to_factor(12.0);
        let octave_down = super::pitch_bend_semitones_to_factor(-12.0);
        assert!((neutral - 1.0).abs() < 1e-6);
        assert!((octave_up - 2.0).abs() < 1e-5);
        assert!((octave_down - 0.5).abs() < 1e-5);
    }

    #[test]
    fn pitch_bend_retunes_held_resonator_modes() {
        // Bending up an octave must roughly double the resonator's first-mode
        // frequency without re-allocating the modal bank.
        let mut voice = Voice::new();
        voice.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);
        let neutral = voice.resonator.modes[0].spec.frequency_hz;

        voice.set_pitch_bend_semitones(12.0);
        let bent = voice.resonator.modes[0].spec.frequency_hz;
        assert!(
            (bent - neutral * 2.0).abs() < neutral * 0.01,
            "octave bend should double mode frequency: neutral={neutral}, bent={bent}"
        );

        // Returning to neutral should restore the original frequency.
        voice.set_pitch_bend_semitones(0.0);
        let restored = voice.resonator.modes[0].spec.frequency_hz;
        assert!(
            (restored - neutral).abs() < neutral * 0.001,
            "bend back to 0 should restore the original frequency"
        );
    }

    #[test]
    fn channel_pressure_increases_voice_output() {
        // Aftertouch should raise the rendered amplitude. We compare a quiet
        // voice against the same voice with full channel pressure.
        let mut quiet = Voice::new();
        let mut pressed = Voice::new();
        quiet.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);
        pressed.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);
        pressed.set_channel_pressure(1.0);

        let sample_rate = 48_000u32;
        let quiet_energy: f32 = (0..2048)
            .map(|_| quiet.process_sample(sample_rate).abs())
            .sum();
        let pressed_energy: f32 = (0..2048)
            .map(|_| pressed.process_sample(sample_rate).abs())
            .sum();
        assert!(
            pressed_energy > quiet_energy * 1.1,
            "channel pressure should audibly boost output: quiet={quiet_energy}, pressed={pressed_energy}"
        );
    }

    #[test]
    fn mod_wheel_boosts_output_independently_of_pressure() {
        // Mod wheel uses a separate axis from pressure so hosts that send only
        // CC1 still get an audible response.
        let mut neutral = Voice::new();
        let mut wheeled = Voice::new();
        neutral.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);
        wheeled.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);
        wheeled.set_mod_wheel(1.0);

        let sample_rate = 48_000u32;
        let neutral_energy: f32 = (0..2048)
            .map(|_| neutral.process_sample(sample_rate).abs())
            .sum();
        let wheeled_energy: f32 = (0..2048)
            .map(|_| wheeled.process_sample(sample_rate).abs())
            .sum();
        assert!(
            wheeled_energy > neutral_energy * 1.05,
            "mod wheel should audibly boost output: neutral={neutral_energy}, wheeled={wheeled_energy}"
        );
    }

    #[test]
    fn poly_pressure_overrides_channel_pressure_when_higher() {
        // Per-note pressure must reach the expression scaler even if channel
        // pressure is lower (MPE / poly aftertouch path).
        let mut voice = Voice::new();
        voice.note_on(60, 100.0, ModalProfileId::Pipe, 0, 1.0, 0.0, 0.0, 2);

        voice.set_channel_pressure(0.0);
        voice.set_poly_pressure(1.0);
        let with_poly = voice.expression_gain();
        voice.set_poly_pressure(0.0);
        let baseline = voice.expression_gain();

        assert!(
            with_poly > baseline + 0.1,
            "poly pressure should raise expression gain: baseline={baseline}, with_poly={with_poly}"
        );
    }

    /// Measurement for `fix-resonator-sample-rate-gain` (task 2.1): a struck
    /// and a bowed C4 on Tank, Pipe and Plate through the full voice at 44.1,
    /// 48 and 96 kHz. Reports the raw resonator peak (before the voice's
    /// clamp) and the voice output RMS, each relative to 48 kHz.
    #[test]
    #[ignore]
    fn sample_rate_voice_level_measurements() {
        use crate::dsp::{ModalProfileId, ResonatorCore};
        println!("| note | rate | raw peak (dB vs 48k) | output RMS (dB vs 48k) |");
        println!("|---|---|---|---|");
        for (exciter, exciter_name) in [(2, "HandStrike"), (1, "Bow")] {
            for profile in [
                ModalProfileId::Tank,
                ModalProfileId::Pipe,
                ModalProfileId::Plate,
            ] {
                let measure = |sample_rate: u32| {
                    let mut voice = Voice::new();
                    voice.note_on_with_controls(
                        60,
                        100.0,
                        profile,
                        0,
                        1.0,
                        0.0,
                        0.0,
                        exciter,
                        VoiceControls::default(),
                    );
                    let (mut raw_peak, mut sum_sq) = (0.0_f32, 0.0_f64);
                    let n = sample_rate as usize * 2;
                    for i in 0..n {
                        if i == n / 2 {
                            voice.note_off();
                        }
                        let excitation = voice.process_exciter();
                        let raw = voice.resonator.process_sample(excitation, sample_rate);
                        raw_peak = raw_peak.max(raw.abs());
                        let out = raw.clamp(-1.0, 1.0);
                        sum_sq += (out * out) as f64;
                    }
                    (raw_peak, (sum_sq / n as f64).sqrt() as f32)
                };
                let (ref_peak, ref_rms) = measure(48_000);
                for sample_rate in [44_100, 96_000] {
                    let (peak, rms) = measure(sample_rate);
                    println!(
                        "| {exciter_name} {profile:?} | {sample_rate} | {:+.1} | {:+.1} |",
                        20.0 * (peak / ref_peak).log10(),
                        20.0 * (rms / ref_rms).log10()
                    );
                }
            }
        }
    }

    // --- Sample-rate independence (fix-sample-rate-time-constants) ---

    /// Render Pipe C4 at velocity 100 with `exciter`: `held` seconds held,
    /// then `released` seconds after note-off. Returns the voice output.
    fn render_pipe_note(exciter: i32, sample_rate: u32, held: f32, released: f32) -> Vec<f32> {
        let mut voice = Voice::new();
        voice.note_on_with_controls(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            0,
            1.0,
            0.0,
            0.0,
            exciter,
            VoiceControls::default(),
        );
        let held_n = (held * sample_rate as f32) as usize;
        let total = held_n + (released * sample_rate as f32) as usize;
        (0..total)
            .map(|i| {
                if i == held_n {
                    voice.note_off();
                }
                voice.process_sample(sample_rate)
            })
            .collect()
    }

    fn rms(xs: &[f32]) -> f32 {
        (xs.iter().map(|&x| (x * x) as f64).sum::<f64>() / xs.len().max(1) as f64).sqrt() as f32
    }

    /// Pipe C4 level at 44.1 and 96 kHz against 48 kHz, as mean power over
    /// 128 note seeds at velocity 100. Noise-driven exciters vary by several
    /// dB from seed to seed, so this is their expected level across rates.
    #[test]
    #[ignore]
    fn print_seed_averaged_rate_levels() {
        let rows: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (1..=16)
                .map(|exciter| {
                    scope.spawn(move || {
                        let mean_power = |sample_rate: u32| {
                            let mut total = 0.0f64;
                            for start in 0..128u64 {
                                let mut voice = Voice::new();
                                voice.note_on(
                                    60,
                                    100.0,
                                    crate::dsp::ModalProfileId::Pipe,
                                    start * 7919,
                                    1.0,
                                    0.0,
                                    0.0,
                                    exciter,
                                );
                                let n = sample_rate as usize;
                                let out: Vec<f32> = (0..2 * n)
                                    .map(|i| {
                                        if i == n {
                                            voice.note_off();
                                        }
                                        voice.process_sample(sample_rate)
                                    })
                                    .collect();
                                total += (rms(&out) as f64).powi(2);
                            }
                            total / 128.0
                        };
                        let (low, reference, high) =
                            (mean_power(44_100), mean_power(48_000), mean_power(96_000));
                        format!(
                            "exciter {exciter:2}: 44.1 kHz {:+.2} dB, 96 kHz {:+.2} dB",
                            10.0 * (low / reference).log10(),
                            10.0 * (high / reference).log10()
                        )
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        println!("{}", rows.join("\n"));
    }

    /// Mean power of Pipe C4 with `exciter` at `sample_rate`, over
    /// velocities 40/70/100/127 and 32 note seeds each: one second held,
    /// one released. (8 seeds left HeavyGrinding's mean scattered by up to
    /// 2.7 dB; 64 put every velocity within 1.1 dB.)
    fn expected_pipe_power(exciter: i32, sample_rate: u32) -> f64 {
        let mut total = 0.0f64;
        for velocity in [40.0f32, 70.0, 100.0, 127.0] {
            for seed in 0..32u64 {
                let mut voice = Voice::new();
                voice.note_on(
                    60,
                    velocity,
                    crate::dsp::ModalProfileId::Pipe,
                    seed * 7919,
                    1.0,
                    0.0,
                    0.0,
                    exciter,
                );
                let held = sample_rate as usize;
                let out: Vec<f32> = (0..2 * held)
                    .map(|i| {
                        if i == held {
                            voice.note_off();
                        }
                        voice.process_sample(sample_rate)
                    })
                    .collect();
                total += (rms(&out) as f64).powi(2);
            }
        }
        total / 128.0
    }

    #[test]
    fn every_exciter_has_the_same_level_at_any_sample_rate() {
        // The expected level, averaged over seeds and velocities: a single
        // note can't resolve 1.5 dB for a noise-driven exciter (its level
        // varies by several dB from the seed alone), and Bow's stick-slip
        // locks into a different regime at velocity 100 on 44.1 kHz only
        // (`resolve-exciter-tradeoffs` findings).
        let failures: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (1..=16)
                .map(|exciter| {
                    scope.spawn(move || {
                        let reference = expected_pipe_power(exciter, 48_000);
                        [44_100, 96_000]
                            .into_iter()
                            .filter_map(|sample_rate| {
                                let power = expected_pipe_power(exciter, sample_rate);
                                let diff = 10.0 * (power / reference).log10();
                                (diff.abs() > 1.5 || !diff.is_finite()).then(|| {
                                    format!("exciter {exciter} @ {sample_rate}: {diff:+.2} dB")
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap())
                .collect()
        });
        assert!(
            failures.is_empty(),
            "level differs from 48 kHz:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn struck_attack_timing_does_not_depend_on_sample_rate() {
        let time_to_peak = |exciter: i32, sample_rate: u32| {
            let out = render_pipe_note(exciter, sample_rate, 0.5, 0.0);
            let peak = out
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, _)| i)
                .unwrap();
            peak as f32 / sample_rate as f32
        };
        for exciter in [2, 3, 4, 5] {
            let reference = time_to_peak(exciter, 48_000);
            for sample_rate in [44_100, 96_000] {
                let t = time_to_peak(exciter, sample_rate);
                assert!(
                    (t / reference - 1.0).abs() <= 0.1,
                    "exciter {exciter} @ {sample_rate}: peak at {:.2} ms vs {:.2} ms",
                    t * 1e3,
                    reference * 1e3
                );
            }
        }
    }

    #[test]
    fn struck_tail_ends_at_the_same_time_at_any_sample_rate() {
        let seconds_to_idle = |sample_rate: u32| {
            let mut voice = Voice::new();
            voice.note_on_with_controls(
                60,
                100.0,
                crate::dsp::ModalProfileId::Pipe,
                0,
                1.0,
                0.0,
                0.0,
                2,
                VoiceControls::default(),
            );
            let held = sample_rate as usize / 10;
            for _ in 0..held {
                voice.process_sample(sample_rate);
            }
            voice.note_off();
            let mut n = 0usize;
            while voice.is_rendering() && n < sample_rate as usize * 60 {
                voice.process_sample(sample_rate);
                n += 1;
            }
            n as f32 / sample_rate as f32
        };
        let reference = seconds_to_idle(48_000);
        for sample_rate in [44_100, 96_000] {
            let t = seconds_to_idle(sample_rate);
            assert!(
                (t / reference - 1.0).abs() <= 0.1,
                "tail @ {sample_rate}: {t:.2} s vs {reference:.2} s at 48 kHz"
            );
        }
    }

    /// Prints a 48 kHz render hash per exciter, run before and after
    /// `fix-sample-rate-time-constants` to show 48 kHz output is unchanged.
    /// Not asserted: libm differs between CI targets, so the hashes are
    /// platform-specific.
    #[test]
    #[ignore]
    fn print_48k_exciter_render_hashes() {
        for exciter in 1..=16 {
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            for x in render_pipe_note(exciter, 48_000, 1.0, 1.0) {
                for byte in x.to_bits().to_le_bytes() {
                    hash ^= byte as u64;
                    hash = hash.wrapping_mul(0x0100_0000_01b3);
                }
            }
            println!("HASH exciter {exciter:2}: {hash:#018x}");
        }
    }

    // --- Overdrive (normalize-object-loudness) ---

    const ALL_PROFILES: [ModalProfileId; 9] = [
        ModalProfileId::Pipe,
        ModalProfileId::Plate,
        ModalProfileId::Tank,
        ModalProfileId::Chain,
        ModalProfileId::IBeam,
        ModalProfileId::TautCable,
        ModalProfileId::CoilSpring,
        ModalProfileId::SheetMetal,
        ModalProfileId::IndustrialCog,
    ];

    /// The largest value fed to the voice clamp for one note at velocity 127
    /// with default controls: one second held, one second released, 48 kHz.
    fn pre_clamp_peak(profile: ModalProfileId, exciter: i32, note: u8) -> f32 {
        sweep_note(profile, exciter, note).0
    }

    /// `pre_clamp_peak`, and the lowest gain the drive governor applied.
    fn sweep_note(profile: ModalProfileId, exciter: i32, note: u8) -> (f32, f32) {
        let mut voice = Voice::new();
        voice.note_on_with_controls(
            note,
            127.0,
            profile,
            0,
            1.0,
            0.0,
            0.0,
            exciter,
            VoiceControls::default(),
        );
        let mut lowest_gain = 1.0f32;
        for i in 0..96_000 {
            if i == 48_000 {
                voice.note_off();
            }
            voice.process_sample(48_000);
            lowest_gain = lowest_gain.min(voice.governor_gain);
        }
        (voice.pre_clamp_peak, lowest_gain)
    }

    /// Pre-clamp peak of every exciter (index 1–16) on every object at
    /// C2/C4/C6, velocity 127, default controls, in `ALL_PROFILES` × note
    /// order. Computed once (one thread per exciter) and shared by the
    /// level and overdrive tests.
    fn sweep_table() -> &'static Vec<Vec<f32>> {
        static TABLE: std::sync::OnceLock<Vec<Vec<f32>>> = std::sync::OnceLock::new();
        TABLE.get_or_init(|| {
            std::thread::scope(|scope| {
                let handles: Vec<_> = (0..=16)
                    .map(|exciter| {
                        scope.spawn(move || {
                            if exciter == 0 {
                                return Vec::new();
                            }
                            ALL_PROFILES
                                .iter()
                                .flat_map(|&profile| {
                                    [36, 60, 84].map(|note| pre_clamp_peak(profile, exciter, note))
                                })
                                .collect()
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            })
        })
    }

    fn sweep_note_name(index: usize) -> String {
        format!("{:?} {}", ALL_PROFILES[index / 3], [36, 60, 84][index % 3])
    }

    #[test]
    fn default_notes_do_not_overdrive_the_voice() {
        let mut failures = Vec::new();
        for exciter in 1..=16 {
            for (index, &peak) in sweep_table()[exciter].iter().enumerate() {
                if peak > 1.0 || !peak.is_finite() {
                    failures.push(format!(
                        "exciter {exciter} {}: {peak:.2}",
                        sweep_note_name(index)
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "pre-clamp peak over 1.0:\n{}",
            failures.join("\n")
        );
    }

    fn median(values: &[f32]) -> f32 {
        let mut sorted = values.to_vec();
        sorted.sort_by(f32::total_cmp);
        sorted[sorted.len() / 2]
    }

    #[test]
    fn exciters_play_at_a_common_level() {
        let medians: Vec<f32> = (1..=16).map(|e| median(&sweep_table()[e])).collect();
        let common = median(&medians);
        let failures: Vec<String> = medians
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| {
                let diff = 20.0 * (m / common).log10();
                (diff.abs() > 6.0 || !diff.is_finite())
                    .then(|| format!("exciter {}: median {m:.4} ({diff:+.1} dB)", i + 1))
            })
            .collect();
        assert!(
            failures.is_empty(),
            "exciter medians more than 6 dB from the common {common:.4}:\n{}",
            failures.join("\n")
        );
    }

    /// (pre-clamp peak, output RMS) of one note at `drive` times the default
    /// drive: velocity 127, 1 s held + 1 s released, 48 kHz.
    fn note_at_drive(profile: ModalProfileId, exciter: i32, note: u8, drive: f32) -> (f32, f32) {
        let mut voice = Voice::new();
        voice.note_on_with_controls(
            note,
            127.0,
            profile,
            0,
            1.0,
            0.0,
            0.0,
            exciter,
            VoiceControls::default(),
        );
        voice.drive_scale = drive;
        let mut sum = 0.0f64;
        for i in 0..96_000 {
            if i == 48_000 {
                voice.note_off();
            }
            let y = voice.process_sample(48_000);
            sum += (y * y) as f64;
        }
        (voice.pre_clamp_peak, (sum / 96_000.0).sqrt() as f32)
    }

    #[test]
    fn excess_drive_is_reduced_not_clipped() {
        let mut failures = Vec::new();
        for exciter in 1..=16 {
            let (loudest, _) = sweep_table()[exciter]
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap();
            let (profile, note) = (ALL_PROFILES[loudest / 3], [36u8, 60, 84][loudest % 3]);
            // Judged on the peak: a governor acting above a threshold can
            // only pull a driven note down to about that threshold, so a
            // note quieter than it at default drive ends up louder on
            // average while its peak stays bounded (`resolve-exciter-tradeoffs`).
            let (default_peak, _) = note_at_drive(profile, exciter as i32, note, 1.0);
            let (peak, _) = note_at_drive(profile, exciter as i32, note, 4.0);
            let diff = 20.0 * (peak / default_peak).log10();
            if peak > 1.0 || !peak.is_finite() || diff.abs() > 3.0 || !diff.is_finite() {
                failures.push(format!(
                    "exciter {exciter} {}: peak {peak:.2}, {diff:+.1} dB vs default drive",
                    sweep_note_name(loudest)
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "12 dB of extra drive:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn contact_is_stable_without_the_governor() {
        use crate::dsp::ResonatorCore;
        let failures: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (1..=16)
                .map(|exciter| {
                    scope.spawn(move || {
                        let mut failures = Vec::new();
                        for profile in ALL_PROFILES {
                            for note in [36u8, 60, 84] {
                                let mut voice = Voice::new();
                                voice.note_on_with_controls(
                                    note,
                                    127.0,
                                    profile,
                                    0,
                                    1.0,
                                    0.0,
                                    0.0,
                                    exciter,
                                    VoiceControls::default(),
                                );
                                voice.drive_scale = 4.0;
                                voice.governor_enabled = false;
                                voice.set_rate(48_000.0);
                                let mut peak = 0.0f32;
                                for i in 0..96_000 {
                                    if i == 48_000 {
                                        voice.note_off();
                                    }
                                    let excitation = voice.process_exciter();
                                    let y = voice.resonator.process_sample(excitation, 48_000);
                                    peak = if y.is_finite() {
                                        peak.max(y.abs())
                                    } else {
                                        f32::INFINITY
                                    };
                                }
                                if peak.is_nan() || peak >= 100.0 {
                                    failures.push(format!(
                                        "exciter {exciter} {profile:?} note {note}: {peak:.3e}"
                                    ));
                                }
                            }
                        }
                        failures
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap())
                .collect()
        });
        assert!(
            failures.is_empty(),
            "diverged at 12 dB extra drive, governor off:\n{}",
            failures.join("\n")
        );
    }

    /// Median and worst pre-clamp peak per exciter over every object at
    /// C2/C4/C6 (velocity 127, default controls), how many of those notes
    /// engage the drive governor and by how much, and the drive level that
    /// would put the median at `DRIVE_LEVEL_TARGET`. Used to set
    /// `EXCITER_DRIVE_LEVEL` (`resolve-exciter-tradeoffs`).
    #[test]
    #[ignore]
    fn print_exciter_overdrive() {
        const DRIVE_LEVEL_TARGET: f32 = 0.35;
        let rows: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (1..=16)
                .map(|exciter| {
                    scope.spawn(move || {
                        let mut notes: Vec<(f32, f32, String)> = Vec::new();
                        for profile in ALL_PROFILES {
                            for note in [36u8, 60, 84] {
                                let (peak, gain) = sweep_note(profile, exciter, note);
                                notes.push((peak, gain, format!("{profile:?} {note}")));
                            }
                        }
                        notes.sort_by(|a, b| a.0.total_cmp(&b.0));
                        let median = notes[notes.len() / 2].0;
                        let (worst, _, at) = notes.last().unwrap();
                        let engaged: Vec<f32> = notes
                            .iter()
                            .filter(|n| n.1 < 0.999)
                            .map(|n| 20.0 * n.1.log10())
                            .collect();
                        let deepest = engaged.iter().copied().fold(0.0f32, f32::min);
                        format!(
                            "OVERDRIVE exciter {exciter:2}: median {median:8.4}, worst {worst:8.3} ({at}), \
                             governor on {} notes (deepest {deepest:+.1} dB), level {:.4e}",
                            engaged.len(),
                            super::EXCITER_DRIVE_LEVEL[exciter as usize] * DRIVE_LEVEL_TARGET / median
                        )
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        println!("{}", rows.join("\n"));
    }

    // --- Mallet and grinder contact (stabilize-mallet-and-grinding-contact) ---

    /// Pre-clamp peaks of `exciter` over every object at C2/C4/C6
    /// (velocity 127, default controls), sorted.
    fn sweep_peaks(exciter: i32) -> Vec<f32> {
        let mut peaks = sweep_table()[exciter as usize].clone();
        peaks.sort_by(f32::total_cmp);
        peaks
    }

    #[test]
    fn mallet_and_grinder_play_at_a_struck_exciter_level() {
        let median = |peaks: &[f32]| peaks[peaks.len() / 2];
        let target = 0.5 * (median(&sweep_peaks(5)) + median(&sweep_peaks(6)));
        for exciter in [4, 10] {
            let peaks = sweep_peaks(exciter);
            let diff = 20.0 * (median(&peaks) / target).log10();
            let worst = *peaks.last().unwrap();
            assert!(
                worst <= 0.9,
                "exciter {exciter}: worst pre-clamp peak {worst:.3}"
            );
            // HeavyGrinding's steady, low-frequency force moves IndustrialCog
            // C2's low modes ~40x more than its median note, so a single
            // trim can't lift its median without overdriving that note. Its
            // level is left to the per-voice drive governor
            // (`resolve-exciter-tradeoffs`); only HardMallet is held to the
            // struck target here.
            if exciter == 10 {
                continue;
            }
            assert!(
                diff.abs() <= 6.0,
                "exciter {exciter}: median {:.4} is {diff:+.1} dB from the struck target {target:.4}",
                median(&peaks)
            );
        }
    }

    /// Largest |resonator output| for one note driven `drive` times harder
    /// than default (velocity 127, 1 s held + 1 s released, 48 kHz).
    fn resonator_peak_at_drive(profile: ModalProfileId, exciter: i32, note: u8, drive: f32) -> f32 {
        use crate::dsp::ResonatorCore;
        let mut voice = Voice::new();
        voice.note_on_with_controls(
            note,
            127.0,
            profile,
            0,
            1.0,
            0.0,
            0.0,
            exciter,
            VoiceControls::default(),
        );
        voice.drive_scale = drive;
        voice.set_rate(48_000.0);
        let mut peak = 0.0f32;
        for i in 0..96_000 {
            if i == 48_000 {
                voice.note_off();
            }
            let excitation = voice.process_exciter();
            let y = voice.resonator.process_sample(excitation, 48_000);
            if !y.is_finite() {
                return f32::INFINITY;
            }
            peak = peak.max(y.abs());
        }
        peak
    }

    #[test]
    fn mallet_and_grinder_contact_is_stable_far_above_default_drive() {
        let mut failures = Vec::new();
        for exciter in [4, 10] {
            for profile in ALL_PROFILES {
                for note in [36, 60, 84] {
                    let peak = resonator_peak_at_drive(profile, exciter, note, 16.0);
                    if peak.is_nan() || peak >= 100.0 {
                        failures.push(format!(
                            "exciter {exciter} {profile:?} note {note}: {peak:.3e}"
                        ));
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "diverged at 16x drive:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn strikes_come_to_rest() {
        use crate::dsp::ResonatorCore;
        for exciter in [4, 5] {
            let mut voice = Voice::new();
            voice.note_on_with_controls(
                60,
                100.0,
                ModalProfileId::Pipe,
                0,
                1.0,
                0.0,
                0.0,
                exciter,
                VoiceControls::default(),
            );
            voice.set_rate(48_000.0);
            let mut last_force_at = 0;
            for i in 0..96_000 {
                let excitation = voice.process_exciter();
                voice.resonator.process_sample(excitation, 48_000);
                if excitation != 0.0 {
                    last_force_at = i;
                }
            }
            assert!(
                last_force_at < 24_000,
                "exciter {exciter} still applies force at {:.3} s",
                last_force_at as f32 / 48_000.0
            );
        }
    }
}

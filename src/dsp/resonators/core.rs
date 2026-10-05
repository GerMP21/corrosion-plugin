//! Second-order modal resonator core used by the voice layer.
//!
//! The voice module feeds excitation into these modes per sample, while the
//! interaction bus updates coupling and strike-position coefficients without
//! allocating on the audio thread.

use std::f32::consts::PI;

use crate::dsp::resonators::{
    ChainResonator, CoilSpringResonator, IBeamResonator, IndustrialCogResonator, PipeResonator,
    PlateResonator, ResonatorAlgorithm, SheetMetalResonator, TankResonator, TautCableResonator,
};

/// Curated per-object "character" controls exposed to the user.
///
/// Each field maps to one algorithmic-resonator generator parameter (see
/// `docs/backlog.md` → "Algorithmic resonator engine"). Only the field that
/// matches the active object is consulted at note-on; the rest are ignored.
/// Size/decay/brightness-like behavior stays covered by the global controls.
#[derive(Clone, Copy, Debug)]
pub struct CharacterParams {
    pub pipe_diameter: f32,
    pub plate_aspect: f32,
    pub plate_stiffness: f32,
    pub tank_volume: f32,
    pub tank_cavity_mix: f32,
    pub chain_link_mass: f32,
    pub chain_instability: f32,
    pub beam_shear: f32,
    pub cable_braid: f32,
    pub cable_tension_drop: f32,
    pub spring_dispersion: f32,
    pub spring_slosh: f32,
    pub sheet_thinness: f32,
    pub cog_dissonance: f32,
}

impl Default for CharacterParams {
    fn default() -> Self {
        Self {
            pipe_diameter: 0.5,
            plate_aspect: 1.0,
            plate_stiffness: 1.0,
            tank_volume: 0.5,
            tank_cavity_mix: 0.6,
            chain_link_mass: 0.5,
            chain_instability: 0.3,
            beam_shear: 0.5,
            cable_braid: 0.3,
            cable_tension_drop: 0.4,
            spring_dispersion: 0.5,
            spring_slosh: 0.3,
            sheet_thinness: 0.4,
            cog_dissonance: 0.1,
        }
    }
}

/// Per-sample dynamic behavior retained from the object's algorithm so the
/// resonator can modulate mode frequencies while it rings.
#[derive(Clone, Debug)]
enum Dynamics {
    None,
    /// Cable tension drop: amplitude raises pitch, which falls back as it decays.
    TautCable(TautCableResonator),
    /// Sheet-metal buckling: low-frequency displacement warps every mode.
    SheetMetal(SheetMetalResonator),
}

/// Every object profile, in `ModalProfileId` order.
const ALL_PROFILES: [crate::dsp::ModalProfileId; 9] = [
    crate::dsp::ModalProfileId::Pipe,
    crate::dsp::ModalProfileId::Plate,
    crate::dsp::ModalProfileId::Tank,
    crate::dsp::ModalProfileId::Chain,
    crate::dsp::ModalProfileId::IBeam,
    crate::dsp::ModalProfileId::TautCable,
    crate::dsp::ModalProfileId::CoilSpring,
    crate::dsp::ModalProfileId::SheetMetal,
    crate::dsp::ModalProfileId::IndustrialCog,
];

/// Largest base mode count across the object profile tables.
const MAX_BASE_MODE_COUNT: usize = {
    let mut max = 0;
    let mut i = 0;
    while i < ALL_PROFILES.len() {
        let count = crate::dsp::ModalProfile::from_id(ALL_PROFILES[i]).mode_count();
        if count > max {
            max = count;
        }
        i += 1;
    }
    max
};

/// Upper clamp on the quality-mode `mode_count_scale`. An integer, so
/// `MAX_MODES` below is exact: `round(base · scale) ≤ base · this`.
const MAX_MODE_COUNT_SCALE_INT: usize = 4;
const MAX_MODE_COUNT_SCALE: f32 = MAX_MODE_COUNT_SCALE_INT as f32;

/// Most raw (pre-damage) modes one note-on can generate.
const MAX_RAW_MODES: usize = MAX_BASE_MODE_COUNT * MAX_MODE_COUNT_SCALE_INT;

/// Worst-case number of modes in one note-on bank: the largest profile at the
/// `mode_count_scale` cap, doubled for damage companion modes. Voices reserve
/// this much up front so note-on never allocates (review C3).
pub const MAX_MODES: usize = MAX_RAW_MODES * 2;

#[derive(Clone, Copy)]
struct AlgorithmTransformControls {
    rust_amount: crate::dsp::RustAmount,
    damage_amount: crate::dsp::DamageAmount,
    thickness_amount: crate::dsp::ThicknessAmount,
    heat_amount: crate::dsp::HeatAmount,
    sludge_amount: crate::dsp::SludgeAmount,
}

#[derive(Clone, Copy, Debug)]
/// Coefficients for a damped second-order modal resonator.
pub struct ResonatorCoefficients {
    /// Input gain.
    pub b0: f32,
    /// Feedback coefficient for the first delay.
    pub a1: f32,
    /// Feedback coefficient for the second delay.
    pub a2: f32,
}

impl ResonatorCoefficients {
    /// Compute resonator coefficients for a modal specification.
    pub fn for_mode(mode: crate::dsp::ModalModeSpec, sample_rate: u32) -> Self {
        let safe_sample_rate = sample_rate.max(1) as f32;
        let decay_seconds = mode.decay_seconds.max(f32::EPSILON);
        let omega = 2.0 * PI * mode.frequency_hz / safe_sample_rate;
        let r = (-1.0 / (decay_seconds * safe_sample_rate)).exp();

        Self {
            b0: mode.gain,
            a1: -2.0 * r * omega.cos(),
            a2: r * r,
        }
    }
}

#[derive(Clone, Debug)]
/// Cached second-order state for one modal peak.
pub struct SecondOrderMode {
    pub(crate) spec: crate::dsp::ModalModeSpec,
    /// Note-on frequency, kept immutable so per-sample dynamic pitch effects
    /// (cable tension drop, sheet-metal warp) scale from a fixed base instead
    /// of compounding their own prior output.
    base_frequency_hz: f32,
    /// Note-on decay before damping/brightness are applied; lets held-note
    /// damping automation re-derive `spec.decay_seconds` instead of compounding.
    base_decay_seconds: f32,
    /// Note-on gain before damping/brightness are applied; mirror of
    /// `base_decay_seconds` for brightness automation.
    base_gain: f32,
    coefficients: ResonatorCoefficients,
    cached_sample_rate: Option<u32>,
    y1: f32,
    y2: f32,
}

impl SecondOrderMode {
    /// Create a modal state from a modal specification.
    pub fn new(spec: crate::dsp::ModalModeSpec) -> Self {
        Self {
            spec,
            base_frequency_hz: spec.frequency_hz,
            base_decay_seconds: spec.decay_seconds,
            base_gain: spec.gain,
            coefficients: ResonatorCoefficients::for_mode(spec, 48_000),
            cached_sample_rate: None,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Override the mode frequency and rebuild coefficients in place.
    ///
    /// Used by the per-sample dynamic hooks. Allocation-free.
    pub fn set_frequency(&mut self, frequency_hz: f32, sample_rate: u32) {
        self.spec.frequency_hz = frequency_hz;
        self.coefficients = ResonatorCoefficients::for_mode(self.spec, sample_rate);
        self.cached_sample_rate = Some(sample_rate);
    }

    /// Re-derive `decay_seconds`/`gain` from the note-on base values using new
    /// damping/brightness scalars, then rebuild coefficients. Allocation-free.
    ///
    /// Mirrors `ModalModeSpec::adjusted_for_controls` so live automation
    /// matches what would have happened if the same controls were captured at
    /// note-on.
    pub fn set_damping_brightness(
        &mut self,
        damping: f32,
        brightness: f32,
        mode_weight: f32,
        sample_rate: u32,
    ) {
        let damping_scale = (1.0 + (0.5 - damping.clamp(0.0, 1.0)) * 1.2).max(0.1);
        let brightness_tilt = 1.0 + (brightness.clamp(0.0, 1.0) - 0.5) * (0.5 + 1.0 * mode_weight);
        self.spec.decay_seconds = (self.base_decay_seconds * damping_scale).max(f32::EPSILON);
        self.spec.gain = (self.base_gain * brightness_tilt.max(0.1)).max(f32::EPSILON);
        self.coefficients = ResonatorCoefficients::for_mode(self.spec, sample_rate);
        self.cached_sample_rate = Some(sample_rate);
    }

    /// Process one excitation sample through the resonator.
    pub fn process(&mut self, excitation: f32, sample_rate: u32) -> f32 {
        if self.cached_sample_rate != Some(sample_rate) {
            self.coefficients = ResonatorCoefficients::for_mode(self.spec, sample_rate);
            self.cached_sample_rate = Some(sample_rate);
        }

        let sample = self.coefficients.b0 * excitation
            - self.coefficients.a1 * self.y1
            - self.coefficients.a2 * self.y2;

        self.y2 = self.y1;
        self.y1 = sample;
        sample
    }
}

#[derive(Clone, Debug)]
/// Modal resonator assembled from a profile and control transforms.
pub struct ModalResonator {
    pub(crate) profile: crate::dsp::ModalProfileId,
    #[allow(dead_code)]
    pub(crate) size_scale: crate::dsp::SizeScale,
    #[allow(dead_code)]
    pub(crate) rust_amount: crate::dsp::RustAmount,
    #[allow(dead_code)]
    pub(crate) damage_amount: crate::dsp::DamageAmount,
    #[allow(dead_code)]
    pub(crate) thickness_amount: crate::dsp::ThicknessAmount,
    #[allow(dead_code)]
    pub(crate) heat_amount: crate::dsp::HeatAmount,
    #[allow(dead_code)]
    pub(crate) sludge_amount: crate::dsp::SludgeAmount,
    pub(crate) modes: Vec<SecondOrderMode>,
    interaction_bus: crate::dsp::BidirectionalInteractionBus,
    /// Per-sample dynamic behavior (cable/sheet); `None` for static objects.
    dynamics: Dynamics,
    /// Smoothed running amplitude estimate driving the cable tension drop.
    dyn_amplitude: f32,
    /// Smoothed low-frequency displacement driving the sheet-metal warp.
    dyn_lf_displacement: f32,
    /// Multiplicative pitch-bend factor applied on top of mode base frequencies.
    /// `1.0` is neutral; written by `set_pitch_bend_factor` from MIDI pitch bend
    /// or poly tuning events. Layered with object dynamics in `apply_dynamics`.
    pitch_bend_factor: f32,
    /// Note-on scratch for raw generator output. Keeps `MAX_MODES` capacity
    /// so `rebuild_in_place` never allocates.
    raw_specs: Vec<crate::dsp::ModalModeSpec>,
    /// Note-on scratch for the rust/thickness/heat/sludge/damage-transformed
    /// specs, also kept at `MAX_MODES` capacity.
    transformed_specs: Vec<crate::dsp::ModalModeSpec>,
}

impl ModalResonator {
    const REFERENCE_PITCH_HZ: f32 = 220.0;

    /// An empty resonator whose mode, scratch, and interaction buffers are
    /// preallocated for the worst-case bank (`MAX_MODES`).
    fn with_note_capacity(profile_id: crate::dsp::ModalProfileId) -> Self {
        Self {
            profile: profile_id,
            size_scale: crate::dsp::SizeScale::default(),
            rust_amount: crate::dsp::RustAmount::default(),
            damage_amount: crate::dsp::DamageAmount::default(),
            thickness_amount: crate::dsp::ThicknessAmount::default(),
            heat_amount: crate::dsp::HeatAmount::default(),
            sludge_amount: crate::dsp::SludgeAmount::default(),
            modes: Vec::with_capacity(MAX_MODES),
            interaction_bus: crate::dsp::BidirectionalInteractionBus::with_capacity(MAX_MODES),
            dynamics: Dynamics::None,
            dyn_amplitude: 0.0,
            dyn_lf_displacement: 0.0,
            pitch_bend_factor: 1.0,
            raw_specs: Vec::with_capacity(MAX_MODES),
            transformed_specs: Vec::with_capacity(MAX_MODES),
        }
    }

    /// Apply the material transforms to `modes`, replacing the contents of
    /// `out`. Stops at `MAX_MODES` so a preallocated `out` never grows.
    fn transform_algorithm_modes_into(
        modes: &[crate::dsp::ModalModeSpec],
        controls: AlgorithmTransformControls,
        out: &mut Vec<crate::dsp::ModalModeSpec>,
    ) {
        out.clear();
        let mode_count = modes.len().max(1);

        for (index, &mode) in modes.iter().enumerate() {
            let rusted = mode.corroded(controls.rust_amount, index, mode_count);
            let thickened = rusted.thickened(controls.thickness_amount, index);
            let heated = thickened.heated(controls.heat_amount);
            let sludge_loaded = heated.sludge_loaded(controls.sludge_amount);
            let damaged = sludge_loaded.damaged(controls.damage_amount, index, mode_count);

            for damaged_mode in damaged {
                if out.len() == MAX_MODES {
                    return;
                }
                out.push(damaged_mode);
            }
        }
    }

    /// Generate the algorithmic modal bank for an object, configured by the
    /// curated character controls. The profile tables now contribute only
    /// `mode_count` (budget/test metadata); they no longer drive the sound.
    ///
    /// `mode_count_scale` multiplies the base mode count; QualityMode supplies
    /// 0.5/1.0/1.5/2.0 so Eco runs leaner banks and Render builds richer ones.
    ///
    /// Replaces the contents of `out` with the raw modes and returns any
    /// per-sample dynamic behavior the object retains (cable tension drop,
    /// sheet-metal warp).
    fn generate_algorithm_modes_into(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        target_frequency_hz: f32,
        character: CharacterParams,
        mode_count_scale: f32,
        out: &mut Vec<crate::dsp::ModalModeSpec>,
    ) -> Dynamics {
        out.clear();
        let base = crate::dsp::ModalProfile::from_id(profile_id)
            .mode_count()
            .max(1);
        let scale = if mode_count_scale.is_finite() && mode_count_scale > 0.0 {
            mode_count_scale.clamp(0.25, MAX_MODE_COUNT_SCALE)
        } else {
            1.0
        };
        let mode_count = ((base as f32 * scale).round() as usize).max(1);

        match profile_id {
            crate::dsp::ModalProfileId::Pipe => {
                let resonator = PipeResonator {
                    tube_diameter: character.pipe_diameter,
                    sustain_time: 1.0,
                };
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::Plate => {
                let resonator = PlateResonator {
                    aspect_ratio: character.plate_aspect,
                    metal_stiffness: character.plate_stiffness,
                };
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::Tank => {
                let resonator = TankResonator {
                    tank_volume: character.tank_volume,
                    wall_thickness: 0.5,
                    cavity_mix: character.tank_cavity_mix,
                };
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::Chain => {
                let resonator = ChainResonator {
                    link_mass: character.chain_link_mass,
                    chain_length: mode_count,
                    instability: character.chain_instability,
                    friction_decay: 2.0,
                };
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::IBeam => {
                let resonator = IBeamResonator::with_character(character.beam_shear);
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::TautCable => {
                let resonator = TautCableResonator::with_character(
                    character.cable_braid,
                    character.cable_tension_drop,
                );
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::TautCable(resonator)
            }
            crate::dsp::ModalProfileId::CoilSpring => {
                let resonator = CoilSpringResonator::with_character(
                    character.spring_dispersion,
                    character.spring_slosh,
                );
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::SheetMetal => {
                let resonator = SheetMetalResonator::with_character(character.sheet_thinness);
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::SheetMetal(resonator)
            }
            crate::dsp::ModalProfileId::IndustrialCog => {
                let resonator = IndustrialCogResonator::with_character(character.cog_dissonance);
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
        }
    }

    // Internal constructor intentionally mirrors all modal control transforms.
    #[allow(clippy::too_many_arguments)]
    fn from_profile(
        profile: crate::dsp::ModalProfile,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
        target_frequency_hz: Option<f32>,
        damping: f32,
        brightness: f32,
    ) -> Self {
        let pitch_scale = target_frequency_hz
            .map(|target| (target / Self::REFERENCE_PITCH_HZ.max(f32::EPSILON)).max(f32::EPSILON));
        let mode_count = profile.mode_count().max(1);
        let mut modes: Vec<SecondOrderMode> = profile
            .scaled_mode_specs_extended(
                size_scale,
                rust_amount,
                damage_amount,
                thickness_amount,
                heat_amount,
                sludge_amount,
            )
            .into_iter()
            .enumerate()
            .map(|(index, mode)| {
                let mode_weight = if mode_count <= 1 {
                    0.0
                } else {
                    index as f32 / (mode_count - 1) as f32
                };
                let mode = if let Some(scale) = pitch_scale {
                    mode.retuned(scale)
                } else {
                    mode
                };
                mode.adjusted_for_controls(damping, brightness, mode_weight)
            })
            .map(SecondOrderMode::new)
            .collect();
        // Reserve the worst-case bank so a later `rebuild_in_place` (note-on
        // on the audio thread) never has to grow these buffers.
        modes.reserve(MAX_MODES.saturating_sub(modes.len()));

        let mut interaction_bus = crate::dsp::BidirectionalInteractionBus::with_capacity(MAX_MODES);
        interaction_bus.initialize(modes.len());

        Self {
            profile: profile.id,
            size_scale,
            rust_amount,
            damage_amount,
            thickness_amount,
            heat_amount,
            sludge_amount,
            modes,
            interaction_bus,
            dynamics: Dynamics::None,
            dyn_amplitude: 0.0,
            dyn_lf_displacement: 0.0,
            pitch_bend_factor: 1.0,
            raw_specs: Vec::with_capacity(MAX_MODES),
            transformed_specs: Vec::with_capacity(MAX_MODES),
        }
    }

    /// Create a resonator from the default profile.
    pub fn with_profile(profile_id: crate::dsp::ModalProfileId) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            crate::dsp::SizeScale::default(),
            crate::dsp::RustAmount::default(),
            crate::dsp::DamageAmount::default(),
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }

    /// Create a resonator with a profile and size control.
    pub fn with_profile_and_size(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
    ) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            size_scale,
            crate::dsp::RustAmount::default(),
            crate::dsp::DamageAmount::default(),
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }

    /// Create a resonator with profile, size, and rust controls.
    pub fn with_profile_size_and_rust(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
    ) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            size_scale,
            rust_amount,
            crate::dsp::DamageAmount::default(),
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }

    /// Create a resonator with size, rust, and damage controls.
    pub fn with_profile_size_rust_and_damage(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
    ) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            size_scale,
            rust_amount,
            damage_amount,
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }

    /// Create a resonator with the full control surface and pitch target.
    // Public full-control constructor preserves the existing voice call shape.
    #[allow(clippy::too_many_arguments)]
    pub fn with_profile_controls_and_note(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
        target_frequency_hz: f32,
        damping: f32,
        brightness: f32,
    ) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            size_scale,
            rust_amount,
            damage_amount,
            thickness_amount,
            heat_amount,
            sludge_amount,
            Some(target_frequency_hz),
            damping,
            brightness,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_algorithm_controls_and_note(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
        target_frequency_hz: f32,
        damping: f32,
        brightness: f32,
        character: CharacterParams,
        mode_count_scale: f32,
    ) -> Self {
        let mut resonator = Self::with_note_capacity(profile_id);
        resonator.rebuild_in_place(
            profile_id,
            size_scale,
            rust_amount,
            damage_amount,
            thickness_amount,
            heat_amount,
            sludge_amount,
            target_frequency_hz,
            damping,
            brightness,
            character,
            mode_count_scale,
        );
        resonator
    }

    /// Rebuild this resonator's modal bank for a new note, reusing its
    /// preallocated buffers. Produces the same bank as
    /// `with_algorithm_controls_and_note` without allocating or freeing, so
    /// the voice can call it from note-on on the audio thread (review C3).
    #[allow(clippy::too_many_arguments)]
    pub fn rebuild_in_place(
        &mut self,
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
        target_frequency_hz: f32,
        damping: f32,
        brightness: f32,
        character: CharacterParams,
        mode_count_scale: f32,
    ) {
        let dynamics = Self::generate_algorithm_modes_into(
            profile_id,
            size_scale,
            target_frequency_hz,
            character,
            mode_count_scale,
            &mut self.raw_specs,
        );
        // Generators must stay within the preallocated raw buffer; growing
        // it here would allocate on the audio thread.
        debug_assert!(
            self.raw_specs.len() <= MAX_RAW_MODES,
            "{profile_id:?} generated {} raw modes > {MAX_RAW_MODES}",
            self.raw_specs.len()
        );
        Self::transform_algorithm_modes_into(
            &self.raw_specs,
            AlgorithmTransformControls {
                rust_amount,
                damage_amount,
                thickness_amount,
                heat_amount,
                sludge_amount,
            },
            &mut self.transformed_specs,
        );
        self.modes.clear();
        self.modes.extend(
            self.transformed_specs
                .iter()
                .copied()
                .map(SecondOrderMode::new),
        );

        self.profile = profile_id;
        self.size_scale = crate::dsp::SizeScale::default();
        self.rust_amount = crate::dsp::RustAmount::default();
        self.damage_amount = crate::dsp::DamageAmount::default();
        self.thickness_amount = crate::dsp::ThicknessAmount::default();
        self.heat_amount = crate::dsp::HeatAmount::default();
        self.sludge_amount = crate::dsp::SludgeAmount::default();
        self.interaction_bus.reset(self.modes.len());
        self.dynamics = dynamics;
        self.dyn_amplitude = 0.0;
        self.dyn_lf_displacement = 0.0;
        self.pitch_bend_factor = 1.0;

        // Damping/brightness are kept off `base_decay_seconds`/`base_gain` so
        // held-note automation can re-derive them; apply them here at note-on.
        self.set_live_resonator_controls(damping, brightness, 48_000);
    }
}

impl Default for ModalResonator {
    /// Construct the default pipe resonator.
    fn default() -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::pipe(),
            crate::dsp::SizeScale::default(),
            crate::dsp::RustAmount::default(),
            crate::dsp::DamageAmount::default(),
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }
}

/// Audio-rate resonator API used by the voice module.
pub trait ResonatorCore {
    /// Process one mono excitation sample.
    fn process_sample(&mut self, excitation: f32, sample_rate: u32) -> f32;
    /// Process one sample and return a stereo pair.
    fn process_sample_stereo(
        &mut self,
        excitation: f32,
        sample_rate: u32,
        width: f32,
    ) -> (f32, f32);
}

impl ModalResonator {
    /// Silence the resonator in place: zero every mode's filter history and
    /// the dynamics trackers. The bank itself is kept; the next note-on
    /// rebuilds it. Allocation-free.
    pub fn clear_state(&mut self) {
        for mode in &mut self.modes {
            mode.y1 = 0.0;
            mode.y2 = 0.0;
        }
        self.dyn_amplitude = 0.0;
        self.dyn_lf_displacement = 0.0;
        self.interaction_bus.reset(self.modes.len());
    }

    /// Apply live damping/brightness updates to every mode.
    ///
    /// Modes keep their note-on `base_decay_seconds` / `base_gain`; this method
    /// re-derives `spec.decay_seconds` / `spec.gain` and rebuilds biquad
    /// coefficients in place. Allocation-free; intended to be called once per
    /// buffer from the host layer for held-note automation.
    pub fn set_live_resonator_controls(&mut self, damping: f32, brightness: f32, sample_rate: u32) {
        let mode_count = self.modes.len();
        if mode_count == 0 {
            return;
        }
        for (index, mode) in self.modes.iter_mut().enumerate() {
            let mode_weight = if mode_count <= 1 {
                0.0
            } else {
                index as f32 / (mode_count - 1) as f32
            };
            mode.set_damping_brightness(damping, brightness, mode_weight, sample_rate);
        }
    }

    /// Update interaction parameters from the voice layer.
    pub fn set_interaction_params(
        &mut self,
        strike_position: f32,
        coupling_stiffness: f32,
        position_wander: f32,
        position_envelope: f32,
        fundamental_anchor: f32,
        sample_rate: f32,
    ) {
        self.interaction_bus
            .state
            .set_strike_position(strike_position);
        self.interaction_bus.state.coupling_stiffness = coupling_stiffness.clamp(0.0, 1.0);
        self.interaction_bus.state.fundamental_lock = fundamental_anchor > 0.0;
        self.interaction_bus.state.fundamental_minimum = fundamental_anchor.clamp(0.0, 1.0);
        let wander_rate = 0.1 + position_envelope.clamp(0.0, 1.0) * 4.9;
        self.interaction_bus.state.set_position_wander(
            position_wander,
            wander_rate,
            sample_rate.max(1.0),
        );
        self.interaction_bus.update();
    }

    /// Apply the object's per-sample dynamic pitch behavior, recomputing the
    /// affected mode coefficients from their immutable base frequency. Layers
    /// the current `pitch_bend_factor` on top so MIDI pitch bend tracks the
    /// dynamic shift. No-op for static objects without active pitch bend.
    /// Allocation-free.
    fn apply_dynamics(&mut self, sample_rate: u32) {
        let dyn_factor = match &self.dynamics {
            Dynamics::None => 1.0,
            Dynamics::TautCable(inst) => inst.dynamic_pitch_factor(self.dyn_amplitude),
            Dynamics::SheetMetal(inst) => inst.warp_factor(self.dyn_lf_displacement),
        };
        let factor = dyn_factor * self.pitch_bend_factor;
        if !factor.is_finite() {
            return;
        }
        for mode in &mut self.modes {
            let base = mode.base_frequency_hz;
            mode.set_frequency(base * factor, sample_rate);
        }
    }

    #[inline]
    fn has_dynamics(&self) -> bool {
        // The dynamics-aware sample path is hot enough that we only re-enter it
        // when there is a real-time pitch effect to apply: a cable/sheet hook,
        // or a non-unity pitch bend factor.
        !matches!(self.dynamics, Dynamics::None) || (self.pitch_bend_factor - 1.0).abs() > 1e-6
    }

    /// Apply a multiplicative pitch-bend factor to the modal bank and rebuild
    /// coefficients once. Routed from `Voice::set_pitch_bend` so MIDI pitch bend
    /// and poly tuning can re-pitch a held note without re-allocating modes.
    pub fn set_pitch_bend_factor(&mut self, factor: f32, sample_rate: u32) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let factor = factor.clamp(0.25, 4.0);
        if (factor - self.pitch_bend_factor).abs() < 1e-6 {
            return;
        }
        self.pitch_bend_factor = factor;
        // For static objects there is no per-sample apply_dynamics callback, so
        // rebuild biquads here. For dynamic objects the next sample will pick
        // it up via apply_dynamics — but doing the rebuild once now keeps the
        // response immediate.
        for mode in &mut self.modes {
            let base = mode.base_frequency_hz;
            mode.set_frequency(base * factor, sample_rate);
        }
    }

    /// Current multiplicative pitch-bend factor (1.0 = no bend).
    #[inline]
    pub fn pitch_bend_factor(&self) -> f32 {
        self.pitch_bend_factor
    }

    /// Update the smoothed amplitude / low-frequency estimates that drive the
    /// dynamic hooks. `lf_sum` is the summed displacement of the lowest modes.
    #[inline]
    fn track_dynamic_state(&mut self, amp_sum: f32, lf_sum: f32) {
        self.dyn_amplitude = self.dyn_amplitude * 0.99 + amp_sum * 0.01;
        self.dyn_lf_displacement = self.dyn_lf_displacement * 0.95 + lf_sum * 0.05;
    }

    /// Return the current summed displacement.
    pub fn get_displacement(&self) -> f32 {
        self.interaction_bus.state.resonator_displacement
    }

    /// Return the current summed velocity estimate.
    pub fn get_velocity(&self) -> f32 {
        self.interaction_bus.state.resonator_velocity
    }
}

impl ResonatorCore for ModalResonator {
    fn process_sample(&mut self, excitation: f32, sample_rate: u32) -> f32 {
        self.interaction_bus.update();
        let coupling = self
            .interaction_bus
            .state
            .coupling_stiffness
            .clamp(0.0, 1.0);
        self.interaction_bus.state.set_exciter_force(excitation);

        let dynamic = self.has_dynamics();
        if dynamic {
            self.apply_dynamics(sample_rate);
        }

        let mut mode_sum = 0.0f32;
        let mut interaction_displacement = 0.0f32;
        let mut amp_sum = 0.0f32;
        let mut lf_sum = 0.0f32;

        for (index, mode) in self.modes.iter_mut().enumerate() {
            let coeff = self
                .interaction_bus
                .mode_coefficients
                .get(index)
                .copied()
                .unwrap_or(1.0);
            let weighted_force = excitation * coeff;
            let blended_force = excitation * (1.0 - coupling) + weighted_force * coupling;
            let mode_output = mode.process(blended_force, sample_rate);
            mode_sum += mode_output;
            interaction_displacement += mode_output * coeff;
            if dynamic {
                amp_sum += mode_output.abs();
                if index < 3 {
                    lf_sum += mode_output;
                }
            }
        }

        if dynamic {
            self.track_dynamic_state(amp_sum, lf_sum);
        }

        self.interaction_bus.state.resonator_velocity =
            interaction_displacement - self.interaction_bus.state.resonator_displacement;
        self.interaction_bus.state.resonator_displacement = interaction_displacement;

        match self.profile {
            crate::dsp::ModalProfileId::Pipe => mode_sum,
            crate::dsp::ModalProfileId::Plate => mode_sum,
            crate::dsp::ModalProfileId::Tank => mode_sum,
            crate::dsp::ModalProfileId::Chain => mode_sum,
            crate::dsp::ModalProfileId::IBeam => mode_sum,
            crate::dsp::ModalProfileId::TautCable => mode_sum,
            crate::dsp::ModalProfileId::CoilSpring => mode_sum,
            crate::dsp::ModalProfileId::SheetMetal => mode_sum,
            crate::dsp::ModalProfileId::IndustrialCog => mode_sum,
        }
    }

    fn process_sample_stereo(
        &mut self,
        excitation: f32,
        sample_rate: u32,
        width: f32,
    ) -> (f32, f32) {
        let mode_count = self.modes.len();
        if mode_count == 0 {
            return (0.0, 0.0);
        }

        self.interaction_bus.update();
        let coupling = self
            .interaction_bus
            .state
            .coupling_stiffness
            .clamp(0.0, 1.0);
        self.interaction_bus.state.set_exciter_force(excitation);

        let dynamic = self.has_dynamics();
        if dynamic {
            self.apply_dynamics(sample_rate);
        }

        let mut left_sum = 0.0f32;
        let mut right_sum = 0.0f32;
        let mut interaction_displacement = 0.0f32;
        let mut amp_sum = 0.0f32;
        let mut lf_sum = 0.0f32;

        for (index, mode) in self.modes.iter_mut().enumerate() {
            let coeff = self
                .interaction_bus
                .mode_coefficients
                .get(index)
                .copied()
                .unwrap_or(1.0);
            let weighted_force = excitation * coeff;
            let blended_force = excitation * (1.0 - coupling) + weighted_force * coupling;
            let sample = mode.process(blended_force, sample_rate);
            let mode_position = index as f32 / mode_count.max(1) as f32;
            let pan_spread = width * mode_position;
            let pan_direction = if index.is_multiple_of(2) { 1.0 } else { -1.0 };
            let pan = (pan_spread * pan_direction).clamp(-1.0, 1.0);
            let left_gain = 0.5 * (1.0 - pan);
            let right_gain = 0.5 * (1.0 + pan);

            left_sum += sample * left_gain;
            right_sum += sample * right_gain;
            interaction_displacement += sample * coeff;
            if dynamic {
                amp_sum += sample.abs();
                if index < 3 {
                    lf_sum += sample;
                }
            }
        }

        if dynamic {
            self.track_dynamic_state(amp_sum, lf_sum);
        }

        self.interaction_bus.state.resonator_velocity =
            interaction_displacement - self.interaction_bus.state.resonator_displacement;
        self.interaction_bus.state.resonator_displacement = interaction_displacement;
        (left_sum, right_sum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::ModalProfileId;

    const PROFILES: [ModalProfileId; 9] = [
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
    /// QualityMode Eco / Normal / High / Render mode-count scales.
    const QUALITY_SCALES: [f32; 4] = [0.5, 1.0, 1.5, 2.0];
    const DAMAGE: [f32; 3] = [0.0, 0.5, 1.0];
    const FIXTURE: &str = include_str!("../../../tests/fixtures/mode_bank_hashes.txt");

    /// Builds a note-on bank the way `Voice::note_on_with_controls` does,
    /// with every transform engaged so they all reach the fingerprint.
    fn note_on_bank(profile: ModalProfileId, scale: f32, damage: f32) -> ModalResonator {
        ModalResonator::with_algorithm_controls_and_note(
            profile,
            crate::dsp::SizeScale::new(1.2),
            crate::dsp::RustAmount::new(0.4),
            crate::dsp::DamageAmount::new(damage),
            crate::dsp::ThicknessAmount::new(0.3),
            crate::dsp::HeatAmount::new(0.2),
            crate::dsp::SludgeAmount::new(0.1),
            261.63,
            0.45,
            0.6,
            CharacterParams::default(),
            scale,
        )
    }

    /// FNV-1a over the mode count and every mode's spec and base values.
    fn bank_hash(resonator: &ModalResonator) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut feed = |bits: u32| {
            for byte in bits.to_le_bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        };
        feed(resonator.modes.len() as u32);
        for mode in &resonator.modes {
            for value in [
                mode.spec.frequency_hz,
                mode.spec.decay_seconds,
                mode.spec.gain,
                mode.base_frequency_hz,
                mode.base_decay_seconds,
                mode.base_gain,
            ] {
                feed(value.to_bits());
            }
        }
        hash
    }

    fn current_fingerprints() -> Vec<String> {
        let mut lines = Vec::new();
        for profile in PROFILES {
            for scale in QUALITY_SCALES {
                for damage in DAMAGE {
                    let bank = note_on_bank(profile, scale, damage);
                    lines.push(format!(
                        "{profile:?} scale={scale} damage={damage} modes={} {:#018x}",
                        bank.modes.len(),
                        bank_hash(&bank)
                    ));
                }
            }
        }
        lines
    }

    /// `MAX_MODES` must cover every bank a note-on can build. The transform
    /// truncates at `MAX_MODES`, so checking the final bank would pass
    /// vacuously; check the generator's raw count and the untruncated
    /// damage expansion instead.
    #[test]
    fn note_on_banks_fit_max_modes() {
        let extremes = [0.0, 1.0, 2.0];
        let mut raw = Vec::new();
        for profile in PROFILES {
            for scale in [0.25, 0.5, 1.0, 1.5, 2.0, 4.0, 100.0] {
                for value in extremes {
                    let character = CharacterParams {
                        pipe_diameter: value,
                        plate_aspect: value,
                        plate_stiffness: value,
                        tank_volume: value,
                        tank_cavity_mix: value,
                        chain_link_mass: value,
                        chain_instability: value,
                        beam_shear: value,
                        cable_braid: value,
                        cable_tension_drop: value,
                        spring_dispersion: value,
                        spring_slosh: value,
                        sheet_thinness: value,
                        cog_dissonance: value,
                    };
                    ModalResonator::generate_algorithm_modes_into(
                        profile,
                        crate::dsp::SizeScale::new(1.0),
                        261.63,
                        character,
                        scale,
                        &mut raw,
                    );
                    let name = format!("{profile:?} scale={scale} character={value}");
                    assert!(
                        raw.len() <= MAX_RAW_MODES,
                        "{name}: {} raw modes > {MAX_RAW_MODES}",
                        raw.len()
                    );
                    let mode_count = raw.len().max(1);
                    let damaged: usize = raw
                        .iter()
                        .enumerate()
                        .map(|(index, mode)| {
                            mode.damaged(crate::dsp::DamageAmount::new(10.0), index, mode_count)
                                .count()
                        })
                        .sum();
                    assert!(
                        damaged <= MAX_MODES,
                        "{name}: {damaged} damaged modes > MAX_MODES {MAX_MODES}"
                    );
                }
            }
        }
    }

    #[test]
    fn rebuilding_a_reused_resonator_matches_a_fresh_one() {
        // One long-lived resonator, rebuilt for every case in turn (growing
        // and shrinking the bank), must match a freshly built bank exactly.
        let mut reused = ModalResonator::with_profile(ModalProfileId::Pipe);
        for profile in PROFILES {
            for scale in QUALITY_SCALES {
                for damage in DAMAGE {
                    reused.rebuild_in_place(
                        profile,
                        crate::dsp::SizeScale::new(1.2),
                        crate::dsp::RustAmount::new(0.4),
                        crate::dsp::DamageAmount::new(damage),
                        crate::dsp::ThicknessAmount::new(0.3),
                        crate::dsp::HeatAmount::new(0.2),
                        crate::dsp::SludgeAmount::new(0.1),
                        261.63,
                        0.45,
                        0.6,
                        CharacterParams::default(),
                        scale,
                    );
                    let fresh = note_on_bank(profile, scale, damage);
                    assert_eq!(
                        bank_hash(&reused),
                        bank_hash(&fresh),
                        "{profile:?} scale={scale} damage={damage}"
                    );
                    assert_eq!(
                        reused.interaction_bus.mode_coefficients,
                        fresh.interaction_bus.mode_coefficients
                    );
                }
            }
        }
    }

    /// Note-on mode banks must not change when allocation is removed from the
    /// note-on path (review C3). The fixture was captured from the allocating
    /// implementation; regenerate it only for an intended sound change with
    /// `CORROSION_UPDATE_GOLDEN=1 cargo test mode_banks_match_golden`.
    #[test]
    fn mode_banks_match_golden_fixture() {
        let current = current_fingerprints();
        if std::env::var_os("CORROSION_UPDATE_GOLDEN").is_some() {
            let path = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/mode_bank_hashes.txt"
            );
            std::fs::write(path, current.join("\n") + "\n").expect("write fixture");
            return;
        }
        let golden: Vec<&str> = FIXTURE.lines().collect();
        assert_eq!(golden.len(), current.len(), "fixture case count changed");
        for (expected, actual) in golden.iter().zip(&current) {
            assert_eq!(*expected, actual.as_str());
        }
    }
}

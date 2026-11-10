use crate::dsp::profiles::{
    chain, coil_spring, ibeam, industrial_cog, pipe, plate, sheet_metal, tank, taut_cable,
};

pub use chain::CHAIN_MODAL_PROFILE_MODES;
pub use coil_spring::COILSPRING_MODAL_PROFILE_MODES;
pub use ibeam::IBEAM_MODAL_PROFILE_MODES;
pub use industrial_cog::COG_MODAL_PROFILE_MODES;
pub use pipe::PIPE_MODAL_PROFILE_MODES;
pub use plate::PLATE_MODAL_PROFILE_MODES;
pub use sheet_metal::SHEETMETAL_MODAL_PROFILE_MODES;
pub use tank::TANK_MODAL_PROFILE_MODES;
pub use taut_cable::TAUTCABLE_MODAL_PROFILE_MODES;

#[derive(Clone, Copy, Debug)]
pub struct ModalModeSpec {
    pub frequency_hz: f32,
    pub decay_seconds: f32,
    pub gain: f32,
}

impl ModalModeSpec {
    pub const fn new(frequency_hz: f32, decay_seconds: f32, gain: f32) -> Self {
        Self {
            frequency_hz,
            decay_seconds,
            gain,
        }
    }

    pub fn scaled_for_size(
        self,
        size_scale: crate::dsp::SizeScale,
        mode_index: usize,
        mode_count: usize,
    ) -> Self {
        let scale = size_scale.factor();
        let low_mode_weight = if mode_count <= 1 {
            1.0
        } else {
            1.0 - (mode_index as f32 / (mode_count - 1) as f32)
        };
        let resonance_tilt = 1.0 + (scale - 1.0) * 0.35 * low_mode_weight;

        Self {
            frequency_hz: self.frequency_hz / scale,
            decay_seconds: self.decay_seconds * scale,
            gain: (self.gain * resonance_tilt).max(f32::EPSILON),
        }
    }

    /// Rust as a damping increase (`fix-rust-damage-heat-response`): the
    /// decay is divided by `1 + rust·(0.2 + 0.54·w²)` and the gain by
    /// `1 + rust·0.6·w^1.5`, `w` running from the fundamental (0) to the top
    /// mode (1). Divisors never reach zero, so every mode keeps ringing and
    /// the low thud survives the whole 0–5 range while the highs die first.
    /// (It used to multiply by `1 − rust·k`, written for 0–1, which reached
    /// zero at Rust 1.3 for the top mode and 3.3 for the fundamental.)
    pub fn corroded(
        self,
        rust_amount: crate::dsp::RustAmount,
        mode_index: usize,
        mode_count: usize,
    ) -> Self {
        let rust = rust_amount.amount();
        let w = if mode_count <= 1 {
            0.0
        } else {
            mode_index as f32 / (mode_count - 1) as f32
        };
        let decay_divisor = 1.0 + rust * (0.2 + 0.54 * w * w);
        let gain_divisor = 1.0 + rust * 0.6 * w.powf(1.5);

        Self {
            frequency_hz: self.frequency_hz,
            decay_seconds: (self.decay_seconds / decay_divisor).max(f32::EPSILON),
            gain: (self.gain / gain_divisor).max(f32::EPSILON),
        }
    }

    pub fn thickened(
        self,
        thickness_amount: crate::dsp::ThicknessAmount,
        mode_index: usize,
    ) -> Self {
        let thickness = thickness_amount.amount();
        let n = (mode_index + 1) as f32;
        let stiffness = 1.0 + (thickness - 0.5) * 0.15 * n * n;

        Self {
            frequency_hz: (self.frequency_hz * stiffness.max(0.25).sqrt()).max(f32::EPSILON),
            decay_seconds: self.decay_seconds,
            gain: self.gain,
        }
    }

    /// Heat: the metal softens. The pitch drops by up to 5 %, and the high
    /// modes lose level and ring shorter than the low ones: `gain /= 1 +
    /// heat·2·w^1.5`, `decay /= 1 + heat·0.5·w`, `w` from the fundamental
    /// (0) to the top mode (1). The darkening was a uniform gain, which the
    /// note-on bank normalization cancelled (`fix-rust-damage-heat-response`).
    /// The thermal warble is a pitch dynamic on the resonator.
    pub fn heated(
        self,
        heat_amount: crate::dsp::HeatAmount,
        mode_index: usize,
        mode_count: usize,
    ) -> Self {
        let heat = heat_amount.amount();
        let w = if mode_count <= 1 {
            0.0
        } else {
            mode_index as f32 / (mode_count - 1) as f32
        };
        let pitch_drop = 1.0 - heat * 0.05;

        Self {
            frequency_hz: (self.frequency_hz * pitch_drop.max(0.5)).max(f32::EPSILON),
            decay_seconds: (self.decay_seconds / (1.0 + heat * 0.5 * w)).max(f32::EPSILON),
            gain: (self.gain / (1.0 + heat * 2.0 * w.powf(1.5))).max(f32::EPSILON),
        }
    }

    pub fn sludge_loaded(self, sludge_amount: crate::dsp::SludgeAmount) -> Self {
        let sludge = sludge_amount.amount();
        let mass_ratio = (1.0 / (1.0 + sludge)).sqrt();
        let decay_loss = 1.0 / (1.0 + sludge * 1.5);
        let gain_loss = 1.0 - sludge * 0.35;

        Self {
            frequency_hz: (self.frequency_hz * mass_ratio).max(f32::EPSILON),
            decay_seconds: (self.decay_seconds * decay_loss.max(0.1)).max(f32::EPSILON),
            gain: (self.gain * gain_loss.max(0.1)).max(f32::EPSILON),
        }
    }

    /// Split a mode into its damaged primary and a detuned companion.
    ///
    /// Yields one mode (unchanged) at zero damage and two otherwise. Returned
    /// as a fixed-size iterator rather than a `Vec` so note-on can build the
    /// modal bank without allocating on the audio thread (review C3).
    pub fn damaged(
        self,
        damage_amount: crate::dsp::DamageAmount,
        mode_index: usize,
        mode_count: usize,
    ) -> std::iter::Take<std::array::IntoIter<Self, 2>> {
        let damage = damage_amount.amount();
        if damage <= 0.0 {
            return [self, self].into_iter().take(1);
        }
        // Damage drives everything through a saturating amount: equal to
        // `damage` near 0 (today's character for light damage), 2.85 at 10.
        // Detunes stay bounded, so the fundamental keeps its note, and no
        // decay factor reaches zero (`fix-rust-damage-heat-response`).
        let amount = 3.0 * (1.0 - (-damage / 3.0).exp());

        let mode_position = if mode_count <= 1 {
            0.0
        } else {
            mode_index as f32 / (mode_count - 1) as f32
        };
        let low_mode_weight = 1.0 - mode_position;
        let detune_direction = if mode_index.is_multiple_of(2) {
            -1.0
        } else {
            1.0
        };
        let primary_detune = amount * (0.004 + 0.018 * mode_position);
        let companion_detune = amount * (0.010 + 0.030 * mode_position);
        let primary_decay_tilt = 1.0 / (1.0 + amount * (0.04 + 0.08 * low_mode_weight));
        let primary_gain_tilt = 1.0 + amount * (0.05 + 0.12 * mode_position);
        let companion_gain =
            (self.gain * amount * (0.16 + 0.20 * mode_position)).min(self.gain * primary_gain_tilt);
        let companion_decay = self.decay_seconds * (0.28 + 0.18 * low_mode_weight);

        let primary = Self {
            frequency_hz: (self.frequency_hz * (1.0 + primary_detune * detune_direction))
                .max(f32::EPSILON),
            decay_seconds: (self.decay_seconds * primary_decay_tilt).max(f32::EPSILON),
            gain: (self.gain * primary_gain_tilt).max(f32::EPSILON),
        };
        let companion = Self {
            frequency_hz: (self.frequency_hz * (1.0 - companion_detune * detune_direction))
                .max(f32::EPSILON),
            decay_seconds: companion_decay.max(f32::EPSILON),
            gain: companion_gain.max(f32::EPSILON),
        };

        [primary, companion].into_iter().take(2)
    }

    pub fn retuned(self, pitch_scale: f32) -> Self {
        Self {
            frequency_hz: (self.frequency_hz * pitch_scale.max(f32::EPSILON)).max(f32::EPSILON),
            ..self
        }
    }

    pub fn adjusted_for_controls(self, damping: f32, brightness: f32, mode_weight: f32) -> Self {
        let damping_scale = (1.0 + (0.5 - damping.clamp(0.0, 1.0)) * 1.2).max(0.1);
        let brightness_tilt = 1.0 + (brightness.clamp(0.0, 1.0) - 0.5) * (0.5 + 1.0 * mode_weight);

        Self {
            frequency_hz: self.frequency_hz,
            decay_seconds: (self.decay_seconds * damping_scale).max(f32::EPSILON),
            gain: (self.gain * brightness_tilt.max(0.1)).max(f32::EPSILON),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModalProfileId {
    Pipe,
    Plate,
    Tank,
    Chain,
    IBeam,
    TautCable,
    CoilSpring,
    SheetMetal,
    IndustrialCog,
}

#[derive(Clone, Copy, Debug)]
pub struct ModalProfile {
    pub id: ModalProfileId,
    pub modes: &'static [ModalModeSpec],
}

impl ModalProfile {
    pub const fn from_id(id: ModalProfileId) -> Self {
        match id {
            ModalProfileId::Pipe => pipe::pipe(),
            ModalProfileId::Plate => plate::plate(),
            ModalProfileId::Tank => tank::tank(),
            ModalProfileId::Chain => chain::chain(),
            ModalProfileId::IBeam => ibeam::ibeam(),
            ModalProfileId::TautCable => taut_cable::taut_cable(),
            ModalProfileId::CoilSpring => coil_spring::coil_spring(),
            ModalProfileId::SheetMetal => sheet_metal::sheet_metal(),
            ModalProfileId::IndustrialCog => industrial_cog::industrial_cog(),
        }
    }

    pub const fn pipe() -> Self {
        pipe::pipe()
    }

    pub const fn plate() -> Self {
        plate::plate()
    }

    pub const fn tank() -> Self {
        tank::tank()
    }

    pub const fn chain() -> Self {
        chain::chain()
    }

    pub const fn ibeam() -> Self {
        ibeam::ibeam()
    }

    pub const fn taut_cable() -> Self {
        taut_cable::taut_cable()
    }

    pub const fn coil_spring() -> Self {
        coil_spring::coil_spring()
    }

    pub const fn sheet_metal() -> Self {
        sheet_metal::sheet_metal()
    }

    pub const fn industrial_cog() -> Self {
        industrial_cog::industrial_cog()
    }

    pub const fn mode_count(&self) -> usize {
        self.modes.len()
    }

    pub fn modes(&self) -> &[ModalModeSpec] {
        self.modes
    }

    pub fn scaled_mode_specs(
        &self,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
    ) -> Vec<ModalModeSpec> {
        self.scaled_mode_specs_extended(
            size_scale,
            rust_amount,
            damage_amount,
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
        )
    }

    pub fn scaled_mode_specs_extended(
        &self,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
    ) -> Vec<ModalModeSpec> {
        let mode_count = self.modes.len();
        let mut result = Vec::new();

        for (i, mode) in self.modes.iter().enumerate() {
            let sized = mode.scaled_for_size(size_scale, i, mode_count);
            let rusted = sized.corroded(rust_amount, i, mode_count);
            let thickened = rusted.thickened(thickness_amount, i);
            let heated = thickened.heated(heat_amount, i, mode_count);
            let sludge_loaded = heated.sludge_loaded(sludge_amount);
            let damaged = sludge_loaded.damaged(damage_amount, i, mode_count);
            result.extend(damaged);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_profile_pipe_has_modes() {
        let profile = ModalProfile::pipe();
        assert!(!profile.modes().is_empty());
    }

    #[test]
    fn modal_profile_pipe_modes_are_finite() {
        let profile = ModalProfile::pipe();
        for mode in profile.modes() {
            assert!(mode.frequency_hz.is_finite());
            assert!(mode.decay_seconds.is_finite());
            assert!(mode.gain.is_finite());
            assert!(mode.frequency_hz > 0.0);
            assert!(mode.decay_seconds > 0.0);
            assert!(mode.gain >= 0.0);
        }
    }

    #[test]
    fn modal_profile_tank_modes_are_finite() {
        let profile = ModalProfile::tank();
        for mode in profile.modes() {
            assert!(mode.frequency_hz.is_finite());
            assert!(mode.decay_seconds.is_finite());
            assert!(mode.gain.is_finite());
            assert!(mode.frequency_hz > 0.0);
            assert!(mode.decay_seconds > 0.0);
            assert!(mode.gain >= 0.0);
        }
    }

    #[test]
    fn modal_profile_modes_are_finite_for_all_profiles() {
        let profiles = [
            ModalProfile::pipe(),
            ModalProfile::plate(),
            ModalProfile::tank(),
            ModalProfile::chain(),
            ModalProfile::ibeam(),
            ModalProfile::taut_cable(),
            ModalProfile::coil_spring(),
            ModalProfile::sheet_metal(),
            ModalProfile::industrial_cog(),
        ];
        for profile in profiles {
            for mode in profile.modes() {
                assert!(
                    mode.frequency_hz.is_finite(),
                    "frequency_hz not finite for {:?}",
                    profile.id
                );
                assert!(
                    mode.decay_seconds.is_finite(),
                    "decay_seconds not finite for {:?}",
                    profile.id
                );
                assert!(
                    mode.gain.is_finite(),
                    "gain not finite for {:?}",
                    profile.id
                );
                assert!(
                    mode.frequency_hz > 0.0,
                    "frequency_hz <= 0 for {:?}",
                    profile.id
                );
                assert!(
                    mode.decay_seconds > 0.0,
                    "decay_seconds <= 0 for {:?}",
                    profile.id
                );
                assert!(mode.gain >= 0.0, "gain < 0 for {:?}", profile.id);
            }
        }
    }

    #[test]
    fn scaled_for_size_preserves_finite() {
        let mode = ModalModeSpec::new(440.0, 1.0, 0.01);
        let scaled = mode.scaled_for_size(crate::dsp::SizeScale::new(1.5), 0, 3);
        assert!(scaled.frequency_hz.is_finite());
        assert!(scaled.decay_seconds.is_finite());
        assert!(scaled.gain.is_finite());
        assert!(scaled.frequency_hz > 0.0);
        assert!(scaled.decay_seconds > 0.0);
        assert!(scaled.gain >= 0.0);
    }

    #[test]
    fn corroded_preserves_finite() {
        let mode = ModalModeSpec::new(440.0, 1.0, 0.01);
        let corroded = mode.corroded(crate::dsp::RustAmount::new(0.5), 1, 3);
        assert!(corroded.frequency_hz.is_finite());
        assert!(corroded.decay_seconds.is_finite());
        assert!(corroded.gain.is_finite());
        assert!(corroded.frequency_hz > 0.0);
        assert!(corroded.decay_seconds > 0.0);
        assert!(corroded.gain >= 0.0);
    }

    #[test]
    fn damaged_expands_and_preserves_finite() {
        let mode = ModalModeSpec::new(440.0, 1.0, 0.01);
        let damaged: Vec<_> = mode
            .damaged(crate::dsp::DamageAmount::new(0.5), 1, 3)
            .collect();
        assert_eq!(damaged.len(), 2, "damage should add a companion mode");
        assert!(
            !damaged.is_empty(),
            "damaged() should return at least the primary mode"
        );
        for m in &damaged {
            assert!(m.frequency_hz.is_finite(), "frequency_hz not finite");
            assert!(m.decay_seconds.is_finite(), "decay_seconds not finite");
            assert!(m.gain.is_finite(), "gain not finite");
            assert!(m.frequency_hz > 0.0, "frequency_hz <= 0");
            assert!(m.decay_seconds > 0.0, "decay_seconds <= 0");
            assert!(m.gain >= 0.0, "gain < 0");
        }
    }

    #[test]
    fn damaged_with_zero_returns_single_mode() {
        let mode = ModalModeSpec::new(440.0, 1.0, 0.01);
        let damaged: Vec<_> = mode
            .damaged(crate::dsp::DamageAmount::new(0.0), 1, 3)
            .collect();
        assert_eq!(damaged.len(), 1);
        assert_eq!(damaged[0].frequency_hz, mode.frequency_hz);
        assert_eq!(damaged[0].decay_seconds, mode.decay_seconds);
        assert_eq!(damaged[0].gain, mode.gain);
    }
}

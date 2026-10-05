//! I-Beam resonator model using Timoshenko Thick Beam Theory.
//!
//! Frequency formula: f_n = f_1 * n^2 / sqrt(1 + C * n^2)

use crate::dsp::resonators::ResonatorAlgorithm;
use crate::dsp::ModalModeSpec;

/// Resonator model for I-beam-like modal spectra.
#[derive(Clone, Debug)]
pub struct IBeamResonator {
    beam_mass: f32,
    shear_density: f32,
    rigidity_damping: f32,
}

impl IBeamResonator {
    pub fn new() -> Self {
        Self {
            beam_mass: 1.0,
            shear_density: 0.5,
            rigidity_damping: 0.7,
        }
    }

    /// Build from the exposed Shear Density control; mass and rigidity damping
    /// stay covered by the global Size/Damping path.
    pub fn with_character(shear_density: f32) -> Self {
        Self {
            beam_mass: 1.0,
            shear_density,
            rigidity_damping: 0.7,
        }
    }
}

impl Default for IBeamResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for IBeamResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let f1 = fundamental_hz / size_scale.factor();
        let c = self.shear_density * 2.0;

        out.extend((1..=mode_count).map(|n| {
            let nf = n as f32;
            // Timoshenko beam: f_n = f_1 * n^2 / sqrt(1 + C * n^2)
            let freq = f1 * nf * nf / (1.0 + c * nf * nf).sqrt();
            // High frequencies decay quickly
            let decay = 1.5 - self.rigidity_damping * 0.8 * (nf / mode_count as f32);
            let gain = (0.02 / nf) * self.beam_mass;

            ModalModeSpec::new(freq, decay, gain)
        }));
    }
}

//! Industrial Cog resonator model using Circular Free-Boundary Plate with Mode Splitting.
//!
//! Frequency formula: f_n split into f_n(1 ± ε) pairs for metallic beating

use crate::dsp::resonators::ResonatorAlgorithm;
use crate::dsp::ModalModeSpec;

/// Resonator model for split-pair cog and blade spectra.
#[derive(Clone, Debug)]
pub struct IndustrialCogResonator {
    blade_radius: f32,
    tooth_dissonance: f32,
    blade_thickness: f32,
}

impl IndustrialCogResonator {
    const BESSEL_ROOTS: [f32; 6] = [3.83, 7.02, 10.17, 13.32, 16.47, 19.64];

    pub fn new() -> Self {
        Self {
            blade_radius: 0.5,
            tooth_dissonance: 0.1,
            blade_thickness: 0.5,
        }
    }

    /// Build from the exposed Tooth Dissonance control; radius and thickness
    /// stay covered by the global Size/Damping path.
    pub fn with_character(tooth_dissonance: f32) -> Self {
        Self {
            blade_radius: 0.5,
            tooth_dissonance,
            blade_thickness: 0.5,
        }
    }
}

impl Default for IndustrialCogResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for IndustrialCogResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let scale = fundamental_hz / (size_scale.factor() * (0.5 + self.blade_radius));
        for (i, &root) in Self::BESSEL_ROOTS
            .iter()
            .take(mode_count.min(6))
            .enumerate()
        {
            let base_freq = scale * root * root * 0.01;
            let epsilon = self.tooth_dissonance * 0.03 * ((i + 1) as f32);

            let decay = 0.5 + self.blade_thickness * 1.5;
            let gain = 0.015 / ((i + 1) as f32);

            // Mode splitting creates metallic beating
            out.push(ModalModeSpec::new(base_freq * (1.0 - epsilon), decay, gain));
            out.push(ModalModeSpec::new(
                base_freq * (1.0 + epsilon),
                decay * 0.9,
                gain * 0.8,
            ));
        }
    }
}

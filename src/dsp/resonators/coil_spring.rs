//! Heavy Coil Spring resonator model using Highly Dispersive Helical Waveguide.
//!
//! Frequency formula: f_n = f_1 * (n^alpha + Jitter_n), with comb-filter pickup structure

use crate::dsp::resonators::ResonatorAlgorithm;
use crate::dsp::ModalModeSpec;

/// Resonator model for dispersive coil-spring spectra.
#[derive(Clone, Debug)]
pub struct CoilSpringResonator {
    coil_length: f32,
    dispersion_chirp: f32,
    spring_slosh: f32,
}

impl CoilSpringResonator {
    pub fn new() -> Self {
        Self {
            coil_length: 0.5,
            dispersion_chirp: 0.5,
            spring_slosh: 0.3,
        }
    }

    /// Build from the exposed Dispersion Chirp / Spring Slosh controls; coil
    /// length stays covered by the global Size/pitch path.
    pub fn with_character(dispersion_chirp: f32, spring_slosh: f32) -> Self {
        Self {
            coil_length: 0.5,
            dispersion_chirp,
            spring_slosh,
        }
    }
}

impl Default for CoilSpringResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for CoilSpringResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let f1 = fundamental_hz / size_scale.factor();
        let alpha = 0.5 + self.coil_length * 0.5;

        out.extend((1..=mode_count).map(|n| {
            let nf = n as f32;
            let jitter = (nf * 1.618).fract() * self.spring_slosh * 0.5;
            let freq = f1 * (nf.powf(alpha) + jitter);
            let decay = 1.0 + 0.5 * (nf / mode_count as f32);
            let dispersion = (nf * self.dispersion_chirp * std::f32::consts::PI).cos();
            let gain = (0.02 / nf) * dispersion.abs();

            ModalModeSpec::new(freq, decay, gain.max(0.001))
        }));
    }
}

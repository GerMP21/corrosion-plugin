//! Taut Cable resonator model using Stiff String with Tension-Modulated Pitch.
//!
//! Frequency formula: f_n = n * f_0 * sqrt(1 + B * n^2), with f_0(t) rising with amplitude

use crate::dsp::resonators::ResonatorAlgorithm;
use crate::dsp::ModalModeSpec;

/// Braid Stiffness's default (`ResonatorCharacter::default`).
const DEFAULT_BRAID_STIFFNESS: f32 = 0.3;

/// Resonator model for stiff cable-like spectra.
#[derive(Clone, Debug)]
pub struct TautCableResonator {
    cable_tension: f32,
    braid_stiffness: f32,
    tension_drop: f32,
}

impl TautCableResonator {
    pub fn new() -> Self {
        Self {
            cable_tension: 0.5,
            braid_stiffness: 0.3,
            tension_drop: 0.4,
        }
    }

    /// Build a cable from the exposed character controls, leaving base tuning
    /// to the global Size/pitch path.
    pub fn with_character(braid_stiffness: f32, tension_drop: f32) -> Self {
        Self {
            cable_tension: 0.5,
            braid_stiffness,
            tension_drop,
        }
    }

    /// Per-sample pitch multiplier driven by the cable's running amplitude
    /// (the "boing": a hard strike stretches the wire, raising the pitch, which
    /// falls back as the strike decays). Bounded so feedback cannot run away.
    pub fn dynamic_pitch_factor(&self, total_amplitude: f32) -> f32 {
        (1.0 + self.tension_drop * total_amplitude * 0.1).clamp(0.25, 4.0)
    }
}

impl Default for TautCableResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for TautCableResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let tension_factor = 0.8 + self.cable_tension * 0.4;
        let f0 = (fundamental_hz * tension_factor) / size_scale.factor();
        // Stiffer braid detunes the partials upward. The square law keeps
        // the default (0.3) exactly as it was and makes the top of the range
        // clearly inharmonic (`fix-inert-controls`: linear, it moved the
        // second partial 2 % at most).
        let b = self.braid_stiffness * 0.01 * (self.braid_stiffness / DEFAULT_BRAID_STIFFNESS);

        out.extend((1..=mode_count).map(|n| {
            let nf = n as f32;
            let freq = nf * f0 * (1.0 + b * nf * nf).sqrt();
            let decay = 2.0 - 0.5 * (nf / mode_count as f32);
            let gain = 0.025 / (nf * nf * 0.5 + 0.5);

            ModalModeSpec::new(freq, decay, gain)
        }));
    }

    fn update_dynamic_frequencies(&mut self, modes: &mut [ModalModeSpec], total_amplitude: f32) {
        let pitch_shift = self.dynamic_pitch_factor(total_amplitude);
        for mode in modes.iter_mut() {
            mode.frequency_hz *= pitch_shift;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The third partial over three times the first.
    fn third_partial_stretch(braid: f32) -> f32 {
        let mut modes = Vec::new();
        TautCableResonator::with_character(braid, 0.4).generate_modes_into(
            100.0,
            8,
            crate::dsp::SizeScale::default(),
            &mut modes,
        );
        modes[2].frequency_hz / (3.0 * modes[0].frequency_hz)
    }

    /// `fix-inert-controls`: the stiffest braid is clearly inharmonic.
    #[test]
    fn stiff_braid_stretches_the_partials() {
        let (loose, stiff) = (third_partial_stretch(0.0), third_partial_stretch(1.0));
        assert!(
            stiff / loose > 1.05,
            "third partial stretch: braid 0 {loose:.4}, braid 1 {stiff:.4}"
        );
    }
}

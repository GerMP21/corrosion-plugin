//! Sheet Metal resonator model using 2D Plate with Dynamic Buckling.
//!
//! Frequency formula: ω_n(t) = ω_{n,0} * (1 + β * (sum low-freq displacement)^2)

use crate::dsp::resonators::ResonatorAlgorithm;
use crate::dsp::ModalModeSpec;

/// Resonator model for sheet-metal plate spectra.
#[derive(Clone, Debug)]
pub struct SheetMetalResonator {
    sheet_size: f32,
    metal_thinness: f32,
    edge_damping: f32,
}

impl SheetMetalResonator {
    pub fn new() -> Self {
        Self {
            sheet_size: 0.5,
            metal_thinness: 0.4,
            edge_damping: 0.3,
        }
    }

    /// Build a sheet from the exposed character control; size and edge damping
    /// stay covered by the global Size/Damping path.
    pub fn with_character(metal_thinness: f32) -> Self {
        Self {
            sheet_size: 0.5,
            metal_thinness,
            edge_damping: 0.3,
        }
    }

    /// Buckling warp multiplier from the running low-frequency displacement.
    /// Bounded so the chaotic warp cannot diverge.
    pub fn warp_factor(&self, lf_displacement: f32) -> f32 {
        (1.0 + self.metal_thinness * lf_displacement * lf_displacement * 10.0).clamp(0.25, 4.0)
    }
}

impl Default for SheetMetalResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for SheetMetalResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let base = fundamental_hz / (size_scale.factor() * self.sheet_size);

        out.extend((1..=mode_count).map(|n| {
            let nf = n as f32;
            let freq = base * (nf * nf);
            let warp_factor = 1.0 + self.metal_thinness * 0.2;
            let decay = (2.0 - self.edge_damping) * (1.0 + nf / mode_count as f32);
            let gain = 0.03 / nf;

            ModalModeSpec::new(freq * warp_factor, decay, gain)
        }));
    }

    /// Dynamic buckling: frequencies wobble based on LF displacement
    /// Warp_factor(t) = 1 + β * (sum LF displacement)^2
    fn apply_warping(&mut self, modes: &mut [ModalModeSpec], lf_displacement: f32) {
        let warp = self.warp_factor(lf_displacement);
        for mode in modes.iter_mut() {
            mode.frequency_hz *= warp;
        }
    }
}

//! Algorithmic resonator implementations following the detailed specifications.
//!
//! Each resonator type has its own unique physical model:
//! - Pipe: 1D cylindrical waveguide with stiffness dispersion
//! - Plate: 2D Kirchhoff-Love plate modes
//! - Tank: 3D cylindrical shell + Helmholtz cavity
//! - Chain: Chaotic weakly-coupled oscillators
//!
//! Extended resonators also have dynamic behaviors:
//! - TautCable: Real-time tension modulation
//! - SheetMetal: Dynamic buckling/warping
//! - CoilSpring: Dispersive phase offsets
//! - IndustrialCog: Mode splitting

use crate::dsp::ModalModeSpec;

/// Trait for algorithmic resonator implementations.
/// Each object type implements its own physical model for mode generation.
pub trait ResonatorAlgorithm {
    /// Generate mode specifications for this resonator type.
    /// Called during note-on to create the modal bank.
    ///
    /// Appends the modes to `out` instead of returning a new `Vec`, so the
    /// note-on path can refill a preallocated buffer without allocating on
    /// the audio thread (review C3).
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    );

    /// Allocating convenience wrapper around `generate_modes_into` for
    /// offline and test callers.
    fn generate_modes(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
    ) -> Vec<ModalModeSpec> {
        let mut modes = Vec::new();
        self.generate_modes_into(fundamental_hz, mode_count, size_scale, &mut modes);
        modes
    }

    /// Update dynamic frequencies for tension-modulated resonators (Cable, etc).
    /// Called per-sample or per-block for real-time pitch effects.
    fn update_dynamic_frequencies(&mut self, _modes: &mut [ModalModeSpec], _total_amplitude: f32) {
        // Default: no dynamic behavior
    }

    /// Apply dynamic warping for buckling resonators (SheetMetal, etc).
    fn apply_warping(&mut self, _modes: &mut [ModalModeSpec], _lf_displacement: f32) {
        // Default: no warping
    }

    /// Get coupling coefficients for chaotic resonators (Chain).
    fn get_coupling(&self, _mode_index: usize) -> f32 {
        // Default: no coupling
        0.0
    }
}

/// 1D Pipe resonator with stiffness dispersion.
/// f_n = n * f_1 * sqrt(1 + B * n^2)
#[derive(Clone, Debug)]
pub struct PipeResonator {
    /// Controls inharmonicity. Wider pipe = higher B = more bell-like.
    pub tube_diameter: f32,
    /// Base sustain time scaling
    pub sustain_time: f32,
}

impl PipeResonator {
    pub fn new() -> Self {
        Self {
            tube_diameter: 0.5,
            sustain_time: 1.0,
        }
    }
}

impl Default for PipeResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for PipeResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let f1 = fundamental_hz / size_scale.factor();
        let b = self.tube_diameter * 0.1; // Stiffness coefficient

        out.extend((1..=mode_count).map(|n| {
            let nf = n as f32;
            // Stiffness formula: f_n = n * f_1 * sqrt(1 + B * n^2)
            let freq = nf * f1 * (1.0 + b * nf * nf).sqrt();
            // Decay decreases with mode number
            let decay = self.sustain_time * (1.5 - 0.15 * nf);
            // Gain follows 1/n roughly
            let gain = 0.02 / nf;

            ModalModeSpec::new(freq, decay, gain)
        }));
    }
}

/// 2D Plate resonator using Kirchhoff-Love theory.
/// f_{m,n} = K * ((m/Lx)^2 + (n/Ly)^2)
#[derive(Clone, Debug)]
pub struct PlateResonator {
    /// Aspect ratio Lx/Ly. Square = 1.0, long rectangle = high value.
    pub aspect_ratio: f32,
    /// Material stiffness coefficient
    pub metal_stiffness: f32,
}

impl PlateResonator {
    pub fn new() -> Self {
        Self {
            aspect_ratio: 1.0,
            metal_stiffness: 1.0,
        }
    }
}

impl Default for PlateResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for PlateResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let k = self.metal_stiffness * fundamental_hz / size_scale.factor();
        let lx = 1.0;
        let ly = lx / self.aspect_ratio.max(0.1);

        // Generate 2D grid modes (m, n pairs)
        let grid_size = (mode_count as f32).sqrt().ceil() as usize;
        let start = out.len();

        for m in 1..=grid_size {
            for n in 1..=grid_size {
                if out.len() - start >= mode_count {
                    break;
                }
                let mf = m as f32;
                let nf = n as f32;

                // 2D Kirchhoff-Love: f_{m,n} = K * ((m/Lx)^2 + (n/Ly)^2)
                let freq = k * ((mf / lx).powi(2) + (nf / ly).powi(2));
                // Dense, inharmonic cluster with faster decay
                let decay = 0.4 - 0.02 * (mf + nf);
                let gain = 0.008 / (mf + nf);

                out.push(ModalModeSpec::new(freq, decay, gain));
            }
        }
    }
}

/// 3D Tank resonator combining shell modes and Helmholtz cavity.
#[derive(Clone, Debug)]
pub struct TankResonator {
    /// Tank volume affecting Helmholtz resonance
    pub tank_volume: f32,
    /// Wall thickness affecting decay
    pub wall_thickness: f32,
    /// Balance between cavity boom and shell ring
    pub cavity_mix: f32,
}

impl TankResonator {
    pub fn new() -> Self {
        Self {
            tank_volume: 0.5,
            wall_thickness: 0.5,
            cavity_mix: 0.6,
        }
    }
}

impl Default for TankResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for TankResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let f_base = fundamental_hz / size_scale.factor();

        // Helmholtz cavity resonance (low frequency boom). The cavity follows
        // the played note (a deep sub-octave) rather than sitting at a fixed Hz,
        // so the tank stays in tune across the keyboard. Larger volumes push the
        // boom further below the fundamental.
        let cavity_freq = (f_base * 0.5 / (1.0 + self.tank_volume)).max(f32::EPSILON);
        let cavity_gain = 0.05 * self.cavity_mix;
        let cavity_decay = 2.5 * (1.0 + self.tank_volume); // Long sustain for boom

        out.push(ModalModeSpec::new(cavity_freq, cavity_decay, cavity_gain));

        // Shell modes (structural vibrations)
        let shell_modes = mode_count.saturating_sub(1);
        for i in 0..shell_modes {
            let n = (i + 1) as f32;
            // Dense clustering in low-mids for cylindrical shell
            let freq = f_base * (0.8 + n * 0.6);
            // Thinner walls = longer sustain and wobble
            let decay = (2.0 - self.wall_thickness) * (1.0 + n * 0.1);
            let gain = 0.015 * (1.0 - self.cavity_mix) / n;

            out.push(ModalModeSpec::new(freq, decay, gain));
        }
    }
}

/// Chain resonator with chaotic weakly-coupled oscillators.
/// Uses Gaussian Orthogonal Ensemble for chaotic frequencies.
#[derive(Clone, Debug)]
pub struct ChainResonator {
    /// Base frequency range
    pub link_mass: f32,
    /// Number of links (mode count)
    pub chain_length: usize,
    /// Coupling coefficient for chaotic energy bleed
    pub instability: f32,
    /// Friction decay multiplier
    pub friction_decay: f32,
}

impl ChainResonator {
    pub fn new() -> Self {
        Self {
            link_mass: 0.5,
            chain_length: 10,
            instability: 0.3,
            friction_decay: 2.0,
        }
    }

    /// Generate GOE-distributed frequencies (chaotic, repelling).
    ///
    /// The cluster is anchored to the played note so the chain tracks pitch;
    /// heavier links shift the whole cluster lower.
    fn goe_frequency(&self, index: usize, fundamental_hz: f32) -> f32 {
        let n = index as f32;
        // Chaotic distribution using random-like but deterministic sequence
        let chaos = (n * 1.618_034).fract(); // Golden ratio conjugate
        let base = fundamental_hz / (0.5 + self.link_mass);
        base * (1.0 + n * 0.15 + chaos * self.instability)
    }
}

impl Default for ChainResonator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResonatorAlgorithm for ChainResonator {
    fn generate_modes_into(
        &self,
        fundamental_hz: f32,
        mode_count: usize,
        size_scale: crate::dsp::SizeScale,
        out: &mut Vec<ModalModeSpec>,
    ) {
        let count = mode_count.min(self.chain_length);

        out.extend((0..count).map(|i| {
            // GOE chaotic frequencies, anchored to the played note
            let freq = self.goe_frequency(i, fundamental_hz) / size_scale.factor();
            // High damping for individual modes
            let decay = 0.4 / self.friction_decay;
            // Dense cluster
            let gain = 0.018;

            ModalModeSpec::new(freq, decay, gain)
        }));
    }

    fn get_coupling(&self, mode_index: usize) -> f32 {
        // Dynamic coupling: amplitude of mode n modulates mode n+1
        self.instability * (1.0 + 0.1 * (mode_index as f32))
    }
}

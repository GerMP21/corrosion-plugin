//! FEM-style body resonator stage.
//!
//! This is a compact modal approximation used to add material-specific body
//! coloration after the drive stage.
/// Modal body resonator with eight FEM-inspired modes.
#[derive(Clone, Debug)]
pub struct FemBodyResonator {
    material: f32,
    volume: f32,

    // Modal resonator states (8 modes for FEM approximation)
    y: [f32; 8],
    y1: [f32; 8],

    // Mode frequencies based on material
    base_freqs: [f32; 8],

    sample_rate: f32,
}

impl FemBodyResonator {
    /// Creates the body resonator with default material and volume.
    pub fn new() -> Self {
        Self {
            material: 0.5,
            volume: 0.5,
            y: [0.0; 8],
            y1: [0.0; 8],
            base_freqs: [220.0, 380.0, 550.0, 720.0, 890.0, 1060.0, 1230.0, 1400.0],
            sample_rate: 48000.0,
        }
    }

    /// Sets material and cavity volume controls. Volume sets the body's size:
    /// larger volumes move every mode lower. It does not change the output
    /// level.
    pub fn set_parameters(&mut self, material: f32, volume: f32) {
        self.material = material.clamp(0.0, 1.0);
        self.volume = volume.clamp(0.0, 1.0);
        self.update_mode_frequencies();
    }

    /// Updates mode frequency calculations for the current sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_mode_frequencies();
    }

    fn update_mode_frequencies(&mut self) {
        let material_shift = 0.7 + self.material * 0.6;
        let volume_shift = 0.5 + self.volume * 1.0;

        for i in 0..8 {
            self.base_freqs[i] = [220.0, 380.0, 550.0, 720.0, 890.0, 1060.0, 1230.0, 1400.0][i]
                * material_shift
                / volume_shift;
        }
    }

    /// Processes one sample through the body resonance bank.
    pub fn process(&mut self, input: f32) -> f32 {
        let mut sum = 0.0;

        for i in 0..8 {
            let f0 = self.base_freqs[i];
            let omega_norm = 2.0 * std::f32::consts::PI * f0 / self.sample_rate;
            let decay = 0.01 + self.material * 0.005 + (i as f32) * 0.002;

            let force = input * 0.05;
            let new_y = force + (2.0 - omega_norm * omega_norm - decay) * self.y[i]
                - (1.0 - decay) * self.y1[i];

            self.y1[i] = self.y[i];
            self.y[i] = new_y.clamp(-10.0, 10.0);

            let material_gain = 0.8 + self.material * 0.4;
            sum += self.y[i] * material_gain * (1.0 / (i + 1) as f32);
        }

        // Volume only retunes the modes (see `update_mode_frequencies`); the
        // mix level is set by the post chain's BODY amount.
        sum.clamp(-1.0, 1.0)
    }

    /// Clears the resonator state history.
    pub fn reset(&mut self) {
        self.y = [0.0; 8];
        self.y1 = [0.0; 8];
    }
}

impl Default for FemBodyResonator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn larger_chassis_resonates_lower() {
        let mut small = FemBodyResonator::new();
        small.set_sample_rate(48_000.0);
        small.set_parameters(0.5, 0.2);
        let mut large = FemBodyResonator::new();
        large.set_sample_rate(48_000.0);
        large.set_parameters(0.5, 0.8);
        assert!(
            large.base_freqs[0] < small.base_freqs[0],
            "lowest mode should move down: {} -> {}",
            small.base_freqs[0],
            large.base_freqs[0]
        );
    }

    #[test]
    fn volume_applies_no_output_gain() {
        // Same tuning, different `volume` field: if volume only retunes the
        // modes, the outputs must match exactly.
        for volume in [0.2, 0.8] {
            let mut tuned = FemBodyResonator::new();
            tuned.set_sample_rate(48_000.0);
            tuned.set_parameters(0.5, volume);
            let mut retuned_only = tuned.clone();
            retuned_only.volume = 1.0 - volume;
            for i in 0..4_800 {
                let x = if i == 0 { 1.0 } else { 0.0 };
                assert_eq!(
                    tuned.process(x),
                    retuned_only.process(x),
                    "volume {volume}, sample {i}"
                );
            }
        }
    }

    #[test]
    fn processes_without_nan() {
        let mut body = FemBodyResonator::new();
        body.set_sample_rate(48000.0);

        for i in 0..100 {
            let input = (i as f32 * 0.1).sin() * 0.5;
            let output = body.process(input);
            assert!(output.is_finite(), "Output should be finite");
        }
    }

    #[test]
    fn different_materials_produce_different_outputs() {
        let mut body1 = FemBodyResonator::new();
        let mut body2 = FemBodyResonator::new();
        body1.set_sample_rate(48000.0);
        body2.set_sample_rate(48000.0);
        body1.set_parameters(0.0, 0.5);
        body2.set_parameters(1.0, 0.5);

        let mut energy1 = 0.0f32;
        let mut energy2 = 0.0f32;

        for i in 0..480 {
            let input = if i == 0 { 1.0 } else { 0.0 };
            energy1 += body1.process(input).abs();
            energy2 += body2.process(input).abs();
        }

        assert!(
            (energy1 - energy2).abs() > 0.01,
            "Different materials should produce different energy"
        );
    }
}

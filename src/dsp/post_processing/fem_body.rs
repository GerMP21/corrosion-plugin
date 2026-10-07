//! FEM-style body resonator stage.
//!
//! This is a compact modal approximation used to add material-specific body
//! coloration after the drive stage.

use crate::dsp::utils::rate;
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

    /// Per-mode decay and the input gain for `sample_rate`. Both are tuned
    /// per 48 kHz sample and converted so the body rings for the same time
    /// at the same level at any rate (`fix-sample-rate-time-constants`);
    /// they are the original values at 48 kHz.
    decay: [f32; 8],
    force_gain: f32,
}

impl FemBodyResonator {
    /// Creates the body resonator with default material and volume.
    pub fn new() -> Self {
        let mut body = Self {
            material: 0.5,
            volume: 0.5,
            y: [0.0; 8],
            y1: [0.0; 8],
            base_freqs: [220.0, 380.0, 550.0, 720.0, 890.0, 1060.0, 1230.0, 1400.0],
            sample_rate: 48000.0,
            decay: [0.0; 8],
            force_gain: 0.05,
        };
        body.update_decay();
        body
    }

    /// Sets material and cavity volume controls. Volume sets the body's size:
    /// larger volumes move every mode lower. It applies no output gain, though
    /// lower modes ring with more resonant gain (see `process`).
    pub fn set_parameters(&mut self, material: f32, volume: f32) {
        self.material = material.clamp(0.0, 1.0);
        self.volume = volume.clamp(0.0, 1.0);
        self.update_mode_frequencies();
        self.update_decay();
    }

    /// Updates mode frequency calculations for the current sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_mode_frequencies();
        self.update_decay();
    }

    fn update_decay(&mut self) {
        let ratio = rate::ratio(self.sample_rate);
        for (i, decay) in self.decay.iter_mut().enumerate() {
            // `1 − decay` is the squared pole radius per 48 kHz sample.
            *decay = rate::smoother(0.01 + self.material * 0.005 + (i as f32) * 0.002, ratio);
        }
        // Each mode's gain grows as 1/ω², like the resonator's (see
        // `dsp::resonators::core`).
        self.force_gain = if ratio == 1.0 {
            0.05
        } else {
            0.05 * ratio * ratio
        };
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
            let decay = self.decay[i];

            let force = input * self.force_gain;
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

    /// Impulse response of the default body to a 1 ms unit pulse:
    /// (peak, seconds until the envelope falls 40 dB below that peak).
    fn pulse_response(sample_rate: f32) -> (f32, f32) {
        let mut body = FemBodyResonator::new();
        body.set_sample_rate(sample_rate);
        body.set_parameters(0.5, 0.5);
        let pulse = (sample_rate * 0.001).round() as usize;
        let out: Vec<f32> = (0..sample_rate as usize * 4)
            .map(|i| body.process(if i < pulse { 1.0 } else { 0.0 }))
            .collect();
        let peak = out.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
        let floor = peak * 0.01;
        let last_loud = out.iter().rposition(|x| x.abs() > floor).unwrap_or(0);
        (peak, last_loud as f32 / sample_rate)
    }

    #[test]
    fn body_sounds_the_same_at_any_sample_rate() {
        let (peak, ring) = pulse_response(48_000.0);
        for sample_rate in [44_100.0, 96_000.0] {
            let (p, r) = pulse_response(sample_rate);
            let db = 20.0 * (p / peak).log10();
            assert!(db.abs() <= 1.0, "{sample_rate}: peak {db:+.2} dB vs 48 kHz");
            assert!(
                (r / ring - 1.0).abs() <= 0.1,
                "{sample_rate}: rings {r:.3} s vs {ring:.3} s at 48 kHz"
            );
        }
    }

    /// 48 kHz hash of `pulse_response`'s render, for the before/after check
    /// in `fix-sample-rate-time-constants` (not asserted; libm differs
    /// between CI targets).
    #[test]
    #[ignore]
    fn print_48k_body_render_hash() {
        let mut body = FemBodyResonator::new();
        body.set_sample_rate(48_000.0);
        body.set_parameters(0.5, 0.5);
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for i in 0..96_000 {
            for byte in body
                .process(if i < 48 { 1.0 } else { 0.0 })
                .to_bits()
                .to_le_bytes()
            {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        }
        println!("HASH body: {hash:#018x}");
    }
}

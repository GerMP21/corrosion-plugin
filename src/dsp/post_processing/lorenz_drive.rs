//! Chaotic drive stage for the post chain.
//!
//! The drive combines a Lorenz attractor modulation path with a soft tube
//! saturation curve to introduce motion without breaking real-time safety.
//!
//! The amount (0–5) raises the input gain (`1 + amount / 2`) and crossfades
//! from the dry input to the saturated signal: wet share `amount / 5`, dry
//! share `1 − amount / 5`.
/// Chaotic drive processor with Lorenz modulation and tube saturation.
pub struct LorenzDrive {
    /// Smoothed drive amount; lags `target_drive_amount` to kill zipper noise.
    drive_amount: f32,
    target_drive_amount: f32,
    bias_starvation: f32,
    chaos_depth: f32,

    // Lorenz attractor state
    x: f32,
    y: f32,
    z: f32,

    // Tube saturation state
    tube_state: f32,

    sample_rate: f32,
    /// Per-sample one-pole smoother coefficient for `drive_amount`.
    smoothing_coeff: f32,
    /// First `set_parameters` snaps; later calls smooth.
    smoothing_primed: bool,
}

impl LorenzDrive {
    const SIGMA: f32 = 10.0;
    const BETA: f32 = 8.0 / 3.0;
    /// One-pole smoother tau in seconds for drive automation.
    const SMOOTHING_TAU_SECONDS: f32 = 0.02;

    /// Creates the drive with neutral settings and stable initial state.
    pub fn new() -> Self {
        let mut drive = Self {
            drive_amount: 0.0,
            target_drive_amount: 0.0,
            bias_starvation: 0.0,
            chaos_depth: 0.0,
            x: 0.1,
            y: 0.0,
            z: 0.0,
            tube_state: 0.0,
            sample_rate: 48000.0,
            smoothing_coeff: 0.0,
            smoothing_primed: false,
        };
        drive.recompute_smoothing_coeff();
        drive
    }

    /// Sets drive amount, bias starvation, and chaos depth.
    pub fn set_parameters(&mut self, drive: f32, starvation: f32, chaos: f32) {
        self.target_drive_amount = drive.clamp(0.0, 5.0);
        self.bias_starvation = starvation.clamp(0.0, 1.0);
        self.chaos_depth = chaos.clamp(0.0, 1.0);
        if !self.smoothing_primed {
            self.smoothing_primed = true;
            self.drive_amount = self.target_drive_amount;
        }
    }

    /// Updates the internal integration step for the current sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.recompute_smoothing_coeff();
    }

    fn recompute_smoothing_coeff(&mut self) {
        let sr = self.sample_rate.max(1.0);
        self.smoothing_coeff = 1.0 - (-1.0 / (Self::SMOOTHING_TAU_SECONDS * sr)).exp();
    }

    /// Processes one sample through the chaotic drive path.
    pub fn process(&mut self, input: f32) -> f32 {
        // Smooth drive automation at audio rate.
        self.drive_amount += (self.target_drive_amount - self.drive_amount) * self.smoothing_coeff;
        if self.drive_amount < 0.001 {
            return input;
        }

        let rho = 28.0;
        let dt = (1.0 / self.sample_rate).min(0.001);

        let driven = input * (1.0 + self.drive_amount * 0.5);

        let dx = Self::SIGMA * (self.y - self.x);
        let dy = self.x * (rho - self.z) - self.y;
        let dz = self.x * self.y - Self::BETA * self.z;

        self.x = (self.x + dx * dt).clamp(-50.0, 50.0);
        self.y = (self.y + dy * dt).clamp(-50.0, 50.0);
        self.z = (self.z + dz * dt).clamp(0.0, 100.0);

        let chaos_mod = 1.0 + self.chaos_depth * ((self.x * 0.02).tanh() * 0.5);

        let starvation_gate = if self.bias_starvation > 0.0 {
            let threshold = 1.0 - self.bias_starvation * 0.7;
            if ((self.z * 0.01).fract().abs() + 0.5) > threshold {
                0.3
            } else {
                1.0
            }
        } else {
            1.0
        };

        let tube_in = (driven * chaos_mod * starvation_gate).clamp(-5.0, 5.0);
        let saturated = Self::tube_saturate(tube_in);

        let wet = saturated.clamp(-1.0, 1.0);
        // A crossfade: the wet share runs from 0 to 1 across the 0–5 amount,
        // and the weights sum to 1, so mixing adds no level of its own
        // (`h01-unclamp-post-drive-amount`).
        let wet_amount = self.drive_amount / 5.0;
        let dry_wet = wet * wet_amount + input * (1.0 - wet_amount);

        dry_wet.clamp(-1.0, 1.0)
    }

    fn tube_saturate(x: f32) -> f32 {
        if x.abs() < 0.5 {
            x
        } else {
            let sign = x.signum();
            let abs_x = x.abs();
            sign * (0.5 + (abs_x - 0.5) / (1.0 + (abs_x - 0.5).powi(2)))
        }
    }

    /// Restores the attractor and saturation state.
    pub fn reset(&mut self) {
        self.x = 0.1;
        self.y = 0.0;
        self.z = 0.0;
        self.tube_state = 0.0;
        self.smoothing_primed = false;
    }
}

impl Default for LorenzDrive {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_drive_is_unity() {
        let mut drive = LorenzDrive::new();
        drive.set_sample_rate(48000.0);

        for i in 0..100 {
            let input = (i as f32 * 0.1).sin() * 0.5;
            let output = drive.process(input);
            assert!((output - input).abs() < 1e-3, "Zero drive should be unity");
        }
    }

    /// Gain a small, unsaturated signal gets at `amount` (chaos and
    /// starvation off): the mix crossfades dry and saturated, so it is
    /// `(1 − w)·1 + w·g`, with `w = amount/5` and the drive's input gain
    /// `g = 1 + amount/2` (`h01-unclamp-post-drive-amount`).
    fn small_signal_gain(amount: f32) -> f32 {
        let mut drive = LorenzDrive::new();
        drive.set_sample_rate(48000.0);
        drive.set_parameters(amount, 0.0, 0.0);
        let input = 0.01;
        drive.process(input) / input
    }

    #[test]
    fn mix_is_a_crossfade() {
        for (amount, expected) in [(2.5, 1.625), (5.0, 3.5)] {
            let gain = small_signal_gain(amount);
            assert!(
                (gain - expected).abs() < 1e-4,
                "amount {amount}: small-signal gain {gain}, crossfade gives {expected}"
            );
        }
        assert_eq!(small_signal_gain(0.0), 1.0);
    }

    #[test]
    fn wet_share_spans_the_whole_amount_range() {
        // Wet shares 0.2, 0.6 and 1.0 give small-signal gains 1.1, 1.9, 3.5.
        for (amount, expected) in [(1.0, 1.1), (3.0, 1.9), (5.0, 3.5)] {
            let gain = small_signal_gain(amount);
            assert!(
                (gain - expected).abs() < 1e-4,
                "amount {amount}: small-signal gain {gain}, expected {expected}"
            );
        }
    }

    #[test]
    fn output_bounded() {
        let mut drive = LorenzDrive::new();
        drive.set_sample_rate(48000.0);
        drive.set_parameters(5.0, 1.0, 1.0);

        for i in 0..1000 {
            let input = (i as f32 * 0.1).sin();
            let output = drive.process(input);
            assert!(output.abs() <= 1.6, "Output should be bounded");
        }
    }

    /// Largest sample-to-sample step of the drive's output for a 100 Hz
    /// sine at amplitude 0.5, Drive Amount 2.5, chaos off, over two seconds.
    fn largest_step(starvation: f32, sample_rate: f32) -> f32 {
        let mut drive = LorenzDrive::new();
        drive.set_sample_rate(sample_rate);
        drive.set_parameters(2.5, starvation, 0.0);
        let frames = (2.0 * sample_rate) as usize;
        let mut prev = 0.0f32;
        let mut largest = 0.0f32;
        for i in 0..frames {
            let t = i as f32 / sample_rate;
            let y = drive.process(0.5 * (std::f32::consts::TAU * 100.0 * t).sin());
            largest = largest.max((y - prev).abs());
            prev = y;
        }
        largest
    }

    #[test]
    #[ignore = "measurement: prints the largest output step per starvation value"]
    fn print_starvation_steps() {
        for sr in [44_100.0, 48_000.0, 96_000.0] {
            for s in [0.0, 0.1, 0.25, 0.35, 0.5, 0.75, 1.0] {
                let step = largest_step(s, sr);
                let base = largest_step(0.0, sr);
                println!(
                    "sr {sr} starvation {s:.2}: step {step:.5} ratio {:.3}",
                    step / base
                );
            }
        }
    }

    #[test]
    fn no_nan_produced() {
        let mut drive = LorenzDrive::new();
        drive.set_sample_rate(48000.0);
        drive.set_parameters(3.0, 0.5, 0.5);

        for i in 0..500 {
            let input = (i as f32 * 0.05).sin();
            let output = drive.process(input);
            assert!(output.is_finite(), "Output should be finite");
        }
    }
}

//! Chaotic drive stage for the post chain.
//!
//! The drive combines a Lorenz attractor modulation path with a soft tube
//! saturation curve to introduce motion without breaking real-time safety.
//!
//! The amount (0–5) raises the input gain (`1 + amount / 2`) and crossfades
//! from the dry input to the saturated signal: wet share `amount / 5`, dry
//! share `1 − amount / 5`.
//!
//! Bias Starvation (0–1) starves the drive's input gain to 0.3 while the
//! attractor's `z` is above a threshold that falls with the control
//! (`38 − 22·starvation`, spanning the attractor's `z` range), so the starved
//! share of time rises from about 5 % at 0.1 to about 87 % at 1. A 5 ms one-pole
//! follower ramps the gain between full and starved, so the switches sag
//! rather than click (`m06-m09-smooth-starvation-and-oversample-clipper`).
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

    /// Starvation gain the follower has reached: 1.0 (full) to 0.3.
    starvation_gain: f32,
    /// Per-sample one-pole coefficient for `starvation_gain`.
    starvation_coeff: f32,

    sample_rate: f32,
    /// Per-sample one-pole smoother coefficient for `drive_amount`.
    smoothing_coeff: f32,
    /// First `set_parameters` snaps; later calls smooth.
    smoothing_primed: bool,
    /// Test-only switch: `false` skips the starvation stage entirely, the
    /// reference for "starvation 0 is bit-identical to no starvation".
    #[cfg(test)]
    starvation_stage_enabled: bool,
}

impl LorenzDrive {
    const SIGMA: f32 = 10.0;
    const BETA: f32 = 8.0 / 3.0;
    /// One-pole smoother tau in seconds for drive automation.
    const SMOOTHING_TAU_SECONDS: f32 = 0.02;
    /// One-pole follower tau in seconds for the starvation gain.
    const STARVATION_TAU_SECONDS: f32 = 0.005;
    /// Input gain while starved.
    const STARVED_GAIN: f32 = 0.3;
    /// Starvation threshold on `z` is `TOP − SPAN·starvation`: about the
    /// attractor's 99th percentile at 0 and its 5th at 1.
    const STARVATION_Z_TOP: f32 = 38.0;
    const STARVATION_Z_SPAN: f32 = 22.0;
    /// Within this of full gain, the follower snaps to exactly 1.0 (and is
    /// skipped from then on while the target stays full).
    const STARVATION_SNAP: f32 = 1e-4;

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
            starvation_gain: 1.0,
            starvation_coeff: 0.0,
            sample_rate: 48000.0,
            smoothing_coeff: 0.0,
            smoothing_primed: false,
            #[cfg(test)]
            starvation_stage_enabled: true,
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
        self.starvation_coeff = 1.0 - (-1.0 / (Self::STARVATION_TAU_SECONDS * sr)).exp();
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

        #[cfg(test)]
        if !self.starvation_stage_enabled {
            let tube_in = (driven * chaos_mod).clamp(-5.0, 5.0);
            return self.mix(input, tube_in);
        }

        let starvation_gate = self.follow_starvation();

        let tube_in = (driven * chaos_mod * starvation_gate).clamp(-5.0, 5.0);
        self.mix(input, tube_in)
    }

    /// The gain the starvation decision asks for right now: starved (0.3)
    /// while `z` is above the threshold, else full (1.0).
    fn starvation_target(&self) -> f32 {
        let threshold = Self::STARVATION_Z_TOP - Self::STARVATION_Z_SPAN * self.bias_starvation;
        if self.bias_starvation > 0.0 && self.z > threshold {
            Self::STARVED_GAIN
        } else {
            1.0
        }
    }

    /// Moves the starvation gain one sample toward its target and returns it.
    /// At full gain with a full target (always, at starvation 0) it returns
    /// exactly 1.0 without touching the follower.
    #[inline(always)]
    fn follow_starvation(&mut self) -> f32 {
        let target = self.starvation_target();
        if target == 1.0 && self.starvation_gain == 1.0 {
            return 1.0;
        }
        self.starvation_gain += (target - self.starvation_gain) * self.starvation_coeff;
        if target == 1.0 && self.starvation_gain >= 1.0 - Self::STARVATION_SNAP {
            self.starvation_gain = 1.0;
        }
        self.starvation_gain
    }

    /// Saturates the tube input and crossfades it with the dry input.
    #[inline(always)]
    fn mix(&self, input: f32, tube_in: f32) -> f32 {
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

    /// Restores the attractor and the starvation gain.
    pub fn reset(&mut self) {
        self.x = 0.1;
        self.y = 0.0;
        self.z = 0.0;
        self.starvation_gain = 1.0;
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

    /// Bias Starvation values the no-clicks and range scenarios sweep.
    const CLICK_SWEEP: [f32; 5] = [0.1, 0.25, 0.35, 0.5, 1.0];

    /// Values whose largest step exceeds 1.5× the starvation-0 step at
    /// `sample_rate`, as `(starvation, ratio)`.
    fn clicking_values(sample_rate: f32) -> Vec<(f32, f32)> {
        let base = largest_step(0.0, sample_rate);
        CLICK_SWEEP
            .iter()
            .map(|&s| (s, largest_step(s, sample_rate) / base))
            .filter(|&(_, ratio)| ratio > 1.5)
            .collect()
    }

    #[test]
    fn starvation_does_not_click() {
        let clicks = clicking_values(48_000.0);
        assert!(
            clicks.is_empty(),
            "largest step over 1.5x the starvation-0 step at 48 kHz: {clicks:?}"
        );
    }

    #[test]
    fn starvation_does_not_click_at_other_rates() {
        for sr in [44_100.0, 96_000.0] {
            let clicks = clicking_values(sr);
            assert!(
                clicks.is_empty(),
                "largest step over 1.5x the starvation-0 step at {sr} Hz: {clicks:?}"
            );
        }
    }

    #[test]
    fn starvation_off_is_bit_identical() {
        // Starvation 0 from the start, and starvation 0.5 for a second then 0:
        // both must match the drive without the starvation stage, the second
        // once its gain has settled back to full (0.2 s, 40 time constants).
        let sr = 48_000.0;
        let run = |starvation_first_second: f32| {
            let mut drive = LorenzDrive::new();
            let mut reference = LorenzDrive::new();
            reference.starvation_stage_enabled = false;
            for d in [&mut drive, &mut reference] {
                d.set_sample_rate(sr);
                d.set_parameters(2.5, 0.0, 0.5);
            }
            drive.set_parameters(2.5, starvation_first_second, 0.5);
            let settle = if starvation_first_second > 0.0 {
                9_600
            } else {
                0
            };
            for i in 0..96_000usize {
                if i == 48_000 {
                    drive.set_parameters(2.5, 0.0, 0.5);
                }
                let x = 0.7 * (std::f32::consts::TAU * 110.0 * i as f32 / sr).sin();
                let (y, y_ref) = (drive.process(x), reference.process(x));
                if i >= 48_000 + settle || starvation_first_second == 0.0 {
                    assert_eq!(
                        y.to_bits(),
                        y_ref.to_bits(),
                        "sample {i}, starvation {starvation_first_second} first"
                    );
                }
            }
        };
        run(0.0);
        run(0.5);
    }

    /// Share of samples the starvation decision asks for the starved gain,
    /// over sixty seconds at 48 kHz, skipping the attractor's first second.
    /// Shorter windows aren't representative: after `reset()` the attractor
    /// spirals around one lobe for several seconds with `z` in about 23–30.
    fn starved_share(starvation: f32) -> f32 {
        let sr = 48_000.0;
        let mut drive = LorenzDrive::new();
        drive.set_sample_rate(sr);
        drive.set_parameters(2.5, starvation, 0.0);
        let (skip, frames) = (48_000usize, 61 * 48_000usize);
        let mut starved = 0usize;
        for i in 0..frames {
            drive.process(0.0);
            if i >= skip && drive.starvation_target() < 1.0 {
                starved += 1;
            }
        }
        starved as f32 / (frames - skip) as f32
    }

    const RANGE_SWEEP: [f32; 5] = [0.1, 0.25, 0.5, 0.75, 1.0];

    #[test]
    fn starvation_keeps_switching_across_its_range() {
        let stuck: Vec<(f32, f32)> = RANGE_SWEEP
            .iter()
            .map(|&s| (s, starved_share(s)))
            .filter(|&(_, share)| !(0.01..=0.99).contains(&share))
            .collect();
        assert!(stuck.is_empty(), "starved share outside 1-99 %: {stuck:?}");
    }

    #[test]
    fn more_starvation_starves_more_often() {
        let shares: Vec<f32> = RANGE_SWEEP.iter().map(|&s| starved_share(s)).collect();
        assert!(
            shares.windows(2).all(|w| w[1] > w[0]),
            "starved share must rise with Bias Starvation {RANGE_SWEEP:?}: {shares:?}"
        );
    }

    #[test]
    #[ignore = "measurement: prints the starved share per starvation value"]
    fn print_starved_shares() {
        for s in [0.05, 0.1, 0.25, 0.35, 0.5, 0.75, 1.0] {
            println!("starvation {s:.2}: starved share {:.3}", starved_share(s));
        }
    }

    #[test]
    #[ignore = "measurement: prints the attractor's z percentiles"]
    fn print_z_percentiles() {
        for sr in [44_100.0f32, 48_000.0, 96_000.0] {
            let mut drive = LorenzDrive::new();
            drive.set_sample_rate(sr);
            drive.set_parameters(2.5, 0.0, 0.0);
            let n = (20.0 * sr) as usize;
            let mut zs = Vec::with_capacity(n);
            for i in 0..n {
                drive.process(0.0);
                if i as f32 >= sr {
                    zs.push(drive.z);
                }
            }
            zs.sort_by(f32::total_cmp);
            let q = |p: f32| zs[((zs.len() - 1) as f32 * p) as usize];
            println!(
                "sr {sr}: min {:.1} p1 {:.1} p5 {:.1} p25 {:.1} p50 {:.1} p75 {:.1} p95 {:.1} p99 {:.1} max {:.1}",
                q(0.0), q(0.01), q(0.05), q(0.25), q(0.5), q(0.75), q(0.95), q(0.99), q(1.0)
            );
        }
        // trace of z for the first seconds at 48 kHz
        let mut drive = LorenzDrive::new();
        drive.set_sample_rate(48_000.0);
        drive.set_parameters(2.5, 0.0, 0.0);
        for i in 0..480_000 {
            drive.process(0.0);
            if i % 24_000 == 0 {
                println!(
                    "t {:.1}s x {:.2} y {:.2} z {:.2}",
                    i as f32 / 48_000.0,
                    drive.x,
                    drive.y,
                    drive.z
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

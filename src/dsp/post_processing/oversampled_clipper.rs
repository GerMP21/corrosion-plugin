//! Oversampled final clipper for the post chain.
//!
//! The input is up-sampled through cascaded 2× half-band IIR stages, the
//! diode curve runs on every high-rate sample, and the same stages bring it
//! back down. Normal, High and Render use 2, 3 and 4 stages (4×, 8×, 16×);
//! Eco clips at the host rate. When oversampling, the curve starts at the
//! ceiling ÷ 1.3, leaving room for the filters' overshoot under the output
//! limiter. Below that the stage is transparent (flat to 22 kHz at 48 kHz);
//! above it the harmonics the curve creates are filtered before they fold
//! back. The half-bands delay low frequencies by
//! about 5–6 samples, which isn't reported as latency
//! (`m06-m09-smooth-starvation-and-oversample-clipper`).
use super::half_band::{HalfBandCoefs, HalfBandState};

/// Most 2× stages (16× oversampling).
const MAX_STAGES: usize = 4;
/// Most high-rate samples per input sample.
const MAX_FACTOR: usize = 1 << MAX_STAGES;
/// Stop-band rejection each half-band is designed for.
const STAGE_ATTENUATION_DB: f64 = 90.0;
/// Pass-band edge as a fraction of the host rate (22 kHz at 48 kHz).
const PASS_EDGE: f64 = 22.0 / 48.0;
/// When oversampling, the curve clips at the ceiling divided by this, so the
/// down-sampling filters' overshoot (the harmonics they remove were holding
/// the peaks down) mostly stays under the output limiter that follows. 1.3
/// keeps a 5 kHz tone at twice the ceiling under the hard clamp.
const CEILING_HEADROOM: f32 = 1.3;

/// Final soft clipper with configurable oversampled processing.
pub struct OversampledClipper {
    ceiling: f32,
    softness: f32,
    /// Where the curve starts clipping when oversampling:
    /// `ceiling / CEILING_HEADROOM`.
    oversampled_threshold: f32,
    /// 2× stages in use: 0 (1×) to `MAX_STAGES` (16×).
    stages: usize,
    /// Stage `i` runs between `2^i` and `2^(i+1)` times the host rate.
    coefs: [HalfBandCoefs; MAX_STAGES],
    /// Up- and down-sampling memory per channel and stage.
    up: [[HalfBandState; MAX_STAGES]; 2],
    down: [[HalfBandState; MAX_STAGES]; 2],
}

impl OversampledClipper {
    /// Creates the clipper with a conservative ceiling and 16× oversampling.
    pub fn new() -> Self {
        let coefs = std::array::from_fn(|stage| {
            // The pass edge stays fixed relative to the host rate, so later
            // stages, at higher rates, get ever wider transitions.
            let transition = 0.25 - PASS_EDGE / f64::from(2u32 << stage);
            HalfBandCoefs::design(STAGE_ATTENUATION_DB, transition)
        });
        Self {
            ceiling: 0.9661,
            softness: 0.5,
            oversampled_threshold: 0.9661 / CEILING_HEADROOM,
            stages: MAX_STAGES,
            coefs,
            up: [[HalfBandState::default(); MAX_STAGES]; 2],
            down: [[HalfBandState::default(); MAX_STAGES]; 2],
        }
    }

    /// Sets the clipping ceiling and knee softness.
    pub fn set_parameters(&mut self, ceiling: f32, softness: f32) {
        self.ceiling = ceiling.clamp(0.5, 1.0);
        self.softness = softness.clamp(0.0, 1.0);
        self.oversampled_threshold = self.ceiling / CEILING_HEADROOM;
    }

    /// Sets the oversampling factor: 1, 2, 4, 8 or 16 (others round down to
    /// a power of two). A change clears the filters' memory.
    pub fn set_oversample_factor(&mut self, factor: usize) {
        let factor = factor.clamp(1, MAX_FACTOR);
        let stages = (usize::BITS - 1 - factor.leading_zeros()) as usize;
        if stages != self.stages {
            self.stages = stages;
            self.reset();
        }
    }

    /// Processes one mono sample through the oversampled limiter.
    pub fn process(&mut self, input: f32) -> f32 {
        self.process_channel(input, 0)
    }

    /// Up-samples one channel stage by stage, clips every high-rate sample,
    /// and down-samples back through the stages in reverse.
    fn process_channel(&mut self, input: f32, channel: usize) -> f32 {
        if self.stages == 0 {
            return self.diode_clip(input, self.ceiling);
        }

        let mut a = [0.0f32; MAX_FACTOR];
        let mut b = [0.0f32; MAX_FACTOR];
        a[0] = input;
        let mut len = 1;
        for stage in 0..self.stages {
            let (coefs, state) = (&self.coefs[stage], &mut self.up[channel][stage]);
            for i in 0..len {
                let pair = state.upsample(coefs, a[i]);
                b[2 * i] = pair[0];
                b[2 * i + 1] = pair[1];
            }
            len *= 2;
            std::mem::swap(&mut a, &mut b);
        }

        let threshold = self.oversampled_threshold;
        for sample in a.iter_mut().take(len) {
            *sample = self.diode_clip(*sample, threshold);
        }

        for stage in (0..self.stages).rev() {
            let (coefs, state) = (&self.coefs[stage], &mut self.down[channel][stage]);
            len /= 2;
            for i in 0..len {
                b[i] = state.downsample(coefs, [a[2 * i], a[2 * i + 1]]);
            }
            std::mem::swap(&mut a, &mut b);
        }
        a[0]
    }

    #[inline(always)]
    fn diode_clip(&self, x: f32, threshold: f32) -> f32 {
        let abs_x = x.abs();

        if abs_x <= threshold {
            x
        } else {
            let sign = x.signum();
            let excess = abs_x - threshold;
            let knee = 2.0 + self.softness * 3.0;
            let compressed = excess / (1.0 + excess * knee);
            sign * (threshold + compressed * (1.0 - threshold) * 0.1)
        }
    }

    /// Processes a stereo frame by clipping each channel independently.
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        (
            self.process_channel(left, 0),
            self.process_channel(right, 1),
        )
    }

    /// Clears the filters' memory.
    pub fn reset(&mut self) {
        for state in self.up.iter_mut().chain(self.down.iter_mut()).flatten() {
            state.reset();
        }
    }
}

impl Default for OversampledClipper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "measurement: prints the clipper's peak output per quality"]
    fn print_peaks() {
        for factor in [1, 4, 8, 16] {
            let mut step = OversampledClipper::new();
            step.set_parameters(0.8, 0.0);
            step.set_oversample_factor(factor);
            let step_peak = (0..100)
                .map(|_| step.process(2.0).abs())
                .fold(0.0, f32::max);
            print!("{factor:>2}x: step 0->2 at ceiling 0.8 peaks {step_peak:.4};");
            for hz in [100.0, 1_000.0, 5_000.0, 10_000.0, 15_000.0] {
                let mut c = OversampledClipper::new();
                c.set_oversample_factor(factor);
                let peak = (0..48_000)
                    .map(|i| {
                        let x =
                            2.0 * 0.9661 * (std::f32::consts::TAU * hz * i as f32 / 48_000.0).sin();
                        c.process(x).abs()
                    })
                    .skip(4_800)
                    .fold(0.0, f32::max);
                print!(" {hz} Hz at 2x ceiling 0.9661 peaks {peak:.4};");
            }
            println!();
        }
    }

    #[test]
    #[ignore = "measurement: worst peak over the curve's threshold, per factor"]
    fn print_overshoot_sweep() {
        let threshold = 0.8f32;
        for factor in [4, 8, 16] {
            let mut worst = (0.0f32, 0.0f32, 0.0f32);
            for over in [1.0f32, 1.1, 1.25, 1.5, 2.0, 3.0, 4.0, 8.0, 16.0] {
                for hz in [
                    50.0f32, 100.0, 300.0, 1_000.0, 2_000.0, 3_000.0, 5_000.0, 7_000.0, 10_000.0,
                    13_000.0, 15_000.0, 19_000.0,
                ] {
                    for softness in [0.0f32, 0.5, 1.0] {
                        let mut c = OversampledClipper::new();
                        c.set_parameters(threshold, softness);
                        c.set_oversample_factor(factor);
                        let peak = (0..24_000)
                            .map(|i| {
                                let x = over
                                    * threshold
                                    * (std::f32::consts::TAU * hz * i as f32 / 48_000.0).sin();
                                c.process(x).abs()
                            })
                            .skip(2_400)
                            .fold(0.0, f32::max);
                        if peak / threshold > worst.0 {
                            worst = (peak / threshold, over, hz);
                        }
                    }
                }
            }
            println!(
                "{factor:>2}x: worst peak {:.4}x threshold at {}x, {} Hz",
                worst.0, worst.1, worst.2
            );
        }
    }

    #[test]
    fn never_exceeds_ceiling() {
        let mut clipper = OversampledClipper::new();
        clipper.set_parameters(0.8, 0.0);

        for _ in 0..100 {
            let input = 2.0;
            let output = clipper.process(input);
            assert!(
                output.abs() <= 0.81,
                "Output should not exceed ceiling significantly"
            );
        }
    }

    #[test]
    fn preserves_low_levels() {
        // A slow sine under the threshold keeps its peak level in every mode
        // (the half-bands delay it by a few samples, so compare peaks, not
        // samples; `output-clipper` checks gain and delay precisely).
        for factor in [1, 4, 8, 16] {
            let mut clipper = OversampledClipper::new();
            clipper.set_parameters(0.9661, 0.0);
            clipper.set_oversample_factor(factor);
            let peak = (0..2_000)
                .map(|i| clipper.process((i as f32 * 0.01).sin() * 0.3).abs())
                .skip(1_000)
                .fold(0.0, f32::max);
            assert!(
                (peak - 0.3).abs() < 0.003,
                "{factor}x: low-level peak {peak}, expected 0.3"
            );
        }
    }

    #[test]
    fn oversampling_factor_changes_output_on_transients() {
        // Drive both clippers with the same signal that swings past the ceiling.
        // With real interpolation across inputs, a higher factor band-limits the
        // clip differently, so the outputs must diverge on the transients.
        let inputs = [0.0, 2.0, -2.0, 1.5, -1.8, 0.4, -0.4, 2.5];

        let mut c1 = OversampledClipper::new();
        c1.set_parameters(0.8, 0.5);
        c1.set_oversample_factor(1);

        let mut c16 = OversampledClipper::new();
        c16.set_parameters(0.8, 0.5);
        c16.set_oversample_factor(16);

        let mut total_diff = 0.0f32;
        for &x in inputs.iter() {
            total_diff += (c1.process(x) - c16.process(x)).abs();
        }

        assert!(
            total_diff > 0.001,
            "1x and 16x oversampling must differ on transients, diff={total_diff}"
        );
    }

    #[test]
    fn stereo_channels_have_independent_state() {
        // The two channels must not share interpolation state.
        let mut clipper = OversampledClipper::new();
        clipper.set_parameters(0.8, 0.5);
        clipper.set_oversample_factor(8);

        let (l1, r1) = clipper.process_stereo(2.0, -2.0);
        assert!(l1.is_finite() && r1.is_finite());
        // Left rising from 0, right falling from 0 — opposite-signed inputs
        // should yield opposite-signed clipped outputs.
        assert!(
            l1 > 0.0 && r1 < 0.0,
            "channels must track their own input sign"
        );
    }

    /// `output-clipper`: Eco is the diode curve on each sample, unfiltered.
    #[test]
    fn eco_is_plain_clipping() {
        let mut clipper = OversampledClipper::new();
        clipper.set_parameters(0.9661, 0.5);
        clipper.set_oversample_factor(1);
        for i in 0..4_800 {
            let x = 2.0 * (i as f32 * 0.37).sin() * (i as f32 * 0.0011).cos();
            assert_eq!(
                clipper.process(x).to_bits(),
                clipper.diode_clip(x, 0.9661).to_bits(),
                "sample {i}"
            );
        }
    }

    #[test]
    fn no_nan_produced() {
        let mut clipper = OversampledClipper::new();

        for i in 0..1000 {
            let input = (i as f32 * 0.1).sin() * 2.0;
            let output = clipper.process(input);
            assert!(output.is_finite(), "Output should be finite");
        }
    }
}

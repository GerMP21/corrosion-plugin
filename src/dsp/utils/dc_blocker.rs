//! A stereo DC blocker for the voice mix (`block-voice-mix-dc`).

/// Removes DC and the slow, sub-audible part of a signal with a
/// second-order Butterworth high-pass (bilinear transform with a prewarped
/// cutoff, so the cutoff is the same in hertz at every sample rate).
///
/// Coefficients and state are `f64`: at a 10 Hz cutoff the poles sit about
/// 0.001 inside the unit circle, where an `f32` direct form would amplify
/// its own rounding noise by about 10⁶ at DC. Allocation-free.
#[derive(Clone, Debug)]
pub struct StereoDcBlocker {
    cutoff_hz: f32,
    sample_rate: f32,
    b0: f64,
    b1: f64,
    a1: f64,
    a2: f64,
    /// Per channel: x[n−1], x[n−2], y[n−1], y[n−2].
    state: [[f64; 4]; 2],
}

impl StereoDcBlocker {
    /// A blocker at `cutoff_hz`, rated for 48 kHz until `set_sample_rate`.
    pub fn new(cutoff_hz: f32) -> Self {
        let mut blocker = Self {
            cutoff_hz,
            sample_rate: 0.0,
            b0: 0.0,
            b1: 0.0,
            a1: 0.0,
            a2: 0.0,
            state: [[0.0; 4]; 2],
        };
        blocker.set_sample_rate(48_000.0);
        blocker
    }

    /// Recompute the coefficients for `sample_rate`; a no-op when it is
    /// unchanged.
    #[inline]
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sample_rate = sample_rate.max(1.0);
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
            self.rebuild();
        }
    }

    #[cold]
    #[inline(never)]
    fn rebuild(&mut self) {
        let w0 = std::f64::consts::TAU * self.cutoff_hz as f64 / self.sample_rate as f64;
        let (cos, sin) = (w0.cos(), w0.sin());
        let alpha = sin * std::f64::consts::FRAC_1_SQRT_2; // sin / (2Q), Q = 1/√2
        let a0 = 1.0 + alpha;
        self.b0 = (1.0 + cos) / 2.0 / a0;
        self.b1 = -(1.0 + cos) / a0;
        self.a1 = -2.0 * cos / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Clear the filter's state.
    pub fn reset(&mut self) {
        self.state = [[0.0; 4]; 2];
    }

    /// One stereo frame without its DC.
    #[inline]
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        (self.channel(0, left), self.channel(1, right))
    }

    #[inline]
    fn channel(&mut self, channel: usize, x: f32) -> f32 {
        let x = x as f64;
        let [x1, x2, y1, y2] = self.state[channel];
        // b2 = b0 for a high-pass.
        let y = self.b0 * (x + x2) + self.b1 * x1 - self.a1 * y1 - self.a2 * y2;
        self.state[channel] = [x, x1, y, y1];
        y as f32
    }
}

#[cfg(test)]
mod tests {
    use super::StereoDcBlocker;

    #[test]
    fn removes_a_constant() {
        let mut blocker = StereoDcBlocker::new(10.0);
        let mut last = (0.0, 0.0);
        for _ in 0..48_000 {
            last = blocker.process(0.5, -0.25);
        }
        assert!(last.0.abs() < 1e-6 && last.1.abs() < 1e-6, "{last:?}");
    }

    #[test]
    fn reset_clears_the_state() {
        let mut blocker = StereoDcBlocker::new(10.0);
        let _ = blocker.process(1.0, 1.0);
        blocker.reset();
        assert_eq!(blocker.process(0.0, 0.0), (0.0, 0.0));
    }

    /// The cutoff is −3 dB at every rate.
    #[test]
    fn cutoff_is_the_same_in_hertz_at_every_rate() {
        for rate in [44_100.0f32, 48_000.0, 96_000.0, 192_000.0] {
            let mut blocker = StereoDcBlocker::new(10.0);
            blocker.set_sample_rate(rate);
            let frames = (4.0 * rate) as usize;
            let (mut energy_in, mut energy_out) = (0.0f64, 0.0f64);
            for n in 0..frames {
                let x = (std::f64::consts::TAU * 10.0 * n as f64 / rate as f64).sin() as f32;
                let (y, _) = blocker.process(x, x);
                if n >= frames / 2 {
                    energy_in += (x as f64).powi(2);
                    energy_out += (y as f64).powi(2);
                }
            }
            let gain_db = 10.0 * (energy_out / energy_in).log10();
            assert!((gain_db + 3.01).abs() < 0.05, "{rate} Hz: {gain_db:.3} dB");
        }
    }
}

//! Lookahead peak limiter for the master output stage.
//!
//! `LookaheadLimiter` introduces a fixed delay so it can react to upcoming
//! peaks *before* they reach the output. It computes a gain reduction from
//! the maximum absolute peak in the lookahead window, applies a fast attack
//! and a slower release, and outputs the delayed input scaled by the gain.
//!
//! Compared to the hard `apply_output_limiter` clamp, the lookahead path
//! avoids inter-sample distortion at the cost of `LOOKAHEAD_SAMPLES` of
//! latency (~1 ms at 48 kHz). Allocation-free; ring buffers are fixed-size.

/// Fixed lookahead window in samples. 48 samples ≈ 1 ms at 48 kHz, ≈ 0.5 ms
/// at 96 kHz. Short enough to keep latency negligible for a synth voice;
/// long enough to catch transient peaks ahead of the clip ceiling.
pub const LOOKAHEAD_SAMPLES: usize = 48;

/// Fixed-window lookahead limiter with smooth gain.
#[derive(Clone, Debug)]
pub struct LookaheadLimiter {
    /// Delay line holding the in-flight samples.
    buffer: [f32; LOOKAHEAD_SAMPLES],
    /// Current write index into `buffer`.
    write_pos: usize,
    /// Ceiling above which the gain starts pulling the signal down. Mirrors
    /// the hard limiter's threshold so the two paths produce comparable
    /// peaks for unprocessed input.
    threshold: f32,
    /// Current gain reduction applied to the delayed sample. `1.0` is unity;
    /// values below `1.0` attenuate.
    gain: f32,
    /// One-pole coefficient for the release ramp; `0.0` snaps to target,
    /// values close to `1.0` smooth the release. Attack is instantaneous so
    /// the limiter never fails to catch a peak.
    release_coeff: f32,
    sample_rate: f32,
}

impl LookaheadLimiter {
    /// Release time in seconds. `50 ms` — short enough that the limiter
    /// recovers between transients without pumping perceptibly on sustained
    /// material.
    const RELEASE_SECONDS: f32 = 0.05;

    /// Build a limiter with the given peak threshold (`0.5..=1.0` typical).
    pub fn new(threshold: f32) -> Self {
        let mut limiter = Self {
            buffer: [0.0; LOOKAHEAD_SAMPLES],
            write_pos: 0,
            threshold: threshold.clamp(0.1, 1.0),
            gain: 1.0,
            release_coeff: 0.0,
            sample_rate: 48_000.0,
        };
        limiter.recompute_release();
        limiter
    }

    /// Update the sample rate and recompute the release coefficient.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.recompute_release();
    }

    /// Update the peak ceiling without resetting the delay line.
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.1, 1.0);
    }

    fn recompute_release(&mut self) {
        // Standard one-pole coefficient for a target tau in seconds.
        self.release_coeff = (-1.0 / (Self::RELEASE_SECONDS * self.sample_rate)).exp();
    }

    /// Reset the delay line and gain history. Use at note pauses or after a
    /// host reset to avoid leaking the previous buffer into a fresh render.
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.gain = 1.0;
    }

    /// Push one sample into the delay line and pull the limited output.
    ///
    /// The returned sample is `LOOKAHEAD_SAMPLES` old; downstream code must
    /// accept the corresponding latency.
    pub fn process(&mut self, input: f32) -> f32 {
        // Pull the delayed sample, then overwrite with the new input so the
        // next call sees this sample as the "oldest" in the buffer.
        let delayed = self.buffer[self.write_pos];
        self.buffer[self.write_pos] = input;
        self.write_pos = (self.write_pos + 1) % LOOKAHEAD_SAMPLES;

        // Find the max peak in the lookahead window. With a tiny fixed-size
        // window the linear scan is cheaper than a max-heap or sorted ring.
        let mut peak = 0.0f32;
        for &sample in &self.buffer {
            let abs = sample.abs();
            if abs > peak {
                peak = abs;
            }
        }

        let target_gain = if peak > self.threshold {
            self.threshold / peak
        } else {
            1.0
        };

        if target_gain < self.gain {
            // Instant attack — never miss a peak.
            self.gain = target_gain;
        } else {
            // Smooth release back to unity (or to the new lower ceiling).
            self.gain = target_gain + (self.gain - target_gain) * self.release_coeff;
        }

        // Final safety clamp catches the edge case where two peaks slip
        // through the window boundary at the same gain step.
        (delayed * self.gain).clamp(-self.threshold, self.threshold)
    }
}

impl Default for LookaheadLimiter {
    fn default() -> Self {
        Self::new(0.9661)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_quiet_signal_unchanged_after_latency() {
        // Sub-threshold input must come out untouched once the delay flushes.
        let mut limiter = LookaheadLimiter::new(0.9);
        limiter.set_sample_rate(48_000.0);

        // Prime the delay with zeros.
        for _ in 0..LOOKAHEAD_SAMPLES {
            limiter.process(0.0);
        }
        // Push a sine that stays below threshold and read the latency-aligned
        // output.
        let mut inputs = Vec::with_capacity(256);
        for i in 0..256 {
            let s = (i as f32 * 0.1).sin() * 0.5;
            inputs.push(s);
            let out = limiter.process(s);
            if i >= LOOKAHEAD_SAMPLES {
                let expected = inputs[i - LOOKAHEAD_SAMPLES];
                assert!(
                    (out - expected).abs() < 1e-4,
                    "quiet signal should pass cleanly: expected={expected}, got={out}"
                );
            }
        }
    }

    #[test]
    fn limits_peaks_above_threshold() {
        let threshold = 0.5;
        let mut limiter = LookaheadLimiter::new(threshold);
        limiter.set_sample_rate(48_000.0);

        let mut peak_out = 0.0_f32;
        // Drive a square pulse 2x the threshold.
        for i in 0..256 {
            let s = if i % 10 < 5 { 1.0 } else { -1.0 };
            let out = limiter.process(s);
            peak_out = peak_out.max(out.abs());
        }
        assert!(
            peak_out <= threshold + 1e-3,
            "output peak should stay at/below threshold: peak={peak_out}, threshold={threshold}"
        );
    }

    #[test]
    fn detects_peak_before_it_arrives_at_output() {
        // The whole point of lookahead: the gain is already reduced when the
        // peak reaches the output sample. We feed a single big spike and
        // verify the delayed spike comes out below the threshold without a
        // clip artifact.
        let threshold = 0.5;
        let mut limiter = LookaheadLimiter::new(threshold);
        limiter.set_sample_rate(48_000.0);

        // Fill with zeros, then a single huge spike.
        let mut spike_out = 0.0_f32;
        for i in 0..LOOKAHEAD_SAMPLES + 16 {
            let input = if i == 4 { 5.0 } else { 0.0 };
            let out = limiter.process(input);
            spike_out = spike_out.max(out.abs());
        }
        assert!(
            spike_out <= threshold + 1e-3,
            "spike should be limited even though gain is computed from lookahead window: out={spike_out}"
        );
        assert!(
            spike_out > 0.05,
            "spike should still produce a real output, not a hole: out={spike_out}"
        );
    }

    #[test]
    fn reset_clears_delay_line() {
        // After reset, the limiter should produce zeros for the latency
        // window even if it was previously processing loud signal.
        let mut limiter = LookaheadLimiter::new(0.9);
        limiter.set_sample_rate(48_000.0);
        for _ in 0..256 {
            limiter.process(0.8);
        }
        limiter.reset();
        for _ in 0..LOOKAHEAD_SAMPLES {
            let out = limiter.process(0.0);
            assert_eq!(out, 0.0, "reset must clear the delay buffer");
        }
    }
}

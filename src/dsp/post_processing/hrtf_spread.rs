//! HRTF-inspired stereo spread stage.
//!
//! The spreader applies short delay, simple spectral shaping, and crossfeed to
//! widen the stereo image while keeping the signal coherent.
/// Stereo spreader with delay-based width and proximity shaping.
pub struct HrtfSpread {
    spread_width: f32,
    listener_proximity: f32,

    // Delay lines for spatialization
    delay_buffer_left: [f32; 64],
    delay_buffer_right: [f32; 64],
    delay_index: usize,

    // Filter states for HRTF approximation
    left_lpf: f32,
    right_lpf: f32,
    left_hpf: f32,
    right_hpf: f32,

    sample_rate: f32,
}

impl HrtfSpread {
    /// Creates the spreader with centered, neutral settings.
    pub fn new() -> Self {
        Self {
            spread_width: 0.5,
            listener_proximity: 0.5,
            delay_buffer_left: [0.0; 64],
            delay_buffer_right: [0.0; 64],
            delay_index: 0,
            left_lpf: 0.0,
            right_lpf: 0.0,
            left_hpf: 0.0,
            right_hpf: 0.0,
            sample_rate: 48000.0,
        }
    }

    /// Sets stereo width and listener proximity.
    pub fn set_parameters(&mut self, width: f32, proximity: f32) {
        self.spread_width = width.clamp(0.0, 1.0);
        self.listener_proximity = proximity.clamp(0.0, 1.0);
    }

    /// Updates delay timing for the current sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Processes one stereo frame through the spread stage.
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        if self.spread_width < 0.001 {
            return (left, right);
        }

        let mono = (left + right) * 0.5;
        let stereo_diff = (left - right) * 0.5;

        // Calculate delay based on width (interaural time difference)
        let max_delay_samples = (self.sample_rate * 0.001) as usize; // 1ms max
        let delay_samples = (max_delay_samples as f32 * self.spread_width) as usize;

        // Store in delay buffers
        self.delay_buffer_left[self.delay_index] = mono;
        self.delay_buffer_right[self.delay_index] = mono;

        // Read delayed samples
        let delayed_left =
            self.delay_buffer_left[(self.delay_index + 64 - delay_samples.min(63)) % 64];
        let delayed_right =
            self.delay_buffer_right[(self.delay_index + 64 - delay_samples.min(63)) % 64];

        self.delay_index = (self.delay_index + 1) % 64;

        // Proximity affects spectral balance
        // Closer = more lows and highs, less mids
        let proximity_factor = self.listener_proximity;

        // Low-pass filter state (for proximity effect)
        self.left_lpf = self.left_lpf * 0.8 + delayed_left * 0.2;
        self.right_lpf = self.right_lpf * 0.8 + delayed_right * 0.2;

        // High-pass component
        self.left_hpf = self.left_hpf * 0.5 + (delayed_left - self.left_lpf) * 0.5;
        self.right_hpf = self.right_hpf * 0.5 + (delayed_right - self.right_lpf) * 0.5;

        // Mix based on proximity
        let left_out = self.left_lpf * (0.5 + proximity_factor * 0.3)
            + self.left_hpf * (0.3 + proximity_factor * 0.2)
            + stereo_diff * (1.0 - self.spread_width * 0.5);

        let right_out = self.right_lpf * (0.5 + proximity_factor * 0.3)
            + self.right_hpf * (0.3 + proximity_factor * 0.2)
            - stereo_diff * (1.0 - self.spread_width * 0.5);

        // Apply width-based crossfeed
        let crossfeed = self.spread_width * 0.3;
        let final_left = left_out * (1.0 - crossfeed) + right_out * crossfeed;
        let final_right = right_out * (1.0 - crossfeed) + left_out * crossfeed;

        (final_left.clamp(-1.5, 1.5), final_right.clamp(-1.5, 1.5))
    }

    /// Clears delay and filter state.
    pub fn reset(&mut self) {
        self.delay_buffer_left.fill(0.0);
        self.delay_buffer_right.fill(0.0);
        self.delay_index = 0;
        self.left_lpf = 0.0;
        self.right_lpf = 0.0;
        self.left_hpf = 0.0;
        self.right_hpf = 0.0;
    }
}

impl Default for HrtfSpread {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_width_preserves_stereo() {
        let mut spread = HrtfSpread::new();
        spread.set_sample_rate(48000.0);
        spread.set_parameters(0.0, 0.5);

        let left = 0.7;
        let right = 0.3;
        let (out_l, out_r) = spread.process(left, right);

        assert!(
            (out_l - left).abs() < 1e-3,
            "Zero width should preserve left"
        );
        assert!(
            (out_r - right).abs() < 1e-3,
            "Zero width should preserve right"
        );
    }

    #[test]
    fn output_bounded() {
        let mut spread = HrtfSpread::new();
        spread.set_sample_rate(48000.0);
        spread.set_parameters(1.0, 1.0);

        for i in 0..100 {
            let left = (i as f32 * 0.1).sin();
            let right = (i as f32 * 0.1 + 1.0).cos();
            let (out_l, out_r) = spread.process(left, right);
            assert!(out_l.abs() <= 1.5, "Left should be bounded");
            assert!(out_r.abs() <= 1.5, "Right should be bounded");
        }
    }
}

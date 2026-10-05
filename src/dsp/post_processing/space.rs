//! Space algorithms used by the post-processing chain.
//!
//! The available modes cover a factory room reverb, a spring tank, and a
//! feedback echo, all intended to match the industrial sound vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Selects which spatial effect is active in the post chain.
pub enum SpaceMode {
    Off,
    Factory,
    Spring,
    Echo,
}

/// Factory-room reverb approximation with comb and allpass diffusion.
pub struct FactoryReverb {
    size: f32,
    clutter: f32,
    wall_impedance: f32,

    // Comb filters for FDTD approximation
    comb_buffers: [[f32; 2048]; 4],
    comb_indices: [usize; 4],
    comb_delays: [usize; 4],
    // Immutable base delays; comb_delays is always recomputed from these so
    // repeated updates can't compound.
    base_comb_delays: [usize; 4],

    // Allpass filters
    allpass_buffers: [[f32; 512]; 2],
    allpass_indices: [usize; 2],

    // Diffusion network
    diffusion_y: [f32; 4],

    sample_rate: f32,
}

impl FactoryReverb {
    /// Creates the factory reverb with default size and damping.
    pub fn new() -> Self {
        Self {
            size: 0.5,
            clutter: 0.3,
            wall_impedance: 0.5,
            comb_buffers: [[0.0; 2048]; 4],
            comb_indices: [0; 4],
            comb_delays: [1553, 1657, 1789, 1913],
            base_comb_delays: [1553, 1657, 1789, 1913],
            allpass_buffers: [[0.0; 512]; 2],
            allpass_indices: [0; 2],
            diffusion_y: [0.0; 4],
            sample_rate: 48000.0,
        }
    }

    /// Sets room size, clutter, and wall impedance.
    pub fn set_parameters(&mut self, size: f32, clutter: f32, impedance: f32) {
        self.size = size.clamp(0.0, 1.0);
        self.clutter = clutter.clamp(0.0, 1.0);
        self.wall_impedance = impedance.clamp(0.0, 1.0);
        self.update_delays();
    }

    /// Recomputes delay relationships for the current sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_delays();
    }

    fn update_delays(&mut self) {
        let size_scale = 0.5 + self.size * 1.5;
        for i in 0..4 {
            let scaled = (self.base_comb_delays[i] as f32 * size_scale) as usize;
            self.comb_delays[i] = scaled.clamp(100, 2000);
            // Keep the read/write cursor valid if the delay shrank.
            self.comb_indices[i] %= self.comb_delays[i];
        }
    }

    /// Processes one mono sample through the room model.
    pub fn process(&mut self, input: f32) -> f32 {
        if self.size < 0.001 {
            return 0.0;
        }

        // Comb filter section (FDTD room modes)
        let mut comb_sum = 0.0;
        let feedback = 0.7 + self.wall_impedance * 0.28;
        let clutter_mod = 1.0 - self.clutter * 0.3;

        for i in 0..4 {
            let delay = self.comb_delays[i];
            let buffer_val = self.comb_buffers[i][self.comb_indices[i]];

            let new_val = input + buffer_val * feedback * clutter_mod;
            self.comb_buffers[i][self.comb_indices[i]] = new_val;
            self.comb_indices[i] = (self.comb_indices[i] + 1) % delay;

            comb_sum += buffer_val;
        }

        let comb_out = comb_sum * 0.25;

        // Allpass diffusion
        let mut ap_out = comb_out;
        for i in 0..2 {
            let delay = 200 + i * 100;
            let buffer_val = self.allpass_buffers[i][self.allpass_indices[i]];
            let new_val = ap_out + buffer_val * 0.5;
            self.allpass_buffers[i][self.allpass_indices[i]] = new_val;
            self.allpass_indices[i] = (self.allpass_indices[i] + 1) % delay;
            ap_out = buffer_val - ap_out * 0.5;
        }

        ap_out * self.size
    }

    /// Clears the delay and diffusion state.
    pub fn reset(&mut self) {
        self.comb_buffers = [[0.0; 2048]; 4];
        self.allpass_buffers = [[0.0; 512]; 2];
        self.diffusion_y = [0.0; 4];
    }
}

impl Default for FactoryReverb {
    fn default() -> Self {
        Self::new()
    }
}

/// Spring-tank reverb approximation with dispersion.
pub struct SpringReverb {
    tension: f32,
    stiffness: f32,
    tank_size: f32,

    // Delay line for spring simulation
    delay_buffer: [f32; 8192],
    delay_index: usize,
    delay_length: usize,

    // Dispersion filter (creates the "pew" sound)
    dispersion_state: f32,

    sample_rate: f32,
}

impl SpringReverb {
    /// Creates the spring reverb with default tension and tank size.
    pub fn new() -> Self {
        Self {
            tension: 0.5,
            stiffness: 0.5,
            tank_size: 0.5,
            delay_buffer: [0.0; 8192],
            delay_index: 0,
            delay_length: 4000,
            dispersion_state: 0.0,
            sample_rate: 48000.0,
        }
    }

    /// Sets spring tension, stiffness, and tank size.
    pub fn set_parameters(&mut self, tension: f32, stiffness: f32, tank_size: f32) {
        self.tension = tension.clamp(0.0, 1.0);
        self.stiffness = stiffness.clamp(0.0, 1.0);
        self.tank_size = tank_size.clamp(0.0, 1.0);
        self.update_delay();
    }

    /// Recomputes the delay length for the current sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_delay();
    }

    fn update_delay(&mut self) {
        let base_delay = (self.sample_rate * 0.08) as usize; // 80ms base
        self.delay_length = (base_delay as f32 * (0.5 + self.tank_size)) as usize;
        self.delay_length = self.delay_length.clamp(1000, 8000);
    }

    /// Processes one mono sample through the spring model.
    pub fn process(&mut self, input: f32) -> f32 {
        if self.tank_size < 0.001 {
            return 0.0;
        }

        let delay_sample = self.delay_buffer[self.delay_index];

        // Feedback with dispersion (stiffness creates high-freq outrun)
        let dispersion_coeff = 0.5 + self.stiffness * 0.4;
        self.dispersion_state =
            self.dispersion_state * (1.0 - dispersion_coeff) + delay_sample * dispersion_coeff;

        let feedback = 0.6 + self.tension * 0.35;
        let new_sample = input + self.dispersion_state * feedback;

        self.delay_buffer[self.delay_index] = new_sample;
        self.delay_index = (self.delay_index + 1) % self.delay_length;

        // Output with sloshy character
        delay_sample * self.tank_size * 0.7
    }

    /// Clears the spring state and rewinds the delay line.
    pub fn reset(&mut self) {
        self.delay_buffer = [0.0; 8192];
        self.delay_index = 0;
        self.dispersion_state = 0.0;
    }
}

impl Default for SpringReverb {
    fn default() -> Self {
        Self::new()
    }
}

/// Stereo echo with gentle modulation and damping.
pub struct FactoryEcho {
    delay_time: f32,
    machinery_movement: f32,
    high_frequency_damping: f32,
    delay_buffer_left: [f32; 16384],
    delay_buffer_right: [f32; 16384],
    write_index: usize,
    sample_rate: f32,
    phase: f32,
}

impl FactoryEcho {
    /// Creates the factory echo with default delay and damping.
    pub fn new() -> Self {
        Self {
            delay_time: 0.25,
            machinery_movement: 0.0,
            high_frequency_damping: 0.5,
            delay_buffer_left: [0.0; 16384],
            delay_buffer_right: [0.0; 16384],
            write_index: 0,
            sample_rate: 48000.0,
            phase: 0.0,
        }
    }

    /// Sets delay time, machinery movement, and HF damping.
    pub fn set_parameters(
        &mut self,
        delay_time: f32,
        machinery_movement: f32,
        high_frequency_damping: f32,
    ) {
        self.delay_time = delay_time.clamp(0.0, 1.0);
        self.machinery_movement = machinery_movement.clamp(0.0, 1.0);
        self.high_frequency_damping = high_frequency_damping.clamp(0.0, 1.0);
    }

    /// Stores the active sample rate used to derive delay times.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Processes one stereo frame through the echo lines.
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        let base_delay = (0.02 + self.delay_time * 0.78) * self.sample_rate;
        self.phase += (0.05 + self.machinery_movement * 2.0) / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        let modulation = (self.phase * std::f32::consts::TAU).sin() * self.machinery_movement * 0.1;
        let delay_samples = (base_delay * (1.0 + modulation)).clamp(1.0, 16383.0) as usize;
        let read_index = (self.write_index + 16384 - delay_samples) % 16384;

        let wet_left_raw = self.delay_buffer_left[read_index];
        let wet_right_raw = self.delay_buffer_right[read_index];

        let damp = 1.0 - self.high_frequency_damping.clamp(0.0, 1.0) * 0.6;
        let wet_left = wet_left_raw * damp;
        let wet_right = wet_right_raw * damp;

        self.delay_buffer_left[self.write_index] = left + wet_left * 0.45;
        self.delay_buffer_right[self.write_index] = right + wet_right * 0.45;
        self.write_index = (self.write_index + 1) % 16384;

        (wet_left, wet_right)
    }

    /// Clears the echo buffers and modulation phase.
    pub fn reset(&mut self) {
        self.delay_buffer_left = [0.0; 16384];
        self.delay_buffer_right = [0.0; 16384];
        self.write_index = 0;
        self.phase = 0.0;
    }
}

impl Default for FactoryEcho {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_processes_without_nan() {
        let mut reverb = FactoryReverb::new();
        reverb.set_sample_rate(48000.0);

        for i in 0..100 {
            let input = if i == 0 { 1.0 } else { 0.0 };
            let output = reverb.process(input);
            assert!(output.is_finite(), "Factory output should be finite");
        }
    }

    #[test]
    fn spring_processes_without_nan() {
        let mut reverb = SpringReverb::new();
        reverb.set_sample_rate(48000.0);

        for i in 0..100 {
            let input = if i == 0 { 1.0 } else { 0.0 };
            let output = reverb.process(input);
            assert!(output.is_finite(), "Spring output should be finite");
        }
    }

    #[test]
    fn factory_size_scales_delays_monotonically() {
        let mut small = FactoryReverb::new();
        small.set_sample_rate(48000.0);
        small.set_parameters(0.2, 0.3, 0.5);

        let mut large = FactoryReverb::new();
        large.set_sample_rate(48000.0);
        large.set_parameters(0.9, 0.3, 0.5);

        for i in 0..4 {
            assert!(
                large.comb_delays[i] > small.comb_delays[i],
                "larger factory_size should yield longer comb delays (idx {i}): \
                 large={} small={}",
                large.comb_delays[i],
                small.comb_delays[i]
            );
        }
    }

    #[test]
    fn factory_delays_stable_across_repeated_updates() {
        // Regression: update_delays used to rescale comb_delays from their own
        // prior values, so repeated identical updates railed them to the clamp.
        let mut reverb = FactoryReverb::new();
        reverb.set_sample_rate(48000.0);
        reverb.set_parameters(0.5, 0.3, 0.5);
        let first = reverb.comb_delays;

        for _ in 0..1000 {
            reverb.set_parameters(0.5, 0.3, 0.5);
        }

        assert_eq!(
            first, reverb.comb_delays,
            "repeated identical updates must not drift the comb delays"
        );
    }

    #[test]
    fn zero_size_produces_zero() {
        let mut reverb = FactoryReverb::new();
        reverb.set_sample_rate(48000.0);
        reverb.set_parameters(0.0, 0.5, 0.5);

        let output = reverb.process(1.0);
        assert_eq!(output, 0.0, "Zero size should produce zero output");
    }

    #[test]
    fn echo_processes_without_nan() {
        let mut echo = FactoryEcho::new();
        echo.set_sample_rate(48000.0);
        echo.set_parameters(0.5, 0.5, 0.5);

        for i in 0..200 {
            let input = if i == 0 { 1.0 } else { 0.0 };
            let (left, right) = echo.process_stereo(input, input);
            assert!(left.is_finite());
            assert!(right.is_finite());
        }
    }
}

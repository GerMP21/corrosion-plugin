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

/// The highest sample rate the Space section sizes its buffers for. Every
/// delay is defined in time and converted with the host rate, so no control
/// position clamps up to this rate (`h10-scale-space-effects-with-sample-rate`).
pub const MAX_SAMPLE_RATE: f32 = 192_000.0;

/// The rate the Space section's delays were tuned at. Delays are written as
/// sample counts at this rate and scaled by `sample_rate / REFERENCE_RATE`,
/// which is exactly 1 here, so 48 kHz delays are the tuned values.
const REFERENCE_RATE: f32 = 48_000.0;

/// `samples_at_48k` (a delay tuned at 48 kHz) at `sample_rate`, times
/// `scale`, as a whole number of samples within `1..capacity`.
fn delay_samples(samples_at_48k: f32, sample_rate: f32, scale: f32, capacity: usize) -> usize {
    let samples = samples_at_48k * scale * (sample_rate / REFERENCE_RATE);
    (samples as usize).clamp(1, capacity - 1)
}

/// Buffer length for a delay of up to `max_samples_at_48k` at
/// `MAX_SAMPLE_RATE`, with a little margin.
const fn capacity_for(max_samples_at_48k: usize) -> usize {
    max_samples_at_48k * (MAX_SAMPLE_RATE as usize / REFERENCE_RATE as usize) + 8
}

/// A zeroed delay buffer, allocated when the processor is built (never on
/// the audio thread).
fn delay_buffer(capacity: usize) -> Box<[f32]> {
    vec![0.0; capacity].into_boxed_slice()
}

/// The Factory combs' delays at 48 kHz and size 0.5's neutral scale of 1:
/// mutually prime, so the combs stay distinct at every size and rate.
const COMB_DELAYS_48K: [usize; 4] = [1553, 1657, 1789, 1913];
/// The largest size scale (`0.5 + 1.5·size` at size 1).
const MAX_SIZE_SCALE: usize = 2;
const COMB_CAPACITY: usize = capacity_for(1913 * MAX_SIZE_SCALE);
/// The Factory allpass delays at 48 kHz.
const ALLPASS_DELAYS_48K: [usize; 2] = [200, 300];
const ALLPASS_CAPACITY: usize = capacity_for(300);

/// Factory-room reverb approximation with comb and allpass diffusion.
pub struct FactoryReverb {
    size: f32,
    clutter: f32,
    wall_impedance: f32,

    // Comb filters for FDTD approximation. Sized for the longest delay at
    // `MAX_SAMPLE_RATE`; `comb_delays` sets how much of each is in use.
    comb_buffers: [Box<[f32]>; 4],
    comb_indices: [usize; 4],
    comb_delays: [usize; 4],

    // Allpass filters
    allpass_buffers: [Box<[f32]>; 2],
    allpass_indices: [usize; 2],
    allpass_delays: [usize; 2],

    sample_rate: f32,
}

impl FactoryReverb {
    /// Creates the factory reverb with default size and damping.
    pub fn new() -> Self {
        Self {
            size: 0.5,
            clutter: 0.3,
            wall_impedance: 0.5,
            comb_buffers: std::array::from_fn(|_| delay_buffer(COMB_CAPACITY)),
            comb_indices: [0; 4],
            comb_delays: COMB_DELAYS_48K,
            allpass_buffers: std::array::from_fn(|_| delay_buffer(ALLPASS_CAPACITY)),
            allpass_indices: [0; 2],
            allpass_delays: ALLPASS_DELAYS_48K,
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

    /// Recompute the delays from their 48 kHz values, the size and the
    /// sample rate (never from the previous delays, so repeated updates
    /// can't compound).
    fn update_delays(&mut self) {
        let size_scale = 0.5 + self.size * 1.5;
        let combs = self.comb_delays.iter_mut().zip(&mut self.comb_indices);
        for ((delay, index), base) in combs.zip(COMB_DELAYS_48K) {
            *delay = delay_samples(base as f32, self.sample_rate, size_scale, COMB_CAPACITY);
            // Keep the read/write cursor valid if the delay shrank.
            *index %= *delay;
        }
        let allpasses = self
            .allpass_delays
            .iter_mut()
            .zip(&mut self.allpass_indices);
        for ((delay, index), base) in allpasses.zip(ALLPASS_DELAYS_48K) {
            *delay = delay_samples(base as f32, self.sample_rate, 1.0, ALLPASS_CAPACITY);
            *index %= *delay;
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
            let delay = self.allpass_delays[i];
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
        self.comb_buffers
            .iter_mut()
            .for_each(|buffer| buffer.fill(0.0));
        self.allpass_buffers
            .iter_mut()
            .for_each(|buffer| buffer.fill(0.0));
    }
}

impl Default for FactoryReverb {
    fn default() -> Self {
        Self::new()
    }
}

/// The spring's longest delay (80 ms × 1.5 at the largest tank), at 48 kHz.
const SPRING_CAPACITY: usize = capacity_for(3840 * 3 / 2);

/// Spring-tank reverb approximation with dispersion.
pub struct SpringReverb {
    tension: f32,
    stiffness: f32,
    tank_size: f32,

    // Delay line for spring simulation, sized for the largest tank at
    // `MAX_SAMPLE_RATE`.
    delay_buffer: Box<[f32]>,
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
            delay_buffer: delay_buffer(SPRING_CAPACITY),
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
        // A guard only: the largest tank fits at every supported rate.
        self.delay_length = self.delay_length.clamp(1, SPRING_CAPACITY - 1);
        // A shorter delay must not leave the write position outside the loop.
        self.delay_index %= self.delay_length;
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
        self.delay_buffer.fill(0.0);
        self.delay_index = 0;
        self.dispersion_state = 0.0;
    }
}

impl Default for SpringReverb {
    fn default() -> Self {
        Self::new()
    }
}

/// The echo's longest delay at 48 kHz: 0.8 s with +10 % modulation.
const ECHO_CAPACITY: usize = capacity_for(38_400 * 11 / 10);

/// Stereo echo with gentle modulation and damping.
pub struct FactoryEcho {
    delay_time: f32,
    machinery_movement: f32,
    high_frequency_damping: f32,
    // Sized for the longest delay at `MAX_SAMPLE_RATE`.
    delay_buffer_left: Box<[f32]>,
    delay_buffer_right: Box<[f32]>,
    write_index: usize,
    sample_rate: f32,
    phase: f32,
    /// One-pole low-pass in the feedback path (High Frequency Damping): its
    /// coefficient, 1 when bypassed, and its per-channel state
    /// (`m07-m08-interpolate-echo-and-tidy-spread`).
    damping_coeff: f32,
    damping_state: [f32; 2],
    /// Machinery Movement as the modulation actually uses it, smoothed
    /// toward the knob (`ECHO_MOVEMENT_SMOOTHING_SECONDS`), and the
    /// smoother's per-sample coefficient. Without it, changing the knob
    /// jumped the delay to wherever the free-running LFO was: tens of
    /// samples, an audible tick.
    movement_smoothed: f32,
    movement_coeff: f32,
}

/// The echo's wet level. It was `1 − 0.6 · HF Damping` (the old broadband
/// "damping"); this is that gain at the default 0.5, so the repeats' low end
/// is unchanged at the default and the knob now sets tone only.
const ECHO_WET_GAIN: f32 = 0.7;
/// High Frequency Damping's low-pass cutoff at 0 (bypassed) and at 1.
const ECHO_DAMPING_CUTOFF_OPEN_HZ: f32 = 20_000.0;
const ECHO_DAMPING_CUTOFF_DARK_HZ: f32 = 1_500.0;
/// Time constant of the Machinery Movement smoother (as the master drive's).
const ECHO_MOVEMENT_SMOOTHING_SECONDS: f32 = 0.02;

/// 4-point, 3rd-order Hermite interpolation between `x0` and `x1` at `t`
/// (0..1), with neighbours `xm1` and `x2`. Returns `x0` exactly at `t = 0`.
#[inline]
fn hermite(xm1: f32, x0: f32, x1: f32, x2: f32, t: f32) -> f32 {
    let c1 = 0.5 * (x1 - xm1);
    let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
    let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
    ((c3 * t + c2) * t + c1) * t + x0
}

impl FactoryEcho {
    /// Creates the factory echo with default delay and damping.
    pub fn new() -> Self {
        Self {
            delay_time: 0.25,
            machinery_movement: 0.0,
            high_frequency_damping: 0.5,
            delay_buffer_left: delay_buffer(ECHO_CAPACITY),
            delay_buffer_right: delay_buffer(ECHO_CAPACITY),
            write_index: 0,
            sample_rate: 48000.0,
            phase: 0.0,
            damping_coeff: 1.0,
            damping_state: [0.0; 2],
            movement_smoothed: 0.0,
            movement_coeff: 1.0,
        }
        .with_damping_updated()
        .with_movement_smoother_updated()
    }

    fn with_damping_updated(mut self) -> Self {
        self.update_damping();
        self
    }

    fn with_movement_smoother_updated(mut self) -> Self {
        self.update_movement_smoother();
        self
    }

    fn update_movement_smoother(&mut self) {
        self.movement_coeff =
            1.0 - (-1.0 / (ECHO_MOVEMENT_SMOOTHING_SECONDS * self.sample_rate.max(1.0))).exp();
    }

    /// Re-derive the damping low-pass coefficient from the knob and the
    /// rate: the cutoff falls exponentially from 20 kHz (filter bypassed,
    /// coefficient exactly 1, at 0) to 1.5 kHz at 1, and the coefficient
    /// comes from the cutoff in hertz, so the tone is the same at every rate.
    fn update_damping(&mut self) {
        let amount = self.high_frequency_damping;
        self.damping_coeff = if amount <= 0.0 {
            1.0
        } else {
            let cutoff = ECHO_DAMPING_CUTOFF_OPEN_HZ
                * (ECHO_DAMPING_CUTOFF_DARK_HZ / ECHO_DAMPING_CUTOFF_OPEN_HZ).powf(amount);
            1.0 - (-std::f32::consts::TAU * cutoff / self.sample_rate.max(1.0)).exp()
        };
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
        self.update_damping();
    }

    /// Stores the active sample rate used to derive delay times.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update_damping();
        self.update_movement_smoother();
    }

    /// Processes one stereo frame through the echo lines.
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        let base_delay = (0.02 + self.delay_time * 0.78) * self.sample_rate;
        // The modulation follows the knob through a smoother, so turning
        // Machinery Movement (or automating it) moves the delay gradually.
        // It snaps to exactly 0 once the knob is at 0 and the remainder is
        // inaudible, so an unmodulated echo reads whole samples again.
        self.movement_smoothed +=
            self.movement_coeff * (self.machinery_movement - self.movement_smoothed);
        if self.machinery_movement == 0.0 && self.movement_smoothed < 1e-6 {
            self.movement_smoothed = 0.0;
        }
        let movement = self.movement_smoothed;
        self.phase += (0.05 + movement * 2.0) / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        let modulation = (self.phase * std::f32::consts::TAU).sin() * movement * 0.1;
        // Unmodulated, the delay is rounded to a whole sample (f32 puts 0.8 s
        // at 38 399.998 samples at 48 kHz), so repeats stay single samples at
        // the configured time. Modulated, it is read between samples with a
        // Hermite interpolator, so the sweep glides instead of repeating and
        // skipping samples (`m07-m08-interpolate-echo-and-tidy-spread`). The
        // clamp is a guard only: the longest delay fits at every supported
        // rate, and 2 samples keep the interpolator's newest point written.
        let delay = if movement > 0.0 {
            base_delay * (1.0 + modulation)
        } else {
            base_delay.round()
        }
        .clamp(2.0, (ECHO_CAPACITY - 3) as f32);
        let whole = delay.floor();
        let t = delay - whole;
        let whole = whole as usize;
        let at = |offset: usize| (self.write_index + ECHO_CAPACITY - offset) % ECHO_CAPACITY;
        let (i0, i1, i2, i3) = (at(whole - 1), at(whole), at(whole + 1), at(whole + 2));
        let read = |buffer: &[f32]| {
            if t == 0.0 {
                buffer[i1]
            } else {
                hermite(buffer[i0], buffer[i1], buffer[i2], buffer[i3], t)
            }
        };
        let wet_left_raw = read(&self.delay_buffer_left);
        let wet_right_raw = read(&self.delay_buffer_right);

        // High Frequency Damping: a one-pole low-pass inside the feedback
        // loop, so each repeat is darker than the last; bypassed at 0.
        let k = self.damping_coeff;
        let (wet_left, wet_right) = if k < 1.0 {
            self.damping_state[0] += k * (wet_left_raw - self.damping_state[0]);
            self.damping_state[1] += k * (wet_right_raw - self.damping_state[1]);
            (
                self.damping_state[0] * ECHO_WET_GAIN,
                self.damping_state[1] * ECHO_WET_GAIN,
            )
        } else {
            (wet_left_raw * ECHO_WET_GAIN, wet_right_raw * ECHO_WET_GAIN)
        };

        self.delay_buffer_left[self.write_index] = left + wet_left * 0.45;
        self.delay_buffer_right[self.write_index] = right + wet_right * 0.45;
        self.write_index = (self.write_index + 1) % ECHO_CAPACITY;

        (wet_left, wet_right)
    }

    /// Clears the echo buffers and modulation phase.
    pub fn reset(&mut self) {
        self.delay_buffer_left.fill(0.0);
        self.delay_buffer_right.fill(0.0);
        self.write_index = 0;
        self.phase = 0.0;
        self.damping_state = [0.0; 2];
        self.movement_smoothed = 0.0;
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
    fn spring_reset_rewinds_the_delay_line() {
        let mut reverb = SpringReverb::new();
        reverb.set_sample_rate(48_000.0);
        for i in 0..1_234 {
            reverb.process((i as f32 * 0.1).sin());
        }
        assert_ne!(reverb.delay_index, 0);
        reverb.reset();
        assert_eq!(reverb.delay_index, 0);
    }

    #[test]
    fn spring_delay_index_stays_inside_a_shorter_delay() {
        // Default tank 0.5 at 48 kHz: 3,840 samples. Park the write position
        // past where the tank-0 delay (1,920) ends, then shrink the delay.
        let mut reverb = SpringReverb::new();
        reverb.set_sample_rate(48_000.0);
        for _ in 0..3_000 {
            reverb.process(0.1);
        }
        reverb.set_parameters(0.5, 0.5, 0.0);
        assert_eq!(reverb.delay_length, 1_920);
        assert!(reverb.delay_index < reverb.delay_length);
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

    // --- Sample-rate independence (h10-scale-space-effects-with-sample-rate) ---

    const RATES: [f32; 5] = [44_100.0, 48_000.0, 88_200.0, 96_000.0, 192_000.0];

    /// Seconds for the Schroeder energy decay of `output` to fall 30 dB.
    fn t30(output: &[f32], sample_rate: f32) -> f32 {
        let mut remaining: f64 = output.iter().map(|&y| (y as f64) * (y as f64)).sum();
        let total = remaining;
        for (i, &y) in output.iter().enumerate() {
            if remaining <= total * 1e-3 {
                return i as f32 / sample_rate;
            }
            remaining -= (y as f64) * (y as f64);
        }
        f32::INFINITY
    }

    /// Index of the first sample above 1e-6 in magnitude.
    fn first_arrival(output: &[f32]) -> usize {
        output
            .iter()
            .position(|y| y.abs() > 1e-6)
            .unwrap_or(usize::MAX)
    }

    fn factory_impulse(sample_rate: f32, size: f32, seconds: f32) -> Vec<f32> {
        let mut reverb = FactoryReverb::new();
        reverb.set_sample_rate(sample_rate);
        reverb.set_parameters(size, 0.3, 0.5);
        (0..(seconds * sample_rate) as usize)
            .map(|i| reverb.process(if i == 0 { 1.0 } else { 0.0 }))
            .collect()
    }

    fn echo_impulse(sample_rate: f32, delay_time: f32) -> Vec<f32> {
        let mut echo = FactoryEcho::new();
        echo.set_sample_rate(sample_rate);
        echo.set_parameters(delay_time, 0.0, 0.0);
        (0..(0.9 * sample_rate) as usize)
            .map(|i| echo.process_stereo(if i == 0 { 1.0 } else { 0.0 }, 0.0).0)
            .collect()
    }

    fn spring_impulse(sample_rate: f32, tank: f32) -> Vec<f32> {
        let mut spring = SpringReverb::new();
        spring.set_sample_rate(sample_rate);
        spring.set_parameters(0.5, 0.5, tank);
        (0..(0.2 * sample_rate) as usize)
            .map(|i| spring.process(if i == 0 { 1.0 } else { 0.0 }))
            .collect()
    }

    #[test]
    fn factory_decay_does_not_depend_on_the_sample_rate() {
        let reference = t30(&factory_impulse(48_000.0, 0.5, 4.0), 48_000.0);
        let times: Vec<f32> = RATES
            .iter()
            .map(|&rate| t30(&factory_impulse(rate, 0.5, 4.0), rate))
            .collect();
        println!("FACTORY T30 at {RATES:?} Hz: {times:.4?} s");
        for rate in RATES {
            let time = t30(&factory_impulse(rate, 0.5, 4.0), rate);
            println!("FACTORY T30 {rate}: {time:.4} s");
            assert!(
                (time / reference - 1.0).abs() <= 0.05,
                "{rate} Hz: T30 {time:.4} s vs {reference:.4} s at 48 kHz"
            );
        }
    }

    #[test]
    fn echo_reaches_its_full_delay_at_every_rate() {
        for rate in RATES {
            let arrival = first_arrival(&echo_impulse(rate, 1.0));
            let expected = 0.8 * rate;
            println!(
                "ECHO 0.8 s {rate}: first echo at {arrival} samples ({:.4} s)",
                arrival as f32 / rate
            );
            assert!(
                (arrival as f32 - expected).abs() <= 1.0,
                "{rate} Hz: first echo at {arrival}, expected {expected}"
            );
        }
    }

    #[test]
    fn spring_delay_does_not_depend_on_the_sample_rate() {
        let at = |rate: f32| first_arrival(&spring_impulse(rate, 1.0)) as f32 / rate;
        let (low, high) = (at(48_000.0), at(192_000.0));
        println!("SPRING tank 1: {low:.5} s at 48 kHz, {high:.5} s at 192 kHz");
        assert!(
            (low - high).abs() <= 1.0 / 48_000.0,
            "tank 1: first echo {low:.5} s at 48 kHz, {high:.5} s at 192 kHz"
        );
    }

    #[test]
    fn echo_delay_time_covers_its_whole_range() {
        let rate = 96_000.0;
        let delays: Vec<f32> = (0..=20)
            .map(|step| first_arrival(&echo_impulse(rate, step as f32 / 20.0)) as f32 / rate)
            .collect();
        println!("ECHO sweep at 96 kHz: {delays:.4?}");
        assert!(
            (delays[0] - 0.02).abs() <= 1.0 / rate,
            "Delay Time 0: {:.5} s",
            delays[0]
        );
        assert!(
            (delays[20] - 0.8).abs() <= 1.0 / rate,
            "Delay Time 1: {:.5} s",
            delays[20]
        );
        for pair in delays.windows(2) {
            assert!(pair[1] > pair[0], "not rising: {delays:.4?}");
        }
    }

    #[test]
    fn factory_combs_stay_distinct_at_every_size() {
        for size in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let mut reverb = FactoryReverb::new();
            reverb.set_sample_rate(48_000.0);
            reverb.set_parameters(size, 0.3, 0.5);
            let delays = reverb.comb_delays;
            println!("COMBS size {size}: {delays:?}");
            for i in 0..4 {
                for j in i + 1..4 {
                    assert_ne!(delays[i], delays[j], "size {size}: combs {delays:?}");
                }
            }
        }
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

    // --- Echo modulation and HF Damping (`m07-m08-interpolate-echo-and-tidy-spread`) ---

    /// The echo's wet (left) output for `input`, `frames` long.
    fn echo_run(
        sample_rate: f32,
        delay_time: f32,
        movement: f32,
        damping: f32,
        frames: usize,
        input: impl Fn(usize) -> f32,
    ) -> Vec<f32> {
        let mut echo = FactoryEcho::new();
        echo.set_sample_rate(sample_rate);
        echo.set_parameters(delay_time, movement, damping);
        (0..frames)
            .map(|i| echo.process_stereo(input(i), 0.0).0)
            .collect()
    }

    fn max_step(x: &[f32]) -> f32 {
        x.windows(2).fold(0.0f32, |m, w| m.max((w[1] - w[0]).abs()))
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / x.len().max(1) as f64).sqrt()
            as f32
    }

    /// Echo delay in samples for Delay Time `delay_time`, unmodulated.
    fn echo_delay(sample_rate: f32, delay_time: f32) -> usize {
        ((0.02 + delay_time * 0.78) * sample_rate).round() as usize
    }

    /// Spec "Sine through a moving delay": no repeated or skipped samples.
    #[test]
    fn echo_modulation_glides() {
        let sr = 48_000.0;
        let sine = |i: usize| 0.5 * (std::f32::consts::TAU * 1_000.0 * i as f32 / sr).sin();
        // Two modulation periods at Machinery Movement 1 (2.05 Hz).
        let frames = (2.0 / 2.05 * sr) as usize + echo_delay(sr, 0.25);
        let wet = echo_run(sr, 0.25, 1.0, 0.0, frames, sine);
        let reference: Vec<f32> = (0..frames).map(sine).collect();
        let (wet_step, sine_step) = (max_step(&wet), max_step(&reference));
        println!("GLIDE wet step {wet_step:.4}, sine step {sine_step:.4}");
        assert!(
            wet_step <= 1.2 * sine_step,
            "wet step {wet_step:.4} vs sine step {sine_step:.4}"
        );
    }

    /// Spec "No modulation stays exact".
    #[test]
    fn unmodulated_echo_is_a_single_sample() {
        for damping in [0.0, 0.5] {
            let sr = 48_000.0;
            let d = echo_delay(sr, 0.25);
            let wet = echo_run(sr, 0.25, 0.0, damping, d + 20, |i| {
                if i == 0 {
                    1.0
                } else {
                    0.0
                }
            });
            assert_eq!(first_arrival(&wet), d, "damping {damping}");
            if damping == 0.0 {
                assert!(wet[d + 1..].iter().all(|&y| y == 0.0), "smeared repeat");
            }
        }
    }

    /// Level (RMS) of the first repeat of a steady sine at `freq`.
    fn first_repeat_level(sample_rate: f32, freq: f32, damping: f32) -> f32 {
        let d = echo_delay(sample_rate, 0.25);
        let wet = echo_run(sample_rate, 0.25, 0.0, damping, 2 * d, |i| {
            (std::f32::consts::TAU * freq * i as f32 / sample_rate).sin()
        });
        // The first repeat only, after the filter has settled.
        rms(&wet[d + d / 4..2 * d])
    }

    fn db(ratio: f32) -> f32 {
        20.0 * ratio.log10()
    }

    /// Spec "Highs fade, lows stay".
    #[test]
    fn hf_damping_darkens_without_changing_the_lows() {
        let sr = 48_000.0;
        let low: Vec<f32> = [0.0, 0.5, 1.0]
            .iter()
            .map(|&d| first_repeat_level(sr, 200.0, d))
            .collect();
        let high: Vec<f32> = [0.0, 0.5, 1.0]
            .iter()
            .map(|&d| first_repeat_level(sr, 8_000.0, d))
            .collect();
        println!("HFD low {low:?} high {high:?}");
        assert!(
            db(high[0] / low[0]).abs() <= 0.5,
            "damping 0 tilts: {low:?} {high:?}"
        );
        assert!(db(high[2] / low[2]) <= -12.0, "damping 1: {low:?} {high:?}");
        for l in &low[1..] {
            assert!(db(l / low[0]).abs() <= 0.5, "200 Hz level moves: {low:?}");
        }
    }

    /// Spec "Each repeat is darker".
    #[test]
    fn each_repeat_is_darker() {
        let sr = 48_000.0;
        let d = echo_delay(sr, 0.25);
        let burst = |freq: f32| {
            move |i: usize| {
                if i < d / 2 {
                    (std::f32::consts::TAU * freq * i as f32 / sr).sin()
                } else {
                    0.0
                }
            }
        };
        let high = echo_run(sr, 0.25, 0.0, 1.0, 4 * d, burst(8_000.0));
        let low = echo_run(sr, 0.25, 0.0, 1.0, 4 * d, burst(200.0));
        let ratios: Vec<f32> = (1..=3)
            .map(|k| rms(&high[k * d..k * d + d / 2]) / rms(&low[k * d..k * d + d / 2]))
            .collect();
        println!("REPEATS 8 kHz / 200 Hz: {ratios:?}");
        assert!(
            ratios[1] < ratios[0] && ratios[2] < ratios[1],
            "8 kHz / 200 Hz per repeat: {ratios:?}"
        );
    }

    /// Spec "Same tone at every rate".
    #[test]
    fn hf_damping_tone_does_not_depend_on_the_sample_rate() {
        let reference = first_repeat_level(48_000.0, 8_000.0, 1.0);
        for sr in [44_100.0, 96_000.0] {
            let level = first_repeat_level(sr, 8_000.0, 1.0);
            println!("RATE {sr}: {:+.2} dB from 48 kHz", db(level / reference));
            assert!(
                db(level / reference).abs() <= 1.0,
                "{sr} Hz: {:.2} dB from 48 kHz",
                db(level / reference)
            );
        }
    }

    /// Changing Machinery Movement (from 0, or by automation) doesn't
    /// tick: the modulation depth is smoothed, so the delay can't jump to
    /// wherever the free-running LFO is. Before the smoother, switching from
    /// 0 to 0.01 at Delay Time 1 stepped the delay by tens of samples.
    #[test]
    fn changing_machinery_movement_does_not_tick() {
        for (sr, delay_time, from, to) in [
            (48_000.0f32, 1.0f32, 0.0f32, 0.01f32),
            (48_000.0, 0.25, 0.0, 1.0),
            (44_100.0, 0.37, 0.2, 0.8),
        ] {
            let sine = |i: usize| 0.5 * (std::f32::consts::TAU * 1_000.0 * i as f32 / sr).sin();
            let mut echo = FactoryEcho::new();
            echo.set_sample_rate(sr);
            echo.set_parameters(delay_time, from, 0.0);
            let switch = 3 * echo_delay(sr, delay_time);
            let window = (0.1 * sr) as usize;
            let wet: Vec<f32> = (0..switch + window)
                .map(|i| {
                    if i == switch {
                        echo.set_parameters(delay_time, to, 0.0);
                    }
                    echo.process_stereo(sine(i), 0.0).0
                })
                .collect();
            let before = max_step(&wet[switch - window..switch]);
            let after = max_step(&wet[switch..]);
            // More movement means more Doppler shift and larger steps, so
            // the reference also includes a steady echo at the new setting,
            // over one full LFO period.
            let period = (sr / (0.05 + to * 2.0)) as usize;
            let steady = echo_run(sr, delay_time, to, 0.0, switch + period, sine);
            let steady_to = max_step(&steady[switch..]);
            let reference = before.max(steady_to);
            println!(
                "MOVEMENT {sr} Hz, {from} -> {to}: step {before:.4} before, \
                 {steady_to:.4} steady at {to}, {after:.4} after the change"
            );
            assert!(
                after <= 1.2 * reference,
                "{sr} Hz, Delay Time {delay_time}, movement {from} -> {to}: \
                 step {after:.4} after vs {reference:.4} reference"
            );
        }
    }
}

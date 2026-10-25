//! Tension rise exciter.
//!
//! Models slow force accumulation followed by a threshold break, using an
//! integrate-and-fire style state machine to create creaks and groans. This is
//! a specialty `dsp/exciters` source for stored stress, slip, and release.
//! Use it for beams loading up, cable groans, and delayed structural creaks.

use crate::dsp::utils::noise::Noise;
use crate::dsp::utils::rate;
/// Integrate-and-fire tension exciter for slow stress build-up.
#[derive(Clone, Debug)]
pub struct TensionRise {
    pull_speed: f32,
    break_threshold: f32,
    slip_stochasticity: f32,
    creak_sharpness: f32,

    tension: f32,
    active: bool,
    /// Noise for this note (`resolve-exciter-tradeoffs`).
    noise: Noise,
    impulse_samples: u32,
    /// `48 kHz / fs` (`fix-sample-rate-time-constants`).
    ratio: f32,
    /// Length of the release burst in host samples (50 at 48 kHz).
    burst_frames: u32,
}

impl TensionRise {
    /// Creates a default tension-rise exciter.
    pub fn new() -> Self {
        Self {
            pull_speed: 0.3,
            break_threshold: 0.5,
            slip_stochasticity: 0.2,
            creak_sharpness: 0.5,
            tension: 0.0,
            active: false,
            noise: Noise::default(),
            impulse_samples: 0,
            ratio: 1.0,
            burst_frames: 50,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.burst_frames = rate::frames(50, self.ratio);
    }

    /// Sets pull speed, break threshold, stochasticity, and creak sharpness.
    pub fn set_parameters(
        &mut self,
        speed: f32,
        threshold: f32,
        stochasticity: f32,
        sharpness: f32,
    ) {
        self.pull_speed = speed.clamp(0.001, 10.0);
        self.break_threshold = threshold.clamp(0.001, 10.0);
        self.slip_stochasticity = stochasticity.clamp(0.0, 5.0);
        self.creak_sharpness = sharpness.clamp(0.0, 10.0);
    }

    /// Restart the noise for a new note (the voice seeds it per note).
    pub fn reseed(&mut self, seed: u32) {
        self.noise.reseed(seed);
    }

    /// Arms the tension accumulator with an initial pull speed.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.tension = 0.0;
        self.impulse_samples = 0;
        self.pull_speed *= 0.5 + velocity * 0.5;
    }

    /// Processes one sample of tension accumulation and burst decay.
    pub fn process_sample(&mut self, _resonator_displacement: f32, resonator_velocity: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        if self.impulse_samples > 0 {
            // Decay the release burst over a short fixed window.
            self.impulse_samples -= 1;
            let decay = self.impulse_samples as f32 / self.burst_frames as f32;
            return self.tension * decay * self.creak_sharpness * 5.0;
        }

        // Relative pull is integrated into stored stress each sample.
        let v_rel = self.pull_speed - resonator_velocity;
        self.tension += v_rel * rate::step(0.001, self.ratio);

        // Small random variation makes the break point feel less mechanical.
        let random_variation = self.noise.bipolar() * self.slip_stochasticity;
        let effective_threshold = self.break_threshold * (1.0 + random_variation * 0.3);

        if self.tension > effective_threshold {
            // One-sample break spike: keep its area at any rate.
            let output = self.tension * self.creak_sharpness * 5.0 / self.ratio;
            self.tension = 0.0;
            self.impulse_samples = self.burst_frames;
            return output;
        }

        0.0
    }

    /// Returns whether the tension source is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Stops the tension model.
    pub fn release(&mut self) {
        self.active = false;
    }
}

impl Default for TensionRise {
    fn default() -> Self {
        Self::new()
    }
}

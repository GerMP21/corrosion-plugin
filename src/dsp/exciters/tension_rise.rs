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
    /// This break's threshold, drawn once per break: drawing it every
    /// sample let a faster rate meet more draws on the way up, so the line
    /// broke earlier and softer (−2 dB at 96 kHz) (`resolve-exciter-tradeoffs`).
    threshold: f32,
    /// Samples left before the line starts re-tensioning after a break.
    recovery_samples: u32,
    /// Places the break spike between samples (`resolve-exciter-tradeoffs`).
    events: rate::EventSplitter,
    /// `48 kHz / fs` (`fix-sample-rate-time-constants`).
    ratio: f32,
    /// The pause after a break, in host samples (50 at 48 kHz). (It used to
    /// carry a decaying "release burst", but that was scaled by the tension
    /// the break had just zeroed, so it was always silent; only the pause
    /// remains, `resolve-exciter-tradeoffs`.)
    recovery_frames: u32,
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
            threshold: 0.5,
            recovery_samples: 0,
            events: rate::EventSplitter::default(),
            ratio: 1.0,
            recovery_frames: 50,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.recovery_frames = rate::frames(50, self.ratio);
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
        self.recovery_samples = 0;
        self.events.clear();
        self.pull_speed *= 0.5 + velocity * 0.5;
        self.draw_threshold();
    }

    fn draw_threshold(&mut self) {
        let random_variation = self.noise.bipolar() * self.slip_stochasticity;
        self.threshold = self.break_threshold * (1.0 + random_variation * 0.3);
    }

    /// Processes one sample of tension accumulation and break spikes.
    pub fn process_sample(&mut self, _resonator_displacement: f32, resonator_velocity: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        // Every output is delayed one sample, so the break spike can land
        // between samples at its true time (`resolve-exciter-tradeoffs`).
        let (output, early) = self.next_output(resonator_velocity);
        self.events.add(output, early);
        self.events.take()
    }

    /// This sample's output, and how far before the sample's end (0–1) it
    /// happened: the break's threshold crossing, or the sample end.
    fn next_output(&mut self, resonator_velocity: f32) -> (f32, f32) {
        if self.recovery_samples > 0 {
            // The line pauses briefly after a break before it re-tensions.
            self.recovery_samples -= 1;
            return (0.0, 0.0);
        }

        // Relative pull is integrated into stored stress each sample.
        let v_rel = self.pull_speed - resonator_velocity;
        let previous = self.tension;
        self.tension += v_rel * rate::step(0.001, self.ratio);

        // Small random variation makes the break point feel less mechanical.
        let effective_threshold = self.threshold;

        if self.tension > effective_threshold {
            // One-sample break spike: keep its area at any rate.
            let output = self.tension * self.creak_sharpness * 5.0 / self.ratio;
            let rise = self.tension - previous;
            let early = if rise > 0.0 {
                (self.tension - effective_threshold) / rise
            } else {
                0.0
            };
            self.tension = 0.0;
            self.recovery_samples = self.recovery_frames;
            self.draw_threshold();
            return (output, early);
        }

        (0.0, 0.0)
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

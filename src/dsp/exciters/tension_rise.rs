//! Tension rise exciter.
//!
//! Models slow force accumulation followed by a threshold break, using an
//! integrate-and-fire style state machine to create creaks and groans. This is
//! a specialty `dsp/exciters` source for stored stress, slip, and release.
//! Use it for beams loading up, cable groans, and delayed structural creaks.

/// Integrate-and-fire tension exciter for slow stress build-up.
#[derive(Clone, Debug)]
pub struct TensionRise {
    pull_speed: f32,
    break_threshold: f32,
    slip_stochasticity: f32,
    creak_sharpness: f32,

    tension: f32,
    active: bool,
    rng_phase: f32,
    impulse_samples: u32,
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
            rng_phase: 0.0,
            impulse_samples: 0,
        }
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

    /// Arms the tension accumulator with an initial pull speed.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.tension = 0.0;
        self.impulse_samples = 0;
        self.pull_speed *= 0.5 + velocity * 0.5;
        self.rng_phase = velocity * 50.0;
    }

    /// Processes one sample of tension accumulation and burst decay.
    pub fn process_sample(&mut self, _resonator_displacement: f32, resonator_velocity: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        if self.impulse_samples > 0 {
            // Decay the release burst over a short fixed window.
            self.impulse_samples -= 1;
            let decay = self.impulse_samples as f32 / 50.0;
            return self.tension * decay * self.creak_sharpness * 5.0;
        }

        // Relative pull is integrated into stored stress each sample.
        let v_rel = self.pull_speed - resonator_velocity;
        self.tension += v_rel * 0.001;

        // Small random variation makes the break point feel less mechanical.
        let random_variation = (self.pseudo_random() - 0.5) * 2.0 * self.slip_stochasticity;
        let effective_threshold = self.break_threshold * (1.0 + random_variation * 0.3);

        if self.tension > effective_threshold {
            let output = self.tension * self.creak_sharpness * 5.0;
            self.tension = 0.0;
            self.impulse_samples = 50;
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

    fn pseudo_random(&mut self) -> f32 {
        self.rng_phase += crate::dsp::utils::GOLDEN_STEP;
        ((self.rng_phase.sin() * 43_758.547).fract()).abs()
    }
}

impl Default for TensionRise {
    fn default() -> Self {
        Self::new()
    }
}

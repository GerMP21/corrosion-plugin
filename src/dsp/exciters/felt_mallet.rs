//! Felt mallet exciter.
//!
//! Uses a polynomial contact curve where a soft linear term handles the body
//! of the hit and a steeper power term models the core bottoming out.

use crate::dsp::utils::rate;

/// Soft mallet with a non-linear compression curve.
#[derive(Clone, Debug)]
pub struct FeltMallet {
    /// Overall momentum multiplier.
    mallet_mass: f32,
    /// Soft contact stiffness.
    felt_softness: f32,
    /// Hard core stiffness once the felt bottoms out.
    core_hardness: f32,
    /// Power curve controlling how quickly stiffness rises.
    compression_curve: f32,
    /// Normalized mallet position in the contact model.
    mallet_position: f32,
    /// Normalized mallet velocity in the contact model.
    mallet_velocity: f32,
    /// Whether the mallet is still engaged with the surface.
    active: bool,
    /// `48 kHz / fs` and per-sample constants derived from it
    /// (`fix-sample-rate-time-constants`); exact 48 kHz values at 48 kHz.
    ratio: f32,
    velocity_decay: f32,
}

impl FeltMallet {
    /// Creates a default soft mallet.
    pub fn new() -> Self {
        Self {
            mallet_mass: 1.0,
            felt_softness: 0.3,
            core_hardness: 2.0,
            compression_curve: 3.5,
            mallet_position: 0.0,
            mallet_velocity: 0.0,
            active: false,
            ratio: 1.0,
            velocity_decay: 0.995,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.velocity_decay = rate::decay(0.995, self.ratio);
    }

    /// Sets mass and compression parameters.
    pub fn set_parameters(&mut self, mass: f32, softness: f32, hardness: f32, curve: f32) {
        self.mallet_mass = mass.clamp(0.01, 50.0);
        self.felt_softness = softness.clamp(0.001, 20.0);
        self.core_hardness = hardness.clamp(0.01, 50.0);
        self.compression_curve = curve.clamp(1.0, 10.0);
    }

    /// Arms the mallet with a strike velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.mallet_position = -0.2 - velocity * 0.3;
        self.mallet_velocity = velocity * 3.0 + 1.0;
    }

    /// Processes one sample of non-linear contact force.
    pub fn process_sample(&mut self, resonator_displacement: f32, _resonator_velocity: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        self.mallet_position += self.mallet_velocity * rate::step(0.001, self.ratio);
        let compression = (self.mallet_position - resonator_displacement).max(0.0);

        let soft_component = self.felt_softness * compression;
        let hard_component = self.core_hardness * compression.powf(self.compression_curve);
        let force = (soft_component + hard_component) * self.mallet_mass;

        if compression > 0.01 {
            self.mallet_velocity -= compression * rate::step(0.5, self.ratio);
        }

        self.mallet_velocity += rate::step(0.01, self.ratio);
        self.mallet_velocity *= self.velocity_decay;

        if self.mallet_position > 1.0 && self.mallet_velocity > 0.0 {
            self.active = false;
        }

        force
    }

    /// Returns whether the mallet is still active.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for FeltMallet {
    fn default() -> Self {
        Self::new()
    }
}

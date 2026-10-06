//! Hard mallet exciter.
//!
//! Implements a rigid Hertzian contact with moderate damping and a small
//! impact counter to suppress runaway micro-bounces.

use crate::dsp::utils::rate;

/// Rigid mallet contact with Hertzian force and bounce limiting.
#[derive(Clone, Debug)]
pub struct HardMallet {
    /// Overall momentum multiplier.
    mallet_mass: f32,
    /// Contact stiffness controlling brightness and bite.
    material_stiffness: f32,
    /// Damping that suppresses tiny rebounds.
    impact_damping: f32,
    /// Normalized mallet position in contact space.
    mallet_position: f32,
    /// Normalized mallet velocity in contact space.
    mallet_velocity: f32,
    /// Whether the mallet is still interacting with the surface.
    active: bool,
    /// Number of consecutive impact frames seen so far.
    impact_count: u32,
    /// `48 kHz / fs` and per-sample constants derived from it
    /// (`fix-sample-rate-time-constants`); exact 48 kHz values at 48 kHz.
    ratio: f32,
    velocity_decay: f32,
    impact_damping_decay: f32,
    impact_frames: u32,
}

impl HardMallet {
    /// Creates a default rigid mallet.
    pub fn new() -> Self {
        Self {
            mallet_mass: 2.0,
            material_stiffness: 1.5,
            impact_damping: 0.7,
            mallet_position: 0.0,
            mallet_velocity: 0.0,
            active: false,
            impact_count: 0,
            ratio: 1.0,
            velocity_decay: 0.98,
            impact_damping_decay: 0.5,
            impact_frames: 5,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.velocity_decay = rate::decay(0.98, self.ratio);
        self.impact_damping_decay = rate::decay(0.5, self.ratio);
        self.impact_frames = rate::frames(5, self.ratio);
    }

    /// Updates mass, stiffness, and damping.
    pub fn set_parameters(&mut self, mass: f32, stiffness: f32, damping: f32) {
        self.mallet_mass = mass.clamp(0.01, 50.0);
        self.material_stiffness = stiffness.clamp(0.01, 20.0);
        self.impact_damping = damping.clamp(0.01, 2.0);
    }

    /// Starts a new strike with the supplied velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.mallet_position = -0.3;
        self.mallet_velocity = velocity * 4.0 + 2.0;
        self.impact_count = 0;
    }

    /// Produces one sample of Hertzian impact force.
    pub fn process_sample(&mut self, resonator_displacement: f32, resonator_velocity: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        self.mallet_position += self.mallet_velocity * rate::step(0.0005, self.ratio);

        let penetration = (self.mallet_position - resonator_displacement).max(0.0);
        let hertzian_force = self.material_stiffness * penetration.powf(1.5);

        let v_rel = self.mallet_velocity - resonator_velocity;
        let damping_force = self.impact_damping * v_rel.abs() * v_rel.signum();

        let total_force = (hertzian_force + damping_force.max(0.0)) * self.mallet_mass;

        if penetration > 0.001 {
            self.impact_count += 1;
            let energy_transfer = penetration * self.material_stiffness * 0.3;
            self.mallet_velocity -=
                rate::step(energy_transfer / self.mallet_mass.max(0.01), self.ratio);

            if self.impact_count > self.impact_frames {
                self.mallet_velocity *= self.impact_damping_decay;
            }
        }

        self.mallet_velocity += rate::step(0.02, self.ratio);
        self.mallet_velocity *= self.velocity_decay;

        if self.mallet_position < -0.5 && self.mallet_velocity < 0.0 {
            self.active = false;
        }

        total_force.clamp(0.0, 100.0)
    }

    /// Returns whether the mallet is still active.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for HardMallet {
    fn default() -> Self {
        Self::new()
    }
}

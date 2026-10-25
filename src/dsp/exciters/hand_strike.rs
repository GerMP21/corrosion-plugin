//! Hand-strike exciter.
//!
//! Models a fleshy impact using a Kelvin-Voigt style spring-damper contact.
//! The hand mass and mute decay together shape whether the gesture reads as a
//! slap, mute, or a heavier palm strike.

use crate::dsp::utils::{contact, rate};
use crate::dsp::ContactPrediction;

/// Fleshy impact with strong damping and a simple release state.
#[derive(Clone, Debug)]
pub struct HandStrike {
    /// Overall force multiplier.
    hand_mass: f32,
    /// Spring term controlling initial contact stiffness.
    flesh_stiffness: f32,
    /// Damping term absorbing resonator motion.
    flesh_damping: f32,
    /// Exponential decay applied after the strike.
    mute_decay: f32,
    /// Current hand position in normalized contact space.
    hand_position: f32,
    /// Current hand velocity in normalized contact space.
    hand_velocity: f32,
    /// Whether the contact state is still producing force.
    active: bool,
    /// Linear decay state multiplied into the output each sample.
    decay_state: f32,
    /// Whether the hand has reached the surface yet this strike.
    landed: bool,
    /// `48 kHz / fs` and per-sample constants derived from it
    /// (`fix-sample-rate-time-constants`); exact 48 kHz values at 48 kHz.
    ratio: f32,
    velocity_decay: f32,
    mute_decay_per_sample: f32,
}

impl HandStrike {
    /// Creates a default hand-strike model.
    pub fn new() -> Self {
        Self {
            hand_mass: 1.0,
            flesh_stiffness: 0.1,
            flesh_damping: 0.9,
            mute_decay: 0.95,
            hand_position: 0.0,
            hand_velocity: 0.0,
            active: false,
            decay_state: 1.0,
            landed: false,
            ratio: 1.0,
            velocity_decay: 0.99,
            mute_decay_per_sample: 0.95,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.velocity_decay = rate::decay(0.99, self.ratio);
        self.mute_decay_per_sample = rate::decay(self.mute_decay, self.ratio);
    }

    /// Sets the mass, stiffness, damping, and mute envelope.
    pub fn set_parameters(&mut self, mass: f32, stiffness: f32, damping: f32, mute: f32) {
        self.hand_mass = mass.clamp(0.01, 50.0);
        self.flesh_stiffness = stiffness.clamp(0.001, 10.0);
        self.flesh_damping = damping.clamp(0.01, 2.0);
        self.mute_decay = mute.clamp(0.5, 0.9999);
        self.mute_decay_per_sample = rate::decay(self.mute_decay, self.ratio);
    }

    /// Arms the hand with an initial strike velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.hand_position = -0.1 * velocity;
        self.hand_velocity = velocity * 2.0;
        self.decay_state = 1.0;
        self.landed = false;
    }

    /// Produces a single force sample using the current resonator state.
    pub fn process_sample(&mut self, resonator: ContactPrediction) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Kelvin–Voigt contact, solved against this sample's resonator
        // response (`resolve-exciter-tradeoffs`).
        // The force is taken at the middle of the step, so the approach's
        // area doesn't depend on where the sample grid falls.
        let half_decay = self.velocity_decay.sqrt();
        let velocity = self.hand_velocity * half_decay;
        let position = self.hand_position + 0.5 * velocity * rate::step(0.01, self.ratio);
        let (stiffness, damping) = (self.flesh_stiffness, self.flesh_damping);
        let scale = self.hand_mass * self.decay_state;
        let force = contact::solve_force(|f| {
            let spring_force = stiffness * (position - resonator.displacement(f));
            let damping_force = damping * (velocity - resonator.velocity(f));
            (spring_force + damping_force).max(0.0) * scale
        });
        let (resonator_displacement, resonator_velocity) =
            (resonator.displacement(force), resonator.velocity(force));

        self.hand_velocity *= self.velocity_decay;
        let before = self.hand_position;
        self.hand_position += self.hand_velocity * rate::step(0.01, self.ratio);

        let surface = resonator_displacement - 0.01;
        let mut force = force;
        if self.hand_position > surface {
            // The hand reached the surface partway through this sample: only
            // that part of the step pushed, so the strike's length isn't
            // rounded up to whole samples (`resolve-exciter-tradeoffs`).
            if before < surface && !self.landed {
                force *= ((surface - before) / (self.hand_position - before)).clamp(0.0, 1.0);
            }
            self.landed = true;
            self.hand_position = resonator_displacement;
            self.hand_velocity = resonator_velocity * 0.5;
        }

        self.decay_state *= self.mute_decay_per_sample;
        if self.decay_state < 0.001 {
            self.active = false;
        }

        force
    }

    /// Returns whether the strike is still producing output.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Immediately releases the hand contact.
    pub fn release(&mut self) {
        self.active = false;
    }
}

impl Default for HandStrike {
    fn default() -> Self {
        Self::new()
    }
}

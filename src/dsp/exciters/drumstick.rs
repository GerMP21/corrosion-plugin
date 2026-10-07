//! Drumstick exciter.
//!
//! A light rigid stick modeled with Hertzian contact and a separate bounce
//! state so the resonator can throw the stick backward for micro-collisions.

use crate::dsp::utils::rate;

/// Light rigid stick with explicit micro-bounce tracking.
#[derive(Clone, Debug)]
pub struct Drumstick {
    /// Stick mass used in the contact response.
    stick_mass: f32,
    /// Tip stiffness controlling the attack brightness.
    tip_stiffness: f32,
    /// Energy retained when the stick rebounds.
    restitution_bounciness: f32,
    /// Maximum number of allowed re-strikes.
    micro_bounce_limit: u32,
    /// Stick position in normalized contact space.
    stick_position: f32,
    /// Stick velocity in normalized contact space.
    stick_velocity: f32,
    /// Whether the stick is still participating in the strike.
    active: bool,
    /// Count of observed rebounds so far.
    bounce_count: u32,
    /// Cached bounce cap mirrored from `micro_bounce_limit`.
    max_bounces: u32,
    /// Previous penetration depth used to detect reversal.
    last_penetration: f32,
    /// `48 kHz / fs` and per-sample constants derived from it
    /// (`fix-sample-rate-time-constants`); exact 48 kHz values at 48 kHz.
    ratio: f32,
    velocity_decay: f32,
}

impl Drumstick {
    /// Creates a default rigid stick model.
    pub fn new() -> Self {
        Self {
            stick_mass: 0.3,
            tip_stiffness: 3.0,
            restitution_bounciness: 0.6,
            micro_bounce_limit: 4,
            stick_position: 0.0,
            stick_velocity: 0.0,
            active: false,
            bounce_count: 0,
            max_bounces: 4,
            last_penetration: 0.0,
            ratio: 1.0,
            velocity_decay: 0.999,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.velocity_decay = rate::decay(0.999, self.ratio);
    }

    /// Sets the mass, stiffness, restitution, and bounce cap.
    pub fn set_parameters(
        &mut self,
        mass: f32,
        stiffness: f32,
        restitution: f32,
        bounce_limit: u32,
    ) {
        self.stick_mass = mass.clamp(0.001, 10.0);
        self.tip_stiffness = stiffness.clamp(0.1, 20.0);
        self.restitution_bounciness = restitution.clamp(0.01, 2.0);
        self.micro_bounce_limit = bounce_limit.clamp(1, 50);
        self.max_bounces = self.micro_bounce_limit;
    }

    /// Starts the strike with an initial velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.stick_position = -0.15;
        self.stick_velocity = velocity * 5.0 + 3.0;
        self.bounce_count = 0;
        self.last_penetration = 0.0;
    }

    /// Processes one Hertzian contact sample and updates the stick state.
    pub fn process_sample(&mut self, resonator_displacement: f32, resonator_velocity: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        self.stick_position += self.stick_velocity * rate::step(0.0003, self.ratio);

        let penetration = (self.stick_position - resonator_displacement).max(0.0);
        let hertzian_force = if penetration > 0.0 {
            self.tip_stiffness * penetration.powf(1.5)
        } else {
            0.0
        };

        if penetration > 0.0 {
            let deceleration = hertzian_force / self.stick_mass.max(0.01);
            self.stick_velocity -= deceleration * rate::step(0.1, self.ratio);

            if penetration < self.last_penetration && self.stick_velocity < 0.0 {
                self.bounce_count += 1;
                self.stick_velocity = -self.stick_velocity * self.restitution_bounciness;
                self.stick_velocity += resonator_velocity * 0.3;

                if self.bounce_count >= self.max_bounces {
                    self.stick_velocity = 0.0;
                    self.stick_position = resonator_displacement;
                }
            }
        } else {
            self.stick_velocity += rate::step(0.05, self.ratio);

            if self.bounce_count >= self.max_bounces && self.stick_velocity < 0.0 {
                self.active = false;
            }
        }

        self.last_penetration = penetration;
        self.stick_velocity *= self.velocity_decay;

        hertzian_force * self.stick_mass
    }

    /// Returns whether the stick is still engaged.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns how many bounces have occurred.
    pub fn bounce_count(&self) -> u32 {
        self.bounce_count
    }
}

impl Default for Drumstick {
    fn default() -> Self {
        Self::new()
    }
}

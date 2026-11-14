//! Drumstick exciter.
//!
//! A light rigid stick modeled with Hertzian contact and a separate bounce
//! state so the resonator can throw the stick backward for micro-collisions.

use crate::dsp::utils::{contact, rate};
use crate::dsp::ContactPrediction;

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
    /// Host samples since the strike started, and the longest a strike may
    /// last (0.25 s), however the resonator moves under the stick.
    strike_frames: u32,
    max_strike_frames: u32,
    /// The strike's starting speed, and the re-strikes played after the
    /// first contact (`fix-inert-controls`).
    initial_velocity: f32,
    restrikes: Restrikes,
}

/// The re-strikes after the first contact: the stick comes back `total`
/// times, each at `restitution` of the last one's speed, after a flight that
/// shortens by the same factor. Each is a real contact, solved like the
/// first, that ends as soon as the stick rebounds. Left free to fall back,
/// a stick coming back every few milliseconds damped the object (−50 dB in
/// 100 ms), like one left resting on it.
#[derive(Clone, Copy, Debug, Default)]
struct Restrikes {
    /// Whether the first contact is over and re-strikes are playing.
    playing: bool,
    /// Whether a re-strike's contact is under way.
    striking: bool,
    /// Re-strikes in all, and how many have started.
    total: u32,
    started: u32,
    /// Frame (since the strike began) at which the next one starts.
    next_at: f32,
    /// Flight before the next one, in frames.
    flight: f32,
}

/// The first re-strike's flight at Restitution Bounciness 1, at 48 kHz.
const FIRST_FLIGHT_FRAMES: f32 = 1_920.0;

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
            strike_frames: 0,
            max_strike_frames: 12_000,
            initial_velocity: 0.0,
            restrikes: Restrikes {
                playing: false,
                striking: false,
                total: 0,
                started: 0,
                next_at: 0.0,
                flight: 0.0,
            },
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.velocity_decay = rate::decay(0.999, self.ratio);
        self.max_strike_frames = rate::frames(12_000, self.ratio);
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
        self.initial_velocity = self.stick_velocity;
        self.bounce_count = 0;
        self.last_penetration = 0.0;
        self.strike_frames = 0;
        self.restrikes = Restrikes::default();
    }

    /// The first contact is over: play the re-strikes, if any.
    fn start_restrikes(&mut self) {
        let total = self.max_bounces.saturating_sub(1);
        if total == 0 {
            self.active = false;
            return;
        }
        let flight = FIRST_FLIGHT_FRAMES / self.ratio * self.restitution_bounciness.min(1.0);
        self.restrikes = Restrikes {
            playing: true,
            striking: false,
            total,
            started: 0,
            next_at: self.strike_frames as f32 + flight,
            flight,
        };
    }

    /// A re-strike's contact has ended: wait for the next one, or stop.
    fn end_restrike(&mut self) {
        self.restrikes.striking = false;
        if self.restrikes.started >= self.restrikes.total {
            self.active = false;
        } else {
            self.restrikes.flight *= self.restitution_bounciness.min(1.0);
            self.restrikes.next_at = self.strike_frames as f32 + self.restrikes.flight;
        }
    }

    /// Processes one Hertzian contact sample and updates the stick state.
    pub fn process_sample(&mut self, resonator: ContactPrediction) -> f32 {
        if !self.active {
            return 0.0;
        }
        self.strike_frames += 1;
        if self.strike_frames >= self.max_strike_frames {
            self.active = false;
            return 0.0;
        }

        if self.restrikes.playing && !self.restrikes.striking {
            if (self.strike_frames as f32) < self.restrikes.next_at {
                return 0.0;
            }
            // The stick comes back, at Restitution Bounciness of the last
            // strike's speed. It lands now, on the surface: started above
            // it, it chased a surface moving away for milliseconds, so the
            // strike's timing (and level) depended on the sample rate.
            self.restrikes.started += 1;
            self.restrikes.striking = true;
            let r = self.restitution_bounciness.min(1.0);
            self.stick_position = resonator.displacement(0.0);
            self.stick_velocity = self.initial_velocity * r.powi(self.restrikes.started as i32);
            self.last_penetration = 0.0;
        }

        self.stick_position += self.stick_velocity * rate::step(0.0003, self.ratio);

        // Hertzian contact, solved against this sample's resonator response
        // (`resolve-exciter-tradeoffs`). The returned force is
        // `hertzian_force · stick_mass`.
        let (position, stiffness, mass) =
            (self.stick_position, self.tip_stiffness, self.stick_mass);
        let force = contact::solve_force(|f| {
            stiffness * (position - resonator.displacement(f)).max(0.0).powf(1.5) * mass
        });
        let resonator_displacement = resonator.displacement(force);
        let resonator_velocity = resonator.velocity(force);
        let penetration = (self.stick_position - resonator_displacement).max(0.0);
        let hertzian_force = force / mass;

        if penetration > 0.0 {
            let deceleration = hertzian_force / self.stick_mass.max(0.01);
            self.stick_velocity -= deceleration * rate::step(0.1, self.ratio);

            if self.restrikes.striking {
                // A re-strike ends as soon as the stick turns back: waiting
                // for the penetration to fall as well let a fast-moving
                // surface hold it in a long press.
                if self.stick_velocity < 0.0 {
                    self.end_restrike();
                }
            } else if penetration < self.last_penetration && self.stick_velocity < 0.0 {
                self.bounce_count += 1;
                self.stick_velocity = -self.stick_velocity * self.restitution_bounciness;
                self.stick_velocity += resonator_velocity * 0.3;

                // The last bounce: the stick comes to rest instead of being
                // pinned to the surface and pulled back into it, which made
                // it chatter for as long as the note was held
                // (`resolve-exciter-tradeoffs`).
                if self.bounce_count >= self.max_bounces {
                    self.start_restrikes();
                }
            }
        } else {
            self.stick_velocity += rate::step(0.05, self.ratio);

            if self.restrikes.striking {
                // The surface has left the stick.
                self.end_restrike();
            } else if self.bounce_count >= self.max_bounces && self.stick_velocity < 0.0 {
                self.start_restrikes();
            }
        }

        self.last_penetration = penetration;
        self.stick_velocity *= self.velocity_decay;

        force
    }

    /// Returns whether the stick is still engaged.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns how many bounces have occurred: the first contact's
    /// micro-bounces and the re-strikes after it.
    pub fn bounce_count(&self) -> u32 {
        self.bounce_count + self.restrikes.started
    }
}

impl Default for Drumstick {
    fn default() -> Self {
        Self::new()
    }
}

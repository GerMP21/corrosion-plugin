//! Stiff point scrape exciter.
//!
//! Models a rigid point that catches, bends, and releases as a stiff spring
//! against a moving surface. The math integrates relative motion into a stored
//! deflection term and then converts it back into chatter force.
//! Use it for squeaks, tiny slips, and high-frequency scraping chatter inside
//! the `dsp/exciters` friction family.

use crate::dsp::utils::rate;
/// Rigid-point scrape model with snap-back chatter.
#[derive(Clone, Debug)]
pub struct StiffPointScrape {
    scrape_speed: f32,
    point_pressure: f32,
    chatter_pitch: f32,
    chatter_damping: f32,

    // State
    tip_position: f32,
    tip_velocity: f32,
    accumulated_deflection: f32,
    active: bool,
    slip_threshold: f32,
    /// `48 kHz / fs` (`fix-sample-rate-time-constants`).
    ratio: f32,
}

impl StiffPointScrape {
    /// Creates a default stiff-point scrape exciter.
    pub fn new() -> Self {
        Self {
            scrape_speed: 0.5,
            point_pressure: 0.4,
            chatter_pitch: 0.7,
            chatter_damping: 0.3,
            tip_position: 0.0,
            tip_velocity: 0.0,
            accumulated_deflection: 0.0,
            active: false,
            slip_threshold: 0.1,
            ratio: 1.0,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
    }

    /// Sets scrape speed, contact pressure, chatter pitch, and damping.
    pub fn set_parameters(&mut self, speed: f32, pressure: f32, pitch: f32, damping: f32) {
        self.scrape_speed = speed.clamp(0.0, 10.0);
        self.point_pressure = pressure.clamp(0.01, 10.0);
        self.chatter_pitch = pitch.clamp(0.01, 5.0);
        self.chatter_damping = damping.clamp(0.01, 5.0);
        self.slip_threshold = 0.05 + pressure * 0.2;
    }

    /// Starts the scrape and seeds the point motion.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.tip_position = 0.0;
        self.tip_velocity = 0.0;
        self.accumulated_deflection = 0.0;
        self.scrape_speed *= 0.5 + velocity * 0.5;
    }

    /// Processes one sample of spring wind-up, slip, and chatter.
    pub fn process_sample(&mut self, _resonator_displacement: f32, resonator_velocity: f32) -> f32 {
        if !self.active || self.scrape_speed < 0.001 {
            return 0.0;
        }

        // Relative velocity between scraper and surface
        let v_rel = self.scrape_speed - resonator_velocity;

        // Accumulate deflection (spring winding up)
        self.accumulated_deflection += v_rel * rate::step(0.001, self.ratio);

        // Spring force from deflection
        let k_point = 100.0 + self.chatter_pitch * 900.0; // 100-1000 Hz range
        let spring_force = k_point * self.accumulated_deflection;

        // Damping force: it resists the relative motion, so it drags the
        // surface along with the tip. (It was subtracted, which turned it
        // into negative damping on the resonator: the contact ran away at
        // high drive, `resolve-exciter-tradeoffs`.)
        let d_point = self.chatter_damping * 50.0;
        let damping_force = d_point * v_rel;

        let total_force = spring_force + damping_force;

        // Check for slip - tip snaps forward when threshold exceeded
        if self.accumulated_deflection.abs() > self.slip_threshold {
            // Tip snaps - rapid release of energy
            let snap_force = self.accumulated_deflection.signum() * self.point_pressure * 2.0;
            self.accumulated_deflection *= 0.1; // Reset position
            return snap_force * self.scrape_speed * 5.0;
        }

        // Dynamic friction component
        let mu_dynamic = 0.3;
        let friction_force = total_force * mu_dynamic;

        friction_force.clamp(-2.0, 2.0)
    }

    /// Returns whether the scrape is still moving.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Releases the point from the surface.
    pub fn release(&mut self) {
        self.active = false;
    }
}

impl Default for StiffPointScrape {
    fn default() -> Self {
        Self::new()
    }
}

//! Corrugated drag exciter.
//!
//! Models a scraping or dragging contact moving over periodic surface ridges,
//! using a sinusoidal bump profile to modulate contact force.
//! The algorithm is a simple periodic contact/friction model and fits the
//! `dsp/exciters` friction family as a coarse, macroscopic surface interaction.
//! Use it for conveyor squeals, dragged tools, corrugated metal, or textured
//! industrial drag sounds.

use crate::dsp::utils::rate;
/// Drag exciter for periodic ridge contact.
#[derive(Clone, Debug)]
pub struct CorrugatedDrag {
    drag_speed: f32,
    ridge_spacing: f32,
    ridge_depth: f32,
    exciter_mass: f32,

    position: f32,
    active: bool,
    last_bump_force: f32,
    /// `48 kHz / fs` (`fix-sample-rate-time-constants`).
    ratio: f32,
}

impl CorrugatedDrag {
    /// Creates a default corrugated-drag exciter.
    pub fn new() -> Self {
        Self {
            drag_speed: 0.5,
            ridge_spacing: 0.05,
            ridge_depth: 0.5,
            exciter_mass: 0.5,
            position: 0.0,
            active: false,
            last_bump_force: 0.0,
            ratio: 1.0,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
    }

    /// Sets drag speed, ridge spacing, ridge depth, and exciter mass.
    pub fn set_parameters(&mut self, speed: f32, spacing: f32, depth: f32, mass: f32) {
        self.drag_speed = speed.clamp(0.0, 10.0);
        self.ridge_spacing = spacing.clamp(0.001, 1.0);
        self.ridge_depth = depth.clamp(0.0, 10.0);
        self.exciter_mass = mass.clamp(0.01, 30.0);
    }

    /// Arms the drag model with an initial sweep velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.position = 0.0;
        self.drag_speed *= 0.3 + velocity * 0.7;
    }

    /// Processes one sample of periodic ridge contact force.
    pub fn process_sample(&mut self, _resonator_displacement: f32, resonator_velocity: f32) -> f32 {
        if !self.active || self.drag_speed < 0.001 {
            return 0.0;
        }

        let v_rel = self.drag_speed - resonator_velocity;
        self.position += v_rel * rate::step(0.01, self.ratio);

        // Periodic ridge profile: positive lobes become contact bumps.
        let bump = (2.0 * std::f32::consts::PI * self.position / self.ridge_spacing).sin();
        let bump_force = bump.max(0.0) * self.ridge_depth * self.exciter_mass;

        // Constant normal pressure and Coulomb friction baseline.
        let pressure = 0.5;
        let mu = 0.3;
        let base_force = pressure * mu;

        let total = (base_force + bump_force) * v_rel.signum();
        self.last_bump_force = bump_force;

        total * self.drag_speed * 3.0
    }

    /// Returns whether the drag contact is still active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Releases the drag and stops further contact generation.
    pub fn release(&mut self) {
        self.active = false;
    }
}

impl Default for CorrugatedDrag {
    fn default() -> Self {
        Self::new()
    }
}

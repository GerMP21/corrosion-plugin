//! Bow friction exciter.
//!
//! This model uses a Stribeck-style friction curve plus a moving contact state
//! to approximate stick-slip motion. The output is a force signal that can be
//! fed into the resonator each sample.

use crate::dsp::utils::rate;
use std::f32::consts::PI;

/// Smooth stick-slip bow model with a moving contact state.
#[derive(Clone, Debug)]
pub struct BowExciter {
    /// Normal force applied to the surface.
    pressure: f32,
    /// Target bow speed.
    speed: f32,
    /// Surface roughness that shapes the friction curve and ripple.
    roughness: f32,
    /// Curve control for how quickly stick transitions into slip.
    slip_curve: f32,
    /// Normalized contact position used for repeating surface texture.
    bow_position: f32,
    /// Current motion of the scraper contact state.
    bow_velocity: f32,
    /// Internal contact latch used to reset on trigger.
    stick_state: f32,
    /// Smoothed target speed derived from trigger velocity.
    target_speed: f32,
    /// `48 kHz / fs` and per-sample constants derived from it
    /// (`fix-sample-rate-time-constants`); exact 48 kHz values at 48 kHz.
    ratio: f32,
    speed_smoothing: f32,
}

impl BowExciter {
    /// Creates a default bow model with moderate pressure and speed.
    pub fn new() -> Self {
        Self {
            pressure: 0.5,
            speed: 0.3,
            roughness: 0.2,
            slip_curve: 0.5,
            bow_position: 0.0,
            bow_velocity: 0.0,
            stick_state: 0.0,
            target_speed: 0.0,
            ratio: 1.0,
            speed_smoothing: 0.001,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.speed_smoothing = rate::smoother(0.001, self.ratio);
    }

    /// Sets the physical driving parameters.
    ///
    /// `pressure` and `speed` are clamped to `0.0..=5.0`; `roughness` is
    /// clamped to `0.0..=3.0` because it shapes static grip and ripple depth;
    /// `slip_curve` is clamped to `0.05..=3.0` because it controls the width
    /// of the Stribeck transition.
    pub fn set_parameters(&mut self, pressure: f32, speed: f32, roughness: f32, slip_curve: f32) {
        self.pressure = pressure.clamp(0.0, 5.0);
        self.speed = speed.clamp(0.0, 5.0);
        self.roughness = roughness.clamp(0.0, 3.0);
        self.slip_curve = slip_curve.clamp(0.05, 3.0);
    }

    /// Arms the friction state for a new gesture.
    pub fn trigger(&mut self, velocity: f32) {
        self.target_speed = self.speed * (0.5 + velocity * 0.5);
        self.bow_velocity = self.target_speed * 0.1;
        self.stick_state = 0.0;
        self.bow_position = 0.0;
    }

    /// Produces one sample of friction force.
    ///
    /// The model computes a relative velocity, maps it through a Stribeck
    /// curve, and applies a repeating roughness ripple. No allocations or I/O
    /// occur on the audio thread.
    pub fn process_sample(&mut self, resonator_velocity: f32) -> f32 {
        let v_rel = self.bow_velocity - resonator_velocity;
        let abs_v_rel = v_rel.abs();

        let static_mu = 0.4 + self.roughness * 0.3;
        let dynamic_mu = 0.2 + self.roughness * 0.1;
        let stribeck_speed = 0.01 + self.speed * 0.02 * self.slip_curve;

        let mu = if abs_v_rel < stribeck_speed {
            let t = abs_v_rel / stribeck_speed;
            static_mu - (static_mu - dynamic_mu) * t * t * (3.0 - 2.0 * t)
        } else {
            dynamic_mu + (static_mu - dynamic_mu) * (-(abs_v_rel - stribeck_speed) * 10.0).exp()
        };

        let force = self.pressure * mu * v_rel.signum();

        self.bow_velocity += (self.target_speed - self.bow_velocity) * self.speed_smoothing;
        self.bow_position += rate::step(self.bow_velocity, self.ratio);

        if self.bow_position > 1.0 {
            self.bow_position -= 1.0;
        }

        let roughness_mod = 1.0 + self.roughness * (self.bow_position * 2.0 * PI).sin() * 0.3;
        force * roughness_mod * 2.0
    }

    /// Returns whether the exciter still has meaningful force.
    pub fn is_active(&self) -> bool {
        self.pressure > 0.01 && self.target_speed > 0.001
    }

    /// Releases the contact by zeroing the pressure term.
    pub fn release(&mut self) {
        self.pressure = 0.0;
    }
}

impl Default for BowExciter {
    fn default() -> Self {
        Self::new()
    }
}

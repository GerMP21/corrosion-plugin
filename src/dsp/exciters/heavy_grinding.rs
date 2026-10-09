//! Heavy grinding exciter.
//!
//! Models Coulomb friction plus noisy asperity tearing as a scrape moves across
//! rough material. The math combines a signed friction baseline with filtered
//! random variation to approximate rough concrete, sandpaper, and grinding
//! contact inside the `dsp/exciters` friction family.
//! Use it for abrasive drag, metal-on-stone grinding, or harsh industrial wear.

use crate::dsp::utils::{contact, rate};
use crate::dsp::ContactPrediction;
/// Scratchy friction exciter with tearing-noise modulation.
#[derive(Clone, Debug)]
pub struct HeavyGrinding {
    grind_speed: f32,
    grind_pressure: f32,
    surface_grit: f32,
    grit_color: f32,

    // State
    active: bool,
    noise_state: f32,
    noise_filter_state: f32,
    rng_phase: f32,
    /// `48 kHz / fs` (`fix-sample-rate-time-constants`).
    ratio: f32,
    noise_leak: f32,
    /// Last (cutoff, converted filter coefficient), so the per-sample filter
    /// only re-derives its coefficient when the cutoff changes.
    filter_cache: (f32, f32),
}

impl HeavyGrinding {
    /// Creates a default heavy-grinding exciter.
    pub fn new() -> Self {
        Self {
            grind_speed: 0.5,
            grind_pressure: 0.6,
            surface_grit: 0.5,
            grit_color: 0.3,
            active: false,
            noise_state: 0.0,
            noise_filter_state: 0.0,
            rng_phase: 0.0,
            ratio: 1.0,
            noise_leak: 0.99,
            filter_cache: (f32::NAN, 0.0),
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.noise_leak = rate::decay(0.99, self.ratio);
        self.filter_cache = (f32::NAN, 0.0);
    }

    /// Sets grind speed, pressure, grit amount, and noise color.
    pub fn set_parameters(&mut self, speed: f32, pressure: f32, grit: f32, color: f32) {
        self.grind_speed = speed.clamp(0.0, 10.0);
        self.grind_pressure = pressure.clamp(0.0, 10.0);
        self.surface_grit = grit.clamp(0.0, 10.0);
        self.grit_color = color.clamp(0.0, 5.0);
    }

    /// Starts the grinding motion with an initial scrape velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.grind_speed *= 0.3 + velocity * 0.7;
        self.rng_phase = velocity * 100.0;
    }

    /// Processes one sample of friction and tearing noise, given how the
    /// resonator's contact point responds to this sample's force (already
    /// scaled by whatever the voice multiplies the force by).
    ///
    /// The friction is a smooth curve, `P·μ·tanh(v_rel / v_s)`, solved
    /// together with the resonator's velocity response, so it can't flip
    /// sign against a fast mode a sample late and pump it. The tearing noise
    /// follows the grinder's own speed, so the resonator's motion can't feed
    /// it (`stabilize-mallet-and-grinding-contact`).
    pub fn process_sample(&mut self, resonator: ContactPrediction) -> f32 {
        if !self.active || self.grind_speed < 0.001 {
            return 0.0;
        }
        let grind_speed = self.grind_speed;
        let output_scale = grind_speed * 3.0;

        // Asperity tearing noise (Brownian/chaotic)
        let raw_noise = self.generate_noise();

        // Filter based on grit_color: low-pass for heavy concrete, high-pass for fine sandpaper
        // grit_color 0 = heavy concrete (LPF), 1 = sandpaper (HPF)
        let cutoff = if self.grit_color < 0.5 {
            100.0 + self.grit_color * 1900.0 // 100-1000Hz LPF-ish
        } else {
            1000.0 + (self.grit_color - 0.5) * 6000.0 // 1k-4kHz HPF-ish
        };
        let filtered_noise = self.filter_noise(raw_noise, cutoff);

        // Tearing scales with the grinder's own speed, not the slip. That is
        // the slip whenever the resonator is slow (the usual case). Taking it
        // from the slip let the resonator's own motion modulate the noise
        // force that drives it, which built a high-Q mode up to a large
        // limit cycle (Plate C6).
        let tearing = filtered_noise * grind_speed * self.surface_grit * 2.0 * output_scale;

        // Friction: more force speeds the resonator up, lowering the slip.
        let mu_dynamic = 0.4;
        let friction = self.grind_pressure * mu_dynamic * output_scale;
        let slip_scale = (0.05 * grind_speed).max(1e-4);
        contact::solve_force(|force| {
            friction * ((grind_speed - resonator.velocity(force)) / slip_scale).tanh() + tearing
        })
    }

    /// Returns whether the grinder is still engaged.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Releases the contact and stops the grind.
    pub fn release(&mut self) {
        self.active = false;
    }

    fn generate_noise(&mut self) -> f32 {
        // Brownian noise approximation (accumulated random steps)
        self.rng_phase += 0.1;
        let step = (self.rng_phase.sin() * 43_758.547).fract() * 2.0 - 1.0;
        self.noise_state += step * 0.1;
        self.noise_state *= self.noise_leak; // Leak
        self.noise_state.clamp(-1.0, 1.0)
    }

    fn filter_noise(&mut self, input: f32, cutoff_hz: f32) -> f32 {
        // Simple first-order filter
        if self.filter_cache.0 != cutoff_hz {
            let alpha_48k = (cutoff_hz / 10000.0).clamp(0.01, 0.99);
            self.filter_cache = (cutoff_hz, rate::smoother(alpha_48k, self.ratio));
        }
        let alpha = self.filter_cache.1;
        self.noise_filter_state += alpha * (input - self.noise_filter_state);
        self.noise_filter_state
    }
}

impl Default for HeavyGrinding {
    fn default() -> Self {
        Self::new()
    }
}

//! Stiff point scrape exciter.
//!
//! Models a rigid point that catches, bends, and releases as a stiff spring
//! against a moving surface. The math integrates relative motion into a stored
//! deflection term and then converts it back into chatter force.
//! Use it for squeaks, tiny slips, and high-frequency scraping chatter inside
//! the `dsp/exciters` friction family.

use crate::dsp::utils::rate;

/// Weight of the tip's ring in the force.
const CHATTER_RING_GAIN: f32 = 2.0;
/// Weight of the stick force (the wind-up) in the force.
const STICK_GAIN: f32 = 0.3;
/// Rigid-point scrape model with snap-back chatter.
#[derive(Clone, Debug)]
pub struct StiffPointScrape {
    scrape_speed: f32,
    point_pressure: f32,
    chatter_pitch: f32,
    chatter_damping: f32,

    // State
    /// The tip's ring: a two-pole resonator at the chatter pitch that each
    /// slip strikes (`fix-inert-controls`), its last two outputs.
    ring: [f32; 2],
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
            ring: [0.0; 2],
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
        self.ring = [0.0; 2];
        self.accumulated_deflection = 0.0;
        self.scrape_speed *= 0.5 + velocity * 0.5;
    }

    /// Processes one sample of spring wind-up, slip, and chatter.
    ///
    /// The tip sticks and winds up while the surface drags it; past the
    /// slip threshold (set by Point Pressure) it snaps back and rings. The
    /// force is the stick force (the wind-up, up to Point Pressure) plus that
    /// ring, a two-pole resonator at `100 + 900·pitch` Hz whose decay is set
    /// by Chatter Damping. Before `fix-inert-controls` the spring and damping
    /// terms were clamped at ±2 almost every sample, so Chatter Pitch and
    /// Chatter Damping changed nothing.
    pub fn process_sample(&mut self, _resonator_displacement: f32, resonator_velocity: f32) -> f32 {
        if !self.active || self.scrape_speed < 0.001 {
            return 0.0;
        }

        // Relative velocity between scraper and surface
        let v_rel = self.scrape_speed - resonator_velocity;

        // Accumulate deflection (spring winding up)
        let increment = v_rel * rate::step(0.001, self.ratio);
        self.accumulated_deflection += increment;

        // Slip: the tip snaps back, striking its ring. `early` is how far
        // before this sample the threshold was crossed, in samples, so the
        // ring starts at the slip's true time (`resolve-exciter-tradeoffs`
        // Decision 5): slips come faster than the ring dies, and a strike
        // rounded to the sample grid would add to it at a phase that
        // depends on the rate.
        let mut strike = 0.0;
        let mut early = 0.0;
        let over = self.accumulated_deflection.abs() - self.slip_threshold;
        if over > 0.0 {
            strike = self.accumulated_deflection.signum() * self.point_pressure;
            early = (over / increment.abs().max(f32::MIN_POSITIVE)).clamp(0.0, 1.0);
            self.accumulated_deflection *= 0.1;
        }

        // The tip's ring at the chatter pitch.
        let sample_rate = rate::REFERENCE_RATE / self.ratio;
        let hz = (100.0 + self.chatter_pitch * 900.0).min(0.45 * sample_rate);
        let omega = std::f32::consts::TAU * hz / sample_rate;
        // Decay time from about 60 ms (damping 0.1) to 7 ms (0.9).
        let decay_seconds = 0.006 / self.chatter_damping.max(0.01);
        let r = (-1.0 / (decay_seconds * sample_rate)).exp();
        // A strike of amplitude A at `early` samples ago rings as
        // A·r^(n+e)·sin((n+1+e)ω) from this sample on (n = 0, 1, …): the
        // same amplitude whatever the damping, which sets only how long it
        // lasts. Add its value here and its virtual value one sample back,
        // so the recursion carries it on.
        let mut ring = 2.0 * r * omega.cos() * self.ring[0] - r * r * self.ring[1];
        let mut previous = self.ring[0];
        if strike != 0.0 {
            ring += strike * r.powf(early) * ((1.0 + early) * omega).sin();
            previous += strike * r.powf(early - 1.0) * (early * omega).sin();
        }
        self.ring = [ring, previous];

        let stick = self.point_pressure * self.accumulated_deflection / self.slip_threshold;
        CHATTER_RING_GAIN * ring + STICK_GAIN * stick
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

#[cfg(test)]
mod tests {
    use super::*;

    /// One second of force against a still surface at 48 kHz.
    fn render(pitch: f32, damping: f32) -> Vec<f32> {
        let mut scrape = StiffPointScrape::new();
        scrape.set_sample_rate(48_000.0);
        scrape.set_parameters(0.82, 0.8, pitch, damping);
        scrape.trigger(100.0 / 127.0);
        (0..48_000)
            .map(|_| scrape.process_sample(0.0, 0.0))
            .collect()
    }

    /// Mean absolute first difference over mean absolute value: rises
    /// with the force's dominant frequency.
    fn brightness(x: &[f32]) -> f32 {
        let mean = x.iter().sum::<f32>() / x.len() as f32;
        let level: f32 = x.iter().map(|v| (v - mean).abs()).sum();
        let change: f32 = x.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        change / level.max(1e-9)
    }

    /// Energy of the force's first difference: each slip strikes the ring
    /// equally hard, so a longer ring carries more of it.
    fn ring_energy(x: &[f32]) -> f32 {
        x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum()
    }

    /// Spec "A dedicated exciter control acts" (`fix-inert-controls`):
    /// Chatter Pitch moves the chatter's pitch.
    #[test]
    fn chatter_pitch_raises_the_chatter() {
        let (low, high) = (
            brightness(&render(0.1, 0.66)),
            brightness(&render(1.5, 0.66)),
        );
        assert!(
            high > low * 2.0,
            "brightness {low} at pitch 0.1, {high} at 1.5"
        );
    }

    /// Chatter Damping shortens the ring after each slip.
    #[test]
    fn chatter_damping_shortens_the_ring() {
        let (light, heavy) = (
            ring_energy(&render(0.52, 0.1)),
            ring_energy(&render(0.52, 0.9)),
        );
        assert!(
            light > heavy * 2.0,
            "ring energy {light} at damping 0.1, {heavy} at 0.9"
        );
    }
}

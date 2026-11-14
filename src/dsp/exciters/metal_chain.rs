//! Metal chain exciter.
//!
//! Models a cascade of falling links using staggered Hertzian impacts plus a
//! burst of filtered rattle noise on each strike. It belongs to the `dsp/exciters`
//! heavy-impact family and sits between brush-like impulse clusters and hard
//! mallet hits.
//! Use it for dropped chain, rattling hardware, or clustered metallic impacts.

use crate::dsp::utils::noise::Noise;
use crate::dsp::utils::{contact, rate};
use crate::dsp::ContactPrediction;
/// The Chain Mass control's default (`VoiceControls::default`). A link's
/// momentum sets how far it dents the surface: the contact is solved against
/// the surface pushing back, so without this the impulse was the same for any
/// mass (`fix-inert-controls`). At the default the factor is exactly 1.
const DEFAULT_CHAIN_MASS: f32 = 0.8;
/// Hertzian stiffness of a link's impact.
const IMPACT_STIFFNESS: f32 = 2.0e5;
/// How far a link dents the surface, per unit of its travel in a sample.
const IMPACT_DENT: f32 = 3.0;

/// Multi-link impact exciter with stochastic rattle.
#[derive(Clone, Debug)]
pub struct MetalChain {
    link_count: u32,
    chain_mass: f32,
    drop_spread_ms: f32,
    internal_rattle: f32,
    rattle_color: f32,

    // State
    links: Vec<ChainLink>,
    active: bool,
    time_ms: f32,
    sample_rate: f32,
    /// `48 kHz / fs` (`fix-sample-rate-time-constants`).
    ratio: f32,
    /// The rattle's high-pass, kept across samples.
    rattle_filter: OnePoleHighPass,
    /// Places the impacts between samples (`resolve-exciter-tradeoffs`).
    events: rate::EventSplitter,
    /// Noise for this note (`resolve-exciter-tradeoffs`).
    noise: Noise,
}

#[derive(Clone, Debug)]
struct ChainLink {
    impact_time_ms: f32,
    /// Where the link starts its fall, and how fast.
    position: f32,
    velocity: f32,
    impacted: bool,
    mass: f32,
}

impl ChainLink {
    /// Position and velocity `t` seconds into the note (at rest before the
    /// link's drop time).
    fn fall(&self, t: f32) -> (f32, f32) {
        let t = (t - self.impact_time_ms / 1000.0).max(0.0);
        let rate_hz = rate::REFERENCE_RATE;
        let acceleration = 0.03 * rate_hz;
        let velocity = self.velocity + acceleration * t;
        let position =
            self.position + 0.0005 * rate_hz * (self.velocity * t + 0.5 * acceleration * t * t);
        (position, velocity)
    }

    /// How far the link moves in one 48 kHz sample at time `t`.
    fn travel(&self, t: f32) -> f32 {
        (self.fall(t).1 * 0.0005).abs()
    }
}

impl MetalChain {
    /// Upper clamp on `link_count`; `links` is preallocated to this.
    const MAX_LINKS: u32 = 20;

    /// Creates a default metal-chain exciter.
    pub fn new() -> Self {
        Self {
            link_count: 8,
            chain_mass: 0.5,
            drop_spread_ms: 200.0,
            internal_rattle: 0.3,
            rattle_color: 0.5,
            // Full capacity up front so `trigger` (note-on, audio thread)
            // never grows the buffer.
            links: Vec::with_capacity(Self::MAX_LINKS as usize),
            active: false,
            time_ms: 0.0,
            sample_rate: 48000.0,
            ratio: 1.0,
            rattle_filter: OnePoleHighPass::default(),
            events: rate::EventSplitter::default(),
            noise: Noise::default(),
        }
    }

    /// Sets link count, per-link mass, drop spread, rattle, and noise color.
    pub fn set_parameters(
        &mut self,
        links: u32,
        mass: f32,
        spread_ms: f32,
        rattle: f32,
        color: f32,
    ) {
        self.link_count = links.clamp(2, Self::MAX_LINKS);
        self.chain_mass = mass.clamp(0.1, 2.0);
        self.drop_spread_ms = spread_ms.clamp(50.0, 1000.0);
        self.internal_rattle = rattle.clamp(0.0, 1.0);
        self.rattle_color = color.clamp(0.0, 1.0);
        self.update_rattle_filter();
    }

    /// Updates the sample rate used for time stepping and resonator decay.
    pub fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
        self.ratio = rate::ratio(rate);
        self.update_rattle_filter();
    }

    /// Restart the noise for a new note (the voice seeds it per note).
    pub fn reseed(&mut self, seed: u32) {
        self.noise.reseed(seed);
    }

    /// Starts the staggered link-drop sequence.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.time_ms = 0.0;
        self.links.clear();
        self.events.clear();
        self.rattle_filter.reset();

        // Pre-compute random values to avoid borrow issues
        let link_count = self.link_count;
        let drop_spread = self.drop_spread_ms;
        let chain_mass = self.chain_mass;

        // Create links with staggered drop times
        for i in 0..link_count {
            let progress = i as f32 / link_count.max(1) as f32;
            // Randomize drop times slightly for organic feel
            let rand1 = self.noise.unit();
            let rand2 = self.noise.unit();
            let rand3 = self.noise.unit();
            let rand4 = self.noise.unit();

            let random_offset = (rand1 - 0.5) * 0.2;
            let impact_time = progress * drop_spread * (1.0 + random_offset);

            self.links.push(ChainLink {
                impact_time_ms: impact_time.max(0.0),
                position: -0.3 - rand2 * 0.2,
                velocity: velocity * (2.0 + rand3),
                impacted: false,
                mass: chain_mass * (0.8 + rand4 * 0.4),
            });
        }
    }

    /// Processes one sample of link impacts and rattle energy.
    pub fn process_sample(&mut self, resonator: ContactPrediction) -> f32 {
        if !self.active {
            return 0.0;
        }

        let dt_ms = 1000.0 / self.sample_rate;
        self.time_ms += dt_ms;

        let mut total_force = 0.0;
        let mut all_settled = true;

        // Each falling link follows its exact trajectory from its drop time,
        // the motion the old per-sample steps approximated (velocity rising
        // 0.03 and position moving `0.0005 · velocity` per 48 kHz sample), so
        // the moment it reaches the surface doesn't depend on the grid. Every
        // link that strikes this sample is solved together with the
        // resonator's response (`resolve-exciter-tradeoffs`).
        for link in self.links.iter() {
            if !link.impacted {
                all_settled = false;
            }
        }
        let (time_ms, ratio, dt_s) = (self.time_ms, self.ratio, dt_ms / 1000.0);
        let free_surface = resonator.displacement(0.0);
        let striking =
            |link: &ChainLink| !link.impacted && link.fall(time_ms / 1000.0).0 > free_surface;
        let links = &self.links;
        let reach = self.chain_mass / DEFAULT_CHAIN_MASS;
        let impacts = |surface: f32| -> f32 {
            // A link strikes on the sample its tip passes the unloaded
            // surface. Its penetration on that sample depends on where the
            // grid falls, so the impact uses the penetration half a reference
            // sample into the contact: the event's area then depends only on
            // the link's speed. The pushed-back surface reduces it.
            let push = (surface - free_surface) / ratio;
            links
                .iter()
                .filter(|link| striking(link))
                .map(|link| {
                    let penetration =
                        (0.5 * link.travel(time_ms / 1000.0) * reach * IMPACT_DENT - push).max(0.0);
                    rate::impulse(IMPACT_STIFFNESS * penetration.powf(1.5) * link.mass, ratio)
                })
                .sum()
        };
        let impact_force = contact::solve_force(|f| impacts(resonator.displacement(f)));
        // How early in the sample each link reached the surface, weighted
        // by impact.
        let (mut early_weighted, mut weight) = (0.0f32, 0.0f32);
        for link in self.links.iter_mut() {
            if striking(link) {
                let now = time_ms / 1000.0;
                let (position, _) = link.fall(now);
                let (before, _) = link.fall(now - dt_s);
                let share = link.travel(now).powf(1.5) * link.mass;
                let fraction = (position - free_surface) / (position - before).max(1e-9);
                early_weighted += share * fraction.min(1.0);
                weight += share;
                link.impacted = true;
            }
        }
        let early = if weight > 0.0 {
            early_weighted / weight
        } else {
            0.0
        };
        self.events.add(impact_force, early);
        total_force += self.events.take();

        if self.time_ms > self.drop_spread_ms * 0.3 {
            let grind_noise = rate::white_noise(
                self.generate_noise() * self.internal_rattle * 0.1,
                self.ratio,
            );
            total_force += self.rattle_filter.process(grind_noise);
        }

        if all_settled && self.time_ms > self.drop_spread_ms + 100.0 {
            self.active = false;
        }

        total_force
    }

    /// Returns whether any links are still in flight or striking.
    pub fn is_active(&self) -> bool {
        self.active
    }

    fn generate_noise(&mut self) -> f32 {
        self.noise.bipolar()
    }

    fn update_rattle_filter(&mut self) {
        let cutoff = 200.0 + self.rattle_color * 3000.0;
        self.rattle_filter.set_cutoff(cutoff, self.sample_rate);
    }
}

/// One-pole high-pass `y = a·y₋₁ + (1 + a)/2 · (x − x₋₁)`, `a = exp(−2π·fc/fs)`,
/// unit gain at Nyquist and −3 dB at `fc`.
#[derive(Clone, Debug, Default)]
struct OnePoleHighPass {
    coefficient: f32,
    previous_input: f32,
    previous_output: f32,
}

impl OnePoleHighPass {
    fn set_cutoff(&mut self, cutoff_hz: f32, sample_rate: f32) {
        self.coefficient = (-std::f32::consts::TAU * cutoff_hz / sample_rate.max(1.0)).exp();
    }

    fn reset(&mut self) {
        self.previous_input = 0.0;
        self.previous_output = 0.0;
    }

    fn process(&mut self, input: f32) -> f32 {
        let a = self.coefficient;
        let output = a * self.previous_output + 0.5 * (1.0 + a) * (input - self.previous_input);
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

impl Default for MetalChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::OnePoleHighPass;

    /// Steady-state gain of the filter for a sine at `freq`.
    fn gain(cutoff: f32, freq: f32, sample_rate: f32) -> f32 {
        let mut filter = OnePoleHighPass::default();
        filter.set_cutoff(cutoff, sample_rate);
        let n = (sample_rate * 0.5) as usize;
        let mut peak = 0.0f32;
        for i in 0..n {
            let x = (std::f32::consts::TAU * freq * i as f32 / sample_rate).sin();
            let y = filter.process(x);
            if i > n / 2 {
                peak = peak.max(y.abs());
            }
        }
        peak
    }

    #[test]
    fn rattle_high_pass_is_3_db_down_at_its_cutoff() {
        for sample_rate in [44_100.0, 48_000.0, 96_000.0] {
            for cutoff in [200.0, 1_000.0] {
                let db = 20.0 * gain(cutoff, cutoff, sample_rate).log10();
                assert!(
                    (db + 3.0).abs() < 0.5,
                    "{cutoff} Hz @ {sample_rate}: {db:.2} dB at the cutoff"
                );
                let pass = 20.0 * gain(cutoff, cutoff * 10.0, sample_rate).log10();
                assert!(
                    pass > -0.5,
                    "{cutoff} Hz @ {sample_rate}: {pass:.2} dB a decade up"
                );
                let stop = 20.0 * gain(cutoff, cutoff / 10.0, sample_rate).log10();
                assert!(
                    stop < -15.0,
                    "{cutoff} Hz @ {sample_rate}: {stop:.2} dB a decade down"
                );
            }
        }
    }
}

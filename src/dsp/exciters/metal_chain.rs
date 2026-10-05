//! Metal chain exciter.
//!
//! Models a cascade of falling links using staggered Hertzian impacts plus a
//! burst of filtered rattle noise on each strike. It belongs to the `dsp/exciters`
//! heavy-impact family and sits between brush-like impulse clusters and hard
//! mallet hits.
//! Use it for dropped chain, rattling hardware, or clustered metallic impacts.

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
    noise_state: f32,
    rng_phase: f32,
}

#[derive(Clone, Debug)]
struct ChainLink {
    impact_time_ms: f32,
    position: f32,
    velocity: f32,
    impacted: bool,
    mass: f32,
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
            noise_state: 0.0,
            rng_phase: 0.0,
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
    }

    /// Updates the sample rate used for time stepping and resonator decay.
    pub fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
    }

    /// Starts the staggered link-drop sequence.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.time_ms = 0.0;
        self.links.clear();
        self.rng_phase = velocity * 50.0;

        // Pre-compute random values to avoid borrow issues
        let link_count = self.link_count;
        let drop_spread = self.drop_spread_ms;
        let chain_mass = self.chain_mass;

        // Create links with staggered drop times
        for i in 0..link_count {
            let progress = i as f32 / link_count.max(1) as f32;
            // Randomize drop times slightly for organic feel
            let rand1 = self.pseudo_random(i);
            let rand2 = self.pseudo_random(i + 100);
            let rand3 = self.pseudo_random(i + 200);
            let rand4 = self.pseudo_random(i + 300);

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
    pub fn process_sample(&mut self, resonator_displacement: f32, _resonator_velocity: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        let dt_ms = 1000.0 / self.sample_rate;
        self.time_ms += dt_ms;

        let mut total_force = 0.0;
        let mut all_settled = true;

        let (_, alpha) = self.hpf_params();
        let mut filter_state = self.noise_state;

        for link in self.links.iter_mut() {
            if self.time_ms >= link.impact_time_ms && !link.impacted {
                link.position += link.velocity * 0.0005;
                link.velocity += 0.03;

                let penetration = (link.position - resonator_displacement).max(0.0);
                if penetration > 0.0 {
                    let stiffness = 2.0;
                    let impact_force = stiffness * penetration.powf(1.5) * link.mass;
                    total_force += impact_force;

                    link.velocity *= -0.4;
                    link.impacted = true;
                }

                all_settled = false;
            } else if !link.impacted {
                all_settled = false;
            }
        }

        self.noise_state = filter_state;

        if self.time_ms > self.drop_spread_ms * 0.3 {
            let grind_noise = self.generate_noise() * self.internal_rattle * 0.1;
            total_force += Self::apply_hpf(grind_noise, alpha, &mut filter_state);
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
        self.rng_phase += crate::dsp::utils::GOLDEN_STEP;
        let noise = (self.rng_phase.sin() * 43_758.547).fract();
        noise * 2.0 - 1.0
    }

    fn hpf_params(&self) -> (f32, f32) {
        let cutoff = 200.0 + self.rattle_color * 3000.0;
        let alpha = cutoff / (cutoff + 1000.0 / self.sample_rate);
        (cutoff, alpha)
    }

    fn apply_hpf(input: f32, alpha: f32, state: &mut f32) -> f32 {
        let output = alpha * (*state + input);
        *state = output;
        (input - output).max(0.0)
    }

    fn pseudo_random(&mut self, seed: u32) -> f32 {
        let phase = self.rng_phase + seed as f32 * 0.12345;
        let hash = (phase.sin() * 43_758.547).fract();
        hash.abs()
    }
}

impl Default for MetalChain {
    fn default() -> Self {
        Self::new()
    }
}

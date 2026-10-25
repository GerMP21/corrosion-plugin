//! Specialty exciter collection.
//!
//! This module groups several non-standard physical models used by
//! `dsp/exciters` when the sound source is not a plain strike or friction tool.
//! It includes pneumatic turbulence, electromagnetic hum, tension snaps, and
//! granular particle emission, each expressed with lightweight real-time math.
//! Use these models for air jets, electrical buzz, release clicks, and rain-like
//! granular motion.

use crate::dsp::utils::noise::Noise;
use crate::dsp::utils::rate;
/// Turbulent air-jet exciter with band-limited noise shaping.
#[derive(Clone, Debug)]
pub struct PneumaticJet {
    air_pressure: f32,
    nozzle_width: f32,
    turbulence_chaos: f32,
    active: bool,
    /// Noise for this note (`resolve-exciter-tradeoffs`).
    noise: Noise,
    filter_state: f32,
    /// `48 kHz / fs` and the band filter's coefficient for it
    /// (`fix-sample-rate-time-constants`).
    ratio: f32,
    filter_alpha: f32,
}

impl PneumaticJet {
    /// Creates a default pneumatic-jet exciter.
    pub fn new() -> Self {
        Self {
            air_pressure: 0.6,
            nozzle_width: 0.5,
            turbulence_chaos: 0.3,
            active: false,
            noise: Noise::default(),
            filter_state: 0.0,
            ratio: 1.0,
            filter_alpha: (500.0 + 0.5 * 2000.0) / 10000.0,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.update_filter_alpha();
    }

    fn update_filter_alpha(&mut self) {
        let center_freq = 500.0 + self.nozzle_width * 2000.0;
        self.filter_alpha = rate::smoother(center_freq / 10000.0, self.ratio);
    }

    /// Sets air pressure, nozzle width, and turbulence chaos.
    pub fn set_parameters(&mut self, pressure: f32, width: f32, chaos: f32) {
        self.air_pressure = pressure.clamp(0.0, 10.0);
        self.nozzle_width = width.clamp(0.01, 5.0);
        self.turbulence_chaos = chaos.clamp(0.0, 10.0);
        self.update_filter_alpha();
    }

    /// Restart the noise for a new note (the voice seeds it per note).
    pub fn reseed(&mut self, seed: u32) {
        self.noise.reseed(seed);
    }

    /// Starts the airflow burst with an initial drive velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.air_pressure *= 0.3 + velocity * 0.7;
    }

    /// Processes one sample of jet turbulence and filtered noise.
    pub fn process_sample(&mut self, _res_disp: f32, res_vel: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let v_rel = self.air_pressure - res_vel;
        let noise = rate::white_noise(self.noise.bipolar(), self.ratio);
        // Non-linear jet saturation keeps the air burst from exploding in level.
        let saturated = v_rel * v_rel - self.turbulence_chaos * v_rel * v_rel * v_rel;
        let raw = noise * saturated.max(0.0);
        let q = 0.5 + self.nozzle_width * 4.0;
        self.filter_state += self.filter_alpha * (raw - self.filter_state);
        (raw - self.filter_state * q).clamp(-2.0, 2.0) * self.air_pressure
    }

    /// Returns whether the jet is still active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Stops the airflow model.
    pub fn release(&mut self) {
        self.active = false;
    }
}

impl Default for PneumaticJet {
    fn default() -> Self {
        Self::new()
    }
}

/// Continuous electromagnetic hum source.
#[derive(Clone, Debug)]
pub struct ElectromagneticHum {
    mains_frequency: f32,
    coil_proximity: f32,
    voltage_sag: f32,
    active: bool,
    phase: f32,
    sample_rate: f32,
}

impl ElectromagneticHum {
    /// Creates a default electromagnetic-hum exciter.
    pub fn new() -> Self {
        Self {
            mains_frequency: 60.0,
            coil_proximity: 0.5,
            voltage_sag: 0.0,
            active: false,
            phase: 0.0,
            sample_rate: 48000.0,
        }
    }

    /// Sets mains frequency, coil proximity, and voltage sag.
    pub fn set_parameters(&mut self, freq: f32, proximity: f32, sag: f32) {
        self.mains_frequency = freq.clamp(20.0, 200.0);
        self.coil_proximity = proximity.clamp(0.0, 10.0);
        self.voltage_sag = sag.clamp(0.0, 5.0);
    }

    /// Sets the host sample rate the hum's phase advances at. (The voice
    /// never set this before `fix-sample-rate-time-constants`, so the hum's
    /// pitch was only right at 48 kHz.)
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    /// Starts the hum source with a stronger initial coupling.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.coil_proximity *= 0.3 + velocity * 0.7;
    }

    /// Processes one sample of hum fundamental and harmonics.
    pub fn process_sample(&mut self, _res_disp: f32, _res_vel: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let dt = 1.0 / self.sample_rate;
        self.phase += 2.0 * std::f32::consts::PI * self.mains_frequency * dt;
        if self.phase > 2.0 * std::f32::consts::PI {
            self.phase -= 2.0 * std::f32::consts::PI;
        }
        let fundamental = self.phase.sin();
        // Small harmonic set keeps the hum alive without becoming a tone stack.
        let harmonic3 = (self.phase * 3.0).sin() * self.voltage_sag;
        let harmonic5 = (self.phase * 5.0).sin() * self.voltage_sag * 0.5;
        (fundamental + harmonic3 + harmonic5) * self.coil_proximity * 2.0
    }

    /// Returns whether the hum source is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Stops the hum source.
    pub fn release(&mut self) {
        self.active = false;
    }
}

impl Default for ElectromagneticHum {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook-and-release tension exciter.
#[derive(Clone, Debug)]
pub struct TensionSnap {
    pull_distance: f32,
    hook_stiffness: f32,
    snap_force: f32,
    active: bool,
    hooked: bool,
    pull_position: f32,
    tension: f32,
    /// `48 kHz / fs` (`fix-sample-rate-time-constants`).
    ratio: f32,
}

impl TensionSnap {
    /// Creates a default tension-snap exciter.
    pub fn new() -> Self {
        Self {
            pull_distance: 0.5,
            hook_stiffness: 1.0,
            snap_force: 0.7,
            active: false,
            hooked: false,
            pull_position: 0.0,
            tension: 0.0,
            ratio: 1.0,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
    }

    /// Sets pull distance, hook stiffness, and snap threshold.
    pub fn set_parameters(&mut self, distance: f32, stiffness: f32, snap: f32) {
        self.pull_distance = distance.clamp(0.01, 10.0);
        self.hook_stiffness = stiffness.clamp(0.01, 20.0);
        self.snap_force = snap.clamp(0.01, 20.0);
    }

    /// Arms the hook and seeds the initial pull velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.hooked = true;
        self.pull_position = 0.0;
        self.tension = 0.0;
        self.snap_force *= 0.5 + velocity * 0.5;
    }

    /// Processes one sample of tension build-up and release.
    pub fn process_sample(&mut self, res_disp: f32, _res_vel: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        if self.hooked {
            self.pull_position += rate::step(0.001, self.ratio);
            let dx = self.pull_position - res_disp;
            self.tension = self.hook_stiffness * dx.max(0.0);
            if self.tension > self.snap_force || self.pull_position > self.pull_distance {
                self.hooked = false;
                // One-sample snap spike: keep its area at any rate.
                return self.tension * 3.0 / self.ratio;
            }
            return self.tension * 0.1;
        }
        0.0
    }

    /// Returns whether the hook is still engaged.
    pub fn is_active(&self) -> bool {
        self.active && self.hooked
    }

    /// Drops the hook and stops the tension model.
    pub fn release(&mut self) {
        self.active = false;
    }
}

impl Default for TensionSnap {
    fn default() -> Self {
        Self::new()
    }
}

/// Granular emission exciter with many tiny impacts.
#[derive(Clone, Debug)]
pub struct ParticleRain {
    flow_rate: f32,
    particle_mass: f32,
    mass_variance: f32,
    active: bool,
    spawn_accumulator: f32,
    particles: Vec<Particle>,
    /// Noise for this note (`resolve-exciter-tradeoffs`).
    noise: Noise,
    /// `48 kHz / fs` (`fix-sample-rate-time-constants`).
    ratio: f32,
}

#[derive(Clone, Debug)]
struct Particle {
    age: f32,
    mass: f32,
    velocity: f32,
}

impl ParticleRain {
    /// Most particles alive at once. Spawning is at most 2 per sample (flow is
    /// clamped to 20) and a particle lives at most 30 samples, so 60 is the
    /// physical ceiling; the buffer is preallocated to this so processing
    /// never allocates.
    const MAX_PARTICLES: usize = 64;

    /// Creates a default particle-rain exciter.
    pub fn new() -> Self {
        Self {
            flow_rate: 0.5,
            particle_mass: 0.1,
            mass_variance: 0.3,
            active: false,
            spawn_accumulator: 0.0,
            particles: Vec::with_capacity(Self::MAX_PARTICLES),
            noise: Noise::default(),
            ratio: 1.0,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
    }

    /// Sets flow rate, particle mass, and mass variance.
    pub fn set_parameters(&mut self, flow: f32, mass: f32, variance: f32) {
        self.flow_rate = flow.clamp(0.0, 20.0);
        self.particle_mass = mass.clamp(0.001, 10.0);
        self.mass_variance = variance.clamp(0.0, 5.0);
    }

    /// Restart the noise for a new note (the voice seeds it per note).
    pub fn reseed(&mut self, seed: u32) {
        self.noise.reseed(seed);
    }

    /// Starts particle emission with a velocity-scaled flow.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.flow_rate *= 0.3 + velocity * 0.7;
    }

    /// Processes one sample of particle spawning and impact decay.
    pub fn process_sample(&mut self, res_disp: f32, _res_vel: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        self.spawn_accumulator += rate::step(self.flow_rate * 0.1, self.ratio);
        while self.spawn_accumulator > 1.0 {
            self.spawn_accumulator -= 1.0;
            if self.particles.len() == Self::MAX_PARTICLES {
                continue;
            }
            let variance = self.noise.bipolar() * self.mass_variance;
            self.particles.push(Particle {
                age: 0.0,
                mass: (self.particle_mass * (1.0 + variance)).max(0.01),
                velocity: -0.5,
            });
        }

        let mut total_force = 0.0;
        let decay = 0.99;
        let (velocity_step, age_step) =
            (rate::step(0.05, self.ratio), rate::step(0.001, self.ratio));
        self.particles.retain_mut(|p| {
            p.velocity += velocity_step;
            p.age += age_step;
            let penetration = (p.velocity * 0.01 - res_disp).max(0.0);
            // Each particle contributes a tiny force spike when it penetrates.
            let force = if penetration > 0.0 {
                p.mass * penetration * 10.0
            } else {
                0.0
            };
            total_force += force;
            p.age < 0.1 && p.velocity < 1.0
        });

        total_force * decay
    }

    /// Returns whether emission is still active or particles remain.
    pub fn is_active(&self) -> bool {
        self.active || !self.particles.is_empty()
    }

    /// Stops spawning new particles.
    pub fn release(&mut self) {
        self.active = false;
    }
}

impl Default for ParticleRain {
    fn default() -> Self {
        Self::new()
    }
}

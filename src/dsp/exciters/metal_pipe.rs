//! Metal pipe exciter.
//!
//! Models a stiff metallic impact where the exciter also contains a small modal
//! resonator bank. The contact force drives both the target and the pipe's own
//! ringing, making it a coupled impact/oscillator model inside `dsp/exciters`.
//! Use it for pipe strikes, metallic boings, and resonant hollow-tube impacts.

use crate::dsp::utils::{contact, rate};
use crate::dsp::ContactPrediction;
/// How many resonator displacement units one unit of the pipe's travel
/// spans: its drive level was raised from 0.107 to 0.541
/// (`resolve-exciter-tradeoffs`), and through this scale the pipe meets the
/// surface it met at the old level, so its Hertz contact stays
/// force-limited instead of collapsing into a displacement-limited chatter
/// whose level follows the sample grid (see `HAND_SCALE` in `hand_strike`).
pub const PIPE_SCALE: f32 = 5.06;

/// Coupled impact exciter with internal pipe modes.
#[derive(Clone, Debug)]
pub struct MetalPipe {
    pipe_mass: f32,
    metal_stiffness: f32,
    pipe_pitch: f32,
    pipe_ring_decay: f32,

    // State
    pipe_position: f32,
    pipe_velocity: f32,
    pipe_modes: Vec<PipeMode>,
    active: bool,
    sample_rate: f32,
    /// `48 kHz / fs` and the per-sample velocity decay derived from it
    /// (`fix-sample-rate-time-constants`).
    ratio: f32,
    velocity_decay: f32,
}

#[derive(Clone, Debug)]
struct PipeMode {
    frequency_hz: f32,
    gain: f32,
    y1: f32,
    y2: f32,
}

impl MetalPipe {
    const PIPE_MODE_FREQUENCIES: [f32; 3] = [800.0, 2100.0, 4500.0]; // High metallic ring

    /// Creates a default metal-pipe exciter and initializes pipe modes.
    pub fn new() -> Self {
        let mut exciter = Self {
            pipe_mass: 1.0,
            metal_stiffness: 2.5, // Extreme high
            pipe_pitch: 1.0,
            pipe_ring_decay: 0.9995,
            pipe_position: 0.0,
            pipe_velocity: 0.0,
            pipe_modes: Vec::new(),
            active: false,
            sample_rate: 48000.0,
            ratio: 1.0,
            velocity_decay: 0.995,
        };
        exciter.init_pipe_modes();
        exciter
    }

    fn init_pipe_modes(&mut self) {
        self.pipe_modes = Self::PIPE_MODE_FREQUENCIES
            .iter()
            .enumerate()
            .map(|(i, &freq)| {
                PipeMode {
                    frequency_hz: freq,
                    gain: 1.0 / (i + 1) as f32, // Higher modes quieter
                    y1: 0.0,
                    y2: 0.0,
                }
            })
            .collect();
    }

    /// Sets mass, stiffness, pitch, and ringing decay.
    pub fn set_parameters(&mut self, mass: f32, stiffness: f32, pitch: f32, decay: f32) {
        self.pipe_mass = mass.clamp(0.01, 30.0);
        self.metal_stiffness = stiffness.clamp(0.01, 20.0);
        self.pipe_pitch = pitch.clamp(0.1, 5.0);
        self.pipe_ring_decay = decay.clamp(0.9, 0.99999);
    }

    /// Updates the sample rate used by the modal resonator bank.
    pub fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
        self.ratio = rate::ratio(rate);
        self.velocity_decay = rate::decay(0.995, self.ratio);
    }

    /// Starts the pipe strike and resets modal state.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.pipe_position = -0.2;
        self.pipe_velocity = velocity * 4.0;

        // Reset pipe modes
        for mode in &mut self.pipe_modes {
            mode.y1 = 0.0;
            mode.y2 = 0.0;
        }
    }

    /// Processes one sample of coupled contact and pipe resonance.
    pub fn process_sample(&mut self, target: ContactPrediction) -> f32 {
        if !self.active {
            return 0.0;
        }
        let target = target.in_units(PIPE_SCALE);

        // Update pipe physics
        self.pipe_position += self.pipe_velocity * rate::step(0.0005, self.ratio);

        // Hertzian contact, solved against this sample's resonator response
        // (`resolve-exciter-tradeoffs`).
        let (position, stiffness, mass) =
            (self.pipe_position, self.metal_stiffness, self.pipe_mass);
        let contact_force = contact::solve_force(|f| {
            stiffness * (position - target.displacement(f)).max(0.0).powf(1.5) * mass
        });
        let penetration = (position - target.displacement(contact_force)).max(0.0);

        // Process pipe modes (the pipe rings from the collision)
        let pipe_output = self.process_pipe_modes(-contact_force);

        // Pipe ringing feeds back into pipe position (resonance)
        self.pipe_velocity += pipe_output * rate::step(0.01, self.ratio);

        // Bounce physics
        if penetration > 0.0 {
            self.pipe_velocity -= rate::step(contact_force * 0.05 / self.pipe_mass, self.ratio);
        } else {
            self.pipe_velocity += rate::step(0.02, self.ratio); // Gravity
        }

        // Damping
        self.pipe_velocity *= self.velocity_decay;

        // Deactivate when settled
        if self.pipe_position > 0.5 && self.pipe_velocity.abs() < 0.01 {
            self.active = false;
        }

        // Output force to target
        contact_force
    }

    fn process_pipe_modes(&mut self, input_force: f32) -> f32 {
        let mut output = 0.0;
        let dt = 1.0 / self.sample_rate;
        // The ring decay is a per-48 kHz-sample factor, and the input gain
        // scales as `ratio²` so a force of a given area rings at the same
        // amplitude at any rate (the ring's gain grows as `fs/ω` per sample,
        // and each sample carries `1/fs` of the area).
        let ring_decay = rate::decay(self.pipe_ring_decay, self.ratio);
        let input_gain = (1.0 - self.pipe_ring_decay) * self.ratio * self.ratio;

        for mode in &mut self.pipe_modes {
            let freq = mode.frequency_hz * self.pipe_pitch;
            let omega = 2.0 * std::f32::consts::PI * freq;

            // Simple resonator: y'' + 2*d*y' + w^2*y = F
            // Using simplified difference equation
            let coeff_a1 = -2.0 * (omega * dt).cos() * ring_decay;
            let coeff_a2 = ring_decay * ring_decay;
            let coeff_b0 = input_gain * mode.gain;

            let sample = coeff_b0 * input_force - coeff_a1 * mode.y1 - coeff_a2 * mode.y2;
            mode.y2 = mode.y1;
            mode.y1 = sample;

            output += sample;
        }

        output
    }

    /// Returns whether the pipe strike is still ringing.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for MetalPipe {
    fn default() -> Self {
        Self::new()
    }
}

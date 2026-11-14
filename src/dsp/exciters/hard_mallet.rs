//! Hard mallet exciter.
//!
//! A rigid mallet thrown at the surface. While it is in contact it pushes
//! with a dissipative Hertzian (Hunt–Crossley) force: `k·d^1.5`, damped in
//! proportion to the penetration `d`. The contact is solved together with
//! the resonator's response to this sample's force, so the loop has no
//! one-sample lag and can't run away; the mallet then rebounds and leaves
//! (`stabilize-mallet-and-grinding-contact`).

use crate::dsp::utils::{contact, rate};
use crate::dsp::ContactPrediction;

/// Mallet travel per 48 kHz sample per unit of `mallet_velocity`.
const MALLET_STEP: f32 = 0.0005;
/// Impact Damping's default (`VoiceControls::default`).
const DEFAULT_IMPACT_DAMPING: f32 = 0.46;
/// How strongly Impact Damping takes the rebound away.
const DAMPING_REBOUND: f32 = 1.0;

/// Gap, in contact-space units, beyond which a mallet moving away has left.
const LEAVE_GAP: f32 = 0.02;

/// Longest a strike may last, in 48 kHz samples (0.25 s), however the
/// resonator moves under the mallet.
const MAX_STRIKE_FRAMES: u32 = 12_000;

/// Rigid mallet contact with a dissipative Hertzian force.
#[derive(Clone, Debug)]
pub struct HardMallet {
    /// Overall momentum multiplier.
    mallet_mass: f32,
    /// Contact stiffness controlling brightness and bite.
    material_stiffness: f32,
    /// Contact damping: how much energy each impact loses.
    impact_damping: f32,
    /// Normalized mallet position in contact space.
    mallet_position: f32,
    /// Normalized mallet velocity in contact space.
    mallet_velocity: f32,
    /// Whether the mallet is still interacting with the surface.
    active: bool,
    /// Whether the mallet has touched the surface yet this strike.
    contacted: bool,
    /// Host samples since the strike started.
    strike_frames: u32,
    /// `48 kHz / fs` and per-sample constants derived from it
    /// (`fix-sample-rate-time-constants`); exact 48 kHz values at 48 kHz.
    ratio: f32,
    velocity_decay: f32,
    max_strike_frames: u32,
}

impl HardMallet {
    /// Creates a default rigid mallet.
    pub fn new() -> Self {
        Self {
            mallet_mass: 2.0,
            material_stiffness: 1.5,
            impact_damping: 0.7,
            mallet_position: 0.0,
            mallet_velocity: 0.0,
            active: false,
            contacted: false,
            strike_frames: 0,
            ratio: 1.0,
            velocity_decay: 0.98,
            max_strike_frames: MAX_STRIKE_FRAMES,
        }
    }

    /// Re-derives the per-sample constants for `sample_rate`.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.ratio = rate::ratio(sample_rate);
        self.velocity_decay = rate::decay(0.98, self.ratio);
        self.max_strike_frames = rate::frames(MAX_STRIKE_FRAMES, self.ratio);
    }

    /// Updates mass, stiffness, and damping.
    pub fn set_parameters(&mut self, mass: f32, stiffness: f32, damping: f32) {
        self.mallet_mass = mass.clamp(0.01, 50.0);
        self.material_stiffness = stiffness.clamp(0.01, 20.0);
        self.impact_damping = damping.clamp(0.01, 2.0);
    }

    /// Starts a new strike with the supplied velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.contacted = false;
        self.strike_frames = 0;
        self.mallet_position = -0.3;
        self.mallet_velocity = velocity * 4.0 + 2.0;
    }

    /// Produces one sample of contact force, given how the resonator's
    /// contact point responds to this sample's force (already scaled by
    /// whatever the voice multiplies the force by).
    pub fn process_sample(&mut self, resonator: ContactPrediction) -> f32 {
        if !self.active {
            return 0.0;
        }
        self.strike_frames += 1;

        // The throw: pulled toward the surface, slowed by the air, until the
        // first touch. After that only the contact acts on the mallet.
        if !self.contacted {
            self.mallet_velocity += rate::step(0.02, self.ratio);
            self.mallet_velocity *= self.velocity_decay;
        }
        self.mallet_position += self.mallet_velocity * rate::step(MALLET_STEP, self.ratio);

        let (mass, stiffness) = (self.mallet_mass, self.material_stiffness);
        // Damping per unit of relative velocity, in 48 kHz-sample units;
        // the strike's own velocity (~5 · MALLET_STEP) gives λ·v ≈ 0.1·damping·5.
        let lambda = self.impact_damping * 0.1 / MALLET_STEP;
        let mallet_speed = self.mallet_velocity * MALLET_STEP;
        let position = self.mallet_position;
        let law = |force: f32| {
            let penetration = position - resonator.displacement(force);
            if penetration <= 0.0 {
                return 0.0;
            }
            let v_rel = mallet_speed - resonator.velocity(force);
            mass * stiffness * penetration.powf(1.5) * (1.0 + lambda * v_rel).max(0.0)
        };
        let force = contact::solve_force(law).clamp(0.0, 100.0);

        if force > 0.0 {
            self.contacted = true;
            // Reaction on the mallet, as the old energy transfer scaled it.
            // On the way out the contact gives back less the more it is
            // damped, so a damped mallet leaves late and the strike is dead;
            // exactly the old reaction at the default damping
            // (`fix-inert-controls`: the force is set by the surface it
            // meets, so the damping in the contact law alone did nothing).
            let give_back = if self.mallet_velocity < 0.0 {
                (DEFAULT_IMPACT_DAMPING / self.impact_damping).powf(DAMPING_REBOUND)
            } else {
                1.0
            };
            self.mallet_velocity -= rate::step(0.3 * force / (mass * mass), self.ratio) * give_back;
        } else if self.contacted && self.mallet_velocity < 0.0 {
            let gap = resonator.displacement(0.0) - self.mallet_position;
            if gap > LEAVE_GAP {
                self.active = false;
            }
        }
        if self.strike_frames >= self.max_strike_frames
            || (self.mallet_position < -0.5 && self.mallet_velocity < 0.0)
        {
            self.active = false;
        }

        force
    }

    /// Returns whether the mallet is still active.
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for HardMallet {
    fn default() -> Self {
        Self::new()
    }
}

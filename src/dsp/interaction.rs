/// Bidirectional coupling between exciter and resonator.
///
/// The voice layer uses this bus to convert strike position and resonator
/// feedback into per-mode forces every sample without allocations.
use std::f32::consts::PI;

/// Calculates mode shape coefficient for a given strike position on a 1D object.
///
/// For a 1D object (pipe, cable, etc.), the mode shape at position `P` is
/// `c_n(P) = sin(n * π * P)`, with `P` normalized to `[0, 1]`.
pub fn mode_coefficient_1d(mode_index: usize, position: f32) -> f32 {
    let p = position.clamp(0.0, 1.0);
    let n = (mode_index + 1) as f32;
    (n * PI * p).sin()
}

// `mode_coefficient_2d` (rectangular plate) and `mode_coefficient_circular`
// (cog/blade) were specced for higher-fidelity object models but the
// algorithmic resonator path generates its own per-mode coefficients today.
// Removed 2026-05-28 as part of the P3.1 dead-code sweep — re-introduce them
// alongside the upgraded model if a future feature needs them.

/// Tracks the bidirectional exchange between exciter and resonator.
#[derive(Clone, Debug)]
pub struct InteractionState {
    /// Strike position normalized `[0, 1]`.
    pub strike_position: f32,
    /// Coupling stiffness `[0, 1]` from feed-forward to full feedback.
    pub coupling_stiffness: f32,
    /// Current resonator displacement at the strike point.
    pub resonator_displacement: f32,
    /// Current resonator velocity at the strike point.
    pub resonator_velocity: f32,
    /// Force calculated by the exciter for the current sample.
    pub exciter_force: f32,
    /// Whether fundamental lock is active.
    pub fundamental_lock: bool,
    /// Minimum coefficient for the fundamental when locked.
    pub fundamental_minimum: f32,
    /// Normalized position wander amount.
    pub position_wander_amount: f32,
    /// Position wander rate in radians per sample.
    pub position_wander_rate: f32,
    /// Current wander phase.
    pub position_wander_phase: f32,
    /// How far the note's force envelope moves the strike position this
    /// sample (`position_envelope · envelope · 0.5`), set by the voice each
    /// sample before the position is updated
    /// (`h09-route-envelope-to-strike-position`).
    pub envelope_offset: f32,
    /// The effective strike position after the last update.
    pub effective_position: f32,
}

impl InteractionState {
    /// Create the default interaction state.
    pub fn new() -> Self {
        Self {
            strike_position: 0.5,
            coupling_stiffness: 1.0,
            resonator_displacement: 0.0,
            resonator_velocity: 0.0,
            exciter_force: 0.0,
            fundamental_lock: true,
            fundamental_minimum: 0.3,
            position_wander_amount: 0.0,
            position_wander_rate: 0.1,
            position_wander_phase: 0.0,
            envelope_offset: 0.0,
            effective_position: 0.5,
        }
    }

    /// Set the strike position.
    pub fn set_strike_position(&mut self, position: f32) {
        self.strike_position = position.clamp(0.0, 1.0);
    }

    /// Enable bounded position wandering.
    pub fn set_position_wander(&mut self, amount: f32, rate_hz: f32, sample_rate: f32) {
        self.position_wander_amount = amount.clamp(0.0, 0.5);
        self.position_wander_rate = rate_hz / sample_rate * 2.0 * PI;
    }

    /// Set how far the force envelope moves the strike position this
    /// sample: `amount · envelope · 0.5`, with both clamped to 0..1. At
    /// amount 0 the offset is exactly 0, so the position is unchanged.
    pub fn set_envelope_offset(&mut self, amount: f32, envelope: f32) {
        self.envelope_offset = amount.clamp(0.0, 1.0) * envelope.clamp(0.0, 1.0) * 0.5;
    }

    /// Advance wander phase and return the current effective position:
    /// the strike position plus the envelope offset and the wander, clamped
    /// to 0..1.
    pub fn update_position(&mut self) -> f32 {
        self.position_wander_phase += self.position_wander_rate;
        if self.position_wander_phase > 2.0 * PI {
            self.position_wander_phase -= 2.0 * PI;
        }

        let wander = self.position_wander_phase.sin() * self.position_wander_amount;
        self.effective_position =
            (self.strike_position + self.envelope_offset + wander).clamp(0.0, 1.0);
        self.effective_position
    }

    /// Calculate per-mode excitation coefficients for the current position.
    pub fn calculate_mode_coefficients(&mut self, mode_count: usize) -> Vec<f32> {
        let current_position = self.update_position();

        let mut coefficients: Vec<f32> = (0..mode_count)
            .map(|i| mode_coefficient_1d(i, current_position))
            .collect();

        if self.fundamental_lock && !coefficients.is_empty() {
            coefficients[0] = coefficients[0].max(self.fundamental_minimum);
        }

        coefficients
    }

    /// Set the force calculated by the exciter.
    pub fn set_exciter_force(&mut self, force: f32) {
        self.exciter_force = force;
    }

    // `update_resonator_state` and `distribute_force_to_modes` allocated a
    // `Vec<f32>` per call and were never invoked from the audio path. Removed
    // 2026-05-28 (P3.1).
}

impl Default for InteractionState {
    /// Construct the default interaction state.
    fn default() -> Self {
        Self::new()
    }
}

/// Main bus managing the feedback loop between exciter and resonator.
#[derive(Clone, Debug)]
pub struct BidirectionalInteractionBus {
    /// Shared state used by the voice and resonator.
    pub state: InteractionState,
    /// Cached per-mode coupling coefficients.
    pub mode_coefficients: Vec<f32>,
    /// What `mode_coefficients` were last computed for: (position, lock,
    /// lock minimum, mode count). They are a pure function of these, so an
    /// unchanged key skips the per-mode `sin` (`resolve-exciter-tradeoffs`).
    cached_for: Option<(f32, bool, f32, usize)>,
}

impl BidirectionalInteractionBus {
    /// Create an empty interaction bus.
    pub fn new() -> Self {
        Self {
            state: InteractionState::new(),
            mode_coefficients: Vec::new(),
            cached_for: None,
        }
    }

    /// Create an empty bus whose coefficient buffer can hold `mode_capacity`
    /// modes without reallocating.
    pub fn with_capacity(mode_capacity: usize) -> Self {
        Self {
            state: InteractionState::new(),
            mode_coefficients: Vec::with_capacity(mode_capacity),
            cached_for: None,
        }
    }

    /// Return to the freshly-created state for `mode_count` modes, reusing
    /// the coefficient buffer (allocation-free within its capacity).
    pub fn reset(&mut self, mode_count: usize) {
        self.state = InteractionState::new();
        self.initialize(mode_count);
    }

    /// Initialize buffers for the requested mode count.
    pub fn initialize(&mut self, mode_count: usize) {
        self.mode_coefficients.resize(mode_count, 0.0);
        self.update_coefficients();
    }

    /// Update mode coefficients for the current effective position.
    pub fn update_coefficients(&mut self) {
        let count = self.mode_coefficients.len();
        if count > 0 {
            let current_position = self.state.update_position();
            let key = (
                current_position,
                self.state.fundamental_lock,
                self.state.fundamental_minimum,
                count,
            );
            if self.cached_for == Some(key) {
                return;
            }
            self.cached_for = Some(key);
            for (i, coeff) in self.mode_coefficients.iter_mut().enumerate() {
                *coeff = mode_coefficient_1d(i, current_position);
            }
            if self.state.fundamental_lock && !self.mode_coefficients.is_empty() {
                self.mode_coefficients[0] =
                    self.mode_coefficients[0].max(self.state.fundamental_minimum);
            }
        }
    }

    /// Return the current resonator feedback tuple.
    pub fn get_resonator_feedback(&self) -> (f32, f32) {
        (
            self.state.resonator_displacement,
            self.state.resonator_velocity,
        )
    }

    // `distribute_force` and the backing `per_mode_forces` buffer were never
    // read on the audio path; per-mode coupling is multiplied inline inside
    // `ModalResonator::process_sample`. Removed 2026-05-28 (P3.1).

    /// Refresh the dynamic interaction state.
    pub fn update(&mut self) {
        self.update_coefficients();
    }
}

impl Default for BidirectionalInteractionBus {
    /// Construct the default interaction bus.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_coefficient_center() {
        let c0 = mode_coefficient_1d(0, 0.5);
        assert!(
            (c0 - 1.0).abs() < 0.001,
            "Fundamental at center should be ~1.0"
        );

        let c1 = mode_coefficient_1d(1, 0.5);
        assert!(c1.abs() < 0.001, "2nd mode at center should be ~0.0 (node)");
    }

    #[test]
    fn mode_coefficient_edge() {
        let c0 = mode_coefficient_1d(0, 0.0);
        assert!(c0.abs() < 0.001, "All modes at edge should be ~0.0");
    }

    #[test]
    fn interaction_state_defaults() {
        let state = InteractionState::new();
        assert_eq!(state.strike_position, 0.5);
        assert_eq!(state.coupling_stiffness, 1.0);
        assert!(state.fundamental_lock);
    }

    #[test]
    fn fundamental_lock_works() {
        let mut state = InteractionState::new();
        state.fundamental_lock = true;
        state.fundamental_minimum = 0.5;

        state.set_strike_position(0.0);
        let coeffs = state.calculate_mode_coefficients(4);
        assert!(coeffs[0] >= 0.5, "Fundamental should be locked to minimum");
    }

    #[test]
    fn position_wander_bounds() {
        let mut state = InteractionState::new();
        state.set_strike_position(0.5);
        state.set_position_wander(0.2, 1.0, 48000.0);

        let mut min_pos: f32 = 1.0;
        let mut max_pos: f32 = 0.0;

        for _ in 0..1000 {
            let pos = state.update_position();
            min_pos = min_pos.min(pos);
            max_pos = max_pos.max(pos);
        }

        assert!(min_pos >= 0.0, "Position should stay >= 0");
        assert!(max_pos <= 1.0, "Position should stay <= 1");
    }

    #[test]
    fn bidirectional_bus_lifecycle() {
        let mut bus = BidirectionalInteractionBus::new();
        bus.initialize(4);

        // initialize allocates the mode-coefficient cache and runs an initial
        // update pass — the audio path then mutates these coefficients in
        // place every sample.
        assert_eq!(bus.mode_coefficients.len(), 4);

        bus.update();

        let (disp, _vel) = bus.get_resonator_feedback();
        assert_eq!(disp, 0.0);

        // Mode coefficients must contain at least one non-zero value after
        // initialize; otherwise the resonator path would multiply by zero.
        let sum: f32 = bus.mode_coefficients.iter().map(|c| c.abs()).sum();
        assert!(
            sum > 0.0,
            "Should have non-zero mode coefficients after init"
        );
    }
}

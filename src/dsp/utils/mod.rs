//! Utility DSP modules for testing and offline rendering.
//!
//! These modules are not used in the real-time plugin path but provide
//! utilities for deterministic testing, offline rendering, and budget
//! estimation.

pub mod body;

/// Golden-ratio step used to advance the deterministic pseudo-random phases
/// in exciters, voices and the chain resonator. The exact literal is kept (not
/// `f32::consts::GOLDEN_RATIO`, which may round differently and is newer than
/// some toolchains) so every RNG sequence, and the sound, stays bit-identical.
#[allow(clippy::approx_constant)]
pub const GOLDEN_STEP: f32 = 1.618_034;
pub mod budget;
pub mod deterministic_excitation;

pub use body::BodyResonator;
pub use budget::{
    offline_peak_shared_mode_limit, realtime_mode_count_estimate, realtime_mode_count_estimates,
    safe_realtime_shared_mode_limit, RealtimeModeCountEstimate,
};
pub use deterministic_excitation::ExcitationInput;

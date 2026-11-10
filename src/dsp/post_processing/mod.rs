//! Post-processing signal chain for the Corrosion instrument.
//!
//! This module exposes the real-time-safe post stage used after voice mixing:
//! filtering, drive, body resonance, stereo spread, space, and final limiting.
pub mod fem_body;
pub mod half_band;
pub mod hrtf_spread;
pub mod lookahead_limiter;
pub mod lorenz_drive;
pub mod oversampled_clipper;
pub mod post_chain;
pub mod space;
pub mod wdf_filter;

/// Resonator that adds a material/body response stage.
pub use fem_body::FemBodyResonator;
/// Stereo spreader that approximates listener-dependent width and proximity.
pub use hrtf_spread::HrtfSpread;
/// Short-lookahead peak limiter alternative to the hard clamp.
pub use lookahead_limiter::LookaheadLimiter;
/// Chaotic drive stage used before body and spatial processing.
pub use lorenz_drive::LorenzDrive;
/// Final oversampled clipper / limiter stage.
pub use oversampled_clipper::{DiodeMeter, OversampledClipper};
/// Full post-processing pipeline that wires the individual stages together.
pub use post_chain::{PostProcessingChain, PostQualityMode};
/// Space algorithms and mode selection used by the post chain.
pub use space::{FactoryEcho, FactoryReverb, SpaceMode, SpringReverb};
/// WDF ladder-style tone filter used at the start of the chain.
pub use wdf_filter::WdfLadderFilter;

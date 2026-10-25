//! DSP core module.
//!
//! This module exports the modal object models, excitation primitives,
//! resonator math, envelope generation, and realtime budget helpers used by
//! the voice layer. The voice module owns note lifecycle and sample-rate
//! processing; these types provide the per-note DSP building blocks.

pub mod envelopes;
pub mod exciters;
pub mod interaction;
pub mod post_processing;
pub mod profile;
pub mod profiles;
pub mod resonators;
pub mod transforms;
pub mod utils;

pub use envelopes::{LoopMode, Stage, MSEG};
pub use exciters::{
    BowExciter, CorrugatedDrag, Drumstick, ElectromagneticHum, FeltMallet, HandStrike, HardMallet,
    HeavyGrinding, MetalChain, MetalPipe, ParticleRain, PneumaticJet, StiffPointScrape,
    TensionRise, TensionSnap, WireBrush,
};
pub use interaction::{mode_coefficient_1d, BidirectionalInteractionBus, InteractionState};
pub use post_processing::lookahead_limiter::LOOKAHEAD_SAMPLES as LOOKAHEAD_LIMITER_SAMPLES;
pub use post_processing::{
    FactoryReverb, FemBodyResonator, HrtfSpread, LookaheadLimiter, LorenzDrive, OversampledClipper,
    PostProcessingChain, PostQualityMode, SpaceMode, SpringReverb, WdfLadderFilter,
};
pub use profile::{ModalModeSpec, ModalProfile, ModalProfileId};
pub use resonators::{
    CharacterParams, CoilSpringResonator, ContactPrediction, IBeamResonator,
    IndustrialCogResonator, ModalResonator, ResonatorCoefficients, ResonatorCore, RingBound,
    SecondOrderMode, SheetMetalResonator, TautCableResonator,
};
pub use transforms::{
    DamageAmount, HeatAmount, RustAmount, SizeScale, SludgeAmount, ThicknessAmount,
};
pub use utils::{
    offline_peak_shared_mode_limit, realtime_mode_count_estimate, realtime_mode_count_estimates,
    safe_realtime_shared_mode_limit, BodyResonator, ExcitationInput, RealtimeModeCountEstimate,
};

pub use crate::offline::{
    DamageVariationRenderSpec, RustVariationRenderSpec, DAMAGE_VARIATION_SPECS,
    FAMILY_COMPARISON_SPECS, RUST_VARIATION_SPECS,
};

#[cfg(test)]
pub(crate) use profile::{
    PIPE_MODAL_PROFILE_MODES, PLATE_MODAL_PROFILE_MODES, TANK_MODAL_PROFILE_MODES,
};

#[cfg(test)]
mod tests;

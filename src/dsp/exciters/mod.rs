//! Exciter registry for the DSP layer.
//!
//! This module groups the exciter families used by the physical-modeling engine
//! and exposes a stable enum/lookup layer for the UI, presets, and voice logic.

pub mod bow;
pub mod corrugated_drag;
pub mod drumstick;
pub mod felt_mallet;
pub mod hand_strike;
pub mod hard_mallet;
pub mod heavy_grinding;
pub mod metal_chain;
pub mod metal_pipe;
pub mod other_specialty;
pub mod stiff_point;
pub mod tension_rise;
pub mod wire_brush;

#[cfg(test)]
mod event_timing_tests;

pub use bow::BowExciter;
pub use corrugated_drag::CorrugatedDrag;
pub use drumstick::Drumstick;
pub use felt_mallet::FeltMallet;
pub use hand_strike::HandStrike;
pub use hard_mallet::HardMallet;
pub use heavy_grinding::HeavyGrinding;
pub use metal_chain::MetalChain;
pub use metal_pipe::MetalPipe;
pub use other_specialty::{ElectromagneticHum, ParticleRain, PneumaticJet, TensionSnap};
pub use stiff_point::StiffPointScrape;
pub use tension_rise::TensionRise;
pub use wire_brush::WireBrush;

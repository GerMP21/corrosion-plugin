//! Custom GUI for Corrosion
//!
//! Physical-metaphor layout with 4 sections:
//! - Exciter: Controls how the metal is activated
//! - Object: The resonating body (Pipe/Plate/Tank/Chain)
//! - Damage: Wear and deterioration (Rust/Damage)
//! - Space: Output shaping (Drive/Width/Body/Output)
//!
//! No oscillator/filter/amp framing is used - all controls follow
//! the industrial physical-modeling metaphor.

#[cfg(any(feature = "gui", feature = "web-gui"))]
mod editor;

#[cfg(feature = "gui")]
pub use editor::create_editor;
#[cfg(feature = "web-gui")]
pub use editor::WebEditor;

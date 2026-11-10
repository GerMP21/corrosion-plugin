use include_dir::{include_dir, Dir};
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, resizable_window::ResizableWindow, EguiState};
use std::sync::Arc;

/// Factory presets embedded at compile time so the shipped `.vst3`/`.clap` is
/// self-contained. The previous `fs::read_dir(env!("CARGO_MANIFEST_DIR")/...)`
/// lookup only ever resolved on the build host — once the bundle was copied to
/// a user's plugin folder, the preset bank silently disappeared.
static FACTORY_PRESETS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/presets/factory");

use crate::params::{exciter_model_items, CorrosionParams, ExciterFamily, ExciterType, Object};
use crate::Preset;

/// Parameters deliberately left without an editor control, each with the
/// reason. `editor_covers_every_parameter` fails for any other parameter the
/// editor doesn't draw (`h05-expose-missing-gui-controls`).
// Read by the coverage test only.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const HOST_ONLY_PARAMS: &[(&str, &str)] = &[];

#[cfg(test)]
thread_local! {
    /// Parameters drawn by an editor widget since the last clear (tests only).
    static DRAWN_PARAMS: std::cell::RefCell<std::collections::HashSet<ParamPtr>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

#[cfg(test)]
thread_local! {
    /// The colour the brickwall panel's dot was last drawn in (tests only).
    static DRAWN_DIODE_DOT: std::cell::Cell<Option<egui::Color32>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
thread_local! {
    /// The help text each drawn control was given since the last clear
    /// (tests only; `add-parameter-tooltips`).
    static DRAWN_HELP: std::cell::RefCell<std::collections::HashMap<ParamPtr, String>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// Each drawn control's help badge and widget rects, in that order
    /// (tests only).
    static DRAWN_BADGES: std::cell::RefCell<std::collections::HashMap<ParamPtr, (egui::Rect, egui::Rect)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Records the help text a control was drawn with (tests only; a no-op
/// otherwise).
#[inline]
fn note_help(_param: ParamPtr, _help: &str) {
    #[cfg(test)]
    DRAWN_HELP.with(|drawn| {
        drawn.borrow_mut().insert(_param, _help.to_string());
    });
}

/// Records where a control's help badge and the control itself were drawn
/// (tests only; a no-op otherwise).
#[inline]
fn note_badge(_param: ParamPtr, _badge: egui::Rect, _widget: egui::Rect) {
    #[cfg(test)]
    DRAWN_BADGES.with(|drawn| {
        drawn.borrow_mut().insert(_param, (_badge, _widget));
    });
}

/// Records the colour the diode dot was drawn in (tests only; a no-op
/// otherwise).
#[inline]
fn note_diode_dot(_colour: egui::Color32) {
    #[cfg(test)]
    DRAWN_DIODE_DOT.with(|dot| dot.set(Some(_colour)));
}

/// Records that a widget drew `param` (tests only; a no-op otherwise).
#[inline]
fn note_drawn(_param: ParamPtr) {
    #[cfg(test)]
    DRAWN_PARAMS.with(|drawn| {
        drawn.borrow_mut().insert(_param);
    });
}

// ---------------------------------------------------------------------------
// Design tokens – industrial / brutalist palette from the evidence HTML
// ---------------------------------------------------------------------------

const CHARCOAL: egui::Color32 = egui::Color32::from_rgb(0x0D, 0x0D, 0x0D);
const DARK_STEEL: egui::Color32 = egui::Color32::from_rgb(0x1A, 0x1A, 0x1A);
const CONCRETE: egui::Color32 = egui::Color32::from_rgb(0x33, 0x33, 0x33);
const SAFETY_YELLOW: egui::Color32 = egui::Color32::from_rgb(0xFA, 0xCC, 0x15);
const RUST_ORANGE: egui::Color32 = egui::Color32::from_rgb(0xC2, 0x41, 0x0C);
const DANGER_RED: egui::Color32 = egui::Color32::from_rgb(0xB9, 0x1C, 0x1C);
const OFF_WHITE: egui::Color32 = egui::Color32::from_rgb(0xD4, 0xD4, 0xD4);
const MUTED_GREY: egui::Color32 = egui::Color32::from_rgb(0x73, 0x73, 0x73);
const STEEL_LIGHT: egui::Color32 = egui::Color32::from_rgb(0x55, 0x55, 0x55);
const _STEEL_MID: egui::Color32 = egui::Color32::from_rgb(0x44, 0x44, 0x44);
const RECESSED_BG: egui::Color32 = egui::Color32::from_rgb(0x11, 0x11, 0x11);
const FADER_TRACK_BG: egui::Color32 = egui::Color32::from_rgb(0x00, 0x00, 0x00);
const THUMB_BG: egui::Color32 = egui::Color32::from_rgb(0x44, 0x44, 0x44);
const THUMB_BORDER: egui::Color32 = egui::Color32::from_rgb(0x66, 0x66, 0x66);

/// Logical editor width for the 100% UI scale.
const BASE_EDITOR_WIDTH: u32 = 1080;
/// Logical height the panels are laid out against (`height_pct`) at the
/// 100% UI scale. The keyboard strip sits below it.
const BASE_LAYOUT_HEIGHT: u32 = 768;
/// Logical editor height for the 100% UI scale: the panels' scroll view
/// plus the keyboard strip pinned under it (`add-editor-keyboard`).
const BASE_EDITOR_HEIGHT: u32 =
    BASE_LAYOUT_HEIGHT + (KEYBOARD_STRIP_BASE_HEIGHT + KEYBOARD_STRIP_BASE_GAP) as u32;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

struct EditorUiState {
    last_ui_scale: i32,
    last_requested_size: (u32, u32),
    factory_presets: Vec<FactoryPresetEntry>,
    /// Index into the factory presets followed by the user presets.
    selected_factory_preset: usize,
    /// The user preset folder (`None` when the platform's variables are
    /// unset), its presets, and how many of its files couldn't be read.
    user_preset_dir: Option<std::path::PathBuf>,
    user_presets: Vec<(std::path::PathBuf, Preset)>,
    unreadable_user_presets: usize,
    /// The Save row: the typed name, whether it is waiting on an overwrite
    /// confirmation, and the last save message (`true` for an error).
    save_name: String,
    confirm_overwrite: bool,
    save_message: Option<(String, bool)>,
    /// The randomizer's mode (editor state: it shapes the next click, not the
    /// sound) and its randomness, seeded when the editor opens.
    randomizer_mode: crate::randomizer::RandomizerMode,
    randomizer_noise: crate::dsp::utils::noise::Noise,
    /// The diode stage's reduction, published by the audio thread.
    diode_meter: Arc<crate::dsp::DiodeMeter>,
    /// The reduction the limiter dot shows (dB, capped at full brightness)
    /// and the editor time it was last updated.
    diode_dot_db: f32,
    diode_dot_time: Option<f64>,
    /// The keyboard strip at the bottom (`add-editor-keyboard`).
    keyboard: KeyboardStrip,
}

/// Reductions under this leave the limiter dot at rest.
const DIODE_DOT_REST_DB: f32 = 0.1;
/// The reduction at which the dot is at full brightness.
const DIODE_DOT_FULL_DB: f32 = 6.0;
/// How fast the dot falls back: from full to rest in about 300 ms.
const DIODE_DOT_RELEASE_DB_PER_SECOND: f32 = 20.0;
/// The dot's brightness at rest, as before it was a meter.
const DIODE_DOT_REST_BRIGHTNESS: f32 = 0.3;

/// The dot's colour for a shown reduction: resting under 0.1 dB, then
/// brightening linearly to full `DANGER_RED` at 6 dB
/// (`add-asymmetric-diode-stage`).
fn diode_dot_colour(reduction_db: f32) -> egui::Color32 {
    if reduction_db < DIODE_DOT_REST_DB {
        DANGER_RED.linear_multiply(DIODE_DOT_REST_BRIGHTNESS)
    } else if reduction_db >= DIODE_DOT_FULL_DB {
        DANGER_RED
    } else {
        let t = (reduction_db - DIODE_DOT_REST_DB) / (DIODE_DOT_FULL_DB - DIODE_DOT_REST_DB);
        DANGER_RED
            .linear_multiply(DIODE_DOT_REST_BRIGHTNESS + (1.0 - DIODE_DOT_REST_BRIGHTNESS) * t)
    }
}

impl EditorUiState {
    /// Takes the reduction the audio thread published since the last frame
    /// and returns what the dot shows at editor time `now` (seconds): rising
    /// at once, falling back at `DIODE_DOT_RELEASE_DB_PER_SECOND`.
    fn advance_diode_dot(&mut self, now: f64) -> f32 {
        let fresh = self.diode_meter.take().min(DIODE_DOT_FULL_DB);
        let elapsed = self
            .diode_dot_time
            .map_or(0.0, |previous| (now - previous).max(0.0)) as f32;
        self.diode_dot_time = Some(now);
        let released = (self.diode_dot_db - DIODE_DOT_RELEASE_DB_PER_SECOND * elapsed).max(0.0);
        self.diode_dot_db = fresh.max(released);
        self.diode_dot_db
    }

    fn new(last_ui_scale: i32, last_requested_size: (u32, u32)) -> Self {
        let mut state = Self {
            last_ui_scale,
            last_requested_size,
            factory_presets: load_factory_presets(),
            selected_factory_preset: 0,
            user_preset_dir: crate::presets::user_preset_dir(),
            user_presets: Vec::new(),
            unreadable_user_presets: 0,
            save_name: String::new(),
            confirm_overwrite: false,
            save_message: None,
            diode_meter: Arc::new(crate::dsp::DiodeMeter::new()),
            diode_dot_db: 0.0,
            diode_dot_time: None,
            keyboard: KeyboardStrip::new(Arc::new(crate::gui_notes::EditorNotes::new())),
            randomizer_mode: crate::randomizer::RandomizerMode::Safe,
            randomizer_noise: crate::dsp::utils::noise::Noise::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.subsec_nanos() ^ elapsed.as_secs() as u32)
                    .unwrap_or(1),
            ),
        };
        state.refresh_user_presets();
        state
    }

    /// Re-read the user preset folder (on editor open and after a save).
    fn refresh_user_presets(&mut self) {
        let (presets, unreadable) = match &self.user_preset_dir {
            Some(dir) => crate::presets::list_user_presets(dir),
            None => (Vec::new(), 0),
        };
        self.user_presets = presets;
        self.unreadable_user_presets = unreadable;
    }

    /// Factory presets, then user presets.
    fn preset_count(&self) -> usize {
        self.factory_presets.len() + self.user_presets.len()
    }

    fn preset_name(&self, index: usize) -> &str {
        match index.checked_sub(self.factory_presets.len()) {
            None => &self.factory_presets[index].name,
            Some(user) => &self.user_presets[user].1.name,
        }
    }

    fn preset(&self, index: usize) -> Option<Preset> {
        match index.checked_sub(self.factory_presets.len()) {
            None => Preset::from_json_str(self.factory_presets[index].json).ok(),
            Some(user) => self
                .user_presets
                .get(user)
                .map(|(_, preset)| preset.clone()),
        }
    }

    /// Save the current sound under the typed name, asking before replacing
    /// a user preset (`h05-expose-missing-gui-controls`).
    fn save(&mut self, params: &CorrosionParams, overwrite: bool) {
        use crate::presets::{save_user_preset, SaveOutcome};
        self.confirm_overwrite = false;
        let Some(dir) = self.user_preset_dir.clone() else {
            self.save_message = Some(("No user preset folder on this system".to_string(), true));
            return;
        };
        match save_user_preset(&dir, &self.save_name, params, overwrite) {
            Ok(SaveOutcome::Saved(_)) => {
                let name = self.save_name.trim().to_string();
                self.refresh_user_presets();
                if let Some(user) = self.user_presets.iter().position(|(_, p)| p.name == name) {
                    self.selected_factory_preset = self.factory_presets.len() + user;
                }
                self.save_message = Some((format!("Saved \"{name}\""), false));
            }
            Ok(SaveOutcome::NeedsConfirmation(_)) => self.confirm_overwrite = true,
            Err(error) => self.save_message = Some((format!("Couldn't save: {error}"), true)),
        }
    }
}

#[derive(Clone)]
struct FactoryPresetEntry {
    name: String,
    json: &'static str,
}

#[derive(Clone, Copy)]
struct KnobDragState {
    start_value: f32,
    start_pointer_y: f32,
}

// ---------------------------------------------------------------------------
// Control identifiers and specs (unchanged from original)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum ExciterControlId {
    HandMass,
    FleshStiffness,
    FleshDamping,
    MuteDecay,
    MalletMass,
    FeltSoftness,
    CoreHardness,
    CompressionCurve,
    MaterialStiffness,
    ImpactDamping,
    StickMass,
    TipStiffness,
    RestitutionBounciness,
    MicroBounceLimit,
    WireDensity,
    SpreadDuration,
    BrushWireStiffness,
    AmplitudeRandomization,
    PipeMass,
    MetalStiffness,
    PipePitch,
    PipeRingDecay,
    LinkCount,
    ChainMass,
    DropEnvelopeSpread,
    InternalRattle,
    RattleColor,
    BowPressure,
    BowSpeed,
    RosinGrip,
    SlipCurve,
    ScrapeSpeed,
    PointPressure,
    ChatterPitch,
    ChatterDamping,
    GrindSpeed,
    GrindPressure,
    SurfaceGrit,
    GritColor,
    DragSpeed,
    RidgeSpacing,
    RidgeDepth,
    DragExciterMass,
    PullSpeed,
    BreakThreshold,
    SlipStochasticity,
    CreakSharpness,
    AirPressure,
    NozzleWidth,
    TurbulenceChaos,
    MainsFrequency,
    CoilProximity,
    VoltageSag,
    PullDistance,
    HookStiffness,
    SnapForce,
    FlowRate,
    ParticleMass,
    MassVariance,
}

#[derive(Clone, Copy)]
enum ResonatorControlId {
    // Global controls shared across objects.
    Size,
    Damping,
    // Curated per-object character controls (see docs/backlog.md).
    PipeDiameter,
    PlateAspect,
    PlateStiffness,
    TankVolume,
    TankCavityMix,
    ChainLinkMass,
    ChainInstability,
    BeamShear,
    CableBraid,
    CableTensionDrop,
    SpringDispersion,
    SpringSlosh,
    SheetThinness,
    CogDissonance,
}

#[derive(Clone, Copy)]
struct ExciterControlSpec {
    id: ExciterControlId,
    label: &'static str,
    min: f32,
    max: f32,
}

#[derive(Clone, Copy)]
struct ResonatorControlSpec {
    id: ResonatorControlId,
    label: &'static str,
    min: f32,
    max: f32,
}

struct ExciterPanelSpec {
    title: &'static str,
    description: &'static str,
    controls: &'static [ExciterControlSpec],
}

struct ResonatorPanelSpec {
    title: &'static str,
    description: &'static str,
    controls: &'static [ResonatorControlSpec],
}

// ---------------------------------------------------------------------------
// Exciter control spec tables (unchanged)
// ---------------------------------------------------------------------------

const HAND_STRIKE_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::HandMass,
        label: "Hand Mass",
        min: 0.4,
        max: 3.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::FleshStiffness,
        label: "Flesh Stiffness",
        min: 0.05,
        max: 0.8,
    },
    ExciterControlSpec {
        id: ExciterControlId::FleshDamping,
        label: "Flesh Damping",
        min: 0.3,
        max: 1.8,
    },
    ExciterControlSpec {
        id: ExciterControlId::MuteDecay,
        label: "Mute Decay",
        min: 0.85,
        max: 0.999,
    },
];
const FELT_MALLET_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::MalletMass,
        label: "Mallet Mass",
        min: 0.4,
        max: 3.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::FeltSoftness,
        label: "Felt Softness",
        min: 0.1,
        max: 1.3,
    },
    ExciterControlSpec {
        id: ExciterControlId::CoreHardness,
        label: "Core Hardness",
        min: 0.5,
        max: 4.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::CompressionCurve,
        label: "Compression Curve",
        min: 2.0,
        max: 5.0,
    },
];
const HARD_MALLET_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::MalletMass,
        label: "Mallet Mass",
        min: 0.4,
        max: 3.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::MaterialStiffness,
        label: "Material Stiffness",
        min: 0.5,
        max: 5.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::ImpactDamping,
        label: "Impact Damping",
        min: 0.1,
        max: 1.3,
    },
];
const DRUMSTICK_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::StickMass,
        label: "Stick Mass",
        min: 0.05,
        max: 1.25,
    },
    ExciterControlSpec {
        id: ExciterControlId::TipStiffness,
        label: "Tip Stiffness",
        min: 0.8,
        max: 6.8,
    },
    ExciterControlSpec {
        id: ExciterControlId::RestitutionBounciness,
        label: "Restitution Bounciness",
        min: 0.2,
        max: 0.9,
    },
    ExciterControlSpec {
        id: ExciterControlId::MicroBounceLimit,
        label: "Micro Bounce Limit",
        min: 2.0,
        max: 8.0,
    },
];
const WIRE_BRUSH_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::WireDensity,
        label: "Wire Density",
        min: 10.0,
        max: 130.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::SpreadDuration,
        label: "Spread Duration",
        min: 10.0,
        max: 250.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::BrushWireStiffness,
        label: "Wire Stiffness",
        min: 0.0,
        max: 1.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::AmplitudeRandomization,
        label: "Amplitude Randomization",
        min: 0.0,
        max: 1.0,
    },
];
const METAL_PIPE_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::PipeMass,
        label: "Pipe Mass",
        min: 0.4,
        max: 2.6,
    },
    ExciterControlSpec {
        id: ExciterControlId::MetalStiffness,
        label: "Metal Stiffness",
        min: 0.5,
        max: 5.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::PipePitch,
        label: "Pipe Pitch",
        min: 0.5,
        max: 2.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::PipeRingDecay,
        label: "Pipe Ring Decay",
        min: 0.96,
        max: 0.999,
    },
];
const METAL_CHAIN_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::LinkCount,
        label: "Link Count",
        min: 3.0,
        max: 15.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::ChainMass,
        label: "Chain Mass",
        min: 0.2,
        max: 1.4,
    },
    ExciterControlSpec {
        id: ExciterControlId::DropEnvelopeSpread,
        label: "Drop Envelope Spread",
        min: 40.0,
        max: 400.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::InternalRattle,
        label: "Internal Rattle",
        min: 0.0,
        max: 1.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::RattleColor,
        label: "Rattle Color",
        min: 0.0,
        max: 1.0,
    },
];
const BOW_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::BowPressure,
        label: "Bow Pressure",
        min: 0.2,
        max: 2.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::BowSpeed,
        label: "Bow Speed",
        min: 0.1,
        max: 2.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::RosinGrip,
        label: "Rosin Grip",
        min: 0.05,
        max: 1.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::SlipCurve,
        label: "Slip Curve",
        min: 0.05,
        max: 1.5,
    },
];
const STIFF_POINT_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::ScrapeSpeed,
        label: "Scrape Speed",
        min: 0.1,
        max: 2.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::PointPressure,
        label: "Point Pressure",
        min: 0.1,
        max: 1.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::ChatterPitch,
        label: "Chatter Pitch",
        min: 0.1,
        max: 1.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::ChatterDamping,
        label: "Chatter Damping",
        min: 0.1,
        max: 0.9,
    },
];
const HEAVY_GRINDING_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::GrindSpeed,
        label: "Grind Speed",
        min: 0.1,
        max: 2.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::GrindPressure,
        label: "Grind Pressure",
        min: 0.1,
        max: 1.9,
    },
    ExciterControlSpec {
        id: ExciterControlId::SurfaceGrit,
        label: "Surface Grit",
        min: 0.0,
        max: 1.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::GritColor,
        label: "Grit Color",
        min: 0.0,
        max: 1.0,
    },
];
const CORRUGATED_DRAG_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::DragSpeed,
        label: "Drag Speed",
        min: 0.1,
        max: 2.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::RidgeSpacing,
        label: "Ridge Spacing",
        min: 0.01,
        max: 0.2,
    },
    ExciterControlSpec {
        id: ExciterControlId::RidgeDepth,
        label: "Ridge Depth",
        min: 0.0,
        max: 2.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::DragExciterMass,
        label: "Exciter Mass",
        min: 0.2,
        max: 2.0,
    },
];
const TENSION_RISE_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::PullSpeed,
        label: "Pull Speed",
        min: 0.05,
        max: 1.55,
    },
    ExciterControlSpec {
        id: ExciterControlId::BreakThreshold,
        label: "Break Threshold",
        min: 0.1,
        max: 1.6,
    },
    ExciterControlSpec {
        id: ExciterControlId::SlipStochasticity,
        label: "Slip Stochasticity",
        min: 0.0,
        max: 1.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::CreakSharpness,
        label: "Creak Sharpness",
        min: 0.2,
        max: 1.4,
    },
];
const PNEUMATIC_JET_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::AirPressure,
        label: "Air Pressure",
        min: 0.1,
        max: 2.1,
    },
    ExciterControlSpec {
        id: ExciterControlId::NozzleWidth,
        label: "Nozzle Width",
        min: 0.1,
        max: 1.6,
    },
    ExciterControlSpec {
        id: ExciterControlId::TurbulenceChaos,
        label: "Turbulence Chaos",
        min: 0.0,
        max: 2.0,
    },
];
const ELECTROMAGNETIC_HUM_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::MainsFrequency,
        label: "Mains Frequency",
        min: 40.0,
        max: 120.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::CoilProximity,
        label: "Coil Proximity",
        min: 0.0,
        max: 2.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::VoltageSag,
        label: "Voltage Sag",
        min: 0.0,
        max: 2.0,
    },
];
const TENSION_SNAP_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::PullDistance,
        label: "Pull Distance",
        min: 0.1,
        max: 1.5,
    },
    ExciterControlSpec {
        id: ExciterControlId::HookStiffness,
        label: "Hook Stiffness",
        min: 0.2,
        max: 2.2,
    },
    ExciterControlSpec {
        id: ExciterControlId::SnapForce,
        label: "Snap Force",
        min: 0.1,
        max: 2.0,
    },
];
const PARTICLE_RAIN_CONTROLS: &[ExciterControlSpec] = &[
    ExciterControlSpec {
        id: ExciterControlId::FlowRate,
        label: "Flow Rate",
        min: 0.1,
        max: 3.1,
    },
    ExciterControlSpec {
        id: ExciterControlId::ParticleMass,
        label: "Particle Mass",
        min: 0.05,
        max: 1.0,
    },
    ExciterControlSpec {
        id: ExciterControlId::MassVariance,
        label: "Mass Variance",
        min: 0.0,
        max: 2.0,
    },
];

const PIPE_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Pipe Length",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::PipeDiameter,
        label: "Tube Diameter",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::Damping,
        label: "Sustain Time",
        min: 0.0,
        max: 1.0,
    },
];
const PLATE_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Plate Size",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::PlateAspect,
        label: "Plate Aspect",
        min: 0.1,
        max: 4.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::PlateStiffness,
        label: "Plate Stiffness",
        min: 0.25,
        max: 3.0,
    },
];
const TANK_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Tank Size",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::TankVolume,
        label: "Tank Volume",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::TankCavityMix,
        label: "Cavity Mix",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::Damping,
        label: "Shell Decay",
        min: 0.0,
        max: 1.0,
    },
];
const CHAIN_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Chain Size",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::ChainLinkMass,
        label: "Link Mass",
        min: 0.1,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::ChainInstability,
        label: "Instability",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::Damping,
        label: "Friction Decay",
        min: 0.0,
        max: 1.0,
    },
];
const IBEAM_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Beam Mass",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::BeamShear,
        label: "Shear Density",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::Damping,
        label: "Rigidity Damping",
        min: 0.0,
        max: 1.0,
    },
];
const TAUT_CABLE_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Cable Tension",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::CableBraid,
        label: "Braid Stiffness",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::CableTensionDrop,
        label: "Tension Drop",
        min: 0.0,
        max: 1.0,
    },
];
const COIL_SPRING_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Coil Length",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::SpringDispersion,
        label: "Dispersion Chirp",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::SpringSlosh,
        label: "Spring Slosh",
        min: 0.0,
        max: 1.0,
    },
];
const SHEET_METAL_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Sheet Size",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::SheetThinness,
        label: "Metal Thinness",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::Damping,
        label: "Edge Damping",
        min: 0.0,
        max: 1.0,
    },
];
const INDUSTRIAL_COG_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Blade Radius",
        min: 0.05,
        max: 10.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::CogDissonance,
        label: "Tooth Dissonance",
        min: 0.0,
        max: 1.0,
    },
    ResonatorControlSpec {
        id: ResonatorControlId::Damping,
        label: "Blade Thickness",
        min: 0.0,
        max: 1.0,
    },
];

// ---------------------------------------------------------------------------
// Editor creation
// ---------------------------------------------------------------------------

pub fn create_editor(
    params: Arc<CorrosionParams>,
    editor_state: Arc<EguiState>,
    diode_meter: Arc<crate::dsp::DiodeMeter>,
    editor_notes: Arc<crate::gui_notes::EditorNotes>,
) -> Option<Box<dyn Editor>> {
    let initial_scale = params.ui_scale.value();
    let initial_size = scaled_editor_size(ui_scale_factor(initial_scale));
    persist_editor_size(&editor_state, initial_size);
    let resize_state = editor_state.clone();
    create_egui_editor(
        editor_state,
        EditorUiState {
            diode_meter,
            keyboard: KeyboardStrip::new(editor_notes),
            ..EditorUiState::new(initial_scale, initial_size)
        },
        // The state outlives the window; the audio thread released the
        // editor's notes when it closed, so start the strip clean.
        |_, state| state.keyboard.reopened(),
        move |egui_ctx, setter, state| {
            let ui_scale = params.ui_scale.value();
            let requested_scale = ui_scale_factor(ui_scale);

            if let Some((width, height)) =
                request_window_size(state, ui_scale, &resize_state, setter.raw_context)
            {
                egui_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    width as f32,
                    height as f32,
                )));
            }

            // Lay out for the window the editor actually has, never wider
            // than it: a host that refuses the resize, or a DPI mismatch,
            // shrinks the layout instead of pushing columns off-screen.
            let scale = layout_scale(requested_scale, egui_ctx.screen_rect().width());
            apply_editor_style(egui_ctx, scale);

            ResizableWindow::new("corrosion-editor-window")
                .min_size(egui::vec2(
                    (BASE_EDITOR_WIDTH as f32 * 0.5).round(),
                    (BASE_EDITOR_HEIGHT as f32 * 0.5).round(),
                ))
                .show(egui_ctx, &resize_state, |ui| {
                    render_window_body(ui, &params, setter, state, scale);
                });
        },
    )
}

/// The window's contents: the panels in a scroll view, and the keyboard
/// strip pinned under it so it stays in view (`add-editor-keyboard`).
fn render_window_body(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    state: &mut EditorUiState,
    scale: f32,
) {
    let content_width = ui.available_width();
    let strip_height = (KEYBOARD_STRIP_BASE_HEIGHT + KEYBOARD_STRIP_BASE_GAP) * scale
        + ui.spacing().item_spacing.y;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height((ui.available_height() - strip_height).max(0.0))
        .show(ui, |ui| {
            ui.set_width(content_width);
            ui.set_max_width(content_width);
            render_ui(ui, params, setter, state, scale);
        });
    render_keyboard_strip(ui, &mut state.keyboard, scale);
}

/// The editor's visual style at layout scale `scale`: colours, fonts and
/// spacing. Shared by the editor and its headless layout tests.
fn apply_editor_style(ctx: &egui::Context, scale: f32) {
    // Industrial visual style
    let mut style = egui::Style::default();
    // egui's own widget sizes (combo boxes, buttons, icons, scroll bars) are
    // fixed in points; scale them with the layout so the editor keeps its
    // proportions at every UI scale instead of growing taller than its window
    // at small scales.
    let defaults = egui::style::Spacing::default();
    style.spacing.interact_size = defaults.interact_size * scale;
    style.spacing.button_padding = defaults.button_padding * scale;
    style.spacing.icon_width = defaults.icon_width * scale;
    style.spacing.icon_width_inner = defaults.icon_width_inner * scale;
    style.spacing.icon_spacing = defaults.icon_spacing * scale;
    style.spacing.combo_width = defaults.combo_width * scale;
    style.spacing.slider_width = defaults.slider_width * scale;
    style.spacing.indent = defaults.indent * scale;
    style.spacing.menu_margin =
        egui::Margin::same((defaults.menu_margin.left as f32 * scale).round() as i8);
    style.spacing.scroll.bar_width = defaults.scroll.bar_width * scale;
    style.spacing.item_spacing = egui::vec2(
        height_pct(scale, 0.0052083335),
        width_pct(scale, 0.0027777778),
    );
    style.spacing.window_margin = egui::Margin::same((width_pct(scale, 0.0074074073)) as i8);
    style.visuals = egui::Visuals::dark();
    style.visuals.extreme_bg_color = CHARCOAL;
    style.visuals.window_fill = DARK_STEEL;
    style.visuals.panel_fill = DARK_STEEL;
    style.visuals.faint_bg_color = RECESSED_BG;
    style.visuals.widgets.inactive.bg_fill = CHARCOAL;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, OFF_WHITE);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, CONCRETE);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x22, 0x22, 0x22);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, OFF_WHITE);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0x33, 0x33, 0x33);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, SAFETY_YELLOW);
    style.visuals.selection.bg_fill = SAFETY_YELLOW.linear_multiply(0.3);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, SAFETY_YELLOW);
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(width_pct(scale, 0.016666668)),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(width_pct(scale, 0.010185185)),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(width_pct(scale, 0.010185185)),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(width_pct(scale, 0.008333334)),
    );
    ctx.set_style(style);
}

// ---------------------------------------------------------------------------
// Utility functions (unchanged)
// ---------------------------------------------------------------------------

fn ui_scale_factor(value: i32) -> f32 {
    match value {
        0 => 0.50,
        1 => 0.75,
        2 => 1.00,
        3 => 1.25,
        4 => 1.50,
        _ => 1.00,
    }
}

/// Ask the host to resize the editor when the UI Scale setting changes.
///
/// The host reads the new size from the editor state (`Editor::size`), then
/// resizes its window. Returns the size to give the embedded window once the
/// host has agreed, and `None` when nothing changed or the host refused.
/// Without `request_resize` the host window kept its size and only the
/// contents shrank or grew.
fn request_window_size(
    state: &mut EditorUiState,
    ui_scale: i32,
    editor_state: &Arc<EguiState>,
    context: &dyn nih_plug::prelude::GuiContext,
) -> Option<(u32, u32)> {
    let desired_size = scaled_editor_size(ui_scale_factor(ui_scale));
    if state.last_ui_scale == ui_scale && state.last_requested_size == desired_size {
        return None;
    }
    state.last_ui_scale = ui_scale;
    state.last_requested_size = desired_size;
    persist_editor_size(editor_state, desired_size);
    context.request_resize().then_some(desired_size)
}

/// The layout scale for a window `window_width` points wide when the UI
/// Scale setting asks for `requested`: the requested scale, reduced so the
/// layout is never wider than the window.
fn layout_scale(requested: f32, window_width: f32) -> f32 {
    let fits = window_width / BASE_EDITOR_WIDTH as f32;
    if window_width.is_finite() && window_width > 0.0 && fits < requested {
        fits.max(0.25)
    } else {
        requested
    }
}

fn scaled_editor_size(scale: f32) -> (u32, u32) {
    (
        (BASE_EDITOR_WIDTH as f32 * scale).round() as u32,
        (BASE_EDITOR_HEIGHT as f32 * scale).round() as u32,
    )
}

fn editor_width(scale: f32) -> f32 {
    BASE_EDITOR_WIDTH as f32 * scale
}

/// The panels' layout height (`height_pct` is relative to it); the window
/// is taller by the keyboard strip.
fn editor_height(scale: f32) -> f32 {
    BASE_LAYOUT_HEIGHT as f32 * scale
}

fn width_pct(scale: f32, pct: f32) -> f32 {
    editor_width(scale) * pct
}

fn height_pct(scale: f32, pct: f32) -> f32 {
    editor_height(scale) * pct
}

fn persist_editor_size(editor_state: &Arc<EguiState>, size: (u32, u32)) {
    let serialized = serde_json::json!({ "size": size });
    if let Ok(state) = serde_json::from_value::<EguiState>(serialized) {
        editor_state.set(state);
    }
}

// ---------------------------------------------------------------------------
// Custom industrial widgets
// ---------------------------------------------------------------------------

/// Paint a decorative screw head at the given position.
fn paint_screw(painter: &egui::Painter, center: egui::Pos2, radius: f32) {
    painter.circle_filled(center, radius, egui::Color32::from_rgb(0x33, 0x33, 0x33));
    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(0x11, 0x11, 0x11)),
    );
    // Cross slot
    let slot_len = radius * 0.6;
    let slot_w = 1.0;
    painter.line_segment(
        [
            center - egui::vec2(slot_len, 0.0),
            center + egui::vec2(slot_len, 0.0),
        ],
        egui::Stroke::new(slot_w, egui::Color32::from_rgb(0x11, 0x11, 0x11)),
    );
    painter.line_segment(
        [
            center - egui::vec2(0.0, slot_len),
            center + egui::vec2(0.0, slot_len),
        ],
        egui::Stroke::new(slot_w, egui::Color32::from_rgb(0x11, 0x11, 0x11)),
    );
    // Highlight
    painter.circle_filled(
        center + egui::vec2(-radius * 0.2, -radius * 0.2),
        radius * 0.3,
        egui::Color32::from_white_alpha(30),
    );
}

/// Industrial knob widget. Returns response.
fn industrial_knob(
    ui: &mut egui::Ui,
    label: &str,
    param: &FloatParam,
    min: f32,
    max: f32,
    setter: &ParamSetter,
    accent: egui::Color32,
    scale: f32,
    large: bool,
) -> egui::Response {
    industrial_knob_text(
        ui, label, param, min, max, setter, accent, scale, large, false,
    )
}

/// `industrial_knob`, showing the parameter's own display string (such as
/// Pressure's "+6.0 dB" or Speed's "×1.41") when `param_text` is set
/// (`h06-wire-global-exciter-controls`).
#[allow(clippy::too_many_arguments)]
fn industrial_knob_text(
    ui: &mut egui::Ui,
    label: &str,
    param: &FloatParam,
    min: f32,
    max: f32,
    setter: &ParamSetter,
    accent: egui::Color32,
    scale: f32,
    large: bool,
    param_text: bool,
) -> egui::Response {
    note_drawn(param.as_ptr());
    let radius = if large {
        width_pct(scale, 0.023148149)
    } else {
        width_pct(scale, 0.018518519)
    };
    let knob_height = radius * 2.0;
    let label_height = height_pct(scale, 0.018229166);
    let value_height = height_pct(scale, 0.018229166);
    let desired_size = egui::vec2(
        radius * 2.0 + width_pct(scale, 0.0074074073),
        knob_height + label_height + value_height,
    );
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let drag_start_id = response.id;

    let mut display_value = param.value();

    // Drag interaction
    if response.drag_started() {
        let drag_state = KnobDragState {
            start_value: display_value,
            start_pointer_y: response
                .interact_pointer_pos()
                .map(|pointer| pointer.y)
                .unwrap_or(rect.center().y),
        };
        ui.ctx()
            .data_mut(|data| data.insert_temp(drag_start_id, drag_state));
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        let drag_state = ui
            .ctx()
            .data(|data| data.get_temp::<KnobDragState>(drag_start_id));

        if let (Some(drag_state), Some(pointer)) = (drag_state, response.interact_pointer_pos()) {
            let new_value = knob_drag_value(
                drag_state.start_value,
                pointer.y - drag_state.start_pointer_y,
                min,
                max,
                scale,
            );
            display_value = new_value;
            if (new_value - param.value()).abs() > f32::EPSILON {
                setter.set_parameter(param, new_value);
            }
        }
    }
    if response.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.remove::<KnobDragState>(drag_start_id));
        setter.end_set_parameter(param);
    }

    let normalized = if (max - min).abs() > f32::EPSILON {
        ((display_value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.5
    };

    // Paint knob
    let center = egui::pos2(rect.center().x, rect.min.y + radius);
    let painter = ui.painter();

    // Outer ring shadow
    painter.circle_filled(
        center,
        radius + width_pct(scale, 0.0018518518),
        egui::Color32::from_black_alpha(100),
    );
    // Knob body - conic gradient approximation
    painter.circle_filled(center, radius, egui::Color32::from_rgb(0x33, 0x33, 0x33));
    // Highlight crescent (top-left)
    let highlight_center = center + egui::vec2(-radius * 0.15, -radius * 0.15);
    painter.circle_filled(
        highlight_center,
        radius * 0.85,
        egui::Color32::from_rgb(0x44, 0x44, 0x44),
    );
    painter.circle_filled(
        center,
        radius - width_pct(scale, 0.0009259259),
        egui::Color32::from_rgb(0x33, 0x33, 0x33),
    );
    // Border
    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(
            width_pct(scale, 0.0018518518),
            egui::Color32::from_rgb(0x22, 0x22, 0x22),
        ),
    );

    // Marker line: -135° to +135° range
    let angle_deg = -135.0 + normalized * 270.0;
    let angle_rad = angle_deg.to_radians();
    let marker_inner = radius * 0.4;
    let marker_outer = radius * 0.85;
    let dx = angle_rad.sin();
    let dy = -angle_rad.cos();
    let start = center + egui::vec2(marker_inner * dx, marker_inner * dy);
    let end = center + egui::vec2(marker_outer * dx, marker_outer * dy);
    painter.line_segment(
        [start, end],
        egui::Stroke::new(width_pct(scale, 0.0027777778), accent),
    );

    // Label below knob
    let label_y = center.y + radius + height_pct(scale, 0.0078125);
    paint_label_with_badge(
        ui,
        param.as_ptr(),
        rect,
        egui::pos2(center.x, label_y),
        egui::Align2::CENTER_TOP,
        label.to_uppercase(),
        egui::FontId::proportional(width_pct(scale, 0.008333334)),
        OFF_WHITE,
        help_for(label),
    );
    let painter = ui.painter();

    // Value below label
    let value_text = if param_text {
        param.normalized_value_to_string(param.preview_normalized(display_value), false)
    } else {
        format_value(display_value, min, max)
    };
    painter.text(
        egui::pos2(center.x, label_y + label_height),
        egui::Align2::CENTER_TOP,
        value_text,
        egui::FontId::proportional(width_pct(scale, 0.008333334)),
        accent,
    );

    response
}

fn knob_drag_value(
    drag_start_value: f32,
    drag_delta_y: f32,
    min: f32,
    max: f32,
    scale: f32,
) -> f32 {
    let speed = (max - min) / (width_pct(scale.max(f32::EPSILON), 0.1388889));
    (drag_start_value - drag_delta_y * speed).clamp(min, max)
}

/// Industrial horizontal fader widget.
fn industrial_fader(
    ui: &mut egui::Ui,
    label: &str,
    param: &FloatParam,
    min: f32,
    max: f32,
    setter: &ParamSetter,
    accent: egui::Color32,
    scale: f32,
) -> egui::Response {
    industrial_fader_text(
        ui,
        label,
        param,
        min,
        max,
        setter,
        accent,
        scale,
        FaderText::Plain,
    )
}

/// What a fader shows, and how its travel maps to the parameter.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FaderText {
    /// The plain number; travel is linear from `min` to `max`.
    Plain,
    /// The parameter's own display string (for values with names, such as
    /// Sync's "Off" or "1/8"); travel is linear from `min` to `max`.
    Param,
    /// The parameter's own display string, and travel follows the
    /// parameter's own range, skew included (Wander Rate;
    /// `h09-route-envelope-to-strike-position`). `min` and `max` are ignored.
    ParamRange,
}

/// The text a fader shows for `param`: its own display string (for
/// parameters whose values have names, such as Sync's "Off" or "1/8"), or the
/// plain number.
fn fader_value_text(param: &FloatParam, min: f32, max: f32, text: FaderText) -> String {
    if text != FaderText::Plain {
        param.normalized_value_to_string(param.unmodulated_normalized_value(), false)
    } else {
        format_value(param.value(), min, max)
    }
}

/// `industrial_fader`, showing the parameter's own display string unless
/// `text` is `Plain` (`h05-expose-missing-gui-controls`).
#[allow(clippy::too_many_arguments)]
fn industrial_fader_text(
    ui: &mut egui::Ui,
    label: &str,
    param: &FloatParam,
    min: f32,
    max: f32,
    setter: &ParamSetter,
    accent: egui::Color32,
    scale: f32,
    text: FaderText,
) -> egui::Response {
    note_drawn(param.as_ptr());
    let track_height = height_pct(scale, 0.0078125);
    let thumb_width = width_pct(scale, 0.014814815);
    let thumb_height = height_pct(scale, 0.026041667);
    let value_width = width_pct(scale, 0.040740743);
    let label_width = width_pct(scale, 0.074074075);
    let track_inset = width_pct(scale, 0.0074074073);
    let row_height = thumb_height.max(track_height + height_pct(scale, 0.0052083335));

    let desired_size = egui::vec2(
        ui.available_width(),
        row_height + height_pct(scale, 0.0052083335),
    );
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    let value = param.value();
    let own_range = text == FaderText::ParamRange;
    let normalized = if own_range {
        param.unmodulated_normalized_value()
    } else if (max - min).abs() > f32::EPSILON {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let value_at = |n: f32| {
        if own_range {
            param.preview_plain(n)
        } else {
            min + n * (max - min)
        }
    };

    // Drag interaction
    if response.drag_started() {
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let track_left = rect.min.x + value_width + track_inset;
            let track_right = rect.right() - label_width - track_inset;
            let track_width = track_right - track_left;
            if track_width > 0.0 {
                let new_normalized = ((pointer.x - track_left) / track_width).clamp(0.0, 1.0);
                let new_value = value_at(new_normalized);
                if (new_value - value).abs() > f32::EPSILON {
                    setter.set_parameter(param, new_value);
                }
            }
        }
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if response.clicked() {
        setter.begin_set_parameter(param);
        if let Some(pointer) = response.interact_pointer_pos() {
            let track_left = rect.min.x + value_width + track_inset;
            let track_right = rect.right() - label_width - track_inset;
            let track_width = track_right - track_left;
            if track_width > 0.0 {
                let new_normalized = ((pointer.x - track_left) / track_width).clamp(0.0, 1.0);
                let new_value = value_at(new_normalized);
                if (new_value - value).abs() > f32::EPSILON {
                    setter.set_parameter(param, new_value);
                }
            }
        }
        setter.end_set_parameter(param);
    }

    let painter = ui.painter();
    let cy = rect.center().y;

    // Value display (left)
    let value_text = fader_value_text(param, min, max, text);
    painter.text(
        egui::pos2(rect.min.x + value_width * 0.5, cy),
        egui::Align2::CENTER_CENTER,
        value_text,
        egui::FontId::proportional(width_pct(scale, 0.009259259)),
        accent,
    );

    // Track
    let track_left = rect.min.x + value_width + track_inset;
    let track_right = rect.right() - label_width - track_inset;
    let track_rect = egui::Rect::from_center_size(
        egui::pos2((track_left + track_right) * 0.5, cy),
        egui::vec2(track_right - track_left, track_height),
    );
    painter.rect_filled(track_rect, width_pct(scale, 0.0027777778), FADER_TRACK_BG);
    painter.rect_stroke(
        track_rect,
        width_pct(scale, 0.0027777778),
        egui::Stroke::new(1.0, CONCRETE),
        egui::StrokeKind::Outside,
    );

    // Thumb
    let thumb_x = track_left + normalized * (track_right - track_left);
    let thumb_rect = egui::Rect::from_center_size(
        egui::pos2(thumb_x, cy),
        egui::vec2(thumb_width, thumb_height),
    );
    painter.rect_filled(thumb_rect, width_pct(scale, 0.0018518518), THUMB_BG);
    painter.rect_stroke(
        thumb_rect,
        width_pct(scale, 0.0018518518),
        egui::Stroke::new(width_pct(scale, 0.0018518518), THUMB_BORDER),
        egui::StrokeKind::Outside,
    );
    // Thumb crosshair
    painter.line_segment(
        [
            thumb_rect.center() - egui::vec2(thumb_width * 0.3, 0.0),
            thumb_rect.center() + egui::vec2(thumb_width * 0.3, 0.0),
        ],
        egui::Stroke::new(width_pct(scale, 0.0013888889), OFF_WHITE),
    );

    // Label (right)
    paint_label_with_badge(
        ui,
        param.as_ptr(),
        rect,
        egui::pos2(rect.right() - label_width * 0.5, cy),
        egui::Align2::CENTER_CENTER,
        label.to_uppercase(),
        egui::FontId::proportional(width_pct(scale, 0.008333334)),
        MUTED_GREY,
        help_for(label),
    );

    response
}

/// Industrial vertical fader for envelope parameters.
fn industrial_vfader(
    ui: &mut egui::Ui,
    label: &str,
    param: &FloatParam,
    min: f32,
    max: f32,
    setter: &ParamSetter,
    accent: egui::Color32,
    scale: f32,
) -> egui::Response {
    note_drawn(param.as_ptr());
    let track_width = width_pct(scale, 0.024074074);
    let track_height = height_pct(scale, 0.15625);
    let thumb_width = width_pct(scale, 0.027777778);
    let thumb_height = height_pct(scale, 0.020833334);
    let desired_size = egui::vec2(
        track_width + width_pct(scale, 0.031481482),
        track_height + height_pct(scale, 0.057291668),
    );

    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());

    let value = param.value();
    let normalized = if (max - min).abs() > f32::EPSILON {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.5
    };

    // Drag interaction
    if response.drag_started() {
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let track_top = rect.min.y + height_pct(scale, 0.020833334);
            if track_height > 0.0 {
                let new_normalized = (1.0 - (pointer.y - track_top) / track_height).clamp(0.0, 1.0);
                let new_value = min + new_normalized * (max - min);
                if (new_value - value).abs() > f32::EPSILON {
                    setter.set_parameter(param, new_value);
                }
            }
        }
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if response.clicked() {
        setter.begin_set_parameter(param);
        if let Some(pointer) = response.interact_pointer_pos() {
            let track_top = rect.min.y + height_pct(scale, 0.020833334);
            if track_height > 0.0 {
                let new_normalized = (1.0 - (pointer.y - track_top) / track_height).clamp(0.0, 1.0);
                let new_value = min + new_normalized * (max - min);
                if (new_value - value).abs() > f32::EPSILON {
                    setter.set_parameter(param, new_value);
                }
            }
        }
        setter.end_set_parameter(param);
    }

    let painter = ui.painter();
    let cx = rect.center().x;

    // Label above
    paint_label_with_badge(
        ui,
        param.as_ptr(),
        rect,
        egui::pos2(cx, rect.min.y + height_pct(scale, 0.0078125)),
        egui::Align2::CENTER_TOP,
        label.to_uppercase(),
        egui::FontId::proportional(width_pct(scale, 0.0074074073)),
        MUTED_GREY,
        help_for(label),
    );
    let painter = ui.painter();

    // Track
    let track_top = rect.min.y + height_pct(scale, 0.020833334);
    let track_bottom = track_top + track_height;
    let track_rect = egui::Rect::from_min_max(
        egui::pos2(cx - track_width * 0.5, track_top),
        egui::pos2(cx + track_width * 0.5, track_bottom),
    );
    painter.rect_filled(track_rect, width_pct(scale, 0.0018518518), CHARCOAL);
    painter.rect_stroke(
        track_rect,
        width_pct(scale, 0.0018518518),
        egui::Stroke::new(width_pct(scale, 0.0018518518), CONCRETE),
        egui::StrokeKind::Outside,
    );

    // Fill from bottom
    let fill_height = track_height * normalized;
    let fill_rect = egui::Rect::from_min_max(
        egui::pos2(track_rect.min.x + 1.0, track_bottom - fill_height),
        egui::pos2(track_rect.max.x - 1.0, track_bottom - 1.0),
    );
    if fill_rect.height() > 0.0 {
        painter.rect_filled(fill_rect, 0.0, accent.linear_multiply(0.8));
        // Top border on fill
        painter.line_segment(
            [fill_rect.left_top(), fill_rect.right_top()],
            egui::Stroke::new(width_pct(scale, 0.0018518518), OFF_WHITE),
        );
    }

    // Thumb
    let thumb_y = track_bottom - fill_height;
    let thumb_rect = egui::Rect::from_center_size(
        egui::pos2(cx, thumb_y),
        egui::vec2(thumb_width, thumb_height),
    );
    painter.rect_filled(thumb_rect, width_pct(scale, 0.0018518518), THUMB_BG);
    painter.rect_stroke(
        thumb_rect,
        width_pct(scale, 0.0018518518),
        egui::Stroke::new(width_pct(scale, 0.0018518518), THUMB_BORDER),
        egui::StrokeKind::Outside,
    );
    // Crosshair
    painter.line_segment(
        [
            thumb_rect.center() - egui::vec2(thumb_width * 0.3, 0.0),
            thumb_rect.center() + egui::vec2(thumb_width * 0.3, 0.0),
        ],
        egui::Stroke::new(width_pct(scale, 0.0013888889), OFF_WHITE),
    );
    painter.line_segment(
        [
            thumb_rect.center() - egui::vec2(0.0, thumb_height * 0.3),
            thumb_rect.center() + egui::vec2(0.0, thumb_height * 0.3),
        ],
        egui::Stroke::new(width_pct(scale, 0.0009259259), OFF_WHITE),
    );

    // Value below
    let value_text = format_value(value, min, max);
    painter.text(
        egui::pos2(cx, track_bottom + height_pct(scale, 0.0078125)),
        egui::Align2::CENTER_TOP,
        value_text,
        egui::FontId::proportional(width_pct(scale, 0.008333334)),
        accent,
    );

    response
}

/// Industrial combo box for integer parameters.
fn industrial_combo(
    ui: &mut egui::Ui,
    id: &'static str,
    label: &str,
    param: &IntParam,
    items: &[(i32, &'static str)],
    setter: &ParamSetter,
    _accent: egui::Color32,
    scale: f32,
) {
    note_drawn(param.as_ptr());
    let current = param.value();
    let selected = items
        .iter()
        .find(|(value, _)| *value == current)
        .map(|(_, name)| *name)
        .unwrap_or("Unknown");

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = width_pct(scale, 0.0027777778);
            let widget = ui
                .label(
                    egui::RichText::new(label.to_uppercase())
                        .size(width_pct(scale, 0.008333334))
                        .color(MUTED_GREY),
                )
                .rect;
            // Trial: selector help (`add-parameter-tooltips`). Remove this
            // block and `selector_help` to drop it.
            let help = selector_help(label, selected);
            note_help(param.as_ptr(), help);
            let side = width_pct(scale, 0.008333334);
            let (badge, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
            paint_info_badge(ui, param.as_ptr(), badge, widget, help);
        });
        egui::ComboBox::from_id_salt(id)
            .width(ui.available_width())
            .selected_text(
                egui::RichText::new(selected)
                    .size(width_pct(scale, 0.010185185))
                    .color(OFF_WHITE),
            )
            .show_ui(ui, |ui| {
                for (value, name) in items {
                    let is_selected = current == *value;
                    if ui
                        .selectable_label(
                            is_selected,
                            egui::RichText::new(*name).size(width_pct(scale, 0.010185185)),
                        )
                        .clicked()
                    {
                        setter.begin_set_parameter(param);
                        setter.set_parameter(param, *value);
                        setter.end_set_parameter(param);
                    }
                }
            });
    });
}

// ---------------------------------------------------------------------------
// Inline help (`add-parameter-tooltips`)
// ---------------------------------------------------------------------------

/// Paints `text` and a help badge right of it as one unit, placed by `align`
/// at `pos`. Hovering the badge, and only the badge, shows `help`.
#[allow(clippy::too_many_arguments)]
fn paint_label_with_badge(
    ui: &egui::Ui,
    param: ParamPtr,
    widget: egui::Rect,
    pos: egui::Pos2,
    align: egui::Align2,
    text: String,
    font: egui::FontId,
    colour: egui::Color32,
    help: &'static str,
) {
    note_help(param, help);
    let side = font.size;
    let gap = font.size * 0.3;
    let galley = ui.painter().layout_no_wrap(text, font, colour);
    let size = egui::vec2(galley.size().x + gap + side, galley.size().y.max(side));
    let unit = align.anchor_size(pos, size);
    let text_pos = egui::pos2(unit.min.x, unit.center().y - galley.size().y * 0.5);
    ui.painter().galley(text_pos, galley, colour);
    let badge = egui::Rect::from_center_size(
        egui::pos2(unit.max.x - side * 0.5, unit.center().y),
        egui::vec2(side, side),
    );
    paint_info_badge(ui, param, badge, widget, help);
}

/// The help badge: a 1 px circle with a centred "i" in `MUTED_GREY`, painted
/// (egui's fonts have no ⓘ), with the tooltip on its own hover area.
fn paint_info_badge(
    ui: &egui::Ui,
    param: ParamPtr,
    badge: egui::Rect,
    widget: egui::Rect,
    help: &str,
) {
    note_badge(param, badge, widget);
    let radius = badge.width() * 0.45;
    let painter = ui.painter();
    painter.circle_stroke(
        badge.center(),
        radius,
        egui::Stroke::new(1.0 / ui.ctx().pixels_per_point(), MUTED_GREY),
    );
    painter.text(
        badge.center(),
        egui::Align2::CENTER_CENTER,
        "i",
        egui::FontId::proportional(radius * 1.4),
        MUTED_GREY,
    );
    ui.interact(
        badge,
        ui.id().with(("help-badge", param)),
        egui::Sense::hover(),
    )
    .on_hover_text(help);
}

/// Help text for a control, by its display label (case-insensitive). Slot
/// labels are model-specific, so the help follows the selected exciter or
/// object. Update the text when a control's behaviour changes.
fn help_for(label: &str) -> &'static str {
    match label.to_ascii_lowercase().as_str() {
        // Exciter globals
        "pressure" => "How hard the exciter presses on the object. Higher drives the object harder: louder and more strained.",
        "speed" => "How fast the exciter moves across or against the object. Higher sounds brighter and more agitated.",
        "roughness" => "Irregularity of the contact. Higher adds grit, noise and jitter to the excitation.",
        // Hand strike
        "hand mass" => "Weight of the striking hand. Heavier hands push more low end into the object.",
        "flesh stiffness" => "Stiffness of the hand's flesh at contact. Higher gives a shorter, brighter slap.",
        "flesh damping" => "How much the flesh absorbs the impact. Higher softens the attack.",
        "mute decay" => "How long the hand rests on the object after the strike. Lower mutes the ring sooner.",
        // Mallets, stick
        "mallet mass" => "Weight of the mallet head. Heavier mallets stay in contact longer and bring out the low modes.",
        "felt softness" => "Softness of the felt covering. Higher gives a rounder, darker strike.",
        "core hardness" => "Hardness of the mallet's core under the felt. Higher adds a brighter click to loud hits.",
        "compression curve" => "How quickly the felt stiffens as it is squeezed. Higher makes hard hits much brighter than soft ones.",
        "material stiffness" => "Stiffness of the mallet head. Higher gives a sharper, brighter strike.",
        "impact damping" => "Energy lost in the contact itself. Higher softens and shortens the attack.",
        "stick mass" => "Weight of the drumstick. Heavier sticks drive the object harder and darker.",
        "tip stiffness" => "Stiffness of the stick's tip. Higher gives a sharper, brighter tick.",
        "restitution bounciness" => "How much the stick bounces back after contact. Higher gives livelier, more rebound-rich hits.",
        "micro bounce limit" => "Maximum number of small rebounds after a strike. Higher allows a buzzier, rattling attack.",
        // Brush
        "wire density" => "Number of wires in the brush. Higher gives a denser, smoother swish.",
        "spread duration" => "How long the brush's wires take to land, in milliseconds. Higher smears the hit into a sweep.",
        "wire stiffness" => "Stiffness of the wires (brush wires or spring coils, by model). Higher sounds brighter and tighter.",
        "amplitude randomization" => "Random level variation between wires or hits. Higher sounds looser and more organic.",
        // Pipe, chain
        "pipe mass" => "Weight of the striking pipe. Heavier pipes hit harder and darker.",
        "metal stiffness" => "Stiffness of the striking metal. Higher gives a harder, brighter clang.",
        "pipe pitch" => "Tuning of the striking pipe's own ring. Higher raises its overtone.",
        "pipe ring decay" => "How long the striking pipe's own ring lasts. Higher adds a longer metallic tail to each hit.",
        "link count" => "Number of links in the dropped chain. Higher gives a longer, busier rattle.",
        "chain mass" => "Weight of the dropped chain. Heavier chains hit harder and darker.",
        "drop envelope spread" => "How spread out in time the links land, in milliseconds. Higher stretches the cascade.",
        "internal rattle" => "Rattling between the links. Higher adds more clatter to the strike.",
        "rattle color" => "Tone of the rattle. Higher is brighter.",
        // Bow
        "bow pressure" => "How hard the bow presses on the object. Higher is louder and grittier, and too much makes it crunch.",
        "bow speed" => "How fast the bow moves. Higher gives a brighter, more stable tone.",
        "rosin grip" => "Static friction of the rosin. Higher grips harder, giving a stronger, rougher stick-slip.",
        "slip curve" => "How quickly friction falls once the bow slips. Higher gives a sharper, more aggressive tone.",
        // Scrape, grind, drag
        "scrape speed" => "How fast the point scrapes along the surface. Higher raises the chatter's pitch and density.",
        "point pressure" => "How hard the point presses into the surface. Higher is louder and harsher.",
        "chatter pitch" => "Pitch of the stick-slip chatter. Higher sounds squealier.",
        "chatter damping" => "How quickly each chatter burst dies away. Higher sounds drier and tighter.",
        "grind speed" => "Speed of the grinding wheel. Higher gives a brighter, denser roar.",
        "grind pressure" => "How hard the grinder presses on the object. Higher is louder and more aggressive.",
        "surface grit" => "Coarseness of the ground surface. Higher adds noise and crackle.",
        "grit color" => "Tone of the grit noise. Higher is brighter.",
        "drag speed" => "How fast the exciter drags across the ridges. Higher raises the ridge rate.",
        "ridge spacing" => "Distance between the surface's ridges. Higher spaces the ticks further apart.",
        "ridge depth" => "Depth of the ridges. Higher gives stronger, louder ticks.",
        "exciter mass" => "Weight of the dragged exciter. Heavier sounds lower and heavier.",
        // Tension rise / snap
        "pull speed" => "How fast the tension builds. Higher makes creaks come faster.",
        "break threshold" => "Tension at which the material slips. Higher builds more tension before each creak.",
        "slip stochasticity" => "Randomness of the slips. Higher sounds more irregular.",
        "creak sharpness" => "Sharpness of each creak. Higher sounds brighter and more percussive.",
        "pull distance" => "How far the hook pulls before releasing. Higher gives a bigger snap.",
        "hook stiffness" => "Stiffness of the hook. Higher gives a sharper release.",
        "snap force" => "Force of the release. Higher is louder and harder.",
        // Jet, hum, rain
        "air pressure" => "Pressure of the air jet. Higher is louder and more turbulent.",
        "nozzle width" => "Width of the nozzle. Higher gives a broader, darker hiss.",
        "turbulence chaos" => "Instability of the airflow. Higher sounds rougher and more chaotic.",
        "mains frequency" => "Frequency of the electromagnetic drive, in hertz. It sets the hum's base pitch.",
        "coil proximity" => "How close the coil sits to the object. Higher couples more strongly and sounds louder.",
        "voltage sag" => "Instability of the supply. Higher makes the hum waver and sag.",
        "flow rate" => "How many particles fall per second. Higher gives a denser rain.",
        "particle mass" => "Weight of each particle. Heavier particles hit harder and darker.",
        "mass variance" => "Variation in particle weight. Higher sounds more uneven.",
        // Objects
        "tube diameter" => "Diameter of the pipe. Higher sounds lower and hollower.",
        "pipe length" => "Length of the pipe. Longer pipes sound lower.",
        "shell decay" => "How long the shell rings. Higher sustains longer.",
        "plate aspect" => "Ratio of the plate's sides. Changes how its modes are spread, from even to clustered.",
        "plate stiffness" => "Stiffness of the plate. Higher spreads its overtones and sounds brighter.",
        "plate size" => "Size of the plate. Larger plates sound lower.",
        "tank volume" => "Volume of the tank. Higher sounds lower and boomier.",
        "cavity mix" => "Level of the air cavity's resonance. Higher sounds hollower.",
        "tank size" => "Size of the tank. Larger tanks sound lower.",
        "link mass" => "Weight of each chain link. Heavier links sound lower and heavier.",
        "instability" => "How much the links shift against each other. Higher sounds looser and more chaotic.",
        "chain size" => "Size of the chain. Larger chains sound lower.",
        "beam mass" => "Weight of the beam. Heavier beams sound lower.",
        "shear density" => "Strength of the beam's shear modes. Higher adds more inharmonic overtones.",
        "rigidity damping" => "Damping of the beam's stiffness. Higher shortens the high modes.",
        "braid stiffness" => "Stiffness of the cable's braid. Higher sounds brighter and less harmonic.",
        "tension drop" => "How much the pitch drops on loud notes. Higher bends loud notes down further.",
        "cable tension" => "Tension of the cable. Higher sounds higher.",
        "dispersion chirp" => "How much the spring smears high frequencies. Higher gives a stronger springy chirp.",
        "spring slosh" => "Wobble of the spring. Higher sounds looser and more liquid.",
        "coil length" => "Length of the coil. Longer coils sound lower.",
        "metal thinness" => "Thinness of the sheet. Higher bends and warps the sound more.",
        "edge damping" => "Damping at the sheet's edges. Higher shortens the ring.",
        "sheet size" => "Size of the sheet. Larger sheets sound lower.",
        "tooth dissonance" => "Detuning of the cog's teeth. Higher sounds more dissonant.",
        "blade radius" => "Radius of the cog. Larger cogs sound lower.",
        "blade thickness" => "Thickness of the cog. Higher sounds brighter and stiffer.",
        // Resonator globals
        "strike pos" => "Where the exciter hits the object. Moving it changes which modes ring, so the timbre changes.",
        "coupling" => "How strongly the exciter and object push each other. Higher makes their interaction stronger and wilder.",
        "pos wander" => "How much the strike position drifts while the note plays. Higher makes the timbre move more.",
        "pos env" => "How much the envelope sweeps the strike position. Higher gives a stronger timbral sweep.",
        "wander rate" => "Speed of the strike position's drift. Higher wanders faster.",
        "anchor" => "How firmly the fundamental keeps its pitch while other modes move. Higher keeps notes more in tune.",
        "thickness" => "Thickness of the object. Higher sounds brighter and stiffer.",
        "heat" => "Heats the metal: lowers its pitch, darkens it and adds a slow warble. Higher is hotter.",
        "sludge" => "Coats the object in sludge. Higher damps it and dulls the attack.",
        "rust" => "Corrodes the object: shortens the decay and dulls the upper modes. Higher sounds duller and drier.",
        "damage" => "Dents and cracks the object: detunes its modes and adds buzzing companions. Higher sounds more broken.",
        // Envelope
        "attack" => "Time the note takes to reach full level. Higher fades in slower.",
        "decay" => "Time to fall from the peak to the sustain level. Higher decays slower.",
        "sustain" => "Level held while the note is down. Higher stays louder.",
        "release" => "Time to fade after the note ends. Higher fades longer.",
        "hold" => "Time the envelope holds at its peak before decaying. Higher holds longer.",
        "onset" => "Length of the onset stage that starts the friction envelope. Higher eases the note in slower.",
        "onset lvl" => "Level of the friction envelope's onset stage. Higher starts the note stronger.",
        "sustain time" => "How long the friction envelope sustains in Drone and looped modes. Higher sustains longer.",
        "friction decay" => "Decay time of the friction envelope. Higher fades the friction slower.",
        "env amount" => "How much the envelope shapes the excitation. Lower flattens it.",
        "vel to peak" => "How much velocity changes the envelope's peak. Higher makes soft notes quieter.",
        "vel to level" => "How much velocity changes the note's level. Higher gives more dynamic range.",
        "vel to time" => "How much velocity shortens the envelope. Higher makes hard notes snappier.",
        "curve" => "Shape of the envelope's segments. Turning it away from the middle bends them more.",
        "global time" => "Scales every envelope time. Higher slows the whole envelope.",
        // Processing
        "cutoff" => "Cutoff of the low-pass filter after the voices. Lower sounds darker.",
        "resonance" => "Resonant peak at the cutoff. Higher rings and whistles more.",
        "tolerance" => "Random component drift in the filter. Higher sounds less precise and more analogue.",
        "drive amt" => "Saturation of the Lorenz drive in the post chain. Higher sounds more distorted.",
        "bias starve" => "Starves the drive's supply now and then. Higher makes it sag and sputter more often.",
        "chaos" => "How much the Lorenz attractor modulates the drive. Higher makes the distortion move unpredictably.",
        "chassis mat" => "Material of the body that resonates with the sound. Changes the body's tone.",
        "chassis vol" => "Size of the body. Higher gives lower, slightly louder resonances.",
        "spread" => "Stereo spread of the image. Higher sounds wider.",
        "proximity" => "How close the listener sits. Higher sounds nearer and fuller.",
        "amount" => "Mix of the Space effect. Higher sounds wetter.",
        "factory size" => "Size of the factory room. Higher gives a longer, bigger reverb.",
        "machinery" => "Clutter in the factory room. Higher adds busier, denser reflections.",
        "wall impedance" => "Hardness of the walls. Higher sounds brighter and more reflective.",
        "spring tension" => "Tension of the reverb spring. Higher raises its twang.",
        "spring tank size" | "tank" => "Size of the spring tank. Higher gives a longer reverb.",
        "delay time" => "Time between echoes. Higher spaces them further apart.",
        "sync" => "Locks the echo to the host tempo. Off leaves it on Delay Time.",
        "movement" => "How much the echo's delay wobbles. Higher sounds more warped.",
        "hf damping" => "Darkens each echo repeat. Higher rolls off more highs.",
        "ceiling" => "The output's maximum level. Lower clips the diode stage and limiter earlier.",
        "softness" => "Knee of the diode stage. 0 clips hard; higher bends earlier and sounds warmer.",
        "asymmetry" => "Clips the negative half earlier. Higher adds even harmonics and a fuller grit.",
        "legacy drive" => "Master drive on the voice mix, before the post chain. Higher saturates more.",
        // Header
        "body" => "Level of the resonating body under the sound. Higher sounds fuller and more physical.",
        "width" => "Stereo width of the voices. Higher spreads them wider.",
        "output" => "Final output level, in dB. The ceiling still limits the peaks.",
        // Macros
        "mass" => "Makes everything heavier: exciter weights and object size. Higher sounds bigger and lower.",
        "corrosion" => "Wears the object down: more damping and rust. Higher sounds older and duller.",
        "violence" => "Pushes drive, chaos and damage. Higher sounds harsher and more broken.",
        "brightness" => "Opens the filter and brightens the object. Higher sounds brighter.",
        _ => "",
    }
}

// Trial: selector help (`add-parameter-tooltips`). Remove this function and
// the block in `industrial_combo` that calls it to drop the trial.
/// Help for a selector's current option.
fn selector_help(label: &str, selected: &str) -> &'static str {
    if let Some(exciter) = (1..=16)
        .map(ExciterType::from_int)
        .find(|e| e.name() == selected)
    {
        return exciter_panel(exciter).description;
    }
    if let Some(object) = (0..9).map(Object::from_int).find(|o| o.name() == selected) {
        return resonator_panel(object).description;
    }
    match (label.to_ascii_lowercase().as_str(), selected) {
        ("mode", "Off") => "No Space effect: the sound stays dry.",
        ("mode", "Factory") => "A large factory hall reverb.",
        ("mode", "Spring") => "A spring reverb tank with its twang and chirp.",
        ("mode", "Echo") => "A modulated stereo echo, free or synced to the host tempo.",
        ("mode", "Eco") => "Lowest CPU: fewer modes per voice and no clipper oversampling.",
        ("mode", "Normal") => "The default balance of detail and CPU.",
        ("mode", "High") => "More modes per voice and more clipper oversampling.",
        ("mode", "Render") => "Most detail and least aliasing, for bouncing. Uses the most CPU.",
        ("play", "Tonal") => "Notes play at their pitch and decay naturally.",
        ("play", "Kit") => "Each key plays a fixed variation, like a drum kit.",
        ("play", "Drone") => "Notes sustain while held, for bowed and continuous sounds.",
        ("limiter", "Hard") => "Clamps peaks at the ceiling with no added latency.",
        ("limiter", "Lookahead") => {
            "Turns peaks down smoothly ahead of time. Adds about 1 ms of latency."
        }
        ("loop", "Off") => "The envelope plays once.",
        ("loop", "Forward") => "The envelope repeats between the Start and End stages.",
        ("loop", "Ping-Pong") => {
            "The envelope runs back and forth between the Start and End stages."
        }
        ("start", _) => "The stage where the envelope loop begins.",
        ("end", _) => "The stage where the envelope loop ends.",
        ("scale", _) => "Size of the editor window.",
        _ => "",
    }
}

/// Format a parameter value for display.
fn format_value(value: f32, min: f32, max: f32) -> String {
    let range = max - min;
    if range > 100.0 {
        format!("{:.0}", value)
    } else if range > 10.0 {
        format!("{:.1}", value)
    } else if range > 1.0 {
        format!("{:.2}", value)
    } else {
        format!("{:.3}", value)
    }
}

/// Draw a recessed panel frame.
fn recessed_panel(scale: f32) -> egui::Frame {
    egui::Frame::new()
        .fill(RECESSED_BG)
        .inner_margin(egui::Margin::same(width_pct(scale, 0.005555556) as i8))
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgb(0x22, 0x22, 0x22),
        ))
}

/// Draw a metal panel frame.
fn metal_panel(scale: f32) -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(0x2A, 0x2A, 0x2A))
        .inner_margin(egui::Margin::same(width_pct(scale, 0.0074074073) as i8))
        .stroke(egui::Stroke::new(1.0, STEEL_LIGHT))
}

/// Draw a section heading with accent color.
fn section_heading(ui: &mut egui::Ui, text: &str, accent: egui::Color32, scale: f32) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text)
                .size(width_pct(scale, 0.009259259))
                .color(OFF_WHITE)
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let rect = ui.available_rect_before_wrap();
            let painter = ui.painter();
            // Small accent dot
            painter.circle_filled(
                egui::pos2(
                    rect.right() - width_pct(scale, 0.0037037036),
                    rect.center().y,
                ),
                width_pct(scale, 0.0027777778),
                accent,
            );
        });
    });
    // Separator line
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    painter.line_segment(
        [rect.left_top(), egui::pos2(rect.right(), rect.left_top().y)],
        egui::Stroke::new(1.0, CONCRETE),
    );
    ui.add_space(height_pct(scale, 0.0026041667));
}

// ---------------------------------------------------------------------------
// Main render
// ---------------------------------------------------------------------------

fn render_ui(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    state: &mut EditorUiState,
    scale: f32,
) {
    // Paint shell background
    let shell_rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    painter.rect_filled(shell_rect, 0.0, DARK_STEEL);

    // Screws in corners
    let screw_r = width_pct(scale, 0.005555556);
    let margin = width_pct(scale, 0.011111112);
    paint_screw(
        &painter,
        egui::pos2(shell_rect.min.x + margin, shell_rect.min.y + margin),
        screw_r,
    );
    paint_screw(
        &painter,
        egui::pos2(shell_rect.max.x - margin, shell_rect.min.y + margin),
        screw_r,
    );
    paint_screw(
        &painter,
        egui::pos2(shell_rect.min.x + margin, shell_rect.max.y - margin),
        screw_r,
    );
    paint_screw(
        &painter,
        egui::pos2(shell_rect.max.x - margin, shell_rect.max.y - margin),
        screw_r,
    );

    // Header
    render_header(ui, params, setter, state, scale);

    ui.add_space(height_pct(scale, 0.0052083335));

    // Macros sit between the header and the columns: below the columns
    // they left an odd gap under the shorter ones.
    render_macro_strip(ui, params, setter, scale);

    ui.add_space(height_pct(scale, 0.0052083335));

    // The limiter panel's dot shows the diode stage's reduction; keep
    // repainting while it's lit, so it falls back to rest on its own.
    let now = ui.ctx().input(|input| input.time);
    let dot_db = state.advance_diode_dot(now);
    if dot_db > DIODE_DOT_REST_DB {
        ui.ctx().request_repaint();
    }
    let dot_colour = diode_dot_colour(dot_db);

    let original_item_spacing_x = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 0.0;
    ui.columns(3, |columns| {
        render_exciter_column(&mut columns[0], params, setter, scale);
        render_resonator_column(&mut columns[1], params, setter, scale);
        render_processing_column(&mut columns[2], params, setter, scale, dot_colour);
    });
    ui.spacing_mut().item_spacing.x = original_item_spacing_x;

    // Footer
    render_footer(ui, scale);
}

fn render_header(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    state: &mut EditorUiState,
    scale: f32,
) {
    let panel = metal_panel(scale);
    panel.show(ui, |ui| {
        let original_item_spacing_x = ui.spacing().item_spacing.x;
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.columns(3, |columns| {
            columns[0].horizontal(|ui| {
                metal_panel(scale).show(ui, |ui| {
                    ui.add_space(width_pct(scale, 0.0018518518));
                    ui.label(
                        egui::RichText::new("CORROSION")
                            .size(width_pct(scale, 0.02037037))
                            .color(OFF_WHITE)
                            .strong(),
                    );
                });
                ui.add_space(width_pct(scale, 0.0074074073));
                let (accent_rect, _) = ui.allocate_exact_size(
                    egui::vec2(width_pct(scale, 0.0018518518), height_pct(scale, 0.03125)),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(accent_rect, 0.0, DANGER_RED);
                ui.add_space(width_pct(scale, 0.0037037036));
                let text_block_size =
                    egui::vec2(width_pct(scale, 0.074074075), height_pct(scale, 0.03125));
                let (text_rect, _) = ui.allocate_exact_size(text_block_size, egui::Sense::hover());
                let painter = ui.painter();
                let font = egui::FontId::proportional(width_pct(scale, 0.0074074073));
                let galley1 =
                    painter.layout_no_wrap("INDUSTRIAL RESONATOR".into(), font.clone(), MUTED_GREY);
                let galley2 =
                    painter.layout_no_wrap("MODEL: CR-89X".into(), font.clone(), MUTED_GREY);
                let gap = height_pct(scale, 0.0026041667);
                let total_h = galley1.rect.height() + galley2.rect.height() + gap;
                let y = text_rect.center().y - total_h / 2.0;
                painter.galley(egui::pos2(text_rect.min.x, y), galley1.clone(), MUTED_GREY);
                painter.galley(
                    egui::pos2(text_rect.min.x, y + galley1.rect.height() + gap),
                    galley2,
                    MUTED_GREY,
                );
            });

            columns[1].vertical_centered(|ui| {
                render_preset_loader(ui, params, setter, state, scale);
            });

            columns[2].horizontal(|ui| {
                let original_item_spacing_x = ui.spacing().item_spacing.x;
                ui.spacing_mut().item_spacing.x = 0.0;
                let fader_width = width_pct(scale, 0.024074074);
                let leading_padding = width_pct(scale, 0.0037037036);
                let fader_gap = width_pct(scale, 0.0037037036);
                let combo_width = ((ui.available_width()
                    - leading_padding
                    - fader_width * 3.0
                    - fader_gap * 2.0)
                    / 2.0)
                    .max(0.0);

                ui.add_space(leading_padding);

                // The four mode selectors share the space left of the faders
                // as a 2 x 2 grid. They used to sit in one row sized for two,
                // which made this column, and through `ui.columns` the whole
                // editor, about 1.7 times wider than the window.
                ui.allocate_ui_with_layout(
                    egui::vec2(combo_width * 2.0, ui.available_height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        for row in [[0, 1], [2, 3]] {
                            ui.horizontal(|ui| {
                                for index in row {
                                    match index {
                                        0 => {
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(combo_width, ui.available_height()),
                                                egui::Layout::top_down(egui::Align::LEFT),
                                                |ui| {
                                                    industrial_combo(
                                                        ui,
                                                        "ui-scale-header",
                                                        "SCALE",
                                                        &params.ui_scale,
                                                        &[
                                                            (0, "50%"),
                                                            (1, "75%"),
                                                            (2, "100%"),
                                                            (3, "125%"),
                                                            (4, "150%"),
                                                        ],
                                                        setter,
                                                        MUTED_GREY,
                                                        scale,
                                                    );
                                                },
                                            );
                                        }
                                        1 => {
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(combo_width, ui.available_height()),
                                                egui::Layout::top_down(egui::Align::LEFT),
                                                |ui| {
                                                    industrial_combo(
                                                        ui,
                                                        "quality-mode-header",
                                                        "MODE",
                                                        &params.quality_mode,
                                                        &[
                                                            (0, "Eco"),
                                                            (1, "Normal"),
                                                            (2, "High"),
                                                            (3, "Render"),
                                                        ],
                                                        setter,
                                                        SAFETY_YELLOW,
                                                        scale,
                                                    );
                                                },
                                            );
                                        }
                                        2 => {
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(combo_width, ui.available_height()),
                                                egui::Layout::top_down(egui::Align::LEFT),
                                                |ui| {
                                                    industrial_combo(
                                                        ui,
                                                        "play-mode-header",
                                                        "PLAY",
                                                        &params.play_mode,
                                                        &[(0, "Tonal"), (1, "Kit"), (2, "Drone")],
                                                        setter,
                                                        SAFETY_YELLOW,
                                                        scale,
                                                    );
                                                },
                                            );
                                        }
                                        3 => {
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(combo_width, ui.available_height()),
                                                egui::Layout::top_down(egui::Align::LEFT),
                                                |ui| {
                                                    industrial_combo(
                                                        ui,
                                                        "limiter-mode-header",
                                                        "LIMITER",
                                                        &params.limiter_mode,
                                                        &[(0, "Hard"), (1, "Lookahead")],
                                                        setter,
                                                        DANGER_RED,
                                                        scale,
                                                    );
                                                },
                                            );
                                        }
                                        _ => {}
                                    }
                                }
                            });
                        }
                    },
                );
                vfader_global(
                    ui,
                    "OUTPUT",
                    &params.output,
                    0.0,
                    util::db_to_gain(40.0),
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(fader_gap);
                vfader_global(
                    ui,
                    "WIDTH",
                    &params.width,
                    -2.0,
                    3.0,
                    setter,
                    RUST_ORANGE,
                    scale,
                );
                ui.add_space(fader_gap);
                vfader_global(
                    ui,
                    "BODY",
                    &params.body,
                    0.0,
                    5.0,
                    setter,
                    DANGER_RED,
                    scale,
                );
                ui.spacing_mut().item_spacing.x = original_item_spacing_x;
            });
        });
        ui.spacing_mut().item_spacing.x = original_item_spacing_x;
    });
}

/// Small vertical fader for global controls (Output, Width, Body).
fn vfader_global(
    ui: &mut egui::Ui,
    label: &str,
    param: &FloatParam,
    min: f32,
    max: f32,
    setter: &ParamSetter,
    accent: egui::Color32,
    scale: f32,
) {
    note_drawn(param.as_ptr());
    let track_width = width_pct(scale, 0.009259259);
    let track_height = height_pct(scale, 0.046875);
    let desired = egui::vec2(
        track_width + width_pct(scale, 0.014814815),
        track_height + height_pct(scale, 0.036458332),
    );

    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    let value = param.value();
    let normalized = if (max - min).abs() > f32::EPSILON {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.5
    };

    if response.drag_started() {
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let track_top = rect.min.y + height_pct(scale, 0.018229166);
            if track_height > 0.0 {
                let new_norm = (1.0 - (pointer.y - track_top) / track_height).clamp(0.0, 1.0);
                let new_val = min + new_norm * (max - min);
                if (new_val - value).abs() > f32::EPSILON {
                    setter.set_parameter(param, new_val);
                }
            }
        }
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if response.clicked() {
        setter.begin_set_parameter(param);
        if let Some(pointer) = response.interact_pointer_pos() {
            let track_top = rect.min.y + height_pct(scale, 0.018229166);
            if track_height > 0.0 {
                let new_norm = (1.0 - (pointer.y - track_top) / track_height).clamp(0.0, 1.0);
                let new_val = min + new_norm * (max - min);
                if (new_val - value).abs() > f32::EPSILON {
                    setter.set_parameter(param, new_val);
                }
            }
        }
        setter.end_set_parameter(param);
    }

    let painter = ui.painter();
    let cx = rect.center().x;

    // Label
    paint_label_with_badge(
        ui,
        param.as_ptr(),
        rect,
        egui::pos2(cx, rect.min.y + height_pct(scale, 0.0052083335)),
        egui::Align2::CENTER_TOP,
        label.to_string(),
        egui::FontId::proportional(width_pct(scale, 0.0074074073)),
        MUTED_GREY,
        help_for(label),
    );
    let painter = ui.painter();

    // Track
    let track_top = rect.min.y + height_pct(scale, 0.018229166);
    let track_rect = egui::Rect::from_min_max(
        egui::pos2(cx - track_width * 0.5, track_top),
        egui::pos2(cx + track_width * 0.5, track_top + track_height),
    );
    painter.rect_filled(track_rect, width_pct(scale, 0.0018518518), RECESSED_BG);
    painter.rect_stroke(
        track_rect,
        width_pct(scale, 0.0018518518),
        egui::Stroke::new(1.0, CONCRETE),
        egui::StrokeKind::Outside,
    );

    // Fill
    let fill_height = track_height * normalized;
    let fill_rect = egui::Rect::from_min_max(
        egui::pos2(track_rect.min.x + 1.0, track_rect.max.y - fill_height),
        egui::pos2(track_rect.max.x - 1.0, track_rect.max.y - 1.0),
    );
    if fill_rect.height() > 0.0 {
        painter.rect_filled(fill_rect, 0.0, accent.linear_multiply(0.8));
        painter.line_segment(
            [fill_rect.left_top(), fill_rect.right_top()],
            egui::Stroke::new(width_pct(scale, 0.0018518518), OFF_WHITE),
        );
    }
}

fn render_preset_loader(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    state: &mut EditorUiState,
    scale: f32,
) {
    metal_panel(scale).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("PRESET BANK")
                    .size(width_pct(scale, 0.0069444445))
                    .color(MUTED_GREY),
            );
            ui.horizontal(|ui| {
                let count = state.preset_count();
                if count == 0 {
                    ui.label(
                        egui::RichText::new("No presets")
                            .size(width_pct(scale, 0.008333334))
                            .color(MUTED_GREY),
                    );
                    return;
                }

                state.selected_factory_preset = state.selected_factory_preset.min(count - 1);
                let selected_name = state.preset_name(state.selected_factory_preset).to_string();
                let mut load = None;

                let arrow_size =
                    egui::vec2(width_pct(scale, 0.016666668), height_pct(scale, 0.0234375));
                if ui
                    .add_sized(
                        arrow_size,
                        egui::Button::new(
                            egui::RichText::new("‹").size(width_pct(scale, 0.012962963)),
                        ),
                    )
                    .clicked()
                {
                    load = Some((state.selected_factory_preset + count - 1) % count);
                }

                let spacing = ui.spacing().item_spacing.x;
                let combo_width = (ui.available_width() - arrow_size.x - spacing).max(0.0);
                let factory_count = state.factory_presets.len();
                egui::ComboBox::from_id_salt("factory-preset-loader")
                    .width(combo_width)
                    .selected_text(
                        egui::RichText::new(selected_name)
                            .size(width_pct(scale, 0.012962963))
                            .color(OFF_WHITE),
                    )
                    .show_ui(ui, |ui| {
                        for index in 0..count {
                            if index == factory_count {
                                // User presets follow the factory bank.
                                ui.separator();
                                ui.label(
                                    egui::RichText::new("USER")
                                        .size(width_pct(scale, 0.0069444445))
                                        .color(MUTED_GREY),
                                );
                            }
                            if ui
                                .selectable_label(
                                    index == state.selected_factory_preset,
                                    egui::RichText::new(state.preset_name(index))
                                        .size(width_pct(scale, 0.012962963)),
                                )
                                .clicked()
                            {
                                load = Some(index);
                            }
                        }
                    });

                if ui
                    .add_sized(
                        arrow_size,
                        egui::Button::new(
                            egui::RichText::new("›").size(width_pct(scale, 0.012962963)),
                        ),
                    )
                    .clicked()
                {
                    load = Some((state.selected_factory_preset + 1) % count);
                }

                if let Some(index) = load {
                    state.selected_factory_preset = index;
                    if let Some(preset) = state.preset(index) {
                        apply_preset(params, setter, &preset);
                    }
                }
            });
            ui.horizontal(|ui| {
                let text_size = width_pct(scale, 0.011111112);
                if state.confirm_overwrite {
                    ui.label(
                        egui::RichText::new(format!("Replace \"{}\"?", state.save_name.trim()))
                            .size(text_size)
                            .color(SAFETY_YELLOW),
                    );
                    if ui
                        .button(egui::RichText::new("Replace").size(text_size))
                        .clicked()
                    {
                        state.save(params, true);
                    }
                    if ui
                        .button(egui::RichText::new("Cancel").size(text_size))
                        .clicked()
                    {
                        state.confirm_overwrite = false;
                    }
                } else {
                    let field_width = (ui.available_width() - width_pct(scale, 0.05)).max(0.0);
                    ui.add_sized(
                        egui::vec2(field_width, height_pct(scale, 0.0234375)),
                        egui::TextEdit::singleline(&mut state.save_name).hint_text("Preset name"),
                    );
                    let save = egui::Button::new(egui::RichText::new("Save").size(text_size));
                    let can_save = !state.save_name.trim().is_empty();
                    if ui.add_enabled(can_save, save).clicked() {
                        state.save(params, false);
                    }
                }
            });
            let notice = match (&state.save_message, state.unreadable_user_presets) {
                (Some((message, is_error)), _) => Some((message.clone(), *is_error)),
                (None, 0) => None,
                (None, n) => Some((format!("{n} user preset file(s) couldn't be read"), true)),
            };
            if let Some((message, is_error)) = notice {
                ui.label(
                    egui::RichText::new(message)
                        .size(width_pct(scale, 0.0069444445))
                        .color(if is_error { DANGER_RED } else { MUTED_GREY }),
                );
            }
            ui.horizontal(|ui| {
                use crate::randomizer::RandomizerMode;
                let text_size = width_pct(scale, 0.011111112);
                egui::ComboBox::from_id_salt("randomizer-mode")
                    .selected_text(
                        egui::RichText::new(state.randomizer_mode.name()).size(text_size),
                    )
                    .show_ui(ui, |ui| {
                        for mode in [
                            RandomizerMode::Safe,
                            RandomizerMode::Object,
                            RandomizerMode::Damage,
                            RandomizerMode::Full,
                        ] {
                            ui.selectable_value(
                                &mut state.randomizer_mode,
                                mode,
                                egui::RichText::new(mode.name()).size(text_size),
                            );
                        }
                    });
                if ui
                    .button(egui::RichText::new("Randomize").size(text_size))
                    .clicked()
                {
                    let patch = crate::randomizer::randomize(
                        state.randomizer_mode,
                        &mut state.randomizer_noise,
                    );
                    for (param, value) in randomizer_assignments(params, &patch) {
                        setter.begin_set_parameter(param);
                        setter.set_parameter(param, value);
                        setter.end_set_parameter(param);
                    }
                }
            });
        });
    });
}

/// The parameters a randomized patch sets, and their values: only these
/// seven change (`h05-expose-missing-gui-controls`).
fn randomizer_assignments<'a>(
    params: &'a CorrosionParams,
    patch: &crate::randomizer::RandomizedParams,
) -> [(&'a FloatParam, f32); 7] {
    [
        (&params.size, patch.size),
        (&params.rust, patch.rust),
        (&params.damage, patch.damage),
        (&params.drive, patch.drive),
        (&params.width, patch.width),
        (&params.body, patch.body),
        (&params.output, patch.output),
    ]
}

/// Load `preset` into the plugin: every parameter except the editor-only
/// ones (`PRESET_SKIPPED_PARAMS`), each as one begin/set/end gesture so the
/// host records it. Driven by the parameter map, so no parameter can be
/// missed (`h04-load-every-preset-field-in-gui`).
fn apply_preset(params: &CorrosionParams, setter: &ParamSetter, preset: &Preset) {
    let values: std::collections::HashMap<String, f32> =
        preset.parameter_values(params).into_iter().collect();
    for (id, ptr, _) in params.param_map() {
        if let Some(&normalized) = values.get(&id) {
            // SAFETY: `ptr` comes from the plugin's own parameter map, which
            // outlives the editor.
            unsafe {
                setter.raw_context.raw_begin_set_parameter(ptr);
                setter
                    .raw_context
                    .raw_set_parameter_normalized(ptr, normalized);
                setter.raw_context.raw_end_set_parameter(ptr);
            }
        }
    }
}

fn load_factory_presets() -> Vec<FactoryPresetEntry> {
    let mut presets: Vec<_> = FACTORY_PRESETS_DIR
        .files()
        .filter(|file| {
            file.path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension == "corrosion-preset")
        })
        .filter_map(|file| {
            let json = file.contents_utf8()?;
            let preset = Preset::from_json_str(json).ok()?;
            Some(FactoryPresetEntry {
                name: preset.name,
                json,
            })
        })
        .collect();

    presets.sort_by(|left, right| left.name.cmp(&right.name));
    presets
}

// ---------------------------------------------------------------------------
// Exciter column
// ---------------------------------------------------------------------------

fn render_exciter_column(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    scale: f32,
) {
    let panel = metal_panel(scale);
    panel.show(ui, |ui| {
        // Column header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("EXCITER")
                    .size(width_pct(scale, 0.014814815))
                    .color(OFF_WHITE)
                    .strong(),
            );
        });
        let rect = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [rect.left_top(), egui::pos2(rect.right(), rect.left_top().y)],
            egui::Stroke::new(width_pct(scale, 0.0018518518), SAFETY_YELLOW),
        );
        ui.add_space(height_pct(scale, 0.0052083335));

        // Model selector
        let exciter = ExciterType::from_int(params.exciter.value());
        industrial_combo(
            ui,
            "exciter-model",
            "Model Type",
            &params.exciter,
            exciter_model_items(),
            setter,
            SAFETY_YELLOW,
            scale,
        );

        // Description
        let panel_spec = exciter_panel(exciter);
        ui.label(
            egui::RichText::new(panel_spec.description)
                .size(width_pct(scale, 0.0074074073))
                .color(MUTED_GREY),
        );
        ui.add_space(height_pct(scale, 0.0052083335));

        // Global Pressure, Speed and Roughness: they act on every exciter
        // model, so they sit with the selector rather than in a model's panel
        // (`h06-wire-global-exciter-controls`).
        let globals: [(&str, &FloatParam); 3] = [
            ("Pressure", &params.exciter_pressure),
            ("Speed", &params.exciter_speed),
            ("Roughness", &params.exciter_roughness),
        ];
        let knob_width = width_pct(scale, 0.053703703);
        let gap = ((ui.available_width() - globals.len() as f32 * knob_width)
            / (globals.len() + 1) as f32)
            .max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(gap);
            for (label, param) in globals {
                industrial_knob_text(
                    ui, label, param, 0.0, 1.0, setter, OFF_WHITE, scale, true, true,
                );
                ui.add_space(gap);
            }
        });
        ui.add_space(height_pct(scale, 0.0052083335));

        if !panel_spec.controls.is_empty() {
            let recessed = recessed_panel(scale);
            recessed.show(ui, |ui| {
                section_heading(ui, panel_spec.title, SAFETY_YELLOW, scale);
                ui.add_space(height_pct(scale, 0.010416667));
                let controls = panel_spec.controls;
                let knob_count = controls.len();
                let cols = if knob_count <= 3 {
                    knob_count
                } else if knob_count <= 5 {
                    2
                } else {
                    3
                };
                let knob_width = width_pct(scale, 0.053703703);
                for row in controls.chunks(cols) {
                    let available = ui.available_width();
                    let total_knob_width = row.len() as f32 * knob_width;
                    let gap = if row.len() > 0 {
                        ((available - total_knob_width) / (row.len() + 1) as f32).max(0.0)
                    } else {
                        0.0
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(gap);
                        for spec in row.iter() {
                            industrial_knob(
                                ui,
                                spec.label,
                                exciter_param_ref(params, spec.id),
                                spec.min,
                                spec.max,
                                setter,
                                SAFETY_YELLOW,
                                scale,
                                true,
                            );
                            ui.add_space(gap);
                        }
                    });
                    ui.add_space(height_pct(scale, 0.0078125));
                }
            });
        }

        ui.add_space(height_pct(scale, 0.0052083335));

        // Envelope (variant per family)
        render_envelope(ui, params, setter, exciter.family(), scale);

        ui.add_space(height_pct(scale, 0.0052083335));

        // Gesture modifiers
        let recessed = recessed_panel(scale);
        recessed.show(ui, |ui| {
            section_heading(ui, "Gesture Modifiers", SAFETY_YELLOW, scale);
            industrial_fader(
                ui,
                "Env Amount",
                &params.env_amount,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Vel To Peak",
                &params.velocity_to_peak,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Global Time",
                &params.global_time_scale,
                0.1,
                10.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Vel To Level",
                &params.velocity_to_level,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Vel To Time",
                &params.velocity_to_time,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Curve",
                &params.curve_tension,
                -1.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
        });
    });
}

fn render_envelope(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    family: ExciterFamily,
    scale: f32,
) {
    let recessed = recessed_panel(scale);
    recessed.show(ui, |ui| match family {
        ExciterFamily::Hit => {
            section_heading(ui, "Force Envelope (AD)", SAFETY_YELLOW, scale);
            ui.label(
                egui::RichText::new("One-shot impact envelope")
                    .size(width_pct(scale, 0.0074074073))
                    .color(MUTED_GREY),
            );
            ui.add_space(height_pct(scale, 0.0052083335));
            let fader_w = width_pct(scale, 0.055555556);
            let gap = width_pct(scale, 0.018518519);
            let avail = ui.available_width();
            let total = 2.0 * fader_w + gap;
            let offset = ((avail - total) / 2.0).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(offset);
                industrial_vfader(
                    ui,
                    "Attack",
                    &params.env_attack,
                    0.001,
                    2.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Release",
                    &params.env_release,
                    0.01,
                    5.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
            });
        }
        ExciterFamily::Friction => {
            section_heading(ui, "Force Envelope (MSEG)", SAFETY_YELLOW, scale);
            ui.label(
                egui::RichText::new("6-stage gesture envelope")
                    .size(width_pct(scale, 0.0074074073))
                    .color(MUTED_GREY),
            );
            ui.add_space(height_pct(scale, 0.0052083335));
            // Four faders on the first row (Onset Time and Onset Level side
            // by side, `m01-m04-m05-clarify-parameters`), three on the
            // second, at the ADSR panel's spacing.
            let fader_w = width_pct(scale, 0.055555556);
            let gap = width_pct(scale, 0.0074074073);
            let avail = ui.available_width();
            let offset_for =
                |faders: f32| ((avail - (faders * fader_w + (faders - 1.0) * gap)) / 2.0).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(offset_for(4.0));
                industrial_vfader(
                    ui,
                    "Onset",
                    &params.mseg_onset,
                    0.001,
                    1.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Onset Lvl",
                    &params.mseg_onset_level,
                    0.0,
                    1.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Attack",
                    &params.mseg_attack,
                    0.001,
                    2.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Hold",
                    &params.mseg_hold,
                    0.0,
                    2.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
            });
            ui.horizontal(|ui| {
                ui.add_space(offset_for(3.0));
                industrial_vfader(
                    ui,
                    "Decay",
                    &params.mseg_decay,
                    0.01,
                    5.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Sustain",
                    &params.mseg_sustain,
                    0.0,
                    1.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Release",
                    &params.mseg_release,
                    0.01,
                    5.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
            });
            ui.add_space(height_pct(scale, 0.0052083335));
            industrial_combo(
                ui,
                "loop-mode",
                "Loop",
                &params.loop_mode,
                &[(0, "Off"), (1, "Forward"), (2, "Ping-Pong")],
                setter,
                SAFETY_YELLOW,
                scale,
            );
            ui.horizontal(|ui| {
                let combo_width = ui.available_width() / 2.0;
                ui.allocate_ui_with_layout(
                    egui::vec2(combo_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        industrial_combo(
                            ui,
                            "loop-start",
                            "Start",
                            &params.loop_start_stage,
                            &[(0, "0"), (1, "1"), (2, "2"), (3, "3"), (4, "4"), (5, "5")],
                            setter,
                            SAFETY_YELLOW,
                            scale,
                        );
                    },
                );
                ui.allocate_ui_with_layout(
                    egui::vec2(combo_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        industrial_combo(
                            ui,
                            "loop-end",
                            "End",
                            &params.loop_end_stage,
                            &[(0, "0"), (1, "1"), (2, "2"), (3, "3"), (4, "4"), (5, "5")],
                            setter,
                            SAFETY_YELLOW,
                            scale,
                        );
                    },
                );
            });
        }
        ExciterFamily::Specialty => {
            section_heading(ui, "Force Envelope (ADSR)", SAFETY_YELLOW, scale);
            ui.label(
                egui::RichText::new("ADSR force shaping")
                    .size(width_pct(scale, 0.0074074073))
                    .color(MUTED_GREY),
            );
            ui.add_space(height_pct(scale, 0.0052083335));
            let fader_w = width_pct(scale, 0.055555556);
            let gap = width_pct(scale, 0.0074074073);
            let avail = ui.available_width();
            let total = 4.0 * fader_w + 3.0 * gap;
            let offset = ((avail - total) / 2.0).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(offset);
                industrial_vfader(
                    ui,
                    "Attack",
                    &params.env_attack,
                    0.001,
                    2.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Decay",
                    &params.env_decay,
                    0.01,
                    5.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Sustain",
                    &params.env_sustain,
                    0.0,
                    1.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
                ui.add_space(gap);
                industrial_vfader(
                    ui,
                    "Release",
                    &params.env_release,
                    0.01,
                    5.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                );
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Resonator column
// ---------------------------------------------------------------------------

fn render_resonator_column(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    scale: f32,
) {
    let panel = metal_panel(scale);
    panel.show(ui, |ui| {
        // Column header with accent border
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("RESONATOR")
                    .size(width_pct(scale, 0.014814815))
                    .color(OFF_WHITE)
                    .strong(),
            );
        });
        let rect = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [rect.left_top(), egui::pos2(rect.right(), rect.left_top().y)],
            egui::Stroke::new(width_pct(scale, 0.0018518518), RUST_ORANGE),
        );
        ui.add_space(height_pct(scale, 0.0052083335));

        // Model selector
        let object = Object::from_int(params.object.value());
        let object_items: Vec<(i32, &'static str)> = (0..=8)
            .map(|value| (value, Object::from_int(value).name()))
            .collect();
        industrial_combo(
            ui,
            "resonator-model",
            "Material",
            &params.object,
            &object_items,
            setter,
            RUST_ORANGE,
            scale,
        );

        // Description
        let panel_spec = resonator_panel(object);
        ui.label(
            egui::RichText::new(panel_spec.description)
                .size(width_pct(scale, 0.0074074073))
                .color(MUTED_GREY),
        );
        ui.add_space(height_pct(scale, 0.0052083335));

        // Resonator controls (knobs)
        if !panel_spec.controls.is_empty() {
            let recessed = recessed_panel(scale);
            recessed.show(ui, |ui| {
                section_heading(ui, panel_spec.title, RUST_ORANGE, scale);
                ui.add_space(width_pct(scale, 0.0074074073));
                let controls = panel_spec.controls;
                let knob_count = controls.len();
                let cols = if knob_count <= 3 {
                    knob_count
                } else {
                    (knob_count as f32).sqrt().ceil() as usize
                };
                let knob_width = width_pct(scale, 0.053703703);
                for row in controls.chunks(cols) {
                    let available = ui.available_width();
                    let total_knob_width = row.len() as f32 * knob_width;
                    let gap = if row.len() > 0 {
                        ((available - total_knob_width) / (row.len() + 1) as f32).max(0.0)
                    } else {
                        0.0
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(gap);
                        for spec in row.iter() {
                            industrial_knob(
                                ui,
                                spec.label,
                                resonator_param_ref(params, spec.id),
                                spec.min,
                                spec.max,
                                setter,
                                RUST_ORANGE,
                                scale,
                                true,
                            );
                            ui.add_space(gap);
                        }
                    });
                    ui.add_space(height_pct(scale, 0.0078125));
                }
            });
        }

        // Brightness tilts every object's mode gains, so it sits with the
        // object's own controls rather than in one object's panel
        // (`h05-expose-missing-gui-controls`).
        industrial_fader(
            ui,
            "Brightness",
            &params.res_brightness,
            0.0,
            1.0,
            setter,
            OFF_WHITE,
            scale,
        );

        ui.add_space(height_pct(scale, 0.0052083335));

        // Environmental Exposure (rust/damage/heat/sludge/thickness)
        let recessed = recessed_panel(scale);
        recessed.show(ui, |ui| {
            section_heading(ui, "Environmental Exposure", DANGER_RED, scale);
            industrial_fader(
                ui,
                "Rust",
                &params.rust,
                0.0,
                5.0,
                setter,
                RUST_ORANGE,
                scale,
            );
            industrial_fader(
                ui,
                "Damage",
                &params.damage,
                0.0,
                10.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Heat",
                &params.heat,
                0.0,
                1.0,
                setter,
                DANGER_RED,
                scale,
            );
            industrial_fader(
                ui,
                "Sludge",
                &params.sludge,
                0.0,
                1.0,
                setter,
                SAFETY_YELLOW,
                scale,
            );
            industrial_fader(
                ui,
                "Thickness",
                &params.thickness,
                0.0,
                1.0,
                setter,
                MUTED_GREY,
                scale,
            );
        });

        ui.add_space(height_pct(scale, 0.0052083335));

        // Mechanical Linkage
        let recessed = recessed_panel(scale);
        recessed.show(ui, |ui| {
            section_heading(ui, "Mechanical Linkage", OFF_WHITE, scale);
            industrial_fader(
                ui,
                "Strike Pos",
                &params.strike_position,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Coupling",
                &params.coupling_stiffness,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Pos Wander",
                &params.position_wander,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader_text(
                ui,
                "Wander Rate",
                &params.position_wander_rate,
                0.1,
                5.0,
                setter,
                OFF_WHITE,
                scale,
                FaderText::ParamRange,
            );
            industrial_fader(
                ui,
                "Pos Env",
                &params.position_envelope,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Anchor",
                &params.fundamental_anchor,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
        });
    });
}

// ---------------------------------------------------------------------------
// Processing column
// ---------------------------------------------------------------------------

fn render_processing_column(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    scale: f32,
    dot_colour: egui::Color32,
) {
    let panel = metal_panel(scale);
    panel.show(ui, |ui| {
        // Column header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("PROCESSING")
                    .size(width_pct(scale, 0.014814815))
                    .color(OFF_WHITE)
                    .strong(),
            );
        });
        let rect = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [rect.left_top(), egui::pos2(rect.right(), rect.left_top().y)],
            egui::Stroke::new(width_pct(scale, 0.0018518518), CONCRETE),
        );
        ui.add_space(height_pct(scale, 0.0052083335));

        // Filter Bank (knobs)
        let recessed = recessed_panel(scale);
        recessed.show(ui, |ui| {
            section_heading(ui, "Filter Bank", OFF_WHITE, scale);
            ui.add_space(width_pct(scale, 0.0074074073));
            let large_knob_w = width_pct(scale, 0.053703703);
            let cutoff_offset = ((ui.available_width() - large_knob_w) / 2.0).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(cutoff_offset);
                industrial_knob(
                    ui,
                    "Cutoff",
                    &params.filter_cutoff,
                    20.0,
                    20000.0,
                    setter,
                    DANGER_RED,
                    scale,
                    true,
                );
            });
            ui.add_space(height_pct(scale, 0.0052083335));
            let small_knob_w = width_pct(scale, 0.044444445);
            let pair_w = 2.0 * small_knob_w + width_pct(scale, 0.022222223);
            let pair_offset = ((ui.available_width() - pair_w) / 2.0).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(pair_offset);
                industrial_knob(
                    ui,
                    "Resonance",
                    &params.filter_resonance,
                    0.0,
                    1.0,
                    setter,
                    OFF_WHITE,
                    scale,
                    false,
                );
                ui.add_space(width_pct(scale, 0.022222223));
                industrial_knob(
                    ui,
                    "Tolerance",
                    &params.component_tolerance,
                    0.0,
                    1.0,
                    setter,
                    OFF_WHITE,
                    scale,
                    false,
                );
            });
        });

        ui.add_space(height_pct(scale, 0.0052083335));

        // Overdrive Unit
        let recessed = recessed_panel(scale);
        recessed.show(ui, |ui| {
            section_heading(ui, "Overdrive Unit", OFF_WHITE, scale);
            industrial_fader(
                ui,
                "Drive Amt",
                &params.drive_amount,
                0.0,
                5.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Bias Starve",
                &params.bias_starvation,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Chaos",
                &params.chaos_depth,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Legacy Drive",
                &params.drive,
                0.0,
                5.0,
                setter,
                OFF_WHITE,
                scale,
            );
        });

        ui.add_space(height_pct(scale, 0.0052083335));

        // Dispersion (Body and spread)
        let recessed = recessed_panel(scale);
        recessed.show(ui, |ui| {
            section_heading(ui, "Dispersion", OFF_WHITE, scale);
            industrial_fader(
                ui,
                "Chassis Mat",
                &params.chassis_material,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Chassis Vol",
                &params.chassis_volume,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Spread",
                &params.spread_width,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Proximity",
                &params.listener_proximity,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
        });

        ui.add_space(height_pct(scale, 0.0052083335));

        // Space
        let recessed = recessed_panel(scale);
        recessed.show(ui, |ui| {
            section_heading(ui, "Space", OFF_WHITE, scale);
            industrial_combo(
                ui,
                "space-mode",
                "Mode",
                &params.space_mode,
                &[(0, "Off"), (1, "Factory"), (2, "Spring"), (3, "Echo")],
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Amount",
                &params.space_amount,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            match params.space_mode.value() {
                1 => {
                    industrial_fader(
                        ui,
                        "Factory Size",
                        &params.factory_size,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                    industrial_fader(
                        ui,
                        "Machinery",
                        &params.machinery_clutter,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                    industrial_fader(
                        ui,
                        "Wall Impedance",
                        &params.wall_impedance,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                }
                2 => {
                    industrial_fader(
                        ui,
                        "Spring Tension",
                        &params.spring_tension,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                    industrial_fader(
                        ui,
                        "Wire Stiffness",
                        &params.wire_stiffness,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                    industrial_fader(
                        ui,
                        "Tank Size",
                        &params.spring_tank_size,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                }
                3 => {
                    // Sync names its division, or "Off" while Delay Time
                    // runs free (`h03-default-tempo-sync-off`).
                    industrial_fader_text(
                        ui,
                        "Sync",
                        &params.sync_rate,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                        FaderText::Param,
                    );
                    industrial_fader(
                        ui,
                        "Delay Time",
                        &params.delay_time,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                    industrial_fader(
                        ui,
                        "Movement",
                        &params.machinery_movement,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                    industrial_fader(
                        ui,
                        "HF Damping",
                        &params.high_frequency_damping,
                        0.0,
                        1.0,
                        setter,
                        OFF_WHITE,
                        scale,
                    );
                }
                _ => {}
            }
        });

        ui.add_space(height_pct(scale, 0.0052083335));

        // Brickwall Limiter (danger-red bordered)
        let limiter_frame = egui::Frame::new()
            .fill(RECESSED_BG)
            .inner_margin(egui::Margin::same((height_pct(scale, 0.0078125)) as i8))
            .stroke(egui::Stroke::new(
                width_pct(scale, 0.0018518518),
                DANGER_RED,
            ));
        limiter_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("BRICKWALL LIMITER")
                        .size(width_pct(scale, 0.009259259))
                        .color(DANGER_RED)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (dot_rect, _) = ui.allocate_exact_size(
                        egui::vec2(
                            width_pct(scale, 0.0074074073),
                            width_pct(scale, 0.0074074073),
                        ),
                        egui::Sense::hover(),
                    );
                    note_diode_dot(dot_colour);
                    ui.painter().circle_filled(
                        dot_rect.center(),
                        height_pct(scale, 0.0052083335),
                        dot_colour,
                    );
                });
            });
            let rect = ui.available_rect_before_wrap();
            ui.painter().line_segment(
                [rect.left_top(), egui::pos2(rect.right(), rect.left_top().y)],
                egui::Stroke::new(1.0, DANGER_RED),
            );
            ui.add_space(width_pct(scale, 0.0018518518));
            industrial_fader(
                ui,
                "Ceiling",
                &params.analog_ceiling,
                0.5,
                1.0,
                setter,
                DANGER_RED,
                scale,
            );
            industrial_fader(
                ui,
                "Softness",
                &params.diode_softness,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
            industrial_fader(
                ui,
                "Asymmetry",
                &params.diode_asymmetry,
                0.0,
                1.0,
                setter,
                OFF_WHITE,
                scale,
            );
        });
    });
}

/// The four macros, each a global bias across a cluster of destinations
/// (neutral at 0.5), in one strip above the footer
/// (`h05-expose-missing-gui-controls`).
fn render_macro_strip(
    ui: &mut egui::Ui,
    params: &CorrosionParams,
    setter: &ParamSetter,
    scale: f32,
) {
    let macros: [(&str, &FloatParam); 4] = [
        ("MASS", &params.macro_mass),
        ("CORROSION", &params.macro_corrosion),
        ("VIOLENCE", &params.macro_violence),
        ("BRIGHTNESS", &params.macro_brightness),
    ];
    recessed_panel(scale).show(ui, |ui| {
        section_heading(ui, "Macros", SAFETY_YELLOW, scale);
        let knob_width = width_pct(scale, 0.053703703);
        let gap = ((ui.available_width() - knob_width * macros.len() as f32)
            / (macros.len() + 1) as f32)
            .max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(gap);
            for (label, param) in macros {
                industrial_knob(
                    ui,
                    label,
                    param,
                    0.0,
                    1.0,
                    setter,
                    SAFETY_YELLOW,
                    scale,
                    false,
                );
                ui.add_space(gap);
            }
        });
    });
}

fn render_footer(ui: &mut egui::Ui, scale: f32) {
    ui.add_space(height_pct(scale, 0.0052083335));
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    // Top border
    painter.line_segment(
        [
            egui::pos2(rect.min.x, rect.min.y),
            egui::pos2(rect.max.x, rect.min.y),
        ],
        egui::Stroke::new(width_pct(scale, 0.0018518518), CONCRETE),
    );
    // Background
    let footer_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x, rect.min.y + width_pct(scale, 0.018518519)),
    );
    painter.rect_filled(footer_rect, 0.0, CHARCOAL);
    // Text
    painter.text(
        egui::pos2(
            footer_rect.min.x + width_pct(scale, 0.0074074073),
            footer_rect.center().y,
        ),
        egui::Align2::LEFT_CENTER,
        "CORROSION ENGINE v0.1.0",
        egui::FontId::proportional(width_pct(scale, 0.0074074073)),
        MUTED_GREY,
    );
    painter.text(
        egui::pos2(
            footer_rect.max.x - width_pct(scale, 0.0074074073),
            footer_rect.center().y,
        ),
        egui::Align2::RIGHT_CENTER,
        "Industrial Physical Modeling",
        egui::FontId::proportional(width_pct(scale, 0.0074074073)),
        MUTED_GREY,
    );
    // Claim the footer's space so the keyboard strip lands below it.
    ui.allocate_rect(footer_rect, egui::Sense::hover());
}

// ---------------------------------------------------------------------------
// Keyboard strip (`add-editor-keyboard`)
// ---------------------------------------------------------------------------

/// The strip's height at 100 % UI scale, and the gap above it.
const KEYBOARD_STRIP_BASE_HEIGHT: f32 = 64.0;
const KEYBOARD_STRIP_BASE_GAP: f32 = 8.0;
/// Black keys' width and length, relative to a white key.
const BLACK_KEY_WIDTH: f32 = 0.6;
const BLACK_KEY_LENGTH: f32 = 0.62;
/// Mouse velocity at the top of a key (it reaches 1.0 at the bottom).
const KEY_TOP_VELOCITY: f32 = 0.25;

/// The keyboard strip's editor-side state.
struct KeyboardStrip {
    sender: crate::gui_notes::NoteSender,
    /// The note the mouse holds, if any.
    mouse_note: Option<u8>,
    /// Whether the strip was live last frame: window focused and the
    /// pointer over the editor.
    focused: bool,
}

impl KeyboardStrip {
    fn new(shared: Arc<crate::gui_notes::EditorNotes>) -> Self {
        Self {
            sender: crate::gui_notes::NoteSender::new(shared),
            mouse_note: None,
            focused: true,
        }
    }

    /// The window opened again: the audio thread released the editor's
    /// notes when it closed.
    fn reopened(&mut self) {
        self.sender.forget_held();
        self.mouse_note = None;
        self.focused = true;
    }

    /// Release everything the strip holds (focus lost).
    fn release_all(&mut self) {
        self.sender.release_all_held();
        self.mouse_note = None;
    }
}

/// The strip's keys inside `rect`: white keys first, then black keys, so
/// [`key_at`] can test the black keys (drawn on top) first.
fn strip_keys(rect: egui::Rect) -> Vec<(u8, egui::Rect)> {
    use crate::gui_notes::{is_black_key, STRIP_HIGH_NOTE, STRIP_LOW_NOTE};
    let notes = STRIP_LOW_NOTE..=STRIP_HIGH_NOTE;
    let whites = notes.clone().filter(|&n| !is_black_key(n)).count();
    let white_width = rect.width() / whites as f32;
    let mut keys = Vec::with_capacity(notes.clone().count());
    let mut blacks = Vec::new();
    let mut white_index = 0;
    for note in notes {
        if is_black_key(note) {
            let boundary = rect.min.x + white_index as f32 * white_width;
            let half = white_width * BLACK_KEY_WIDTH * 0.5;
            blacks.push((
                note,
                egui::Rect::from_min_max(
                    egui::pos2(boundary - half, rect.min.y),
                    egui::pos2(
                        boundary + half,
                        rect.min.y + rect.height() * BLACK_KEY_LENGTH,
                    ),
                ),
            ));
        } else {
            let left = rect.min.x + white_index as f32 * white_width;
            keys.push((
                note,
                egui::Rect::from_min_max(
                    egui::pos2(left, rect.min.y),
                    egui::pos2(left + white_width, rect.max.y),
                ),
            ));
            white_index += 1;
        }
    }
    keys.extend(blacks);
    keys
}

/// The note under `pos`, black keys first.
fn key_at(keys: &[(u8, egui::Rect)], pos: egui::Pos2) -> Option<u8> {
    keys.iter()
        .rev()
        .find(|(_, rect)| rect.contains(pos))
        .map(|&(note, _)| note)
}

/// Mouse velocity for a press at height `y` on a key spanning `key`.
fn press_velocity(key: egui::Rect, y: f32) -> u8 {
    let depth = ((y - key.min.y) / key.height().max(1.0)).clamp(0.0, 1.0);
    let velocity = KEY_TOP_VELOCITY + (1.0 - KEY_TOP_VELOCITY) * depth;
    (velocity * 127.0).round().clamp(1.0, 127.0) as u8
}

/// The letter of a key event's key: the physical key when the backend
/// gives one, else the logical key (egui-baseview gives no physical key).
fn key_letter(key: egui::Key, physical_key: Option<egui::Key>) -> Option<char> {
    let name = physical_key.unwrap_or(key).name();
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(letter), None) if letter.is_ascii_alphabetic() => Some(letter),
        _ => None,
    }
}

/// The mapped-key actions in one frame's events, as (letter, pressed,
/// repeat). X11 auto-repeat arrives as a key-up immediately followed by a
/// key-down of the same key, which baseview doesn't flag: such a pair
/// becomes one repeat, so a held key doesn't retrigger. A person can't
/// release and press a key again within one frame.
fn key_actions(events: &[egui::Event]) -> Vec<(char, bool, bool)> {
    let mut actions: Vec<(char, bool, bool)> = Vec::new();
    for event in events {
        if let egui::Event::Key {
            key,
            physical_key,
            pressed,
            repeat,
            ..
        } = *event
        {
            let Some(letter) = key_letter(key, physical_key) else {
                continue;
            };
            if pressed && actions.last() == Some(&(letter, false, false)) {
                actions.pop();
                actions.push((letter, true, true));
            } else {
                actions.push((letter, pressed, repeat));
            }
        }
    }
    actions
}

/// The keyboard strip: mouse, computer keys and the held-note highlights.
fn render_keyboard_strip(ui: &mut egui::Ui, strip: &mut KeyboardStrip, scale: f32) {
    use crate::gui_notes::{note_letter, NoteSource};
    let top = ui.cursor().min.y;
    ui.add_space(KEYBOARD_STRIP_BASE_GAP * scale);
    let size = egui::vec2(ui.available_width(), KEYBOARD_STRIP_BASE_HEIGHT * scale);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    ui.painter().rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.min.x, top), rect.max),
        0.0,
        DARK_STEEL,
    );
    let keys = strip_keys(rect);
    #[cfg(test)]
    DRAWN_KEYBOARD.with(|drawn| *drawn.borrow_mut() = Some((rect, keys.clone())));

    // Computer keys, only while the editor has focus, the pointer is over
    // it, and no text field takes the typing. baseview reports focus loss
    // only on macOS; on Windows and X11 a key held while focus moves away
    // would never see its key-up, so the pointer leaving the window (which
    // baseview reports everywhere) releases the strip's notes too.
    let ctx = ui.ctx().clone();
    let (events, focused, has_pointer) = ctx.input(|input| {
        (
            input.events.clone(),
            input.focused,
            input.pointer.has_pointer(),
        )
    });
    let lost_focus = !focused
        || !has_pointer
        || events.iter().any(|event| {
            matches!(
                event,
                egui::Event::WindowFocused(false) | egui::Event::PointerGone
            )
        });
    if lost_focus && strip.focused {
        strip.release_all();
        // egui marks a press of a key it still thinks is down as a repeat,
        // and the key-up may have gone to another window: forget the mapped
        // keys so the next press plays.
        ctx.input_mut(|input| {
            input.keys_down.retain(|&key| {
                key_letter(key, None)
                    .and_then(crate::gui_notes::key_note)
                    .is_none()
            });
        });
    }
    strip.focused = !lost_focus;
    let gate_open = strip.focused && !ctx.wants_keyboard_input();
    for (letter, pressed, repeat) in key_actions(&events) {
        strip.sender.key_event(letter, pressed, repeat, gate_open);
    }

    // Mouse: the key under the pointer while the button is down on the
    // strip; moving onto another key is a glissando.
    let target = if response.is_pointer_button_down_on() && strip.focused {
        ctx.input(|input| input.pointer.interact_pos())
            .and_then(|pos| key_at(&keys, pos).map(|note| (note, pos)))
    } else {
        None
    };
    if target.map(|(note, _)| note) != strip.mouse_note {
        if let Some(previous) = strip.mouse_note.take() {
            strip.sender.release(previous, NoteSource::Mouse);
        }
        if let Some((note, pos)) = target {
            let key_rect = keys
                .iter()
                .find(|(n, _)| *n == note)
                .map_or(rect, |&(_, r)| r);
            strip
                .sender
                .press(note, press_velocity(key_rect, pos.y), NoteSource::Mouse);
            strip.mouse_note = Some(note);
        }
    }
    strip.sender.flush();

    // Paint: white keys, then black keys over them.
    let held = strip.sender.shared().held();
    let painter = ui.painter();
    let outline = egui::Stroke::new((scale).max(0.5), RECESSED_BG);
    let font = egui::FontId::proportional(width_pct(scale, 0.0083333));
    let mut lit = false;
    for &(note, key_rect) in &keys {
        let black = crate::gui_notes::is_black_key(note);
        let on = strip.sender.holds_any(note) || held.contains(note);
        lit |= on;
        let fill = match (black, on) {
            (false, false) => OFF_WHITE,
            (false, true) => DANGER_RED.linear_multiply(0.75),
            (true, false) => RECESSED_BG,
            (true, true) => DANGER_RED,
        };
        let key_rect = if black {
            key_rect
        } else {
            key_rect.shrink2(egui::vec2(0.5 * scale, 0.0))
        };
        painter.rect_filled(key_rect, 1.0 * scale, fill);
        painter.rect_stroke(key_rect, 1.0 * scale, outline, egui::StrokeKind::Inside);
        if let Some(letter) = note_letter(note) {
            painter.text(
                egui::pos2(key_rect.center().x, key_rect.max.y - 3.0 * scale),
                egui::Align2::CENTER_BOTTOM,
                letter.to_string(),
                font.clone(),
                if black {
                    OFF_WHITE.linear_multiply(0.6)
                } else {
                    MUTED_GREY
                },
            );
        }
    }
    // Keep repainting while anything is held or lit, so the highlight
    // follows notes that end on the audio side.
    if lit || strip.mouse_note.is_some() {
        ctx.request_repaint();
    } else {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

/// The strip's rect and its keys, as drawn.
#[cfg(test)]
type DrawnKeyboard = (egui::Rect, Vec<(u8, egui::Rect)>);

#[cfg(test)]
thread_local! {
    /// The strip's rect and keys as last drawn, for the layout tests.
    static DRAWN_KEYBOARD: std::cell::RefCell<Option<DrawnKeyboard>> =
        const { std::cell::RefCell::new(None) };
}

// ---------------------------------------------------------------------------
// Panel spec functions (unchanged)
// ---------------------------------------------------------------------------

fn exciter_panel(exciter: ExciterType) -> ExciterPanelSpec {
    match exciter {
        ExciterType::Bow => ExciterPanelSpec {
            title: "Bow",
            description: "Smooth stick-slip bow friction with pressure, speed, grip, and slip controls from the documented bow model.",
            controls: BOW_CONTROLS,
        },
        ExciterType::HandStrike => ExciterPanelSpec {
            title: "Hand strike physics",
            description: "Fleshy Kelvin-Voigt impact with heavy damping and an explicit mute tail.",
            controls: HAND_STRIKE_CONTROLS,
        },
        ExciterType::FeltMallet => ExciterPanelSpec {
            title: "Felt mallet contact",
            description: "Soft non-linear mallet with a compressive felt layer over a harder core.",
            controls: FELT_MALLET_CONTROLS,
        },
        ExciterType::HardMallet => ExciterPanelSpec {
            title: "Hard mallet contact",
            description: "Rigid Hertzian strike tuned around mass, brightness, and bounce suppression.",
            controls: HARD_MALLET_CONTROLS,
        },
        ExciterType::Drumstick => ExciterPanelSpec {
            title: "Drumstick bounce",
            description: "Light rigid impact with explicit rebound energy and micro-bounce count.",
            controls: DRUMSTICK_CONTROLS,
        },
        ExciterType::WireBrush => ExciterPanelSpec {
            title: "Wire brush cluster",
            description: "Impulse-cloud brush model with density, spread, stiffness, and stochastic variance.",
            controls: WIRE_BRUSH_CONTROLS,
        },
        ExciterType::MetalPipe => ExciterPanelSpec {
            title: "Metal pipe coupling",
            description: "Bi-directional pipe impact with a ringing exciter body feeding back into the collision.",
            controls: METAL_PIPE_CONTROLS,
        },
        ExciterType::MetalChain => ExciterPanelSpec {
            title: "Metal chain cascade",
            description: "Clustered heavy-link impacts with separate timing spread and high-frequency rattle coloration.",
            controls: METAL_CHAIN_CONTROLS,
        },
        ExciterType::StiffPoint => ExciterPanelSpec {
            title: "Stiff point chatter",
            description: "Rigid point scrape where chatter pitch and damping define the squeal character.",
            controls: STIFF_POINT_CONTROLS,
        },
        ExciterType::HeavyGrinding => ExciterPanelSpec {
            title: "Grinding friction",
            description: "Heavy rough drag with separate baseline force, tearing grit, and noise color.",
            controls: HEAVY_GRINDING_CONTROLS,
        },
        ExciterType::CorrugatedDrag => ExciterPanelSpec {
            title: "Corrugated drag topology",
            description: "Drag motion across ridges with explicit spacing, depth, and moving contact mass.",
            controls: CORRUGATED_DRAG_CONTROLS,
        },
        ExciterType::TensionRise => ExciterPanelSpec {
            title: "Tension-rise slip",
            description: "Integrate-and-fire creak model driven by pull speed, threshold, randomness, and crack sharpness.",
            controls: TENSION_RISE_CONTROLS,
        },
        ExciterType::PneumaticJet => ExciterPanelSpec {
            title: "Pneumatic jet drive",
            description: "Band-limited turbulence exciter shaped by nozzle focus and overload chaos.",
            controls: PNEUMATIC_JET_CONTROLS,
        },
        ExciterType::ElectromagneticHum => ExciterPanelSpec {
            title: "Electromagnetic drive",
            description: "Continuous Lorentz-force injection with direct control of mains pitch, proximity, and sag.",
            controls: ELECTROMAGNETIC_HUM_CONTROLS,
        },
        ExciterType::TensionSnap => ExciterPanelSpec {
            title: "Tension snap release",
            description: "Hook-and-release mechanism with explicit pull distance, stiffness, and break force.",
            controls: TENSION_SNAP_CONTROLS,
        },
        ExciterType::ParticleRain => ExciterPanelSpec {
            title: "Particle rain stream",
            description: "Continuous debris cloud controlled by flow density, particle weight, and variance.",
            controls: PARTICLE_RAIN_CONTROLS,
        },
    }
}

fn resonator_panel(object: Object) -> ResonatorPanelSpec {
    match object {
        Object::Pipe => ResonatorPanelSpec {
            title: "Pipe geometry",
            description: "Longitudinal pipe response with dedicated length, diameter, and sustain controls.",
            controls: PIPE_RESONATOR_CONTROLS,
        },
        Object::Plate => ResonatorPanelSpec {
            title: "Plate surface",
            description: "2D plate response balancing overall span, aspect ratio, and material stiffness.",
            controls: PLATE_RESONATOR_CONTROLS,
        },
        Object::Tank => ResonatorPanelSpec {
            title: "Tank cavity",
            description: "Shell-and-cavity behavior with volume, wall thickness, cavity balance, and shell decay.",
            controls: TANK_RESONATOR_CONTROLS,
        },
        Object::Chain => ResonatorPanelSpec {
            title: "Chain instability",
            description: "Linked resonator behavior shaped by link weight, chain length, instability, and friction loss.",
            controls: CHAIN_RESONATOR_CONTROLS,
        },
        Object::IBeam => ResonatorPanelSpec {
            title: "I-beam structure",
            description: "Beam-like modes with explicit mass, shear density, and rigidity damping emphasis.",
            controls: IBEAM_RESONATOR_CONTROLS,
        },
        Object::TautCable => ResonatorPanelSpec {
            title: "Cable tension",
            description: "Stiff string response with tension, braid stiffness, and dynamic pitch-drop control.",
            controls: TAUT_CABLE_RESONATOR_CONTROLS,
        },
        Object::CoilSpring => ResonatorPanelSpec {
            title: "Coil spring dispersion",
            description: "Dispersive spring response controlled by coil length, chirp density, and slosh instability.",
            controls: COIL_SPRING_RESONATOR_CONTROLS,
        },
        Object::SheetMetal => ResonatorPanelSpec {
            title: "Sheet metal buckling",
            description: "Thin-sheet response with size, thinness, and edge damping rather than generic brightness knobs.",
            controls: SHEET_METAL_RESONATOR_CONTROLS,
        },
        Object::IndustrialCog => ResonatorPanelSpec {
            title: "Cog and blade split",
            description: "Circular blade behavior with radius, tooth dissonance, and blade thickness control.",
            controls: INDUSTRIAL_COG_RESONATOR_CONTROLS,
        },
    }
}

// ---------------------------------------------------------------------------
// Param ref functions (unchanged)
// ---------------------------------------------------------------------------

fn exciter_param_ref(params: &CorrosionParams, id: ExciterControlId) -> &FloatParam {
    match id {
        ExciterControlId::HandMass => &params.hand_mass,
        ExciterControlId::FleshStiffness => &params.flesh_stiffness,
        ExciterControlId::FleshDamping => &params.flesh_damping,
        ExciterControlId::MuteDecay => &params.mute_decay,
        ExciterControlId::MalletMass => &params.mallet_mass,
        ExciterControlId::FeltSoftness => &params.felt_softness,
        ExciterControlId::CoreHardness => &params.core_hardness,
        ExciterControlId::CompressionCurve => &params.compression_curve,
        ExciterControlId::MaterialStiffness => &params.material_stiffness,
        ExciterControlId::ImpactDamping => &params.impact_damping,
        ExciterControlId::StickMass => &params.stick_mass,
        ExciterControlId::TipStiffness => &params.tip_stiffness,
        ExciterControlId::RestitutionBounciness => &params.restitution_bounciness,
        ExciterControlId::MicroBounceLimit => &params.micro_bounce_limit,
        ExciterControlId::WireDensity => &params.wire_density,
        ExciterControlId::SpreadDuration => &params.spread_duration,
        ExciterControlId::BrushWireStiffness => &params.brush_wire_stiffness,
        ExciterControlId::AmplitudeRandomization => &params.amplitude_randomization,
        ExciterControlId::PipeMass => &params.pipe_mass,
        ExciterControlId::MetalStiffness => &params.metal_stiffness,
        ExciterControlId::PipePitch => &params.pipe_pitch,
        ExciterControlId::PipeRingDecay => &params.pipe_ring_decay,
        ExciterControlId::LinkCount => &params.link_count,
        ExciterControlId::ChainMass => &params.chain_mass,
        ExciterControlId::DropEnvelopeSpread => &params.drop_envelope_spread,
        ExciterControlId::InternalRattle => &params.internal_rattle,
        ExciterControlId::RattleColor => &params.rattle_color,
        ExciterControlId::BowPressure => &params.bow_pressure,
        ExciterControlId::BowSpeed => &params.bow_speed,
        ExciterControlId::RosinGrip => &params.rosin_grip,
        ExciterControlId::SlipCurve => &params.slip_curve,
        ExciterControlId::ScrapeSpeed => &params.scrape_speed,
        ExciterControlId::PointPressure => &params.point_pressure,
        ExciterControlId::ChatterPitch => &params.chatter_pitch,
        ExciterControlId::ChatterDamping => &params.chatter_damping,
        ExciterControlId::GrindSpeed => &params.grind_speed,
        ExciterControlId::GrindPressure => &params.grind_pressure,
        ExciterControlId::SurfaceGrit => &params.surface_grit,
        ExciterControlId::GritColor => &params.grit_color,
        ExciterControlId::DragSpeed => &params.drag_speed,
        ExciterControlId::RidgeSpacing => &params.ridge_spacing,
        ExciterControlId::RidgeDepth => &params.ridge_depth,
        ExciterControlId::DragExciterMass => &params.drag_exciter_mass,
        ExciterControlId::PullSpeed => &params.pull_speed,
        ExciterControlId::BreakThreshold => &params.break_threshold,
        ExciterControlId::SlipStochasticity => &params.slip_stochasticity,
        ExciterControlId::CreakSharpness => &params.creak_sharpness,
        ExciterControlId::AirPressure => &params.air_pressure,
        ExciterControlId::NozzleWidth => &params.nozzle_width,
        ExciterControlId::TurbulenceChaos => &params.turbulence_chaos,
        ExciterControlId::MainsFrequency => &params.mains_frequency,
        ExciterControlId::CoilProximity => &params.coil_proximity,
        ExciterControlId::VoltageSag => &params.voltage_sag,
        ExciterControlId::PullDistance => &params.pull_distance,
        ExciterControlId::HookStiffness => &params.hook_stiffness,
        ExciterControlId::SnapForce => &params.snap_force,
        ExciterControlId::FlowRate => &params.flow_rate,
        ExciterControlId::ParticleMass => &params.particle_mass,
        ExciterControlId::MassVariance => &params.mass_variance,
    }
}

fn resonator_param_ref(params: &CorrosionParams, id: ResonatorControlId) -> &FloatParam {
    match id {
        ResonatorControlId::Size => &params.size,
        ResonatorControlId::Damping => &params.res_damping,
        ResonatorControlId::PipeDiameter => &params.pipe_diameter,
        ResonatorControlId::PlateAspect => &params.plate_aspect,
        ResonatorControlId::PlateStiffness => &params.plate_stiffness,
        ResonatorControlId::TankVolume => &params.tank_volume,
        ResonatorControlId::TankCavityMix => &params.tank_cavity_mix,
        ResonatorControlId::ChainLinkMass => &params.chain_link_mass,
        ResonatorControlId::ChainInstability => &params.chain_instability,
        ResonatorControlId::BeamShear => &params.beam_shear,
        ResonatorControlId::CableBraid => &params.cable_braid,
        ResonatorControlId::CableTensionDrop => &params.cable_tension_drop,
        ResonatorControlId::SpringDispersion => &params.spring_dispersion,
        ResonatorControlId::SpringSlosh => &params.spring_slosh,
        ResonatorControlId::SheetThinness => &params.sheet_thinness,
        ResonatorControlId::CogDissonance => &params.cog_dissonance,
    }
}

// ---------------------------------------------------------------------------
// Tests (unchanged)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// A host stand-in for headless editor renders: it accepts every
    /// parameter gesture and does nothing with it.
    struct NullGui;

    impl nih_plug::prelude::GuiContext for NullGui {
        fn plugin_api(&self) -> nih_plug::prelude::PluginApi {
            nih_plug::prelude::PluginApi::Clap
        }
        fn request_resize(&self) -> bool {
            false
        }
        unsafe fn raw_begin_set_parameter(&self, _param: nih_plug::prelude::ParamPtr) {}
        unsafe fn raw_set_parameter_normalized(
            &self,
            _param: nih_plug::prelude::ParamPtr,
            _normalized: f32,
        ) {
        }
        unsafe fn raw_end_set_parameter(&self, _param: nih_plug::prelude::ParamPtr) {}
        fn get_state(&self) -> nih_plug::prelude::PluginState {
            unimplemented!("not used by headless renders")
        }
        fn set_state(&self, _state: nih_plug::prelude::PluginState) {}
    }

    /// A host that records resize requests and accepts or refuses them.
    struct ResizeRecordingGui {
        accept: bool,
        requests: std::sync::atomic::AtomicUsize,
    }

    impl nih_plug::prelude::GuiContext for ResizeRecordingGui {
        fn plugin_api(&self) -> nih_plug::prelude::PluginApi {
            nih_plug::prelude::PluginApi::Vst3
        }
        fn request_resize(&self) -> bool {
            self.requests
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.accept
        }
        unsafe fn raw_begin_set_parameter(&self, _param: nih_plug::prelude::ParamPtr) {}
        unsafe fn raw_set_parameter_normalized(
            &self,
            _param: nih_plug::prelude::ParamPtr,
            _normalized: f32,
        ) {
        }
        unsafe fn raw_end_set_parameter(&self, _param: nih_plug::prelude::ParamPtr) {}
        fn get_state(&self) -> nih_plug::prelude::PluginState {
            unimplemented!("not used by resize tests")
        }
        fn set_state(&self, _state: nih_plug::prelude::PluginState) {}
    }

    /// Changing UI Scale asks the host to resize its window (it used to
    /// only send a viewport command, so the host window kept its size and the
    /// contents shrank inside it), and the host reads the new size from the
    /// editor state.
    #[test]
    fn ui_scale_change_requests_a_host_resize() {
        let editor_state = nih_plug_egui::EguiState::from_size(810, 630);
        let mut state = super::EditorUiState::new(1, (810, 630));
        let host = ResizeRecordingGui {
            accept: true,
            requests: std::sync::atomic::AtomicUsize::new(0),
        };

        // Same scale: nothing to do.
        assert_eq!(
            super::request_window_size(&mut state, 1, &editor_state, &host),
            None
        );
        assert_eq!(host.requests.load(std::sync::atomic::Ordering::Relaxed), 0);

        // 75 % -> 100 %: one request, the new size reported to the host.
        assert_eq!(
            super::request_window_size(&mut state, 2, &editor_state, &host),
            Some((1080, 840))
        );
        assert_eq!(host.requests.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(editor_state.size(), (1080, 840));

        // Asked once per change, not every frame.
        assert_eq!(
            super::request_window_size(&mut state, 2, &editor_state, &host),
            None
        );
        assert_eq!(host.requests.load(std::sync::atomic::Ordering::Relaxed), 1);

        // A host that refuses: the embedded window isn't resized.
        let refusing = ResizeRecordingGui {
            accept: false,
            requests: std::sync::atomic::AtomicUsize::new(0),
        };
        assert_eq!(
            super::request_window_size(&mut state, 0, &editor_state, &refusing),
            None
        );
        assert_eq!(
            refusing.requests.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    /// A new instance opens at 75 % (810 x 630, with the keyboard strip),
    /// small enough for a host window on a 1080p screen.
    #[test]
    fn editor_opens_at_75_percent_by_default() {
        let params = crate::params::CorrosionParams::default();
        assert_eq!(params.ui_scale.value(), 1);
        assert_eq!(params.editor_state.size(), (810, 630));
        assert_eq!(
            super::scaled_editor_size(super::ui_scale_factor(1)),
            (810, 630)
        );
    }

    /// Right edge of the editor's content rendered headlessly, with the
    /// editor's own style, in a window `window` points wide and tall when UI
    /// Scale asks for `requested`.
    fn rendered_right_edge(
        params: &crate::params::CorrosionParams,
        requested: f32,
        window: (f32, f32),
    ) -> f32 {
        use nih_plug_egui::egui;
        let gui = NullGui;
        let setter = nih_plug::prelude::ParamSetter::new(&gui);
        let mut state = super::EditorUiState::new(1, (810, 576));
        let scale = super::layout_scale(requested, window.0);
        let ctx = egui::Context::default();
        super::apply_editor_style(&ctx, scale);
        let mut right = 0.0;
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(window.0, window.1),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    super::render_ui(ui, params, &setter, &mut state, scale);
                    right = ui.min_rect().max.x;
                });
            });
        }
        right
    }

    /// The editor's content is never wider than its window, at every UI
    /// Scale and for every exciter, and also when the window is narrower
    /// than the scale asks for (a host that refused the resize, or a DPI
    /// mismatch). The header's mode selectors used to make it about 1.7
    /// times wider than the window, pushing the third column off-screen.
    #[test]
    fn editor_content_fits_the_window_width() {
        use crate::params::exciter_param;
        let mut failures = Vec::new();
        for exciter in 1..=16 {
            let params = crate::params::CorrosionParams {
                exciter: exciter_param(exciter),
                ..Default::default()
            };
            let mut windows: Vec<(String, f32, (f32, f32))> = (0..=4)
                .map(|ui_scale| {
                    let requested = super::ui_scale_factor(ui_scale);
                    let (w, h) = super::scaled_editor_size(requested);
                    (
                        format!("UI Scale {ui_scale}"),
                        requested,
                        (w as f32, h as f32),
                    )
                })
                .collect();
            windows.push(("100 % in a 810-wide window".into(), 1.0, (810.0, 576.0)));
            windows.push((
                "100 % at 150 % DPI (720 points)".into(),
                1.0,
                (720.0, 512.0),
            ));
            for (label, requested, window) in windows {
                let right = rendered_right_edge(&params, requested, window);
                if right > window.0 + 1.0 {
                    failures.push(format!(
                        "exciter {exciter}, {label}: content reaches {right:.0} in a {:.0}-wide window",
                        window.0
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn layout_scale_never_exceeds_the_window() {
        assert_eq!(super::layout_scale(0.75, 810.0), 0.75);
        assert_eq!(super::layout_scale(0.75, 1080.0), 0.75);
        assert!((super::layout_scale(1.0, 810.0) - 0.75).abs() < 1e-6);
        assert_eq!(super::layout_scale(1.0, 0.0), 1.0);
    }

    // --- The limiter panel's diode dot (`add-asymmetric-diode-stage`) ---

    /// Renders one editor frame at `time` seconds and returns the colour the
    /// diode dot was drawn in.
    fn draw_dot(state: &mut super::EditorUiState, time: f64) -> nih_plug_egui::egui::Color32 {
        let params = crate::params::CorrosionParams::default();
        let gui = NullGui;
        let setter = nih_plug::prelude::ParamSetter::new(&gui);
        super::DRAWN_DIODE_DOT.with(|dot| dot.set(None));
        let ctx = nih_plug_egui::egui::Context::default();
        let input = nih_plug_egui::egui::RawInput {
            screen_rect: Some(nih_plug_egui::egui::Rect::from_min_size(
                nih_plug_egui::egui::Pos2::ZERO,
                nih_plug_egui::egui::vec2(super::editor_width(1.0), super::editor_height(1.0)),
            )),
            time: Some(time),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            nih_plug_egui::egui::CentralPanel::default().show(ctx, |ui| {
                super::render_ui(ui, &params, &setter, state, 1.0);
            });
        });
        super::DRAWN_DIODE_DOT
            .with(|dot| dot.get())
            .expect("the limiter panel draws its dot")
    }

    fn dot_state() -> super::EditorUiState {
        super::EditorUiState::new(2, (super::BASE_EDITOR_WIDTH, super::BASE_EDITOR_HEIGHT))
    }

    /// Runs a 200 Hz sine at `amplitude` through a default Render clipper for
    /// 0.1 s and publishes its reduction to `meter`, as the plugin does.
    fn clip_into(meter: &crate::dsp::DiodeMeter, amplitude: f32) {
        let mut clipper = crate::dsp::OversampledClipper::new();
        clipper.set_sample_rate(48_000.0);
        clipper.set_oversample_factor(16);
        clipper.set_parameters(0.9661, 0.5, 0.25);
        clipper.reset();
        for i in 0..4_800 {
            let x = amplitude * (std::f32::consts::TAU * 200.0 * i as f32 / 48_000.0).sin();
            clipper.process(x);
        }
        meter.publish(clipper.take_block_reduction_db());
    }

    fn resting() -> nih_plug_egui::egui::Color32 {
        super::DANGER_RED.linear_multiply(0.3)
    }

    #[test]
    fn diode_dot_rests_when_the_signal_stays_under_the_knee() {
        let mut state = dot_state();
        clip_into(&state.diode_meter, 0.2);
        assert_eq!(draw_dot(&mut state, 0.0), resting());
    }

    #[test]
    fn diode_dot_is_full_when_the_stage_clips_hard() {
        let mut state = dot_state();
        // A sine at eight times the ceiling loses well over 6 dB.
        clip_into(&state.diode_meter, 8.0 * 0.9661);
        assert_eq!(draw_dot(&mut state, 0.0), super::DANGER_RED);
    }

    #[test]
    fn diode_dot_falls_back_within_a_second() {
        let mut state = dot_state();
        clip_into(&state.diode_meter, 8.0 * 0.9661);
        assert_eq!(draw_dot(&mut state, 0.0), super::DANGER_RED);
        // No more reduction: the dot dims frame by frame, steadily, and is
        // back at rest within a second.
        let mut previous = super::DANGER_RED.r();
        let mut at_rest = None;
        for frame in 1..=60 {
            let time = f64::from(frame) / 60.0;
            let colour = draw_dot(&mut state, time);
            assert!(colour.r() <= previous, "brightened at {time:.3} s");
            previous = colour.r();
            if colour == resting() && at_rest.is_none() {
                at_rest = Some(time);
            }
        }
        let at_rest = at_rest.expect("back at rest within a second");
        assert!(at_rest > 0.1, "dropped to rest at once ({at_rest:.3} s)");
    }

    /// IDs of the parameters the editor draws a widget for, rendering one
    /// frame of the real editor UI headlessly with `params`.
    fn drawn_parameter_ids(params: &crate::params::CorrosionParams) -> Vec<String> {
        use nih_plug::prelude::Params;
        super::DRAWN_PARAMS.with(|drawn| drawn.borrow_mut().clear());
        let gui = NullGui;
        let setter = nih_plug::prelude::ParamSetter::new(&gui);
        let mut state =
            super::EditorUiState::new(2, (super::BASE_EDITOR_WIDTH, super::BASE_EDITOR_HEIGHT));
        let ctx = nih_plug_egui::egui::Context::default();
        let input = nih_plug_egui::egui::RawInput {
            screen_rect: Some(nih_plug_egui::egui::Rect::from_min_size(
                nih_plug_egui::egui::Pos2::ZERO,
                nih_plug_egui::egui::vec2(super::editor_width(1.0), super::editor_height(1.0)),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            nih_plug_egui::egui::CentralPanel::default().show(ctx, |ui| {
                super::render_ui(ui, params, &setter, &mut state, 1.0);
            });
        });
        let drawn = super::DRAWN_PARAMS.with(|drawn| drawn.borrow().clone());
        params
            .param_map()
            .into_iter()
            .filter(|(_, ptr, _)| drawn.contains(ptr))
            .map(|(id, _, _)| id)
            .collect()
    }

    // --- Inline help (`add-parameter-tooltips`) ---

    /// Every editor configuration whose panels differ: each exciter, object
    /// and Space mode.
    fn panel_variants() -> Vec<crate::params::CorrosionParams> {
        use crate::params::{exciter_param, object_param};
        let mut variants = Vec::new();
        for exciter in 1..=16 {
            variants.push(crate::params::CorrosionParams {
                exciter: exciter_param(exciter),
                ..Default::default()
            });
        }
        for object in 0..9 {
            variants.push(crate::params::CorrosionParams {
                object: object_param(object),
                ..Default::default()
            });
        }
        for mode in 0..4 {
            variants.push(crate::params::CorrosionParams {
                space_mode: nih_plug::prelude::IntParam::new(
                    "Space Mode",
                    mode,
                    nih_plug::prelude::IntRange::Linear { min: 0, max: 3 },
                ),
                ..Default::default()
            });
        }
        variants
    }

    /// Sentences in `text`: terminators followed by a space, plus the last.
    fn sentence_count(text: &str) -> usize {
        let inner = text
            .trim()
            .matches(". ")
            .chain(text.trim().matches("! "))
            .chain(text.trim().matches("? "))
            .count();
        inner + 1
    }

    /// Spec "Help coverage check": every control drawn for every exciter,
    /// object and Space mode has help of one or two sentences, short enough
    /// for a tooltip.
    #[test]
    fn every_drawn_control_has_help() {
        use nih_plug::prelude::Params;
        let mut failures = std::collections::BTreeSet::new();
        for params in panel_variants() {
            super::DRAWN_HELP.with(|help| help.borrow_mut().clear());
            let drawn = drawn_parameter_ids(&params);
            let help = super::DRAWN_HELP.with(|help| help.borrow().clone());
            let exciter = params.exciter.value();
            let object = params.object.value();
            for (id, ptr, _) in params.param_map() {
                if !drawn.contains(&id) {
                    continue;
                }
                let context = format!("{id} (exciter {exciter}, object {object})");
                match help.get(&ptr) {
                    None => {
                        failures.insert(format!("{context}: no help"));
                    }
                    Some(text) if text.trim().is_empty() => {
                        failures.insert(format!("{context}: empty help"));
                    }
                    Some(text) if sentence_count(text) > 2 => {
                        failures.insert(format!("{context}: more than two sentences: {text}"));
                    }
                    Some(text) if text.chars().count() > 200 => {
                        failures.insert(format!(
                            "{context}: {} characters: {text}",
                            text.chars().count()
                        ));
                    }
                    Some(_) => {}
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} problems:\n{}",
            failures.len(),
            failures.into_iter().collect::<Vec<_>>().join("\n")
        );
    }

    /// Every exciter and object slot spec carries help, including models a
    /// render might not reach.
    #[test]
    fn every_slot_spec_has_help() {
        use crate::params::{ExciterType, Object};
        let mut missing = Vec::new();
        for exciter in 1..=16 {
            let exciter = ExciterType::from_int(exciter);
            for spec in super::exciter_panel(exciter).controls {
                if super::help_for(spec.label).trim().is_empty() {
                    missing.push(format!("{} / {}", exciter.name(), spec.label));
                }
            }
        }
        for object in 0..9 {
            let object = Object::from_int(object);
            for spec in super::resonator_panel(object).controls {
                if super::help_for(spec.label).trim().is_empty() {
                    missing.push(format!("{} / {}", object.name(), spec.label));
                }
            }
        }
        assert!(missing.is_empty(), "no help: {missing:?}");
    }

    /// Renders the default editor at 100 % for `frames` frames, the pointer
    /// at `pointer`, 0.1 s apart, and returns every text shape's string from
    /// the last frame (tooltips included).
    fn texts_with_pointer_at(
        params: &crate::params::CorrosionParams,
        pointer: Option<nih_plug_egui::egui::Pos2>,
    ) -> Vec<String> {
        use nih_plug_egui::egui;
        let gui = NullGui;
        let setter = nih_plug::prelude::ParamSetter::new(&gui);
        let mut state =
            super::EditorUiState::new(2, (super::BASE_EDITOR_WIDTH, super::BASE_EDITOR_HEIGHT));
        let ctx = egui::Context::default();
        super::apply_editor_style(&ctx, 1.0);
        let mut texts = Vec::new();
        for frame in 0..12 {
            let mut events = Vec::new();
            if let Some(pos) = pointer {
                events.push(egui::Event::PointerMoved(pos));
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(super::editor_width(1.0), super::editor_height(1.0)),
                )),
                time: Some(frame as f64 * 0.1),
                events,
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    super::render_ui(ui, params, &setter, &mut state, 1.0);
                });
            });
            texts = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect();
        }
        texts
    }

    /// The badge and widget rects of the Cutoff knob, and its help text.
    fn cutoff_badge(
        params: &crate::params::CorrosionParams,
    ) -> (nih_plug_egui::egui::Rect, nih_plug_egui::egui::Rect, String) {
        use nih_plug::prelude::Param;
        let _ = texts_with_pointer_at(params, None);
        let ptr = params.filter_cutoff.as_ptr();
        let (badge, widget) = super::DRAWN_BADGES
            .with(|badges| badges.borrow().get(&ptr).copied())
            .expect("the Cutoff knob draws a help badge");
        let help = super::DRAWN_HELP
            .with(|help| help.borrow().get(&ptr).cloned())
            .expect("the Cutoff knob has help");
        (badge, widget, help)
    }

    /// Spec "Hovering the badge shows help".
    #[test]
    fn hovering_the_badge_shows_help() {
        let params = crate::params::CorrosionParams::default();
        let (badge, _, help) = cutoff_badge(&params);
        let texts = texts_with_pointer_at(&params, Some(badge.center()));
        assert!(
            texts.iter().any(|t| t.contains(&help)),
            "no tooltip with {help:?}"
        );
    }

    /// Spec "Hovering the control does not show help".
    #[test]
    fn hovering_the_knob_shows_no_help() {
        let params = crate::params::CorrosionParams::default();
        let (badge, widget, help) = cutoff_badge(&params);
        // The knob's centre, well away from the badge under its label.
        let knob =
            nih_plug_egui::egui::pos2(widget.center().x, widget.min.y + widget.width() * 0.3);
        assert!(!badge.contains(knob));
        let texts = texts_with_pointer_at(&params, Some(knob));
        assert!(
            !texts.iter().any(|t| t.contains(&help)),
            "the knob body opened the help tooltip"
        );
    }

    /// Help text each drawn parameter got in one frame of the editor
    /// rendered with `params`.
    fn drawn_help(
        params: &crate::params::CorrosionParams,
    ) -> std::collections::HashMap<nih_plug::prelude::ParamPtr, String> {
        super::DRAWN_HELP.with(|help| help.borrow_mut().clear());
        let _ = drawn_parameter_ids(params);
        super::DRAWN_HELP.with(|help| help.borrow().clone())
    }

    /// Spec "Help changes with the exciter": each exciter control's tooltip
    /// is the help for that control as the selected exciter uses it, and two
    /// models with different controls show different help.
    #[test]
    fn help_follows_the_selected_exciter() {
        use crate::params::{exciter_param, ExciterType};
        use nih_plug::prelude::Param;
        let mut panels = Vec::new();
        for value in 1..=16 {
            let params = crate::params::CorrosionParams {
                exciter: exciter_param(value),
                ..Default::default()
            };
            let help = drawn_help(&params);
            let exciter = ExciterType::from_int(value);
            let mut texts = Vec::new();
            for spec in super::exciter_panel(exciter).controls {
                let ptr = super::exciter_param_ref(&params, spec.id).as_ptr();
                let expected = super::help_for(spec.label);
                assert_eq!(
                    help.get(&ptr).map(String::as_str),
                    Some(expected),
                    "{} / {}",
                    exciter.name(),
                    spec.label
                );
                texts.push(expected);
            }
            panels.push((exciter.name(), texts));
        }
        let panel = |name: &str| {
            &panels
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("no exciter named {name}"))
                .1
        };
        assert_ne!(
            panel("Hand Strike"),
            panel("Bow"),
            "Hand Strike and Bow show the same help"
        );
    }

    /// Spec "Help changes with the object": the same for the object panel.
    #[test]
    fn help_follows_the_selected_object() {
        use crate::params::{object_param, Object};
        use nih_plug::prelude::Param;
        let mut panels = Vec::new();
        for value in 0..9 {
            let params = crate::params::CorrosionParams {
                object: object_param(value),
                ..Default::default()
            };
            let help = drawn_help(&params);
            let object = Object::from_int(value);
            let mut texts = Vec::new();
            for spec in super::resonator_panel(object).controls {
                let ptr = super::resonator_param_ref(&params, spec.id).as_ptr();
                let expected = super::help_for(spec.label);
                assert_eq!(
                    help.get(&ptr).map(String::as_str),
                    Some(expected),
                    "{} / {}",
                    object.name(),
                    spec.label
                );
                texts.push(expected);
            }
            panels.push(texts);
        }
        for (i, a) in panels.iter().enumerate() {
            for b in &panels[i + 1..] {
                assert_ne!(a, b, "two objects show the same slot help");
            }
        }
    }

    /// Spec "Selector help describes the selection": each selector's
    /// tooltip describes the option it shows, so every option of a selector
    /// has its own text, and exciter and object options use their panel's
    /// description.
    #[test]
    fn selector_help_describes_the_selection() {
        use crate::params::{
            exciter_param, limiter_mode_param, object_param, play_mode_param, quality_mode_param,
            ExciterType, Object,
        };
        use nih_plug::prelude::{IntParam, IntRange, Param};
        type Edit = fn(&mut crate::params::CorrosionParams, i32);
        type Ptr = fn(&crate::params::CorrosionParams) -> nih_plug::prelude::ParamPtr;
        let selectors: [(&str, std::ops::RangeInclusive<i32>, Edit, Ptr); 6] = [
            (
                "Model Type",
                1..=16,
                |p, v| p.exciter = exciter_param(v),
                |p| p.exciter.as_ptr(),
            ),
            (
                "Material",
                0..=8,
                |p, v| p.object = object_param(v),
                |p| p.object.as_ptr(),
            ),
            (
                "MODE",
                0..=3,
                |p, v| p.quality_mode = quality_mode_param(v),
                |p| p.quality_mode.as_ptr(),
            ),
            (
                "PLAY",
                0..=2,
                |p, v| p.play_mode = play_mode_param(v),
                |p| p.play_mode.as_ptr(),
            ),
            (
                "LIMITER",
                0..=1,
                |p, v| p.limiter_mode = limiter_mode_param(v),
                |p| p.limiter_mode.as_ptr(),
            ),
            (
                "Loop",
                0..=2,
                |p, v| {
                    // Loop is drawn only for friction exciters (the MSEG panel).
                    let bow = (1..=16)
                        .find(|&e| ExciterType::from_int(e).name() == "Bow")
                        .expect("a Bow exciter");
                    p.exciter = exciter_param(bow);
                    p.loop_mode = IntParam::new("Loop Mode", v, IntRange::Linear { min: 0, max: 2 })
                },
                |p| p.loop_mode.as_ptr(),
            ),
        ];
        for (label, values, edit, ptr) in selectors {
            let mut seen = std::collections::BTreeMap::new();
            for value in values {
                let mut params = crate::params::CorrosionParams::default();
                edit(&mut params, value);
                let help = drawn_help(&params)
                    .get(&ptr(&params))
                    .cloned()
                    .unwrap_or_default();
                assert!(!help.is_empty(), "{label} = {value}: no help");
                match label {
                    "Model Type" => assert_eq!(
                        help,
                        super::exciter_panel(ExciterType::from_int(value)).description,
                        "{label} = {value}"
                    ),
                    "Material" => assert_eq!(
                        help,
                        super::resonator_panel(Object::from_int(value)).description,
                        "{label} = {value}"
                    ),
                    _ => {}
                }
                if let Some(other) = seen.insert(help.clone(), value) {
                    panic!("{label}: options {other} and {value} share the help {help:?}");
                }
            }
        }
    }

    /// Slot labels that more than one model uses, checked by hand to mean
    /// the same thing in each (help is looked up by label, so a shared
    /// label shares its tooltip).
    const SHARED_SLOT_LABELS: &[&str] = &["Mallet Mass"];

    /// Help is keyed by label, so two models that reuse a label with a
    /// different meaning would silently show the wrong tooltip. Every slot
    /// label shared between models has to be listed in `SHARED_SLOT_LABELS`
    /// after checking that it means the same thing.
    #[test]
    fn shared_slot_labels_are_reviewed() {
        use crate::params::{ExciterType, Object};
        let mut users: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        for exciter in 1..=16 {
            let exciter = ExciterType::from_int(exciter);
            for spec in super::exciter_panel(exciter).controls {
                users
                    .entry(spec.label.to_ascii_lowercase())
                    .or_default()
                    .push(exciter.name().to_string());
            }
        }
        for object in 0..9 {
            let object = Object::from_int(object);
            for spec in super::resonator_panel(object).controls {
                users
                    .entry(spec.label.to_ascii_lowercase())
                    .or_default()
                    .push(object.name().to_string());
            }
        }
        let reviewed: Vec<String> = SHARED_SLOT_LABELS
            .iter()
            .map(|l| l.to_ascii_lowercase())
            .collect();
        let unreviewed: Vec<String> = users
            .iter()
            .filter(|(label, models)| models.len() > 1 && !reviewed.contains(label))
            .map(|(label, models)| format!("{label:?} in {}", models.join(", ")))
            .collect();
        assert!(
            unreviewed.is_empty(),
            "slot labels shared between models; give each its own label or, if it \
             means the same in both, add it to SHARED_SLOT_LABELS: {unreviewed:?}"
        );
        for label in &reviewed {
            assert!(
                users.get(label).is_some_and(|m| m.len() > 1),
                "{label:?} is in SHARED_SLOT_LABELS but no longer shared"
            );
        }
    }

    /// The Cutoff knob's help badge side, with the editor laid out at
    /// `scale` in a window that fits it.
    fn cutoff_badge_side(scale: f32) -> f32 {
        use nih_plug::prelude::Param;
        let params = crate::params::CorrosionParams::default();
        let window = (super::editor_width(scale), super::editor_height(scale));
        let _ = rendered_right_edge(&params, scale, window);
        let ptr = params.filter_cutoff.as_ptr();
        let (badge, _) = super::DRAWN_BADGES
            .with(|badges| badges.borrow().get(&ptr).copied())
            .expect("the Cutoff knob draws a help badge");
        assert!(
            (badge.width() - badge.height()).abs() < 1e-3,
            "badge isn't square"
        );
        badge.width()
    }

    /// Spec "Badge scales with the UI": the badge's size is proportional to
    /// the UI scale (the content-fits-the-window part of the scenario is
    /// `editor_content_fits_the_window_width`).
    #[test]
    fn badge_scales_with_the_ui() {
        let base = cutoff_badge_side(1.0);
        assert!(base > 4.0, "badge is {base} points at 100 %");
        for scale in [0.5, 0.75, 1.5, 2.0] {
            let side = cutoff_badge_side(scale);
            let ratio = side / base;
            assert!(
                (ratio - scale).abs() < 0.02 * scale,
                "at {scale}× the badge is {side:.2} points ({ratio:.3}× the 100 % size)"
            );
        }
    }

    /// Every parameter the editor draws for some exciter, object and space
    /// mode (their panels differ).
    fn all_drawn_parameter_ids() -> std::collections::BTreeSet<String> {
        use crate::params::{exciter_param, object_param};
        let mut ids = std::collections::BTreeSet::new();
        let mut render = |edit: &dyn Fn(&mut crate::params::CorrosionParams)| {
            let mut params = crate::params::CorrosionParams::default();
            edit(&mut params);
            ids.extend(drawn_parameter_ids(&params));
        };
        for exciter in 1..=16 {
            render(&|p| p.exciter = exciter_param(exciter));
        }
        for object in 0..9 {
            render(&|p| p.object = object_param(object));
        }
        for mode in 0..4 {
            render(&|p| {
                p.space_mode = nih_plug::prelude::IntParam::new(
                    "Space Mode",
                    mode,
                    nih_plug::prelude::IntRange::Linear { min: 0, max: 3 },
                )
            });
        }
        ids
    }

    /// Height of the editor's content for `params` at 100 % scale, from a
    /// headless render.
    fn rendered_content_height(params: &crate::params::CorrosionParams) -> f32 {
        let gui = NullGui;
        let setter = nih_plug::prelude::ParamSetter::new(&gui);
        let mut state =
            super::EditorUiState::new(2, (super::BASE_EDITOR_WIDTH, super::BASE_EDITOR_HEIGHT));
        let ctx = nih_plug_egui::egui::Context::default();
        super::apply_editor_style(&ctx, 1.0);
        let input = nih_plug_egui::egui::RawInput {
            screen_rect: Some(nih_plug_egui::egui::Rect::from_min_size(
                nih_plug_egui::egui::Pos2::ZERO,
                nih_plug_egui::egui::vec2(super::editor_width(1.0), super::editor_height(1.0)),
            )),
            ..Default::default()
        };
        let mut height = 0.0;
        let _ = ctx.run(input, |ctx| {
            nih_plug_egui::egui::CentralPanel::default().show(ctx, |ui| {
                super::render_ui(ui, params, &setter, &mut state, 1.0);
                height = ui.min_rect().height();
            });
        });
        height
    }

    #[test]
    #[ignore]
    fn print_editor_content_height() {
        use crate::params::{exciter_param, object_param};
        for exciter in 1..=16 {
            for object in 0..9 {
                let params = crate::params::CorrosionParams {
                    exciter: exciter_param(exciter),
                    object: object_param(object),
                    space_mode: nih_plug::prelude::IntParam::new(
                        "Space Mode",
                        3,
                        nih_plug::prelude::IntRange::Linear { min: 0, max: 3 },
                    ),
                    ..Default::default()
                };
                println!(
                    "HEIGHT {exciter} {object}: {:.0}",
                    rendered_content_height(&params)
                );
            }
        }
    }

    #[test]
    fn saved_preset_appears_without_reopening() {
        let dir = std::env::temp_dir().join(format!(
            "corrosion-editor-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut state =
            super::EditorUiState::new(2, (super::BASE_EDITOR_WIDTH, super::BASE_EDITOR_HEIGHT));
        state.user_preset_dir = Some(dir.clone());
        state.refresh_user_presets();
        let factory = state.factory_presets.len();
        let params = crate::params::CorrosionParams::default();

        state.save_name = "My Grind".to_string();
        state.save(&params, false);
        assert_eq!(state.preset_count(), factory + 1);
        assert_eq!(state.preset_name(factory), "My Grind");
        assert_eq!(
            state.selected_factory_preset, factory,
            "the new preset is selected"
        );
        assert_eq!(state.save_message.as_ref().map(|m| m.1), Some(false));

        // Saving the same name asks first, and Cancel keeps one file.
        state.save(&params, false);
        assert!(state.confirm_overwrite);
        assert_eq!(state.preset_count(), factory + 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn randomize_sets_only_its_seven_parameters() {
        use nih_plug::prelude::{Param, Params};
        let params = crate::params::CorrosionParams::default();
        let mut noise = crate::dsp::utils::noise::Noise::new(5);
        let patch =
            crate::randomizer::randomize(crate::randomizer::RandomizerMode::Full, &mut noise);
        let ptrs: Vec<_> = super::randomizer_assignments(&params, &patch)
            .iter()
            .map(|(param, _)| param.as_ptr())
            .collect();
        let mut ids: Vec<String> = params
            .param_map()
            .into_iter()
            .filter(|(_, ptr, _)| ptrs.contains(ptr))
            .map(|(id, _, _)| id)
            .collect();
        ids.sort();
        assert_eq!(
            ids,
            ["body", "damage", "drive", "output", "rust", "size", "width"]
        );
    }

    #[test]
    fn sync_fader_shows_the_parameter_text() {
        use crate::params::sync_rate_param;
        for (value, text) in [(0.0, "Off"), (0.3, "1/8"), (1.0, "2/1")] {
            let param = sync_rate_param(value);
            assert_eq!(
                super::fader_value_text(&param, 0.0, 1.0, super::FaderText::Param),
                text
            );
        }
    }

    #[test]
    fn editor_covers_every_parameter() {
        // `h05-expose-missing-gui-controls`: every parameter is either drawn
        // by the editor or listed, with a reason, as host-only.
        use nih_plug::prelude::Params;
        let drawn = all_drawn_parameter_ids();
        let host_only: Vec<&str> = super::HOST_ONLY_PARAMS.iter().map(|(id, _)| *id).collect();
        let missing: Vec<String> = crate::params::CorrosionParams::default()
            .param_map()
            .into_iter()
            .map(|(id, _, _)| id)
            .filter(|id| !drawn.contains(id) && !host_only.contains(&id.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "no editor control and not host-only: {missing:?}"
        );
        let both: Vec<&str> = host_only
            .iter()
            .copied()
            .filter(|id| drawn.contains(*id))
            .collect();
        assert!(both.is_empty(), "drawn but listed as host-only: {both:?}");
    }

    use super::{
        exciter_panel, knob_drag_value, load_factory_presets, persist_editor_size, resonator_panel,
        scaled_editor_size, ui_scale_factor,
    };
    use crate::params::{exciter_model_items, ExciterType, Object};
    use nih_plug_egui::EguiState;

    #[test]
    fn ui_scale_resizes_window_from_base_dimensions() {
        // 768 of panels plus the 72-point keyboard strip (`add-editor-keyboard`).
        assert_eq!(scaled_editor_size(ui_scale_factor(0)), (540, 420));
        assert_eq!(scaled_editor_size(ui_scale_factor(1)), (810, 630));
        assert_eq!(scaled_editor_size(ui_scale_factor(2)), (1080, 840));
        assert_eq!(scaled_editor_size(ui_scale_factor(3)), (1350, 1050));
        assert_eq!(scaled_editor_size(ui_scale_factor(4)), (1620, 1260));
    }

    #[test]
    fn ui_scale_persists_host_facing_editor_size() {
        let editor_state = EguiState::from_size(1080, 840);

        persist_editor_size(&editor_state, scaled_editor_size(ui_scale_factor(1)));

        assert_eq!(editor_state.size(), (810, 630));
    }

    #[test]
    fn exciter_panel_resolves_algorithm_specific_control_counts() {
        assert_eq!(exciter_panel(ExciterType::HandStrike).controls.len(), 4);
        assert_eq!(exciter_panel(ExciterType::HardMallet).controls.len(), 3);
        assert_eq!(exciter_panel(ExciterType::MetalChain).controls.len(), 5);
        assert_eq!(exciter_panel(ExciterType::Bow).controls.len(), 4);
    }

    #[test]
    fn exciter_dropdown_items_are_spec_models_not_categories() {
        let names: Vec<_> = exciter_model_items()
            .iter()
            .map(|(_, name)| *name)
            .collect();

        assert_eq!(names[0], "Hand Strike");
        assert!(names.contains(&"Bow"));
        assert!(!names.contains(&"Hit"));
        assert!(!names.contains(&"Scrape"));
        assert!(!names.contains(&"Other"));
        for (value, name) in exciter_model_items() {
            assert_eq!(ExciterType::from_int(*value).name(), *name);
        }
    }

    #[test]
    fn resonator_panel_resolves_object_specific_control_counts() {
        assert_eq!(resonator_panel(Object::Pipe).controls.len(), 3);
        assert_eq!(resonator_panel(Object::Tank).controls.len(), 4);
        assert_eq!(resonator_panel(Object::IndustrialCog).controls.len(), 3);
    }

    #[test]
    fn factory_preset_loader_finds_factory_bank() {
        let presets = load_factory_presets();

        // The factory bank is seeded by `src/bin/seed_presets.rs`. 50 curated
        // presets cover every object × major exciter combination, with
        // alphabetic sort applied inside `load_factory_presets`.
        assert_eq!(presets.len(), 50);
        assert_eq!(presets[0].name, "Beam Drag");
        assert_eq!(presets[49].name, "Workshop Plate Scrape");
    }

    #[test]
    fn knob_drag_value_is_monotonic_with_vertical_motion() {
        let start = 0.5;
        let min = 0.0;
        let max = 1.0;
        let scale = 1.0;

        let dragged_up = knob_drag_value(start, -20.0, min, max, scale);
        let dragged_down = knob_drag_value(start, 20.0, min, max, scale);

        assert!(dragged_up > start);
        assert!(dragged_down < start);
    }

    #[test]
    fn knob_drag_value_clamps_to_range() {
        assert_eq!(knob_drag_value(0.5, -500.0, 0.0, 1.0, 1.0), 1.0);
        assert_eq!(knob_drag_value(0.5, 500.0, 0.0, 1.0, 1.0), 0.0);
    }

    // --- Keyboard strip (`add-editor-keyboard`) ---

    mod keyboard_strip {
        use super::NullGui;
        use crate::gui_notes::{EditorNoteEvent, EditorNotes, KEY_VELOCITY};
        use nih_plug_egui::egui;
        use std::sync::Arc;

        const SCALE: f32 = 0.75;

        /// The editor window at 75 %, driven frame by frame.
        struct Harness {
            ctx: egui::Context,
            state: super::super::EditorUiState,
            shared: Arc<EditorNotes>,
            params: crate::params::CorrosionParams,
            frame: u32,
        }

        impl Harness {
            fn new() -> Self {
                let shared = Arc::new(EditorNotes::new());
                let state = super::super::EditorUiState {
                    keyboard: super::super::KeyboardStrip::new(shared.clone()),
                    ..super::super::EditorUiState::new(1, super::super::scaled_editor_size(SCALE))
                };
                let ctx = egui::Context::default();
                super::super::apply_editor_style(&ctx, SCALE);
                let mut harness = Self {
                    ctx,
                    state,
                    shared,
                    params: crate::params::CorrosionParams::default(),
                    frame: 0,
                };
                harness.run(Vec::new(), true);
                harness
            }

            /// One frame with `events`; returns the drawn texts and where.
            fn run(
                &mut self,
                events: Vec<egui::Event>,
                focused: bool,
            ) -> Vec<(String, egui::Pos2)> {
                let (w, h) = super::super::scaled_editor_size(SCALE);
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(w as f32, h as f32),
                    )),
                    time: Some(f64::from(self.frame) * 0.02),
                    focused,
                    events,
                    ..Default::default()
                };
                self.frame += 1;
                let gui = NullGui;
                let setter = nih_plug::prelude::ParamSetter::new(&gui);
                let (state, params) = (&mut self.state, &self.params);
                let output = self.ctx.run(input, |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            super::super::render_window_body(ui, params, &setter, state, SCALE);
                        });
                });
                output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        // Only what is visible: the scroll view clips the
                        // panels below its edge.
                        egui::Shape::Text(text) if clipped.clip_rect.contains(text.pos) => {
                            Some((text.galley.job.text.clone(), text.pos))
                        }
                        _ => None,
                    })
                    .collect()
            }

            fn keys(&self) -> (egui::Rect, Vec<(u8, egui::Rect)>) {
                super::super::DRAWN_KEYBOARD
                    .with(|drawn| drawn.borrow().clone())
                    .expect("the keyboard strip was drawn")
            }

            fn key_rect(&self, note: u8) -> egui::Rect {
                self.keys()
                    .1
                    .into_iter()
                    .find(|(n, _)| *n == note)
                    .map(|(_, rect)| rect)
                    .unwrap()
            }

            fn sent(&self) -> Vec<EditorNoteEvent> {
                let mut events = Vec::new();
                self.shared.drain(|event| events.push(event));
                events
            }

            fn press_at(&mut self, pos: egui::Pos2) {
                self.run(
                    vec![egui::Event::PointerMoved(pos), button(pos, true)],
                    true,
                );
            }
        }

        fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }
        }

        fn key(letter: egui::Key, pressed: bool, repeat: bool) -> egui::Event {
            egui::Event::Key {
                key: letter,
                physical_key: None,
                pressed,
                repeat,
                modifiers: egui::Modifiers::NONE,
            }
        }

        fn on(note: u8, velocity: u8) -> EditorNoteEvent {
            EditorNoteEvent::On { note, velocity }
        }

        fn off(note: u8) -> EditorNoteEvent {
            EditorNoteEvent::Off { note }
        }

        /// Spec "The strip is the bottom-most element": pinned to the
        /// window's bottom edge under the panels' scroll view, whatever the
        /// panels' height.
        #[test]
        fn the_strip_is_pinned_to_the_bottom() {
            let harness = Harness::new();
            let (rect, _) = harness.keys();
            let (w, h) = super::super::scaled_editor_size(SCALE);
            assert_eq!((w, h), (810, 630));
            assert!(rect.max.y <= h as f32 + 0.5, "strip ends at {}", rect.max.y);
            assert!(rect.max.y >= h as f32 - 1.0, "strip ends at {}", rect.max.y);
            let strip_top = h as f32
                - (super::super::KEYBOARD_STRIP_BASE_HEIGHT
                    + super::super::KEYBOARD_STRIP_BASE_GAP)
                    * SCALE;
            assert!(rect.min.y >= strip_top - 0.5);
            assert!((rect.width() - w as f32).abs() < 1.0);
        }

        /// Spec "The strip covers C3 to C6".
        #[test]
        fn the_strip_covers_c3_to_c6() {
            let harness = Harness::new();
            let (_, keys) = harness.keys();
            let mut notes: Vec<u8> = keys.iter().map(|&(n, _)| n).collect();
            notes.sort_unstable();
            assert_eq!(notes, (48..=84).collect::<Vec<_>>());
            for &(note, rect) in &keys {
                let short = rect.height() < harness.keys().0.height() - 0.5;
                assert_eq!(
                    short,
                    matches!(note % 12, 1 | 3 | 6 | 8 | 10),
                    "note {note}"
                );
            }
        }

        #[test]
        fn mapped_keys_show_their_letters() {
            let mut harness = Harness::new();
            let texts = harness.run(Vec::new(), true);
            let (strip, _) = harness.keys();
            let letters: Vec<&str> = texts
                .iter()
                .filter(|(_, pos)| strip.expand(1.0).contains(*pos))
                .map(|(text, _)| text.as_str())
                .collect();
            assert_eq!(letters.len(), 13, "{letters:?}");
            for letter in [
                "A", "W", "S", "E", "D", "F", "T", "G", "Y", "H", "U", "J", "K",
            ] {
                assert!(letters.contains(&letter), "{letter} missing: {letters:?}");
            }
        }

        /// Spec "Click and release".
        #[test]
        fn click_and_release() {
            let mut harness = Harness::new();
            let pos = harness.key_rect(60).center_bottom() - egui::vec2(0.0, 4.0);
            harness.press_at(pos);
            harness.run(vec![button(pos, false)], true);
            let sent = harness.sent();
            assert!(
                matches!(sent[0], EditorNoteEvent::On { note: 60, .. }),
                "{sent:?}"
            );
            assert_eq!(&sent[1..], &[off(60)]);
        }

        /// Spec "Drag across keys".
        #[test]
        fn drag_across_keys() {
            let mut harness = Harness::new();
            let from = harness.key_rect(60).center_bottom() - egui::vec2(0.0, 4.0);
            let to = harness.key_rect(62).center_bottom() - egui::vec2(0.0, 4.0);
            harness.press_at(from);
            harness.run(vec![egui::Event::PointerMoved(to)], true);
            harness.run(vec![button(to, false)], true);
            let sent = harness.sent();
            assert_eq!(sent.len(), 4, "{sent:?}");
            assert!(matches!(sent[0], EditorNoteEvent::On { note: 60, .. }));
            assert_eq!(sent[1], off(60));
            assert!(matches!(sent[2], EditorNoteEvent::On { note: 62, .. }));
            assert_eq!(sent[3], off(62));
        }

        /// Spec "Velocity follows the press position".
        #[test]
        fn velocity_follows_the_press_position() {
            let mut harness = Harness::new();
            let rect = harness.key_rect(64);
            let mut velocity_at = |y: f32| {
                let pos = egui::pos2(rect.center().x, y);
                harness.press_at(pos);
                harness.run(vec![button(pos, false)], true);
                match harness.sent()[0] {
                    EditorNoteEvent::On { velocity, .. } => velocity,
                    other => panic!("{other:?}"),
                }
            };
            let top = velocity_at(rect.min.y + 1.0);
            let bottom = velocity_at(rect.max.y - 1.0);
            assert!(bottom > top, "top {top}, bottom {bottom}");
        }

        /// Spec "Black key over white key".
        #[test]
        fn black_key_over_white_key() {
            let mut harness = Harness::new();
            let black = harness.key_rect(61);
            let white = harness.key_rect(60);
            let pos = egui::pos2(black.min.x + 1.0, black.center().y);
            assert!(white.contains(pos), "not an overlap");
            harness.press_at(pos);
            assert!(matches!(
                harness.sent()[0],
                EditorNoteEvent::On { note: 61, .. }
            ));
        }

        /// Spec "The home row plays C4 to C5", through the editor's events.
        #[test]
        fn computer_keys_play_through_the_editor() {
            use egui::Key::*;
            let mut harness = Harness::new();
            harness.run(
                vec![
                    egui::Event::PointerMoved(over_editor()),
                    key(A, true, false),
                    key(K, true, false),
                ],
                true,
            );
            harness.run(vec![key(A, true, true)], true);
            harness.run(vec![key(A, false, false), key(K, false, false)], true);
            assert_eq!(
                harness.sent(),
                vec![on(60, KEY_VELOCITY), on(72, KEY_VELOCITY), off(60), off(72)]
            );
        }

        /// A point over the editor, away from the strip and any widget.
        fn over_editor() -> egui::Pos2 {
            egui::pos2(400.0, 5.0)
        }

        /// Spec "Keys need the pointer over the editor": baseview reports no
        /// focus loss on Windows or X11, so the pointer stands in for focus.
        #[test]
        fn keys_need_the_pointer_over_the_editor() {
            use egui::Key::*;
            let mut harness = Harness::new();
            harness.run(vec![egui::Event::PointerGone], true);
            harness.run(vec![key(A, true, false)], true);
            harness.run(vec![key(A, false, false)], true);
            assert!(harness.sent().is_empty());
            harness.run(vec![egui::Event::PointerMoved(over_editor())], true);
            harness.run(vec![key(A, true, false)], true);
            assert_eq!(harness.sent(), vec![on(60, KEY_VELOCITY)]);
        }

        /// Spec "The pointer leaving releases held notes".
        #[test]
        fn the_pointer_leaving_releases_held_notes() {
            use egui::Key::*;
            let mut harness = Harness::new();
            harness.run(
                vec![
                    egui::Event::PointerMoved(over_editor()),
                    key(D, true, false),
                ],
                true,
            );
            assert_eq!(harness.sent(), vec![on(64, KEY_VELOCITY)]);
            harness.run(vec![egui::Event::PointerGone], true);
            assert_eq!(harness.sent(), vec![off(64)]);
            // The key-up lands elsewhere; coming back and pressing D plays.
            harness.run(vec![egui::Event::PointerMoved(over_editor())], true);
            harness.run(vec![key(D, true, false)], true);
            assert_eq!(harness.sent(), vec![on(64, KEY_VELOCITY)]);
        }

        /// Spec "Unfocused editor ignores keys".
        #[test]
        fn an_unfocused_editor_ignores_keys() {
            use egui::Key::*;
            let mut harness = Harness::new();
            harness.run(
                vec![
                    egui::Event::PointerMoved(over_editor()),
                    key(A, true, false),
                    key(S, true, false),
                ],
                false,
            );
            harness.run(vec![key(A, false, false), key(S, false, false)], false);
            assert!(harness.sent().is_empty());
        }

        /// Spec "Typing a preset name does not play".
        #[test]
        fn typing_a_preset_name_does_not_play() {
            use egui::Key::*;
            let mut harness = Harness::new();
            let texts = harness.run(Vec::new(), true);
            let (_, hint) = texts
                .iter()
                .find(|(text, _)| text == "Preset name")
                .expect("the preset name field is drawn");
            let field = *hint + egui::vec2(4.0, 4.0);
            harness.run(
                vec![egui::Event::PointerMoved(field), button(field, true)],
                true,
            );
            harness.run(vec![button(field, false)], true);
            assert!(
                harness.ctx.wants_keyboard_input(),
                "the field did not take focus"
            );
            let _ = harness.sent();
            for (letter, text) in [(A, "a"), (W, "w"), (S, "s")] {
                harness.run(
                    vec![key(letter, true, false), egui::Event::Text(text.into())],
                    true,
                );
                harness.run(vec![key(letter, false, false)], true);
            }
            assert!(harness.sent().is_empty());
            assert_eq!(harness.state.save_name, "aws");
        }

        /// X11 auto-repeat arrives as a key-up and key-down pair in the same
        /// frame, and baseview doesn't flag it: it must not retrigger.
        #[test]
        fn x11_auto_repeat_pairs_do_not_retrigger() {
            use egui::Key::*;
            let mut harness = Harness::new();
            harness.run(
                vec![
                    egui::Event::PointerMoved(over_editor()),
                    key(D, true, false),
                ],
                true,
            );
            for _ in 0..4 {
                harness.run(vec![key(D, false, false), key(D, true, false)], true);
            }
            harness.run(vec![key(D, false, false)], true);
            assert_eq!(harness.sent(), vec![on(64, KEY_VELOCITY), off(64)]);
            // A release and a press of different keys in one frame both count.
            harness.run(vec![key(D, true, false)], true);
            harness.run(vec![key(D, false, false), key(F, true, false)], true);
            assert_eq!(
                harness.sent(),
                vec![on(64, KEY_VELOCITY), off(64), on(65, KEY_VELOCITY)]
            );
        }

        /// Spec "Losing focus releases held notes".
        #[test]
        fn losing_focus_releases_held_notes() {
            use egui::Key::*;
            let mut harness = Harness::new();
            let pos = harness.key_rect(67).center_bottom() - egui::vec2(0.0, 4.0);
            harness.run(
                vec![
                    egui::Event::PointerMoved(over_editor()),
                    key(A, true, false),
                ],
                true,
            );
            harness.press_at(pos);
            let _ = harness.sent();
            harness.run(vec![egui::Event::WindowFocused(false)], false);
            let mut sent = harness.sent();
            sent.sort_by_key(|event| match event {
                EditorNoteEvent::On { note, .. } | EditorNoteEvent::Off { note } => *note,
            });
            assert_eq!(sent, vec![off(60), off(67)]);
            harness.run(vec![key(A, false, false), button(pos, false)], true);
            assert!(harness.sent().is_empty());
        }
    }
}

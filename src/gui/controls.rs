//! Control tables and help texts, without egui (`add-user-guide`).
//!
//! The editor draws each exciter's and object's panel from these tables
//! and shows `help_for`'s text in each control's ⓘ tooltip. They live outside
//! `editor.rs`, which builds only with the `gui` or `web-gui` feature, so the
//! guide's data exporter and its tests can read them in the CI lane
//! (`--no-default-features`).

use crate::params::{CorrosionParams, ExciterType, Object};
use nih_plug::prelude::FloatParam;

// ---------------------------------------------------------------------------
// Control identifiers and specs (unchanged from original)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub(crate) enum ExciterControlId {
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
pub(crate) enum ResonatorControlId {
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
pub(crate) struct ExciterControlSpec {
    pub(crate) id: ExciterControlId,
    pub(crate) label: &'static str,
    pub(crate) min: f32,
    pub(crate) max: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct ResonatorControlSpec {
    pub(crate) id: ResonatorControlId,
    pub(crate) label: &'static str,
    pub(crate) min: f32,
    pub(crate) max: f32,
}

pub(crate) struct ExciterPanelSpec {
    pub(crate) title: &'static str,
    pub(crate) description: &'static str,
    pub(crate) controls: &'static [ExciterControlSpec],
}

pub(crate) struct ResonatorPanelSpec {
    pub(crate) title: &'static str,
    pub(crate) description: &'static str,
    pub(crate) controls: &'static [ResonatorControlSpec],
}

// ---------------------------------------------------------------------------
// Exciter control spec tables (unchanged)
// ---------------------------------------------------------------------------

pub(crate) const HAND_STRIKE_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const FELT_MALLET_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const HARD_MALLET_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const DRUMSTICK_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const WIRE_BRUSH_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const METAL_PIPE_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const METAL_CHAIN_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const BOW_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const STIFF_POINT_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const HEAVY_GRINDING_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const CORRUGATED_DRAG_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const TENSION_RISE_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const PNEUMATIC_JET_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const ELECTROMAGNETIC_HUM_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const TENSION_SNAP_CONTROLS: &[ExciterControlSpec] = &[
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
pub(crate) const PARTICLE_RAIN_CONTROLS: &[ExciterControlSpec] = &[
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

pub(crate) const PIPE_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
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
pub(crate) const PLATE_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
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
pub(crate) const TANK_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
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
pub(crate) const CHAIN_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
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
pub(crate) const IBEAM_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
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
pub(crate) const TAUT_CABLE_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
    ResonatorControlSpec {
        id: ResonatorControlId::Size,
        label: "Cable Length",
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
pub(crate) const COIL_SPRING_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
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
pub(crate) const SHEET_METAL_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
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
pub(crate) const INDUSTRIAL_COG_RESONATOR_CONTROLS: &[ResonatorControlSpec] = &[
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
// Panel spec functions (unchanged)
// ---------------------------------------------------------------------------

pub(crate) fn exciter_panel(exciter: ExciterType) -> ExciterPanelSpec {
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

pub(crate) fn resonator_panel(object: Object) -> ResonatorPanelSpec {
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

pub(crate) fn exciter_param_ref(params: &CorrosionParams, id: ExciterControlId) -> &FloatParam {
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

pub(crate) fn resonator_param_ref(params: &CorrosionParams, id: ResonatorControlId) -> &FloatParam {
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

/// Help text for a control, by its display label (case-insensitive). Slot
/// labels are model-specific, so the help follows the selected exciter or
/// object. Update the text when a control's behaviour changes.
pub(crate) fn help_for(label: &str) -> &'static str {
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
        "cable length" => "Length of the cable. Longer cables sound lower.",
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
pub(crate) fn selector_help(label: &str, selected: &str) -> &'static str {
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
        ("note mode", "Tonal") => {
            "Each key plays the patch at its own pitch. Bowed and scraped exciters follow the envelope, so they fade out even while the key is held."
        }
        ("note mode", "Kit") => {
            "Each key plays its own pad, with its own sound and tuning, set up with the kit bar above the keyboard. Keys without a pad are silent."
        }
        ("note mode", "Drone") => {
            "Like Tonal, but bowed and scraped exciters keep sounding while the key is held (the envelope loops Decay and Sustain). Struck exciters still decay."
        }
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

// ---------------------------------------------------------------------------
// Fixed controls (`add-user-guide`)
// ---------------------------------------------------------------------------

/// The editor's top-level areas, in drawing order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Panel {
    Header,
    Macros,
    Exciter,
    Resonator,
    Processing,
}

impl Panel {
    /// A stable lower-case name, used as the guide's panel key.
    pub fn key(self) -> &'static str {
        match self {
            Panel::Header => "header",
            Panel::Macros => "macros",
            Panel::Exciter => "exciter",
            Panel::Resonator => "resonator",
            Panel::Processing => "processing",
        }
    }
}

/// When the editor shows a fixed control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    Always,
    /// Only for exciters of this family (the envelope panels).
    Family(crate::params::ExciterFamily),
    /// Only in this Space mode (1 Factory, 2 Spring, 3 Echo).
    SpaceMode(i32),
}

/// A control the editor draws whatever the exciter or object, except for the
/// family- and mode-dependent panels named by `shown`. Model-dependent slots
/// are in the exciter and object tables above, not here.
#[derive(Clone, Copy, Debug)]
pub struct FixedControl {
    /// The parameter's ID.
    pub id: &'static str,
    /// The label as the editor draws it (also `help_for`'s key).
    pub label: &'static str,
    pub panel: Panel,
    /// The section heading the control sits under.
    pub group: &'static str,
    pub shown: Shown,
}

const fn fixed(
    id: &'static str,
    label: &'static str,
    panel: Panel,
    group: &'static str,
    shown: Shown,
) -> FixedControl {
    FixedControl {
        id,
        label,
        panel,
        group,
        shown,
    }
}

use crate::params::ExciterFamily::{Friction, Hit, Specialty};
use Panel::{Exciter, Header, Macros, Processing, Resonator};
use Shown::{Always, Family, SpaceMode};

/// Every fixed control, in the editor's drawing order. A parameter shown in
/// several panels (the envelope stages) has one entry per panel.
/// `fixed_controls_match_the_editor` (GUI lane) fails when this table and
/// what the editor draws disagree.
pub const FIXED_CONTROLS: &[FixedControl] = &[
    fixed("ui_scale", "SCALE", Header, "Header", Always),
    fixed("quality_mode", "MODE", Header, "Header", Always),
    fixed("play_mode", "NOTE MODE", Header, "Header", Always),
    fixed("limiter_mode", "LIMITER", Header, "Header", Always),
    fixed("output", "OUTPUT", Header, "Header", Always),
    fixed("width", "WIDTH", Header, "Header", Always),
    fixed("body", "BODY", Header, "Header", Always),
    fixed("macro_mass", "MASS", Macros, "Macros", Always),
    fixed("macro_corrosion", "CORROSION", Macros, "Macros", Always),
    fixed("macro_violence", "VIOLENCE", Macros, "Macros", Always),
    fixed("macro_brightness", "BRIGHTNESS", Macros, "Macros", Always),
    fixed("exciter", "Model Type", Exciter, "Exciter", Always),
    fixed("exciter_pressure", "Pressure", Exciter, "Exciter", Always),
    fixed("exciter_speed", "Speed", Exciter, "Exciter", Always),
    fixed("exciter_roughness", "Roughness", Exciter, "Exciter", Always),
    fixed(
        "env_amount",
        "Env Amount",
        Exciter,
        "Gesture Modifiers",
        Always,
    ),
    fixed(
        "velocity_to_peak",
        "Vel To Peak",
        Exciter,
        "Gesture Modifiers",
        Always,
    ),
    fixed(
        "global_time_scale",
        "Global Time",
        Exciter,
        "Gesture Modifiers",
        Always,
    ),
    fixed(
        "velocity_to_level",
        "Vel To Level",
        Exciter,
        "Gesture Modifiers",
        Always,
    ),
    fixed(
        "velocity_to_time",
        "Vel To Time",
        Exciter,
        "Gesture Modifiers",
        Always,
    ),
    fixed(
        "curve_tension",
        "Curve",
        Exciter,
        "Gesture Modifiers",
        Always,
    ),
    fixed(
        "env_attack",
        "Attack",
        Exciter,
        "Force Envelope (AD)",
        Family(Hit),
    ),
    fixed(
        "env_release",
        "Release",
        Exciter,
        "Force Envelope (AD)",
        Family(Hit),
    ),
    fixed(
        "mseg_onset",
        "Onset",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "mseg_onset_level",
        "Onset Lvl",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "mseg_attack",
        "Attack",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "mseg_hold",
        "Hold",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "mseg_decay",
        "Decay",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "mseg_sustain",
        "Sustain",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "mseg_release",
        "Release",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "loop_mode",
        "Loop",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "loop_start_stage",
        "Start",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "loop_end_stage",
        "End",
        Exciter,
        "Force Envelope (MSEG)",
        Family(Friction),
    ),
    fixed(
        "env_attack",
        "Attack",
        Exciter,
        "Force Envelope (ADSR)",
        Family(Specialty),
    ),
    fixed(
        "env_decay",
        "Decay",
        Exciter,
        "Force Envelope (ADSR)",
        Family(Specialty),
    ),
    fixed(
        "env_sustain",
        "Sustain",
        Exciter,
        "Force Envelope (ADSR)",
        Family(Specialty),
    ),
    fixed(
        "env_release",
        "Release",
        Exciter,
        "Force Envelope (ADSR)",
        Family(Specialty),
    ),
    fixed("object", "Material", Resonator, "Resonator", Always),
    fixed("res_brightness", "Brightness", Resonator, "Object", Always),
    fixed("rust", "Rust", Resonator, "Environmental Exposure", Always),
    fixed(
        "damage",
        "Damage",
        Resonator,
        "Environmental Exposure",
        Always,
    ),
    fixed("heat", "Heat", Resonator, "Environmental Exposure", Always),
    fixed(
        "sludge",
        "Sludge",
        Resonator,
        "Environmental Exposure",
        Always,
    ),
    fixed(
        "thickness",
        "Thickness",
        Resonator,
        "Environmental Exposure",
        Always,
    ),
    fixed(
        "strike_position",
        "Strike Pos",
        Resonator,
        "Mechanical Linkage",
        Always,
    ),
    fixed(
        "coupling_stiffness",
        "Coupling",
        Resonator,
        "Mechanical Linkage",
        Always,
    ),
    fixed(
        "position_wander",
        "Pos Wander",
        Resonator,
        "Mechanical Linkage",
        Always,
    ),
    fixed(
        "position_wander_rate",
        "Wander Rate",
        Resonator,
        "Mechanical Linkage",
        Always,
    ),
    fixed(
        "position_envelope",
        "Pos Env",
        Resonator,
        "Mechanical Linkage",
        Always,
    ),
    fixed(
        "fundamental_anchor",
        "Anchor",
        Resonator,
        "Mechanical Linkage",
        Always,
    ),
    fixed("filter_cutoff", "Cutoff", Processing, "Filter Bank", Always),
    fixed(
        "filter_resonance",
        "Resonance",
        Processing,
        "Filter Bank",
        Always,
    ),
    fixed(
        "component_tolerance",
        "Tolerance",
        Processing,
        "Filter Bank",
        Always,
    ),
    fixed(
        "drive_amount",
        "Drive Amt",
        Processing,
        "Overdrive Unit",
        Always,
    ),
    fixed(
        "bias_starvation",
        "Bias Starve",
        Processing,
        "Overdrive Unit",
        Always,
    ),
    fixed("chaos_depth", "Chaos", Processing, "Overdrive Unit", Always),
    fixed(
        "drive",
        "Legacy Drive",
        Processing,
        "Overdrive Unit",
        Always,
    ),
    fixed(
        "chassis_material",
        "Chassis Mat",
        Processing,
        "Dispersion",
        Always,
    ),
    fixed(
        "chassis_volume",
        "Chassis Vol",
        Processing,
        "Dispersion",
        Always,
    ),
    fixed("spread_width", "Spread", Processing, "Dispersion", Always),
    fixed(
        "listener_proximity",
        "Proximity",
        Processing,
        "Dispersion",
        Always,
    ),
    fixed("space_mode", "Mode", Processing, "Space", Always),
    fixed("space_amount", "Amount", Processing, "Space", Always),
    fixed(
        "factory_size",
        "Factory Size",
        Processing,
        "Space",
        SpaceMode(1),
    ),
    fixed(
        "machinery_clutter",
        "Machinery",
        Processing,
        "Space",
        SpaceMode(1),
    ),
    fixed(
        "wall_impedance",
        "Wall Impedance",
        Processing,
        "Space",
        SpaceMode(1),
    ),
    fixed(
        "spring_tension",
        "Spring Tension",
        Processing,
        "Space",
        SpaceMode(2),
    ),
    fixed(
        "wire_stiffness",
        "Wire Stiffness",
        Processing,
        "Space",
        SpaceMode(2),
    ),
    fixed(
        "spring_tank_size",
        "Tank Size",
        Processing,
        "Space",
        SpaceMode(2),
    ),
    fixed("sync_rate", "Sync", Processing, "Space", SpaceMode(3)),
    fixed(
        "delay_time",
        "Delay Time",
        Processing,
        "Space",
        SpaceMode(3),
    ),
    fixed(
        "machinery_movement",
        "Movement",
        Processing,
        "Space",
        SpaceMode(3),
    ),
    fixed(
        "high_frequency_damping",
        "HF Damping",
        Processing,
        "Space",
        SpaceMode(3),
    ),
    fixed(
        "analog_ceiling",
        "Ceiling",
        Processing,
        "Brickwall Limiter",
        Always,
    ),
    fixed(
        "diode_softness",
        "Softness",
        Processing,
        "Brickwall Limiter",
        Always,
    ),
    fixed(
        "diode_asymmetry",
        "Asymmetry",
        Processing,
        "Brickwall Limiter",
        Always,
    ),
];

/// Whether a fixed control is shown for this exciter and Space mode.
pub fn fixed_control_shown(control: &FixedControl, exciter: ExciterType, space_mode: i32) -> bool {
    match control.shown {
        Shown::Always => true,
        Shown::Family(family) => exciter.family() == family,
        Shown::SpaceMode(mode) => space_mode == mode,
    }
}

// ---------------------------------------------------------------------------
// Selector options (`add-user-guide`)
// ---------------------------------------------------------------------------

/// The UI Scale menu's items, in size order.
pub const UI_SCALE_ITEMS: [(i32, &str); crate::params::UI_SCALE_CHOICES.len()] = {
    let choices = crate::params::UI_SCALE_CHOICES;
    let mut items = [(0, ""); crate::params::UI_SCALE_CHOICES.len()];
    let mut i = 0;
    while i < choices.len() {
        items[i] = (choices[i].0, choices[i].1);
        i += 1;
    }
    items
};
/// Quality modes.
pub const QUALITY_MODE_ITEMS: [(i32, &str); 4] =
    [(0, "Eco"), (1, "Normal"), (2, "High"), (3, "Render")];
/// Note modes.
pub const PLAY_MODE_ITEMS: [(i32, &str); 3] = [(0, "Tonal"), (1, "Kit"), (2, "Drone")];
/// Limiter modes.
pub const LIMITER_MODE_ITEMS: [(i32, &str); 2] = [(0, "Hard"), (1, "Lookahead")];
/// The MSEG loop modes.
pub const LOOP_MODE_ITEMS: [(i32, &str); 3] = [(0, "Off"), (1, "Forward"), (2, "Ping-Pong")];
/// The MSEG loop's start and end stages.
pub const LOOP_STAGE_ITEMS: [(i32, &str); 6] =
    [(0, "0"), (1, "1"), (2, "2"), (3, "3"), (4, "4"), (5, "5")];
/// Space modes.
pub const SPACE_MODE_ITEMS: [(i32, &str); 4] =
    [(0, "Off"), (1, "Factory"), (2, "Spring"), (3, "Echo")];

/// A selector's options (value, name) by parameter ID, as its menu lists
/// them; empty for an ID that isn't a selector.
pub fn selector_items(id: &str) -> Vec<(i32, &'static str)> {
    match id {
        "ui_scale" => UI_SCALE_ITEMS.to_vec(),
        "quality_mode" => QUALITY_MODE_ITEMS.to_vec(),
        "play_mode" => PLAY_MODE_ITEMS.to_vec(),
        "limiter_mode" => LIMITER_MODE_ITEMS.to_vec(),
        "exciter" => crate::params::exciter_model_items().to_vec(),
        "object" => (0..=8).map(|v| (v, Object::from_int(v).name())).collect(),
        "space_mode" => SPACE_MODE_ITEMS.to_vec(),
        "loop_mode" => LOOP_MODE_ITEMS.to_vec(),
        "loop_start_stage" | "loop_end_stage" => LOOP_STAGE_ITEMS.to_vec(),
        _ => Vec::new(),
    }
}

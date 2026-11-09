use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::params::{object_param, CorrosionParams, Object};
use nih_plug::prelude::{FloatParam, FloatRange, IntParam, IntRange};

/// Preset schema version. Bumped whenever the JSON shape changes in a way
/// that requires a migration step beyond what serde defaults can patch.
///
/// History:
/// - "1" / "2" — legacy versions (pre-`extra`); rely on serde defaults.
/// - "3" — algorithmic resonator engine, character params, quality mode.
/// - "4" — macros (Mass/Corrosion/Violence/Brightness), play_mode,
///   limiter_mode. Legacy presets get neutral defaults via the per-field
///   `#[serde(default = …)]` attributes.
pub const PRESET_VERSION: &str = "4";

/// Run an in-place JSON migration on a raw preset value so it conforms to
/// the current schema.
///
/// `serde_json::Value` is used (rather than typed deserialization) so we can
/// rename keys and rewrite values that no longer exist in the typed
/// `Preset` struct. New additive fields are handled automatically by the
/// per-field `#[serde(default = …)]` attributes — this function exists for
/// the breaking changes those defaults can't represent.
///
/// Each `match` arm bumps the version of the JSON in place, then the function
/// re-enters until the version reaches `PRESET_VERSION` (or the input is
/// already current).
pub fn migrate_preset_json(mut value: serde_json::Value) -> serde_json::Value {
    loop {
        let current_version = value
            .get("version")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        match current_version.as_deref() {
            None | Some("") => {
                // Legacy presets without a version field are treated as v1.
                value["version"] = serde_json::Value::String("1".into());
            }
            Some("1") | Some("2") => {
                // Pre-v3 had no `extra` block. Inject an empty object so the
                // serde defaults populate sensible values on the typed path.
                if value.get("extra").is_none() {
                    value["extra"] = serde_json::json!({});
                }
                value["version"] = serde_json::Value::String("3".into());
            }
            Some("3") => {
                // v3 → v4 only added new fields (macros, play_mode, limiter_mode);
                // each has a per-field serde default, so no JSON rewrite is
                // required. Just bump the stamp.
                value["version"] = serde_json::Value::String("4".into());
            }
            Some(_other) => {
                // Any version we don't recognize (newer than this build, or a
                // typo) is left alone — serde will either accept it as v4-shaped
                // or fail with a clear error.
                break;
            }
        }
        if value.get("version").and_then(serde_json::Value::as_str) == Some(PRESET_VERSION) {
            break;
        }
    }
    migrate_wander_rate(&mut value);
    migrate_onset_level(&mut value);
    value
}

/// The onset level an old patch got from Onset before Onset Level existed:
/// Onset's value, a time in seconds, clamped to 0–1. Shared by the preset
/// and host-state migrations (`m01-m04-m05-clarify-parameters`).
pub fn legacy_onset_level(onset: f32) -> f32 {
    onset.clamp(0.0, 1.0)
}

/// Before `mseg_onset_level` existed, Onset set both the friction MSEG's
/// onset time and its level. A preset without the field gets that level, so
/// it sounds as it did. A preset without `mseg_onset` either keeps both
/// defaults, which already agree. The field is optional, so the schema
/// version stays at 4.
fn migrate_onset_level(value: &mut serde_json::Value) {
    let Some(extra) = value
        .get_mut("extra")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    if extra.contains_key("mseg_onset_level") {
        return;
    }
    let Some(onset) = extra.get("mseg_onset").and_then(serde_json::Value::as_f64) else {
        return;
    };
    extra.insert(
        "mseg_onset_level".into(),
        serde_json::json!(legacy_onset_level(onset as f32)),
    );
}

/// Before `position_wander_rate` existed, Position Envelope set the wander's
/// rate (`0.1 + position_envelope · 4.9` Hz) and moved nothing else. A preset
/// without the field gets that rate, and Position Envelope 0, so it sounds
/// as it did (`h09-route-envelope-to-strike-position`). The field is
/// optional, so the schema version stays at 4.
/// The wander rate, in hertz, that Position Envelope `envelope` used to set
/// before Wander Rate existed. Shared by the preset and host-state
/// migrations so both keep an old patch's sound.
pub fn legacy_wander_rate(envelope: f32) -> f32 {
    0.1 + envelope.clamp(0.0, 1.0) * 4.9
}

fn migrate_wander_rate(value: &mut serde_json::Value) {
    let Some(extra) = value
        .get_mut("extra")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    if extra.contains_key("position_wander_rate") {
        return;
    }
    let envelope = extra
        .get("position_envelope")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0) as f32;
    extra.insert(
        "position_wander_rate".into(),
        serde_json::json!(legacy_wander_rate(envelope)),
    );
    extra.insert("position_envelope".into(), serde_json::json!(0.0));
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetParameters {
    pub ui_scale: i32,
    pub env_attack: f32,
    pub env_decay: f32,
    pub env_sustain: f32,
    pub env_release: f32,
    pub mseg_onset: f32,
    pub mseg_onset_level: f32,
    pub mseg_attack: f32,
    pub mseg_hold: f32,
    pub mseg_decay: f32,
    pub mseg_sustain: f32,
    pub mseg_release: f32,
    pub env_amount: f32,
    pub velocity_to_peak: f32,
    pub loop_mode: i32,
    pub loop_start_stage: i32,
    pub loop_end_stage: i32,
    pub sync_rate: f32,
    pub global_time_scale: f32,
    pub velocity_to_level: f32,
    pub velocity_to_time: f32,
    pub curve_tension: f32,
    pub exciter_pressure: f32,
    pub exciter_speed: f32,
    pub exciter_roughness: f32,
    pub hand_mass: f32,
    pub flesh_stiffness: f32,
    pub flesh_damping: f32,
    pub mute_decay: f32,
    pub mallet_mass: f32,
    pub felt_softness: f32,
    pub core_hardness: f32,
    pub compression_curve: f32,
    pub material_stiffness: f32,
    pub impact_damping: f32,
    pub stick_mass: f32,
    pub tip_stiffness: f32,
    pub restitution_bounciness: f32,
    pub micro_bounce_limit: f32,
    pub wire_density: f32,
    pub spread_duration: f32,
    pub brush_wire_stiffness: f32,
    pub amplitude_randomization: f32,
    pub pipe_mass: f32,
    pub metal_stiffness: f32,
    pub pipe_pitch: f32,
    pub pipe_ring_decay: f32,
    pub link_count: f32,
    pub chain_mass: f32,
    pub drop_envelope_spread: f32,
    pub internal_rattle: f32,
    pub rattle_color: f32,
    pub bow_pressure: f32,
    pub bow_speed: f32,
    pub rosin_grip: f32,
    pub slip_curve: f32,
    pub scrape_speed: f32,
    pub point_pressure: f32,
    pub chatter_pitch: f32,
    pub chatter_damping: f32,
    pub grind_speed: f32,
    pub grind_pressure: f32,
    pub surface_grit: f32,
    pub grit_color: f32,
    pub drag_speed: f32,
    pub ridge_spacing: f32,
    pub ridge_depth: f32,
    pub drag_exciter_mass: f32,
    pub pull_speed: f32,
    pub break_threshold: f32,
    pub slip_stochasticity: f32,
    pub creak_sharpness: f32,
    pub air_pressure: f32,
    pub nozzle_width: f32,
    pub turbulence_chaos: f32,
    pub mains_frequency: f32,
    pub coil_proximity: f32,
    pub voltage_sag: f32,
    pub pull_distance: f32,
    pub hook_stiffness: f32,
    pub snap_force: f32,
    pub flow_rate: f32,
    pub particle_mass: f32,
    pub mass_variance: f32,
    pub strike_position: f32,
    pub coupling_stiffness: f32,
    pub position_wander: f32,
    pub position_envelope: f32,
    /// Position wander rate in hertz. Presets that predate it get the rate
    /// their Position Envelope used to set (`migrate_wander_rate`).
    pub position_wander_rate: f32,
    pub fundamental_anchor: f32,
    pub res_damping: f32,
    pub res_brightness: f32,
    pub thickness: f32,
    pub heat: f32,
    pub sludge: f32,
    pub pipe_diameter: f32,
    pub plate_aspect: f32,
    pub plate_stiffness: f32,
    pub tank_volume: f32,
    pub tank_cavity_mix: f32,
    pub chain_link_mass: f32,
    pub chain_instability: f32,
    pub beam_shear: f32,
    pub cable_braid: f32,
    pub cable_tension_drop: f32,
    pub spring_dispersion: f32,
    pub spring_slosh: f32,
    pub sheet_thinness: f32,
    pub cog_dissonance: f32,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub component_tolerance: f32,
    pub drive_amount: f32,
    pub bias_starvation: f32,
    pub chaos_depth: f32,
    pub spread_width: f32,
    pub listener_proximity: f32,
    pub chassis_material: f32,
    pub chassis_volume: f32,
    pub space_mode: i32,
    pub space_amount: f32,
    pub factory_size: f32,
    pub machinery_clutter: f32,
    pub wall_impedance: f32,
    pub spring_tension: f32,
    pub wire_stiffness: f32,
    pub spring_tank_size: f32,
    pub delay_time: f32,
    pub machinery_movement: f32,
    pub high_frequency_damping: f32,
    pub analog_ceiling: f32,
    pub diode_softness: f32,
    // Macros (P2.2 — PRD §12). New presets persist these directly; old
    // presets fall back to the `#[serde(default)]` value at the struct
    // level, which yields `0.0` — we override that with the neutral 0.5
    // midpoint in `into_params` so legacy presets sound unchanged.
    #[serde(default = "default_macro_value")]
    pub macro_mass: f32,
    #[serde(default = "default_macro_value")]
    pub macro_corrosion: f32,
    #[serde(default = "default_macro_value")]
    pub macro_violence: f32,
    #[serde(default = "default_macro_value")]
    pub macro_brightness: f32,
    /// Limiter mode (0=Hard, 1=Lookahead). Legacy presets default to Hard so
    /// they sound identical to the previous behavior on load.
    #[serde(default)]
    pub limiter_mode: i32,
}

fn default_macro_value() -> f32 {
    0.5
}

impl Default for PresetParameters {
    fn default() -> Self {
        Self::from_params(&CorrosionParams::default())
    }
}

impl PresetParameters {
    pub fn from_params(params: &CorrosionParams) -> Self {
        Self {
            ui_scale: params.ui_scale.value(),
            env_attack: params.env_attack.value(),
            env_decay: params.env_decay.value(),
            env_sustain: params.env_sustain.value(),
            env_release: params.env_release.value(),
            mseg_onset: params.mseg_onset.value(),
            mseg_onset_level: params.mseg_onset_level.value(),
            mseg_attack: params.mseg_attack.value(),
            mseg_hold: params.mseg_hold.value(),
            mseg_decay: params.mseg_decay.value(),
            mseg_sustain: params.mseg_sustain.value(),
            mseg_release: params.mseg_release.value(),
            env_amount: params.env_amount.value(),
            velocity_to_peak: params.velocity_to_peak.value(),
            loop_mode: params.loop_mode.value(),
            loop_start_stage: params.loop_start_stage.value(),
            loop_end_stage: params.loop_end_stage.value(),
            sync_rate: params.sync_rate.value(),
            global_time_scale: params.global_time_scale.value(),
            velocity_to_level: params.velocity_to_level.value(),
            velocity_to_time: params.velocity_to_time.value(),
            curve_tension: params.curve_tension.value(),
            exciter_pressure: params.exciter_pressure.value(),
            exciter_speed: params.exciter_speed.value(),
            exciter_roughness: params.exciter_roughness.value(),
            hand_mass: params.hand_mass.value(),
            flesh_stiffness: params.flesh_stiffness.value(),
            flesh_damping: params.flesh_damping.value(),
            mute_decay: params.mute_decay.value(),
            mallet_mass: params.mallet_mass.value(),
            felt_softness: params.felt_softness.value(),
            core_hardness: params.core_hardness.value(),
            compression_curve: params.compression_curve.value(),
            material_stiffness: params.material_stiffness.value(),
            impact_damping: params.impact_damping.value(),
            stick_mass: params.stick_mass.value(),
            tip_stiffness: params.tip_stiffness.value(),
            restitution_bounciness: params.restitution_bounciness.value(),
            micro_bounce_limit: params.micro_bounce_limit.value(),
            wire_density: params.wire_density.value(),
            spread_duration: params.spread_duration.value(),
            brush_wire_stiffness: params.brush_wire_stiffness.value(),
            amplitude_randomization: params.amplitude_randomization.value(),
            pipe_mass: params.pipe_mass.value(),
            metal_stiffness: params.metal_stiffness.value(),
            pipe_pitch: params.pipe_pitch.value(),
            pipe_ring_decay: params.pipe_ring_decay.value(),
            link_count: params.link_count.value(),
            chain_mass: params.chain_mass.value(),
            drop_envelope_spread: params.drop_envelope_spread.value(),
            internal_rattle: params.internal_rattle.value(),
            rattle_color: params.rattle_color.value(),
            bow_pressure: params.bow_pressure.value(),
            bow_speed: params.bow_speed.value(),
            rosin_grip: params.rosin_grip.value(),
            slip_curve: params.slip_curve.value(),
            scrape_speed: params.scrape_speed.value(),
            point_pressure: params.point_pressure.value(),
            chatter_pitch: params.chatter_pitch.value(),
            chatter_damping: params.chatter_damping.value(),
            grind_speed: params.grind_speed.value(),
            grind_pressure: params.grind_pressure.value(),
            surface_grit: params.surface_grit.value(),
            grit_color: params.grit_color.value(),
            drag_speed: params.drag_speed.value(),
            ridge_spacing: params.ridge_spacing.value(),
            ridge_depth: params.ridge_depth.value(),
            drag_exciter_mass: params.drag_exciter_mass.value(),
            pull_speed: params.pull_speed.value(),
            break_threshold: params.break_threshold.value(),
            slip_stochasticity: params.slip_stochasticity.value(),
            creak_sharpness: params.creak_sharpness.value(),
            air_pressure: params.air_pressure.value(),
            nozzle_width: params.nozzle_width.value(),
            turbulence_chaos: params.turbulence_chaos.value(),
            mains_frequency: params.mains_frequency.value(),
            coil_proximity: params.coil_proximity.value(),
            voltage_sag: params.voltage_sag.value(),
            pull_distance: params.pull_distance.value(),
            hook_stiffness: params.hook_stiffness.value(),
            snap_force: params.snap_force.value(),
            flow_rate: params.flow_rate.value(),
            particle_mass: params.particle_mass.value(),
            mass_variance: params.mass_variance.value(),
            strike_position: params.strike_position.value(),
            coupling_stiffness: params.coupling_stiffness.value(),
            position_wander: params.position_wander.value(),
            position_envelope: params.position_envelope.value(),
            position_wander_rate: params.position_wander_rate.value(),
            fundamental_anchor: params.fundamental_anchor.value(),
            res_damping: params.res_damping.value(),
            res_brightness: params.res_brightness.value(),
            thickness: params.thickness.value(),
            heat: params.heat.value(),
            sludge: params.sludge.value(),
            pipe_diameter: params.pipe_diameter.value(),
            plate_aspect: params.plate_aspect.value(),
            plate_stiffness: params.plate_stiffness.value(),
            tank_volume: params.tank_volume.value(),
            tank_cavity_mix: params.tank_cavity_mix.value(),
            chain_link_mass: params.chain_link_mass.value(),
            chain_instability: params.chain_instability.value(),
            beam_shear: params.beam_shear.value(),
            cable_braid: params.cable_braid.value(),
            cable_tension_drop: params.cable_tension_drop.value(),
            spring_dispersion: params.spring_dispersion.value(),
            spring_slosh: params.spring_slosh.value(),
            sheet_thinness: params.sheet_thinness.value(),
            cog_dissonance: params.cog_dissonance.value(),
            filter_cutoff: params.filter_cutoff.value(),
            filter_resonance: params.filter_resonance.value(),
            component_tolerance: params.component_tolerance.value(),
            drive_amount: params.drive_amount.value(),
            bias_starvation: params.bias_starvation.value(),
            chaos_depth: params.chaos_depth.value(),
            spread_width: params.spread_width.value(),
            listener_proximity: params.listener_proximity.value(),
            chassis_material: params.chassis_material.value(),
            chassis_volume: params.chassis_volume.value(),
            space_mode: params.space_mode.value(),
            space_amount: params.space_amount.value(),
            factory_size: params.factory_size.value(),
            machinery_clutter: params.machinery_clutter.value(),
            wall_impedance: params.wall_impedance.value(),
            spring_tension: params.spring_tension.value(),
            wire_stiffness: params.wire_stiffness.value(),
            spring_tank_size: params.spring_tank_size.value(),
            delay_time: params.delay_time.value(),
            machinery_movement: params.machinery_movement.value(),
            high_frequency_damping: params.high_frequency_damping.value(),
            analog_ceiling: params.analog_ceiling.value(),
            diode_softness: params.diode_softness.value(),
            macro_mass: params.macro_mass.value(),
            macro_corrosion: params.macro_corrosion.value(),
            macro_violence: params.macro_violence.value(),
            macro_brightness: params.macro_brightness.value(),
            limiter_mode: params.limiter_mode.value(),
        }
    }

    fn apply_to(self, params: &mut CorrosionParams) {
        params.ui_scale = int_param("UI Scale", self.ui_scale, 0, 4);
        params.env_attack = float_param("Attack", self.env_attack, 0.001, 2.0);
        params.env_decay = float_param("Decay", self.env_decay, 0.01, 5.0);
        params.env_sustain = float_param("Sustain", self.env_sustain, 0.0, 1.0);
        params.env_release = float_param("Release", self.env_release, 0.01, 5.0);
        params.mseg_onset = float_param("Onset Time", self.mseg_onset, 0.001, 1.0);
        params.mseg_onset_level = crate::params::mseg_onset_level_param(self.mseg_onset_level);
        params.mseg_attack = float_param("MSEG Attack", self.mseg_attack, 0.001, 2.0);
        params.mseg_hold = float_param("Hold", self.mseg_hold, 0.0, 2.0);
        params.mseg_decay = float_param("MSEG Decay", self.mseg_decay, 0.01, 5.0);
        params.mseg_sustain = float_param("MSEG Sustain", self.mseg_sustain, 0.0, 1.0);
        params.mseg_release = float_param("MSEG Release", self.mseg_release, 0.01, 5.0);
        params.env_amount = float_param("Env Amount", self.env_amount, 0.0, 1.0);
        params.velocity_to_peak = float_param("Velocity To Peak", self.velocity_to_peak, 0.0, 1.0);
        params.loop_mode = int_param("Loop Mode", self.loop_mode, 0, 2);
        params.loop_start_stage = int_param("Loop Start Stage", self.loop_start_stage, 0, 5);
        params.loop_end_stage = int_param("Loop End Stage", self.loop_end_stage, 0, 5);
        params.sync_rate = float_param("Sync Rate", self.sync_rate, 0.0, 1.0);
        params.global_time_scale =
            float_param("Global Time Scale", self.global_time_scale, 0.1, 10.0);
        params.velocity_to_level =
            float_param("Velocity To Level", self.velocity_to_level, 0.0, 1.0);
        params.velocity_to_time = float_param("Velocity To Time", self.velocity_to_time, 0.0, 1.0);
        params.curve_tension = float_param("Curve Tension", self.curve_tension, -1.0, 1.0);
        params.exciter_pressure = float_param("Pressure", self.exciter_pressure, 0.0, 1.0);
        params.exciter_speed = float_param("Speed", self.exciter_speed, 0.0, 1.0);
        params.exciter_roughness = float_param("Roughness", self.exciter_roughness, 0.0, 1.0);
        params.hand_mass = float_param("Hand Mass", self.hand_mass, 0.4, 3.0);
        params.flesh_stiffness = float_param("Flesh Stiffness", self.flesh_stiffness, 0.05, 0.8);
        params.flesh_damping = float_param("Flesh Damping", self.flesh_damping, 0.3, 1.8);
        params.mute_decay = float_param("Mute Decay", self.mute_decay, 0.85, 0.999);
        params.mallet_mass = float_param("Mallet Mass", self.mallet_mass, 0.4, 3.5);
        params.felt_softness = float_param("Felt Softness", self.felt_softness, 0.1, 1.3);
        params.core_hardness = float_param("Core Hardness", self.core_hardness, 0.5, 4.5);
        params.compression_curve =
            float_param("Compression Curve", self.compression_curve, 2.0, 5.0);
        params.material_stiffness =
            float_param("Material Stiffness", self.material_stiffness, 0.5, 5.0);
        params.impact_damping = float_param("Impact Damping", self.impact_damping, 0.1, 1.3);
        params.stick_mass = float_param("Stick Mass", self.stick_mass, 0.05, 1.25);
        params.tip_stiffness = float_param("Tip Stiffness", self.tip_stiffness, 0.8, 6.8);
        params.restitution_bounciness = float_param(
            "Restitution Bounciness",
            self.restitution_bounciness,
            0.2,
            0.9,
        );
        params.micro_bounce_limit =
            float_param("Micro Bounce Limit", self.micro_bounce_limit, 2.0, 8.0);
        params.wire_density = float_param("Wire Density", self.wire_density, 10.0, 130.0);
        params.spread_duration = float_param("Spread Duration", self.spread_duration, 10.0, 250.0);
        params.brush_wire_stiffness =
            float_param("Brush Wire Stiffness", self.brush_wire_stiffness, 0.0, 1.0);
        params.amplitude_randomization = float_param(
            "Amplitude Randomization",
            self.amplitude_randomization,
            0.0,
            1.0,
        );
        params.pipe_mass = float_param("Pipe Mass", self.pipe_mass, 0.4, 2.6);
        params.metal_stiffness = float_param("Metal Stiffness", self.metal_stiffness, 0.5, 5.5);
        params.pipe_pitch = float_param("Pipe Pitch", self.pipe_pitch, 0.5, 2.5);
        params.pipe_ring_decay = float_param("Pipe Ring Decay", self.pipe_ring_decay, 0.96, 0.999);
        params.link_count = float_param("Link Count", self.link_count, 3.0, 15.0);
        params.chain_mass = float_param("Chain Mass", self.chain_mass, 0.2, 1.4);
        params.drop_envelope_spread = float_param(
            "Drop Envelope Spread",
            self.drop_envelope_spread,
            40.0,
            400.0,
        );
        params.internal_rattle = float_param("Internal Rattle", self.internal_rattle, 0.0, 1.0);
        params.rattle_color = float_param("Rattle Color", self.rattle_color, 0.0, 1.0);
        params.bow_pressure = float_param("Bow Pressure", self.bow_pressure, 0.2, 2.0);
        params.bow_speed = float_param("Bow Speed", self.bow_speed, 0.1, 2.0);
        params.rosin_grip = float_param("Rosin Grip", self.rosin_grip, 0.05, 1.5);
        params.slip_curve = float_param("Slip Curve", self.slip_curve, 0.05, 1.5);
        params.scrape_speed = float_param("Scrape Speed", self.scrape_speed, 0.1, 2.5);
        params.point_pressure = float_param("Point Pressure", self.point_pressure, 0.1, 1.5);
        params.chatter_pitch = float_param("Chatter Pitch", self.chatter_pitch, 0.1, 1.5);
        params.chatter_damping = float_param("Chatter Damping", self.chatter_damping, 0.1, 0.9);
        params.grind_speed = float_param("Grind Speed", self.grind_speed, 0.1, 2.5);
        params.grind_pressure = float_param("Grind Pressure", self.grind_pressure, 0.1, 1.9);
        params.surface_grit = float_param("Surface Grit", self.surface_grit, 0.0, 1.0);
        params.grit_color = float_param("Grit Color", self.grit_color, 0.0, 1.0);
        params.drag_speed = float_param("Drag Speed", self.drag_speed, 0.1, 2.5);
        params.ridge_spacing = float_param("Ridge Spacing", self.ridge_spacing, 0.01, 0.2);
        params.ridge_depth = float_param("Ridge Depth", self.ridge_depth, 0.0, 2.0);
        params.drag_exciter_mass = float_param("Exciter Mass", self.drag_exciter_mass, 0.2, 2.0);
        params.pull_speed = float_param("Pull Speed", self.pull_speed, 0.05, 1.55);
        params.break_threshold = float_param("Break Threshold", self.break_threshold, 0.1, 1.6);
        params.slip_stochasticity =
            float_param("Slip Stochasticity", self.slip_stochasticity, 0.0, 1.0);
        params.creak_sharpness = float_param("Creak Sharpness", self.creak_sharpness, 0.2, 1.4);
        params.air_pressure = float_param("Air Pressure", self.air_pressure, 0.1, 2.1);
        params.nozzle_width = float_param("Nozzle Width", self.nozzle_width, 0.1, 1.6);
        params.turbulence_chaos = float_param("Turbulence Chaos", self.turbulence_chaos, 0.0, 2.0);
        params.mains_frequency = float_param("Mains Frequency", self.mains_frequency, 40.0, 120.0);
        params.coil_proximity = float_param("Coil Proximity", self.coil_proximity, 0.0, 2.0);
        params.voltage_sag = float_param("Voltage Sag", self.voltage_sag, 0.0, 2.0);
        params.pull_distance = float_param("Pull Distance", self.pull_distance, 0.1, 1.5);
        params.hook_stiffness = float_param("Hook Stiffness", self.hook_stiffness, 0.2, 2.2);
        params.snap_force = float_param("Snap Force", self.snap_force, 0.1, 2.0);
        params.flow_rate = float_param("Flow Rate", self.flow_rate, 0.1, 3.1);
        params.particle_mass = float_param("Particle Mass", self.particle_mass, 0.05, 1.0);
        params.mass_variance = float_param("Mass Variance", self.mass_variance, 0.0, 2.0);
        params.strike_position = float_param("Strike Position", self.strike_position, 0.0, 1.0);
        params.coupling_stiffness =
            float_param("Coupling Stiffness", self.coupling_stiffness, 0.0, 1.0);
        params.position_wander = float_param("Position Wander", self.position_wander, 0.0, 1.0);
        params.position_envelope =
            float_param("Position Envelope", self.position_envelope, 0.0, 1.0);
        params.position_wander_rate =
            crate::params::position_wander_rate_param(if self.position_wander_rate.is_finite() {
                self.position_wander_rate.clamp(0.1, 5.0)
            } else {
                0.1
            });
        params.fundamental_anchor =
            float_param("Fundamental Anchor", self.fundamental_anchor, 0.0, 1.0);
        params.res_damping = float_param("Damping", self.res_damping, 0.0, 1.0);
        params.res_brightness = float_param("Resonator Brightness", self.res_brightness, 0.0, 1.0);
        params.thickness = float_param("Thickness", self.thickness, 0.0, 1.0);
        params.heat = float_param("Heat", self.heat, 0.0, 1.0);
        params.sludge = float_param("Sludge", self.sludge, 0.0, 1.0);
        params.pipe_diameter = float_param("Pipe Diameter", self.pipe_diameter, 0.0, 1.0);
        params.plate_aspect = float_param("Plate Aspect", self.plate_aspect, 0.1, 4.0);
        params.plate_stiffness = float_param("Plate Stiffness", self.plate_stiffness, 0.25, 3.0);
        params.tank_volume = float_param("Tank Volume", self.tank_volume, 0.0, 1.0);
        params.tank_cavity_mix = float_param("Cavity Mix", self.tank_cavity_mix, 0.0, 1.0);
        params.chain_link_mass = float_param("Link Mass", self.chain_link_mass, 0.1, 1.0);
        params.chain_instability = float_param("Instability", self.chain_instability, 0.0, 1.0);
        params.beam_shear = float_param("Shear Density", self.beam_shear, 0.0, 1.0);
        params.cable_braid = float_param("Braid Stiffness", self.cable_braid, 0.0, 1.0);
        params.cable_tension_drop = float_param("Tension Drop", self.cable_tension_drop, 0.0, 1.0);
        params.spring_dispersion =
            float_param("Dispersion Chirp", self.spring_dispersion, 0.0, 1.0);
        params.spring_slosh = float_param("Spring Slosh", self.spring_slosh, 0.0, 1.0);
        params.sheet_thinness = float_param("Metal Thinness", self.sheet_thinness, 0.0, 1.0);
        params.cog_dissonance = float_param("Tooth Dissonance", self.cog_dissonance, 0.0, 1.0);
        params.filter_cutoff =
            skewed_float_param("Filter Cutoff", self.filter_cutoff, 20.0, 20000.0, 0.5);
        params.filter_resonance = float_param("Filter Resonance", self.filter_resonance, 0.0, 1.0);
        params.component_tolerance =
            float_param("Component Tolerance", self.component_tolerance, 0.0, 1.0);
        params.drive_amount = float_param("Drive Amount", self.drive_amount, 0.0, 5.0);
        params.bias_starvation = float_param("Bias Starvation", self.bias_starvation, 0.0, 1.0);
        params.chaos_depth = float_param("Chaos Depth", self.chaos_depth, 0.0, 1.0);
        params.spread_width = float_param("Spread Width", self.spread_width, 0.0, 1.0);
        params.listener_proximity =
            float_param("Listener Proximity", self.listener_proximity, 0.0, 1.0);
        params.chassis_material = float_param("Chassis Material", self.chassis_material, 0.0, 1.0);
        params.chassis_volume = float_param("Chassis Volume", self.chassis_volume, 0.0, 1.0);
        params.space_mode = int_param("Space Mode", self.space_mode, 0, 3);
        params.space_amount = float_param("Space Amount", self.space_amount, 0.0, 1.0);
        params.factory_size = float_param("Factory Size", self.factory_size, 0.0, 1.0);
        params.machinery_clutter =
            float_param("Machinery Clutter", self.machinery_clutter, 0.0, 1.0);
        params.wall_impedance = float_param("Wall Impedance", self.wall_impedance, 0.0, 1.0);
        params.spring_tension = float_param("Spring Tension", self.spring_tension, 0.0, 1.0);
        params.wire_stiffness = float_param("Spring Wire Stiffness", self.wire_stiffness, 0.0, 1.0);
        params.spring_tank_size = float_param("Spring Tank Size", self.spring_tank_size, 0.0, 1.0);
        params.delay_time = float_param("Delay Time", self.delay_time, 0.0, 1.0);
        params.machinery_movement =
            float_param("Machinery Movement", self.machinery_movement, 0.0, 1.0);
        params.high_frequency_damping = float_param(
            "High Frequency Damping",
            self.high_frequency_damping,
            0.0,
            1.0,
        );
        params.analog_ceiling = float_param("Analog Ceiling", self.analog_ceiling, 0.5, 1.0);
        params.diode_softness = float_param("Diode Softness", self.diode_softness, 0.0, 1.0);
        params.macro_mass = float_param("Mass", self.macro_mass, 0.0, 1.0);
        params.macro_corrosion = float_param("Corrosion", self.macro_corrosion, 0.0, 1.0);
        params.macro_violence = float_param("Violence", self.macro_violence, 0.0, 1.0);
        params.macro_brightness = float_param("Macro Brightness", self.macro_brightness, 0.0, 1.0);
        params.limiter_mode = crate::params::limiter_mode_param(self.limiter_mode);
    }
}

fn float_param(name: &'static str, value: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, value.clamp(min, max), FloatRange::Linear { min, max })
        .with_value_to_string(std::sync::Arc::new(|value| format!("{value:.6}")))
        .with_string_to_value(std::sync::Arc::new(|string| string.trim().parse().ok()))
}

fn int_param(name: &'static str, value: i32, min: i32, max: i32) -> IntParam {
    IntParam::new(name, value.clamp(min, max), IntRange::Linear { min, max })
}

fn skewed_float_param(
    name: &'static str,
    value: f32,
    min: f32,
    max: f32,
    factor: f32,
) -> FloatParam {
    FloatParam::new(
        name,
        value.clamp(min, max),
        FloatRange::Skewed { min, max, factor },
    )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub version: String,
    pub object: Object,
    pub exciter: i32,
    pub size: f32,
    pub rust: f32,
    pub damage: f32,
    pub drive: f32,
    pub output: f32,
    pub width: f32,
    pub body: f32,
    #[serde(default = "default_quality_mode")]
    pub quality_mode: i32,
    /// Play mode (0=Tonal, 1=Kit, 2=Drone). Defaults to Tonal for legacy
    /// presets so the existing behavior is preserved on load.
    #[serde(default = "default_play_mode")]
    pub play_mode: i32,
    #[serde(default)]
    pub extra: PresetParameters,
}

fn default_quality_mode() -> i32 {
    1
}

fn default_play_mode() -> i32 {
    0 // Tonal
}

/// Parameter IDs that loading a preset leaves alone: editor preferences, not
/// part of a sound. `ui_scale` sizes the editor window.
pub const PRESET_SKIPPED_PARAMS: &[&str] = &["ui_scale"];

impl Preset {
    pub fn from_params(name: impl Into<String>, params: &CorrosionParams) -> Self {
        Self {
            name: name.into(),
            version: PRESET_VERSION.to_string(),
            object: Object::from_int(params.object.value()),
            exciter: params.exciter.value(),
            size: params.size.value(),
            rust: params.rust.value(),
            damage: params.damage.value(),
            drive: params.drive.value(),
            output: params.output.value(),
            width: params.width.value(),
            body: params.body.value(),
            quality_mode: params.quality_mode.value(),
            play_mode: params.play_mode.value(),
            extra: PresetParameters::from_params(params),
        }
    }

    pub fn into_params(self) -> CorrosionParams {
        let mut params = CorrosionParams::default();
        params.object = object_param(self.object.to_int());
        params.exciter = crate::params::exciter_param(self.exciter);
        params.size = float_param("Size", self.size, 0.05, 10.0);
        params.rust = float_param("Rust", self.rust, 0.0, 5.0);
        params.damage = float_param("Damage", self.damage, 0.0, 10.0);
        params.drive = float_param("Drive", self.drive, 0.0, 5.0);
        params.output = crate::params::output_param(self.output);
        params.width = float_param("Width", self.width, -2.0, 3.0);
        params.body = float_param("Body", self.body, 0.0, 5.0);
        params.quality_mode = crate::params::quality_mode_param(self.quality_mode);
        params.play_mode = crate::params::play_mode_param(self.play_mode);
        self.extra.apply_to(&mut params);
        params
    }

    /// The parameter values loading this preset sets, as `(id, normalized
    /// value against `live`'s own parameter)`, for every parameter in
    /// `live`'s parameter map except the editor-only ones in
    /// `PRESET_SKIPPED_PARAMS` (`h04-load-every-preset-field-in-gui`).
    ///
    /// Each value is taken as a plain value from `into_params()` and
    /// normalized by the live parameter of the same ID, so a parameter that
    /// `into_params` builds with a different range still lands on the right
    /// value.
    pub fn parameter_values(&self, live: &CorrosionParams) -> Vec<(String, f32)> {
        use nih_plug::prelude::Params;
        let loaded = self.clone().into_params();
        let loaded_plain: std::collections::HashMap<String, f32> = loaded
            .param_map()
            .into_iter()
            // SAFETY: the pointers come from `loaded`, alive for this call.
            .map(|(id, ptr, _)| (id, unsafe { ptr.unmodulated_plain_value() }))
            .collect();
        live.param_map()
            .into_iter()
            .filter(|(id, _, _)| !PRESET_SKIPPED_PARAMS.contains(&id.as_str()))
            .filter_map(|(id, ptr, _)| {
                let plain = *loaded_plain.get(&id)?;
                // SAFETY: the pointers come from `live`, borrowed for this call.
                Some((id, unsafe { ptr.preview_normalized(plain) }))
            })
            .collect()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, json)
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let json = fs::read_to_string(path)?;
        Self::from_json_str(&json)
    }

    /// Parse a preset from a JSON string, applying the version migration step
    /// before typed deserialization. Used both by `load` and by the host
    /// state-restore path so plugin state and on-disk presets behave the same.
    pub fn from_json_str(json: &str) -> io::Result<Self> {
        let raw: serde_json::Value = serde_json::from_str(json).map_err(io::Error::other)?;
        let migrated = migrate_preset_json(raw);
        let preset: Self = serde_json::from_value(migrated).map_err(io::Error::other)?;
        Ok(preset.sanitized())
    }

    fn sanitized(self) -> Self {
        let params = self.clone().into_params();
        let mut sanitized = Self::from_params(self.name, &params);
        sanitized.version = self.version;
        sanitized.object = self.object;
        sanitized.exciter = params.exciter.value();
        sanitized
    }
}

// ---------------------------------------------------------------------------
// User presets (`h05-expose-missing-gui-controls`)
// ---------------------------------------------------------------------------

/// File extension of preset files, factory and user alike.
pub const PRESET_EXTENSION: &str = "corrosion-preset";

/// The per-user preset folder, from the platform's standard environment
/// variables (no new dependency):
/// - Windows: `%APPDATA%\Corrosion\Presets`
/// - macOS: `$HOME/Library/Application Support/Corrosion/Presets`
/// - others: `$XDG_DATA_HOME/corrosion/presets`, else
///   `$HOME/.local/share/corrosion/presets`
///
/// `None` when the variables it needs are unset. The folder may not exist
/// yet; saving creates it.
pub fn user_preset_dir() -> Option<std::path::PathBuf> {
    user_preset_dir_for(std::env::consts::OS, |name| std::env::var_os(name))
}

/// `user_preset_dir` for operating system `os`, reading variables through
/// `env` (testable on any platform).
pub fn user_preset_dir_for(
    os: &str,
    env: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let non_empty = |name: &str| {
        env(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    match os {
        "windows" => non_empty("APPDATA").map(|dir| dir.join("Corrosion").join("Presets")),
        "macos" => non_empty("HOME").map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Corrosion")
                .join("Presets")
        }),
        _ => non_empty("XDG_DATA_HOME")
            .or_else(|| non_empty("HOME").map(|home| home.join(".local").join("share")))
            .map(|data| data.join("corrosion").join("presets")),
    }
}

/// A file name (without extension) for a preset called `name`, safe on every
/// platform:
/// - path separators and characters invalid on Windows become `_`;
/// - control characters are dropped;
/// - leading and trailing spaces and dots are trimmed;
/// - Windows reserved device names (`CON`, `NUL`, `COM1`, …) get a `_` prefix;
/// - the result is at most 100 characters, and "Untitled" if nothing is left.
pub fn sanitize_file_name(name: &str) -> String {
    let replaced: String = name
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect();
    let trimmed = replaced.trim_matches(|c: char| c == ' ' || c == '.');
    let mut file: String = trimmed.chars().take(100).collect();
    file = file.trim_end_matches([' ', '.']).to_string();
    if file.is_empty() {
        return "Untitled".to_string();
    }
    let stem = file.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0');
    if reserved {
        file.insert(0, '_');
    }
    file
}

/// The user presets in `dir`, sorted by display name (ignoring case), and
/// how many preset files were skipped because they couldn't be read. A
/// missing folder is simply empty.
pub fn list_user_presets(dir: &Path) -> (Vec<(std::path::PathBuf, Preset)>, usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return (Vec::new(), 0);
    };
    let mut presets = Vec::new();
    let mut skipped = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != PRESET_EXTENSION) {
            continue;
        }
        match Preset::load(&path) {
            Ok(preset) => presets.push((path, preset)),
            Err(_) => skipped += 1,
        }
    }
    presets.sort_by_key(|(_, preset)| preset.name.to_lowercase());
    (presets, skipped)
}

/// What `save_user_preset` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    /// The preset was written to this file.
    Saved(std::path::PathBuf),
    /// A user preset already uses this file: nothing was written. Save again
    /// with `overwrite` to replace it.
    NeedsConfirmation(std::path::PathBuf),
}

/// Save `params` as a user preset called `name` in `dir`, under its
/// sanitized file name. An existing file is replaced only with `overwrite`;
/// otherwise nothing is written and the caller asks first. File I/O, so call
/// it from the GUI thread, never the audio thread.
pub fn save_user_preset(
    dir: &Path,
    name: &str,
    params: &CorrosionParams,
    overwrite: bool,
) -> io::Result<SaveOutcome> {
    let name = name.trim();
    if name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "enter a preset name",
        ));
    }
    let path = dir.join(format!("{}.{PRESET_EXTENSION}", sanitize_file_name(name)));
    if path.exists() && !overwrite {
        return Ok(SaveOutcome::NeedsConfirmation(path));
    }
    Preset::from_params(name, params).save(&path)?;
    Ok(SaveOutcome::Saved(path))
}

#[cfg(test)]
mod user_preset_tests {
    use super::*;
    use std::ffi::OsString;

    fn env_of<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    /// A fresh, empty folder under the system temp folder.
    pub(crate) fn temp_dir(tag: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("corrosion-{tag}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn preset_folder_per_platform() {
        use std::path::PathBuf;
        assert_eq!(
            user_preset_dir_for(
                "windows",
                env_of(&[("APPDATA", r"C:\Users\a\AppData\Roaming")])
            ),
            Some(
                PathBuf::from(r"C:\Users\a\AppData\Roaming")
                    .join("Corrosion")
                    .join("Presets")
            )
        );
        assert_eq!(
            user_preset_dir_for("macos", env_of(&[("HOME", "/Users/a")])),
            Some(PathBuf::from(
                "/Users/a/Library/Application Support/Corrosion/Presets"
            ))
        );
        assert_eq!(
            user_preset_dir_for(
                "linux",
                env_of(&[("XDG_DATA_HOME", "/data"), ("HOME", "/home/a")])
            ),
            Some(PathBuf::from("/data/corrosion/presets"))
        );
        assert_eq!(
            user_preset_dir_for("linux", env_of(&[("HOME", "/home/a")])),
            Some(PathBuf::from("/home/a/.local/share/corrosion/presets"))
        );
        assert_eq!(user_preset_dir_for("linux", env_of(&[])), None);
        assert_eq!(
            user_preset_dir_for("windows", env_of(&[("APPDATA", "")])),
            None
        );
    }

    #[test]
    fn file_names_are_sanitized() {
        for (name, file) in [
            ("My Grind", "My Grind"),
            ("a/b\\c", "a_b_c"),
            ("../../etc/passwd", "_.._etc_passwd"),
            ("what?: <x> | \"y\" *", "what__ _x_ _ _y_ _"),
            ("  .hidden.  ", "hidden"),
            ("tab\there", "tabhere"),
            ("", "Untitled"),
            ("...", "Untitled"),
            ("CON", "_CON"),
            ("nul.txt", "_nul.txt"),
            ("COM1", "_COM1"),
            ("COM0", "COM0"),
            ("Console", "Console"),
        ] {
            assert_eq!(sanitize_file_name(name), file, "name {name:?}");
        }
        assert_eq!(sanitize_file_name(&"x".repeat(300)).len(), 100);
    }

    #[test]
    fn saved_preset_reloads_every_parameter() {
        use nih_plug::prelude::Params;
        // A factory preset's parameters stand in for the current sound.
        let path = format!(
            "{}/presets/factory/beam_grind.{PRESET_EXTENSION}",
            env!("CARGO_MANIFEST_DIR")
        );
        let sound = Preset::load(path).unwrap().into_params();
        let dir = temp_dir("save");
        let saved = save_user_preset(&dir, "My Grind", &sound, false).unwrap();
        let SaveOutcome::Saved(file) = saved else {
            panic!("first save should write: {saved:?}");
        };
        assert_eq!(file, dir.join(format!("My Grind.{PRESET_EXTENSION}")));
        let reloaded = Preset::load(&file).unwrap();
        assert_eq!(reloaded.name, "My Grind");
        let live = CorrosionParams::default();
        let plain = |params: &CorrosionParams| -> std::collections::HashMap<String, f32> {
            params
                .param_map()
                .into_iter()
                // SAFETY: the pointers come from `params`, borrowed here.
                .map(|(id, ptr, _)| (id, unsafe { ptr.unmodulated_plain_value() }))
                .collect()
        };
        let (want, live_ptrs) = (plain(&sound), live.param_map());
        for (id, normalized) in reloaded.parameter_values(&live) {
            let ptr = live_ptrs.iter().find(|(pid, _, _)| *pid == id).unwrap().1;
            // SAFETY: `ptr` comes from `live`, alive for this test.
            let got = unsafe { ptr.preview_plain(normalized) };
            assert!(
                (got - want[&id]).abs() <= 1e-4 * want[&id].abs().max(1.0),
                "{id}: saved {}, reloaded {got}",
                want[&id]
            );
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn declined_overwrite_leaves_the_file_alone() {
        let dir = temp_dir("overwrite");
        let params = CorrosionParams::default();
        let SaveOutcome::Saved(file) = save_user_preset(&dir, "Keep", &params, false).unwrap()
        else {
            panic!("first save should write");
        };
        let before = fs::read(&file).unwrap();
        let louder = Preset {
            output: 3.0,
            ..Preset::from_params("Keep", &params)
        }
        .into_params();
        assert_eq!(
            save_user_preset(&dir, "Keep", &louder, false).unwrap(),
            SaveOutcome::NeedsConfirmation(file.clone())
        );
        assert_eq!(
            fs::read(&file).unwrap(),
            before,
            "declined overwrite changed the file"
        );
        assert_eq!(
            save_user_preset(&dir, "Keep", &louder, true).unwrap(),
            SaveOutcome::Saved(file.clone())
        );
        assert_ne!(
            fs::read(&file).unwrap(),
            before,
            "confirmed overwrite should write"
        );
        assert!(save_user_preset(&dir, "   ", &params, false).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn listing_skips_unreadable_files() {
        let dir = temp_dir("list");
        let mut good = Preset::from_params("Zinc Hit", &CorrosionParams::default());
        good.save(dir.join(format!("zinc.{PRESET_EXTENSION}")))
            .unwrap();
        good.name = "anvil".to_string();
        good.save(dir.join(format!("anvil.{PRESET_EXTENSION}")))
            .unwrap();
        fs::write(dir.join(format!("broken.{PRESET_EXTENSION}")), "{ not json").unwrap();
        fs::write(dir.join("notes.txt"), "not a preset").unwrap();
        let (presets, skipped) = list_user_presets(&dir);
        let names: Vec<&str> = presets.iter().map(|(_, p)| p.name.as_str()).collect();
        assert_eq!(names, ["anvil", "Zinc Hit"]);
        assert_eq!(skipped, 1);
        assert_eq!(list_user_presets(&dir.join("missing")), (Vec::new(), 0));
        fs::remove_dir_all(dir).unwrap();
    }
}

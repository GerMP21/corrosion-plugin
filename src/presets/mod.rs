use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::params::{object_param, CorrosionParams, Object};

/// Preset schema version. Bumped whenever the JSON shape changes in a way
/// that requires a migration step beyond what serde defaults can patch.
///
/// History:
/// - "1" / "2" — legacy versions (pre-`extra`); rely on serde defaults.
/// - "3" — algorithmic resonator engine, character params, quality mode.
/// - "4" — macros (Mass/Corrosion/Violence/Brightness), play_mode,
///   limiter_mode. Legacy presets get neutral defaults via the per-field
///   `#[serde(default = …)]` attributes.
pub mod factory;

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
    migrate_diode_stage(&mut value);
    value
}

/// Before `diode_asymmetry` existed, the clipper had a hard, symmetric knee
/// and Diode Softness barely moved it. A preset without the field gets
/// Diode Softness 0 and Diode Asymmetry 0, the setting closest to that,
/// whatever its old Softness (`add-asymmetric-diode-stage`). The field is
/// optional, so the schema version stays at 4.
fn migrate_diode_stage(value: &mut serde_json::Value) {
    let Some(extra) = value
        .get_mut("extra")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    if extra.contains_key("diode_asymmetry") {
        return;
    }
    extra.insert("diode_softness".into(), serde_json::json!(0.0));
    extra.insert("diode_asymmetry".into(), serde_json::json!(0.0));
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
    pub diode_asymmetry: f32,
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
            diode_asymmetry: params.diode_asymmetry.value(),
            macro_mass: params.macro_mass.value(),
            macro_corrosion: params.macro_corrosion.value(),
            macro_violence: params.macro_violence.value(),
            macro_brightness: params.macro_brightness.value(),
            limiter_mode: params.limiter_mode.value(),
        }
    }

    fn apply_to(self, params: &mut CorrosionParams) {
        params.ui_scale = crate::params::ui_scale_param(self.ui_scale);
        params.env_attack = crate::params::env_attack_param(self.env_attack);
        params.env_decay = crate::params::env_decay_param(self.env_decay);
        params.env_sustain = crate::params::env_sustain_param(self.env_sustain);
        params.env_release = crate::params::env_release_param(self.env_release);
        params.mseg_onset = crate::params::mseg_onset_param(self.mseg_onset);
        params.mseg_onset_level = crate::params::mseg_onset_level_param(self.mseg_onset_level);
        params.mseg_attack = crate::params::mseg_attack_param(self.mseg_attack);
        params.mseg_hold = crate::params::mseg_hold_param(self.mseg_hold);
        params.mseg_decay = crate::params::mseg_decay_param(self.mseg_decay);
        params.mseg_sustain = crate::params::mseg_sustain_param(self.mseg_sustain);
        params.mseg_release = crate::params::mseg_release_param(self.mseg_release);
        params.env_amount = crate::params::env_amount_param(self.env_amount);
        params.velocity_to_peak = crate::params::velocity_to_peak_param(self.velocity_to_peak);
        params.loop_mode = crate::params::loop_mode_param(self.loop_mode);
        params.loop_start_stage = crate::params::loop_start_stage_param(self.loop_start_stage);
        params.loop_end_stage = crate::params::loop_end_stage_param(self.loop_end_stage);
        params.sync_rate = crate::params::sync_rate_param(self.sync_rate.clamp(0.0, 1.0));
        params.global_time_scale = crate::params::global_time_scale_param(self.global_time_scale);
        params.velocity_to_level = crate::params::velocity_to_level_param(self.velocity_to_level);
        params.velocity_to_time = crate::params::velocity_to_time_param(self.velocity_to_time);
        params.curve_tension = crate::params::curve_tension_param(self.curve_tension);
        params.exciter_pressure =
            crate::params::exciter_pressure_param(self.exciter_pressure.clamp(0.0, 1.0));
        params.exciter_speed =
            crate::params::exciter_speed_param(self.exciter_speed.clamp(0.0, 1.0));
        params.exciter_roughness =
            crate::params::exciter_roughness_param(self.exciter_roughness.clamp(0.0, 1.0));
        params.hand_mass = crate::params::hand_mass_param(self.hand_mass);
        params.flesh_stiffness = crate::params::flesh_stiffness_param(self.flesh_stiffness);
        params.flesh_damping = crate::params::flesh_damping_param(self.flesh_damping);
        params.mute_decay = crate::params::mute_decay_param(self.mute_decay);
        params.mallet_mass = crate::params::mallet_mass_param(self.mallet_mass);
        params.felt_softness = crate::params::felt_softness_param(self.felt_softness);
        params.core_hardness = crate::params::core_hardness_param(self.core_hardness);
        params.compression_curve = crate::params::compression_curve_param(self.compression_curve);
        params.material_stiffness =
            crate::params::material_stiffness_param(self.material_stiffness);
        params.impact_damping = crate::params::impact_damping_param(self.impact_damping);
        params.stick_mass = crate::params::stick_mass_param(self.stick_mass);
        params.tip_stiffness = crate::params::tip_stiffness_param(self.tip_stiffness);
        params.restitution_bounciness =
            crate::params::restitution_bounciness_param(self.restitution_bounciness);
        params.micro_bounce_limit =
            crate::params::micro_bounce_limit_param(self.micro_bounce_limit);
        params.wire_density = crate::params::wire_density_param(self.wire_density);
        params.spread_duration = crate::params::spread_duration_param(self.spread_duration);
        params.brush_wire_stiffness =
            crate::params::brush_wire_stiffness_param(self.brush_wire_stiffness);
        params.amplitude_randomization =
            crate::params::amplitude_randomization_param(self.amplitude_randomization);
        params.pipe_mass = crate::params::pipe_mass_param(self.pipe_mass);
        params.metal_stiffness = crate::params::metal_stiffness_param(self.metal_stiffness);
        params.pipe_pitch = crate::params::pipe_pitch_param(self.pipe_pitch);
        params.pipe_ring_decay = crate::params::pipe_ring_decay_param(self.pipe_ring_decay);
        params.link_count = crate::params::link_count_param(self.link_count);
        params.chain_mass = crate::params::chain_mass_param(self.chain_mass);
        params.drop_envelope_spread =
            crate::params::drop_envelope_spread_param(self.drop_envelope_spread);
        params.internal_rattle = crate::params::internal_rattle_param(self.internal_rattle);
        params.rattle_color = crate::params::rattle_color_param(self.rattle_color);
        params.bow_pressure = crate::params::bow_pressure_param(self.bow_pressure);
        params.bow_speed = crate::params::bow_speed_param(self.bow_speed);
        params.rosin_grip = crate::params::rosin_grip_param(self.rosin_grip);
        params.slip_curve = crate::params::slip_curve_param(self.slip_curve);
        params.scrape_speed = crate::params::scrape_speed_param(self.scrape_speed);
        params.point_pressure = crate::params::point_pressure_param(self.point_pressure);
        params.chatter_pitch = crate::params::chatter_pitch_param(self.chatter_pitch);
        params.chatter_damping = crate::params::chatter_damping_param(self.chatter_damping);
        params.grind_speed = crate::params::grind_speed_param(self.grind_speed);
        params.grind_pressure = crate::params::grind_pressure_param(self.grind_pressure);
        params.surface_grit = crate::params::surface_grit_param(self.surface_grit);
        params.grit_color = crate::params::grit_color_param(self.grit_color);
        params.drag_speed = crate::params::drag_speed_param(self.drag_speed);
        params.ridge_spacing = crate::params::ridge_spacing_param(self.ridge_spacing);
        params.ridge_depth = crate::params::ridge_depth_param(self.ridge_depth);
        params.drag_exciter_mass = crate::params::drag_exciter_mass_param(self.drag_exciter_mass);
        params.pull_speed = crate::params::pull_speed_param(self.pull_speed);
        params.break_threshold = crate::params::break_threshold_param(self.break_threshold);
        params.slip_stochasticity =
            crate::params::slip_stochasticity_param(self.slip_stochasticity);
        params.creak_sharpness = crate::params::creak_sharpness_param(self.creak_sharpness);
        params.air_pressure = crate::params::air_pressure_param(self.air_pressure);
        params.nozzle_width = crate::params::nozzle_width_param(self.nozzle_width);
        params.turbulence_chaos = crate::params::turbulence_chaos_param(self.turbulence_chaos);
        params.mains_frequency = crate::params::mains_frequency_param(self.mains_frequency);
        params.coil_proximity = crate::params::coil_proximity_param(self.coil_proximity);
        params.voltage_sag = crate::params::voltage_sag_param(self.voltage_sag);
        params.pull_distance = crate::params::pull_distance_param(self.pull_distance);
        params.hook_stiffness = crate::params::hook_stiffness_param(self.hook_stiffness);
        params.snap_force = crate::params::snap_force_param(self.snap_force);
        params.flow_rate = crate::params::flow_rate_param(self.flow_rate);
        params.particle_mass = crate::params::particle_mass_param(self.particle_mass);
        params.mass_variance = crate::params::mass_variance_param(self.mass_variance);
        params.strike_position = crate::params::strike_position_param(self.strike_position);
        params.coupling_stiffness =
            crate::params::coupling_stiffness_param(self.coupling_stiffness);
        params.position_wander = crate::params::position_wander_param(self.position_wander);
        params.position_envelope = crate::params::position_envelope_param(self.position_envelope);
        params.position_wander_rate =
            crate::params::position_wander_rate_param(if self.position_wander_rate.is_finite() {
                self.position_wander_rate.clamp(0.1, 5.0)
            } else {
                0.1
            });
        params.fundamental_anchor =
            crate::params::fundamental_anchor_param(self.fundamental_anchor);
        params.res_damping = crate::params::res_damping_param(self.res_damping);
        params.res_brightness = crate::params::res_brightness_param(self.res_brightness);
        params.thickness = crate::params::thickness_param(self.thickness);
        params.heat = crate::params::heat_param(self.heat);
        params.sludge = crate::params::sludge_param(self.sludge);
        params.pipe_diameter = crate::params::pipe_diameter_param(self.pipe_diameter);
        params.plate_aspect = crate::params::plate_aspect_param(self.plate_aspect);
        params.plate_stiffness = crate::params::plate_stiffness_param(self.plate_stiffness);
        params.tank_volume = crate::params::tank_volume_param(self.tank_volume);
        params.tank_cavity_mix = crate::params::tank_cavity_mix_param(self.tank_cavity_mix);
        params.chain_link_mass = crate::params::chain_link_mass_param(self.chain_link_mass);
        params.chain_instability = crate::params::chain_instability_param(self.chain_instability);
        params.beam_shear = crate::params::beam_shear_param(self.beam_shear);
        params.cable_braid = crate::params::cable_braid_param(self.cable_braid);
        params.cable_tension_drop =
            crate::params::cable_tension_drop_param(self.cable_tension_drop);
        params.spring_dispersion = crate::params::spring_dispersion_param(self.spring_dispersion);
        params.spring_slosh = crate::params::spring_slosh_param(self.spring_slosh);
        params.sheet_thinness = crate::params::sheet_thinness_param(self.sheet_thinness);
        params.cog_dissonance = crate::params::cog_dissonance_param(self.cog_dissonance);
        params.filter_cutoff = crate::params::filter_cutoff_param(self.filter_cutoff);
        params.filter_resonance = crate::params::filter_resonance_param(self.filter_resonance);
        params.component_tolerance =
            crate::params::component_tolerance_param(self.component_tolerance);
        params.drive_amount = crate::params::drive_amount_param(self.drive_amount);
        params.bias_starvation = crate::params::bias_starvation_param(self.bias_starvation);
        params.chaos_depth = crate::params::chaos_depth_param(self.chaos_depth);
        params.spread_width = crate::params::spread_width_param(self.spread_width);
        params.listener_proximity =
            crate::params::listener_proximity_param(self.listener_proximity);
        params.chassis_material = crate::params::chassis_material_param(self.chassis_material);
        params.chassis_volume = crate::params::chassis_volume_param(self.chassis_volume);
        params.space_mode = crate::params::space_mode_param(self.space_mode);
        params.space_amount = crate::params::space_amount_param(self.space_amount);
        params.factory_size = crate::params::factory_size_param(self.factory_size);
        params.machinery_clutter = crate::params::machinery_clutter_param(self.machinery_clutter);
        params.wall_impedance = crate::params::wall_impedance_param(self.wall_impedance);
        params.spring_tension = crate::params::spring_tension_param(self.spring_tension);
        params.wire_stiffness = crate::params::wire_stiffness_param(self.wire_stiffness);
        params.spring_tank_size = crate::params::spring_tank_size_param(self.spring_tank_size);
        params.delay_time = crate::params::delay_time_param(self.delay_time);
        params.machinery_movement =
            crate::params::machinery_movement_param(self.machinery_movement);
        params.high_frequency_damping =
            crate::params::high_frequency_damping_param(self.high_frequency_damping);
        params.analog_ceiling = crate::params::analog_ceiling_param(self.analog_ceiling);
        params.diode_softness = crate::params::diode_softness_param(self.diode_softness);
        params.diode_asymmetry = crate::params::diode_asymmetry_param(self.diode_asymmetry);
        params.macro_mass = crate::params::macro_mass_param(self.macro_mass);
        params.macro_corrosion = crate::params::macro_corrosion_param(self.macro_corrosion);
        params.macro_violence = crate::params::macro_violence_param(self.macro_violence);
        params.macro_brightness = crate::params::macro_brightness_param(self.macro_brightness);
        params.limiter_mode = crate::params::limiter_mode_param(self.limiter_mode);
    }
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
        params.size = crate::params::size_param(self.size);
        params.rust = crate::params::rust_param(self.rust);
        params.damage = crate::params::damage_param(self.damage);
        params.drive = crate::params::drive_param(self.drive);
        params.output = crate::params::output_param(self.output);
        params.width = crate::params::width_param(self.width);
        params.body = crate::params::body_param(self.body);
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

/// A user preset and the folder it is in.
#[derive(Debug, Clone, PartialEq)]
pub struct UserPreset {
    pub path: std::path::PathBuf,
    /// The folder relative to the user preset folder, `/`-separated; empty
    /// for the folder's root.
    pub folder: String,
    pub preset: Preset,
}

/// How deep `list_user_presets` and `list_user_folders` look below the
/// user preset folder.
pub const USER_FOLDER_DEPTH: usize = 4;

/// Whether a directory entry name is hidden (starts with a dot).
fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

/// Walk `dir` (`folder` relative to the user preset folder, `depth` levels
/// down), collecting presets, unreadable preset files and folders.
fn walk_user_folder(
    dir: &Path,
    folder: &str,
    depth: usize,
    presets: &mut Vec<UserPreset>,
    skipped: &mut usize,
    folders: &mut Vec<String>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    folders.push(folder.to_string());
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if is_hidden(&name) {
            continue;
        }
        if path.is_dir() {
            if depth < USER_FOLDER_DEPTH {
                let sub = if folder.is_empty() {
                    name.to_string_lossy().into_owned()
                } else {
                    format!("{folder}/{}", name.to_string_lossy())
                };
                walk_user_folder(&path, &sub, depth + 1, presets, skipped, folders);
            }
            continue;
        }
        if path.extension().is_none_or(|ext| ext != PRESET_EXTENSION) {
            continue;
        }
        match Preset::load(&path) {
            Ok(preset) => presets.push(UserPreset {
                path,
                folder: folder.to_string(),
                preset,
            }),
            Err(_) => *skipped += 1,
        }
    }
}

/// Folder paths sort with the root first, then ignoring case.
fn folder_key(folder: &str) -> (bool, String) {
    (!folder.is_empty(), folder.to_lowercase())
}

/// The user presets in `dir` and its subfolders (up to
/// [`USER_FOLDER_DEPTH`] levels; hidden folders and files are skipped),
/// sorted by folder and then by display name (ignoring case), and how many
/// preset files were skipped because they couldn't be read. A missing
/// folder is simply empty.
pub fn list_user_presets(dir: &Path) -> (Vec<UserPreset>, usize) {
    let (mut presets, mut skipped, mut folders) = (Vec::new(), 0, Vec::new());
    walk_user_folder(dir, "", 0, &mut presets, &mut skipped, &mut folders);
    presets.sort_by_key(|p| (folder_key(&p.folder), p.preset.name.to_lowercase()));
    (presets, skipped)
}

/// The folders a preset can be saved into: the user preset folder (`""`)
/// and its subfolders, as [`list_user_presets`] walks them, root first.
/// Never anything outside `dir`, so never the factory bank.
pub fn list_user_folders(dir: &Path) -> Vec<String> {
    let (mut presets, mut skipped, mut folders) = (Vec::new(), 0, Vec::new());
    walk_user_folder(dir, "", 0, &mut presets, &mut skipped, &mut folders);
    if folders.is_empty() {
        folders.push(String::new());
    }
    folders.sort_by_key(|f| folder_key(f));
    folders
}

/// `folder` (relative, `/`-separated) under `dir`, or an error if it would
/// leave `dir`: only plain, non-hidden names, at most
/// [`USER_FOLDER_DEPTH`] deep.
fn user_subfolder(dir: &Path, folder: &str) -> io::Result<std::path::PathBuf> {
    let refuse = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("\"{folder}\" is not a folder inside the user preset folder"),
        )
    };
    let mut path = dir.to_path_buf();
    if folder.is_empty() {
        return Ok(path);
    }
    let parts: Vec<&str> = folder.split('/').collect();
    if parts.len() > USER_FOLDER_DEPTH {
        return Err(refuse());
    }
    for part in parts {
        let plain = !part.is_empty()
            && !part.starts_with('.')
            && !part.contains(['\\', ':'])
            && Path::new(part)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_)));
        if !plain {
            return Err(refuse());
        }
        path.push(part);
    }
    Ok(path)
}

/// The program and arguments that show `path` in the file manager on `os`
/// (`std::env::consts::OS`): `explorer` on Windows, `open` on macOS,
/// `xdg-open` elsewhere.
pub fn open_folder_command(os: &str, path: &Path) -> (String, Vec<std::ffi::OsString>) {
    let program = match os {
        "windows" => "explorer",
        "macos" => "open",
        _ => "xdg-open",
    };
    (program.to_string(), vec![path.as_os_str().to_owned()])
}

/// Create `path` if it is missing and show it in the file manager. The
/// command runs detached (its exit status is not awaited), so a slow file
/// manager can't block the editor. GUI thread only.
pub fn open_folder(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    let (program, args) = open_folder_command(std::env::consts::OS, path);
    std::process::Command::new(&program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|e| io::Error::new(e.kind(), format!("couldn't run {program}: {e}")))
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

/// Save `params` as a user preset called `name` in `folder` (relative,
/// `/`-separated, `""` for the root) of the user preset folder `dir`, under
/// its sanitized file name. A `folder` that would leave `dir` is refused. An existing file is replaced only with `overwrite`;
/// otherwise nothing is written and the caller asks first. File I/O, so call
/// it from the GUI thread, never the audio thread.
pub fn save_user_preset(
    dir: &Path,
    folder: &str,
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
    let path = user_subfolder(dir, folder)?
        .join(format!("{}.{PRESET_EXTENSION}", sanitize_file_name(name)));
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
            "{}/tests/fixtures/presets/legacy/beam_grind.{PRESET_EXTENSION}",
            env!("CARGO_MANIFEST_DIR")
        );
        let sound = Preset::load(path).unwrap().into_params();
        let dir = temp_dir("save");
        let saved = save_user_preset(&dir, "", "My Grind", &sound, false).unwrap();
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
        let SaveOutcome::Saved(file) = save_user_preset(&dir, "", "Keep", &params, false).unwrap()
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
            save_user_preset(&dir, "", "Keep", &louder, false).unwrap(),
            SaveOutcome::NeedsConfirmation(file.clone())
        );
        assert_eq!(
            fs::read(&file).unwrap(),
            before,
            "declined overwrite changed the file"
        );
        assert_eq!(
            save_user_preset(&dir, "", "Keep", &louder, true).unwrap(),
            SaveOutcome::Saved(file.clone())
        );
        assert_ne!(
            fs::read(&file).unwrap(),
            before,
            "confirmed overwrite should write"
        );
        assert!(save_user_preset(&dir, "", "   ", &params, false).is_err());
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
        let names: Vec<&str> = presets.iter().map(|p| p.preset.name.as_str()).collect();
        assert_eq!(names, ["anvil", "Zinc Hit"]);
        assert_eq!(skipped, 1);
        assert_eq!(list_user_presets(&dir.join("missing")), (Vec::new(), 0));
        fs::remove_dir_all(dir).unwrap();
    }

    /// Write a default preset called `name` at `relative` under `dir`.
    fn write_preset(dir: &Path, relative: &str, name: &str) {
        let path = dir.join(relative);
        Preset::from_params(name, &CorrosionParams::default())
            .save(path)
            .unwrap();
    }

    #[test]
    fn listing_walks_subfolders() {
        let dir = temp_dir("tree");
        write_preset(&dir, &format!("root.{PRESET_EXTENSION}"), "Root Hit");
        write_preset(&dir, &format!("Basses/deep.{PRESET_EXTENSION}"), "Deep Sub");
        write_preset(&dir, &format!("Basses/Old/worn.{PRESET_EXTENSION}"), "Worn");
        // Hidden folders and folders deeper than USER_FOLDER_DEPTH are skipped.
        write_preset(&dir, &format!(".trash/gone.{PRESET_EXTENSION}"), "Gone");
        write_preset(
            &dir,
            &format!("a/b/c/d/e/deep.{PRESET_EXTENSION}"),
            "Too Deep",
        );
        write_preset(&dir, &format!("a/b/c/d/ok.{PRESET_EXTENSION}"), "Four Down");
        fs::write(dir.join(format!("Basses/broken.{PRESET_EXTENSION}")), "{").unwrap();
        let (presets, skipped) = list_user_presets(&dir);
        let listed: Vec<(&str, &str)> = presets
            .iter()
            .map(|p| (p.folder.as_str(), p.preset.name.as_str()))
            .collect();
        assert_eq!(
            listed,
            [
                ("", "Root Hit"),
                ("a/b/c/d", "Four Down"),
                ("Basses", "Deep Sub"),
                ("Basses/Old", "Worn"),
            ]
        );
        assert_eq!(skipped, 1);
        assert_eq!(
            list_user_folders(&dir),
            ["", "a", "a/b", "a/b/c", "a/b/c/d", "Basses", "Basses/Old"]
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saving_into_a_subfolder() {
        let dir = temp_dir("subfolder");
        fs::create_dir_all(dir.join("Basses")).unwrap();
        let params = CorrosionParams::default();
        let saved = save_user_preset(&dir, "Basses", "Deep Sub", &params, false).unwrap();
        let file = dir
            .join("Basses")
            .join(format!("Deep Sub.{PRESET_EXTENSION}"));
        assert_eq!(saved, SaveOutcome::Saved(file.clone()));
        assert!(file.exists());
        let (presets, _) = list_user_presets(&dir);
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].folder, "Basses");
        assert_eq!(presets[0].preset.name, "Deep Sub");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saving_never_leaves_the_user_folder() {
        // The folder selector only offers user folders; a folder that would
        // leave the user preset folder (or a hidden one) is refused anyway,
        // so the factory bank can never be a target.
        let dir = temp_dir("escape");
        let params = CorrosionParams::default();
        for folder in [
            "..",
            "../factory",
            "/tmp",
            "Basses/../..",
            ".hidden",
            "a\\..\\..",
        ] {
            assert!(
                save_user_preset(&dir, folder, "Escape", &params, false).is_err(),
                "saving into {folder:?} should be refused"
            );
        }
        assert!(list_user_presets(&dir).0.is_empty());
        assert!(!dir
            .parent()
            .unwrap()
            .join(format!("Escape.{PRESET_EXTENSION}"))
            .exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn open_folder_command_per_platform() {
        let path = Path::new("/home/me/presets");
        let command = |os| {
            let (program, args) = open_folder_command(os, path);
            (program, args)
        };
        assert_eq!(
            command("windows"),
            ("explorer".to_string(), vec![path.into()])
        );
        assert_eq!(command("macos"), ("open".to_string(), vec![path.into()]));
        assert_eq!(
            command("linux"),
            ("xdg-open".to_string(), vec![path.into()])
        );
        assert_eq!(
            command("freebsd"),
            ("xdg-open".to_string(), vec![path.into()])
        );
    }
}

//! Does each control change the sound? (`fix-inert-controls`.)
//!
//! Every sound-shaping parameter is played at its minimum, middle and
//! maximum on a patch where it applies: the guide clip's patch for it (its
//! exciter or object, the mode that shows it), plus the context it needs to
//! act ([`context`]: the friction exciter for the MSEG and the gesture
//! modifiers, the drive engaged for the Overdrive Unit, the echo on for its
//! controls, two velocities for velocity controls, and so on). A control
//! acts when some pair of its three points differs by at least the floor in
//! level, ring length, octave-band shape or stereo width ([`Response`]).
//! Three points, not two: a control can be symmetric, like Strike Position,
//! whose ends are mirror images with the same modes.

use super::clip_plan::{
    guide_base_patch, parameter_plans, point_value, with_values, ParamPlan, Point, NOTE,
    SAMPLE_RATE,
};
use crate::offline::{render_preset_at_tempo, OfflineNote};
use crate::params::CorrosionParams;
use nih_plug::prelude::Params;

/// The spec's floor: 1 dB of level, 10 % of ring length, 1 dB of band shape
/// beyond the level change, or 1 dB of stereo width.
pub const LEVEL_FLOOR_DB: f32 = 1.0;
/// Ring length ratio floor (10 %).
pub const RING_FLOOR: f32 = 0.10;
/// Band shape floor, dB beyond the level change.
pub const BANDS_FLOOR_DB: f32 = 1.0;
/// Stereo width floor, dB of side over mid.
pub const WIDTH_FLOOR_DB: f32 = 1.0;
/// Envelope floor: mean dB change of the 10 ms levels beyond the level
/// change, so a control that only moves the sound in time (a wander rate,
/// a drop spread) shows.
pub const ENVELOPE_FLOOR_DB: f32 = 1.0;

/// Parameters that don't shape the sound of a note, and why.
pub const NOT_SOUND: &[(&str, &str)] = &[
    ("ui_scale", "sizes the editor window"),
    (
        "quality_mode",
        "trades CPU for detail; measured at Render quality",
    ),
    (
        "play_mode",
        "chooses how keys map to notes (Tonal, Kit, Drone)",
    ),
    ("limiter_mode", "only acts on peaks over the ceiling"),
];

/// What a control needs around it to act, beyond its clip's patch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    /// Parameters set by ID to plain values.
    pub values: Vec<(&'static str, f32)>,
    /// The note's velocities, 0..1: one note each, rendered apart.
    pub velocities: Vec<f32>,
    /// How long the key is held, in seconds.
    pub hold: f32,
    /// The host tempo, for tempo-synced controls.
    pub tempo: Option<f32>,
}

/// The context `id` is measured in.
pub fn context(id: &str) -> Context {
    let bow = ("exciter", 1.0);
    let jet = ("exciter", 13.0);
    let echo = [("space_mode", 3.0), ("space_amount", 0.7)];
    let values: Vec<(&'static str, f32)> = match id {
        // The six gesture modifiers and the global Speed and Roughness act
        // on the friction exciters (Hand Strike ignores them).
        "env_amount" | "velocity_to_peak" | "global_time_scale" | "velocity_to_level"
        | "velocity_to_time" | "curve_tension" | "exciter_speed" | "exciter_roughness"
        | "position_envelope" => vec![bow],
        // The MSEG drives only the friction exciters.
        "mseg_release" => vec![bow],
        // Release shows for the struck and the specialty exciters; the
        // clip's patch is the struck one, so measure the specialty one.
        "env_release" => vec![jet],
        "bias_starvation" | "chaos_depth" => vec![("drive_amount", 3.0)],
        // The diode clips at the ceiling ÷ 1.3: drive the chain into it.
        "diode_softness" | "diode_asymmetry" => {
            vec![
                ("drive", 4.0),
                ("drive_amount", 3.0),
                ("analog_ceiling", 0.3),
            ]
        }
        "space_mode" => vec![("space_amount", 0.7)],
        "component_tolerance" => vec![("filter_cutoff", 1500.0), ("filter_resonance", 0.6)],
        "sync_rate" | "delay_time" | "machinery_movement" | "high_frequency_damping" => {
            echo.to_vec()
        }
        // The strike point only matters while the exciter touches the
        // object: a strike lands once, a bow keeps touching.
        "position_wander_rate" => vec![bow, ("position_wander", 0.5)],
        // The anchor only lifts the fundamental near one of its nodes.
        "fundamental_anchor" => vec![("strike_position", 0.02)],
        _ => Vec::new(),
    };
    let velocities = match id {
        "velocity_to_peak" | "velocity_to_level" | "velocity_to_time" | "compression_curve"
        | "core_hardness" => vec![0.25, 1.0],
        _ => vec![100.0 / 127.0],
    };
    let hold = match id {
        "position_wander" | "position_wander_rate" => 3.0,
        _ => 2.0,
    };
    let tempo = matches!(id, "sync_rate").then_some(120.0);
    Context {
        values,
        velocities,
        hold,
        tempo,
    }
}

/// One rendered point's measurements.
#[derive(Clone, Debug, PartialEq)]
pub struct Features {
    /// RMS of the whole render, dB.
    pub level_db: f32,
    /// Time the render stays within 40 dB of its loudest 10 ms, seconds.
    pub ring: f32,
    /// Octave-band levels of the first second, 63 Hz to 16 kHz, dB.
    pub bands: Vec<f32>,
    /// Side over mid, dB.
    pub width_db: f32,
    /// Levels of successive 10 ms windows, dB.
    pub envelope: Vec<f32>,
}

/// How far apart two points are, in each measure.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Response {
    pub level_db: f32,
    pub ring: f32,
    pub bands_db: f32,
    pub width_db: f32,
    pub envelope_db: f32,
}

impl Response {
    /// Whether any measure reaches its floor.
    pub fn acts(&self) -> bool {
        self.level_db >= LEVEL_FLOOR_DB
            || self.ring >= RING_FLOOR
            || self.bands_db >= BANDS_FLOOR_DB
            || self.width_db >= WIDTH_FLOOR_DB
            || self.envelope_db >= ENVELOPE_FLOOR_DB
    }

    /// The larger of each measure.
    fn max(self, other: Response) -> Response {
        Response {
            level_db: self.level_db.max(other.level_db),
            ring: self.ring.max(other.ring),
            bands_db: self.bands_db.max(other.bands_db),
            width_db: self.width_db.max(other.width_db),
            envelope_db: self.envelope_db.max(other.envelope_db),
        }
    }
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

/// In-place radix-2 FFT of `re`/`im` (length a power of two).
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -2.0 * std::f32::consts::PI / len as f32;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = (angle * k as f32).sin_cos();
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * c - im[b] * s, re[b] * s + im[b] * c);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

/// Measure one render.
pub fn features(frames: &[(f32, f32)]) -> Features {
    let mono: Vec<f32> = frames.iter().map(|(l, r)| 0.5 * (l + r)).collect();
    let side: Vec<f32> = frames.iter().map(|(l, r)| 0.5 * (l - r)).collect();
    let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt();
    let windows: Vec<f32> = mono.chunks(480).map(rms).collect();
    let loudest = windows.iter().copied().fold(0.0, f32::max);
    let last = windows
        .iter()
        .rposition(|w| *w > loudest * 0.01)
        .unwrap_or(0);

    const N: usize = 4096;
    let mut bands = [0.0f32; 9];
    let first_second = &mono[..mono.len().min(SAMPLE_RATE as usize)];
    for frame in first_second.as_chunks::<N>().0 {
        let mut re = frame.to_vec();
        let mut im = vec![0.0; N];
        fft(&mut re, &mut im);
        for (b, energy) in bands.iter_mut().enumerate() {
            let lo = 44.0 * 2f32.powi(b as i32);
            let (k0, k1) = (
                (lo * N as f32 / SAMPLE_RATE as f32) as usize,
                (2.0 * lo * N as f32 / SAMPLE_RATE as f32) as usize,
            );
            *energy += (k0.max(1)..k1)
                .map(|k| re[k] * re[k] + im[k] * im[k])
                .sum::<f32>();
        }
    }
    Features {
        level_db: db(rms(&mono)),
        ring: (last + 1) as f32 * 0.01,
        bands: bands.iter().map(|e| 10.0 * e.max(1e-12).log10()).collect(),
        width_db: db(rms(&side)) - db(rms(&mono)),
        envelope: windows.iter().map(|w| db(*w)).collect(),
    }
}

/// Mean change of the 10 ms levels beyond the level change, over the
/// windows where either render is within 40 dB of its loudest.
fn envelope_change(a: &Features, b: &Features) -> f32 {
    let top = |f: &Features| f.envelope.iter().copied().fold(f32::MIN, f32::max);
    let (floor_a, floor_b) = (top(a) - 40.0, top(b) - 40.0);
    let shift = a.level_db - b.level_db;
    let diffs: Vec<f32> = a
        .envelope
        .iter()
        .zip(&b.envelope)
        .filter(|(x, y)| **x > floor_a || **y > floor_b)
        .map(|(x, y)| ((x.max(floor_a) - y.max(floor_b)) - shift).abs())
        .collect();
    diffs.iter().sum::<f32>() / diffs.len().max(1) as f32
}

/// How far apart two points' measurements are.
pub fn compare(a: &Features, b: &Features) -> Response {
    let level = (a.level_db - b.level_db).abs();
    let shape = a
        .bands
        .iter()
        .zip(&b.bands)
        .map(|(x, y)| ((x - y) - (a.level_db - b.level_db)).abs())
        .sum::<f32>()
        / a.bands.len().max(1) as f32;
    let ring = (a.ring - b.ring).abs() / a.ring.max(b.ring).max(0.01);
    Response {
        level_db: level,
        ring,
        bands_db: shape,
        width_db: (a.width_db - b.width_db).abs(),
        envelope_db: envelope_change(a, b),
    }
}

/// Each point's measurements for `plan`'s parameter, one entry per
/// velocity, on its clip patch in its [`context`].
pub fn measure_points(plan: &ParamPlan) -> Vec<Vec<Features>> {
    let params = CorrosionParams::default();
    let map = params.param_map();
    let ptr = map
        .iter()
        .find(|(id, _, _)| *id == plan.id)
        .map(|(_, ptr, _)| *ptr)
        .expect("planned IDs are parameters");
    let context = context(&plan.id);
    let base = guide_base_patch();
    Point::ALL
        .iter()
        .map(|&point| {
            // The clip's setup, the context, then the point's value on top.
            let mut values: Vec<(&str, f32)> = plan.setup.clone();
            values.extend(context.values.iter().copied());
            values.push((plan.id.as_str(), point_value(&params, &ptr, point)));
            let preset = with_values(&base, &values);
            context
                .velocities
                .iter()
                .map(|&velocity| {
                    let notes = [
                        OfflineNote::on(0, NOTE, velocity),
                        OfflineNote::off((context.hold * SAMPLE_RATE as f32) as usize, NOTE),
                    ];
                    features(&render_preset_at_tempo(
                        &preset,
                        &notes,
                        SAMPLE_RATE,
                        context.hold + 2.0,
                        context.tempo,
                    ))
                })
                .collect()
        })
        .collect()
}

/// The largest difference between any two of `points`. With two
/// velocities, a velocity control also counts when it changes the gap
/// between the soft and the hard note.
pub fn response(points: &[Vec<Features>]) -> Response {
    let mut worst = Response::default();
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            for (a, b) in points[i].iter().zip(&points[j]) {
                worst = worst.max(compare(a, b));
            }
            if let ([soft_a, hard_a], [soft_b, hard_b]) = (&points[i][..], &points[j][..]) {
                let gap = |s: &Features, h: &Features| h.level_db - s.level_db;
                let ring_gap = |s: &Features, h: &Features| h.ring / s.ring.max(0.01);
                worst = worst.max(Response {
                    level_db: (gap(soft_a, hard_a) - gap(soft_b, hard_b)).abs(),
                    ring: (ring_gap(soft_a, hard_a) - ring_gap(soft_b, hard_b)).abs(),
                    ..Response::default()
                });
            }
        }
    }
    worst
}

/// Every sound-shaping parameter's response, in plan order.
pub fn responses() -> Vec<(String, Response)> {
    parameter_plans()
        .into_iter()
        .filter(|p| !NOT_SOUND.iter().any(|(id, _)| *id == p.id))
        .map(|p| {
            let response = response(&measure_points(&p));
            (p.id, response)
        })
        .collect()
}

#[cfg(test)]
mod tests;

//! The guide's audio clips, as a plan (`add-user-guide` Decision 7).
//!
//! [`plan`] lists every clip: three per sound-shaping parameter over a fixed
//! base patch, the exciter and object comparison sets, a representative
//! factory preset per exciter and object, the drum kits and the home page's
//! demos. `render_guide_clips` renders them through the plugin's own output
//! path ([`render_clip`]), levels each set and writes the WAVs and the
//! manifest; `scripts/build-guide-clips.sh` encodes them to Opus.
//!
//! [`fingerprint`] hashes the plan, so the manifest test fails when the plan
//! changes and the clips weren't regenerated. It can't see engine changes:
//! regenerate after a change that moves the sound engine-wide.

use crate::gui::controls::{
    exciter_panel, exciter_param_ref, resonator_panel, resonator_param_ref, Shown, FIXED_CONTROLS,
};
use crate::offline::{render_preset_at_tempo, OfflineNote};
use crate::params::{exciter_model_items, CorrosionParams, ExciterFamily, ExciterType, Object};
use crate::presets::{factory, Preset};
use nih_plug::prelude::{FloatParam, ParamPtr, Params};
use serde_json::Value;

/// Every clip renders at this rate, in Render quality.
pub const SAMPLE_RATE: u32 = 48_000;
/// C3 in the plugin's naming.
pub const NOTE: u8 = 48;
/// MIDI velocity of the plain clips.
pub const VELOCITY: f32 = 100.0;
/// A plain clip's length and note-off, in seconds.
pub const SECONDS: f32 = 3.0;
pub const NOTE_OFF: f32 = 2.0;
/// Every clip renders this long past its last note-off; `trim_to_silence`
/// then cuts it where its tail falls silent (maintainer, 2026-11-13).
pub const RELEASE_TAIL: f32 = 8.0;
/// A clip's tail ends at its last sample above this, relative to its peak.
pub const SILENCE_BELOW_PEAK_DB: f32 = -60.0;
/// The fade at the cut, in seconds.
pub const TRIM_FADE: f32 = 0.05;
/// The level a set's reference clip is brought to.
pub const TARGET_RMS: f32 = factory::TARGET_RMS;
/// No clip's peak goes above this, in dBFS.
pub const PEAK_CEILING_DB: f32 = -1.0;
/// Under this RMS, in dBFS, a clip counts as silent.
pub const SILENCE_DB: f32 = -60.0;

/// The object every exciter plays in the exciter comparison set (chosen by
/// ear by the maintainer, task 4.2).
pub const COMPARISON_OBJECT: Object = Object::Tank;
/// The exciter every object is played with in the object comparison set
/// (chosen by ear by the maintainer, task 4.2).
pub const COMPARISON_EXCITER: ExciterType = ExciterType::FeltMallet;

/// How a parameter is demonstrated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Demo {
    /// Minimum, middle and maximum, one plain note each.
    Triplet,
    /// Minimum, middle and maximum, each clip a soft then a hard note.
    VelocityPair,
    /// Minimum, middle and maximum, each clip one long note.
    LongNote,
    /// Minimum, middle and maximum at a set host tempo.
    Tempo { bpm: f32 },
    /// No clips, with the reason the guide gives.
    Excluded { reason: &'static str },
}

/// One parameter's entry in the plan.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamPlan {
    pub id: String,
    pub demo: Demo,
    /// Other parameters set for this demo: the model that shows a slot, the
    /// mode that shows a control.
    pub setup: Vec<(&'static str, f32)>,
    /// Points expected to be near silent, with the reason.
    pub quiet: Vec<(Point, &'static str)>,
}

/// A parameter clip's point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    Min,
    Mid,
    Max,
}

impl Point {
    pub const ALL: [Point; 3] = [Point::Min, Point::Mid, Point::Max];
    pub fn key(self) -> &'static str {
        match self {
            Point::Min => "min",
            Point::Mid => "mid",
            Point::Max => "max",
        }
    }
}

/// One clip to render.
#[derive(Clone, Debug)]
pub struct ClipSpec {
    /// What pages reference: `param:<id>:<point>` or `set:<set>:<item>`.
    pub reference: String,
    /// Path under `web/guide/public/clips/`, without the extension.
    pub file: String,
    /// Clips sharing one gain.
    pub set: String,
    /// The set's reference clip, brought to [`TARGET_RMS`].
    pub is_reference: bool,
    /// What it plays, for the manifest.
    pub source: String,
    pub preset: Preset,
    pub notes: Vec<OfflineNote>,
    pub seconds: f32,
    pub tempo_bpm: Option<f32>,
    /// Why it may be near silent, if it may.
    pub quiet: Option<&'static str>,
}

/// Parameters with no clips, and why (the guide shows the reason).
const EXCLUDED: &[(&str, &str)] = &[
    ("ui_scale", "Sizes the editor window; it doesn't change the sound."),
    (
        "quality_mode",
        "Every clip renders in Render quality; the modes trade CPU for detail rather than change the character.",
    ),
    (
        "play_mode",
        "The note modes change what keys do; the Kit page has the factory kits' clips instead.",
    ),
    (
        "limiter_mode",
        "Hard and Lookahead differ only on peaks over the ceiling, which the levelled clips stay under.",
    ),
    ("exciter", "Heard in the exciter comparison set."),
    ("object", "Heard in the object comparison set."),
    (
        "space_mode",
        "Each mode is heard on its own controls' clips, which render with that mode on.",
    ),
];

/// Points expected to be near silent, with the reason the guide gives.
const QUIET: &[(&str, Point, &str)] = &[
    (
        "air_pressure",
        Point::Min,
        "With no air pressure, Pneumatic Jet barely moves the object.",
    ),
    ("output", Point::Min, "Output's minimum mutes the plugin."),
    (
        "coil_proximity",
        Point::Min,
        "With the coil at its farthest, the hum doesn't reach the object.",
    ),
    (
        "internal_rattle",
        Point::Min,
        "With no rattle, Metal Chain's drop is almost silent (−62 dBFS); in docs/BACKLOG.md.",
    ),
];

/// Parameters whose effect needs two velocities to be heard.
const VELOCITY_PAIRS: &[&str] = &["velocity_to_peak", "velocity_to_level", "velocity_to_time"];
/// Parameters whose effect needs a long note.
const LONG_NOTES: &[&str] = &["position_wander", "position_wander_rate"];
/// How a home page demo plays its preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoPhrase {
    /// The root, its fifth, its octave and the fifth again, half a second each.
    Notes,
    /// The same phrase as major triads.
    Triads,
    /// The root held for the whole clip.
    Held,
}

/// The home page demos, as (folder, preset, root key, phrase). Chosen by the
/// maintainer (2026-11-13): bass an octave under the guide's note, lead from
/// middle C up, chords high as triads, a held drone and percussion around the
/// guide's note (C3).
pub const HOME_DEMOS: &[(&str, &str, u8, DemoPhrase)] = &[
    ("bass", "Low Sheet Bow", NOTE - 12, DemoPhrase::Notes),
    ("lead", "Driven Pipe Lead", NOTE + 12, DemoPhrase::Notes),
    ("chords", "Glass Cable Choir", NOTE + 24, DemoPhrase::Triads),
    ("drones", "Spring Bow Drone", NOTE, DemoPhrase::Held),
    ("percussion", "Cog Tick", NOTE, DemoPhrase::Notes),
];

/// The note events of a home page demo.
fn demo_notes(root: u8, phrase: DemoPhrase) -> Vec<OfflineNote> {
    let step = (SAMPLE_RATE as f32 * 0.5) as usize;
    let velocity = VELOCITY / 127.0;
    let mut notes = Vec::new();
    if phrase == DemoPhrase::Held {
        notes.push(OfflineNote::on(0, root, velocity));
        notes.push(OfflineNote::off(step * 7, root));
        return notes;
    }
    let chord: &[u8] = if phrase == DemoPhrase::Triads {
        &[0, 4, 7]
    } else {
        &[0]
    };
    for (i, base) in [root, root + 7, root + 12, root + 7]
        .into_iter()
        .enumerate()
    {
        for &interval in chord {
            let key = base + interval;
            notes.push(OfflineNote::on(i * step, key, velocity));
            notes.push(OfflineNote::off(i * step + step * 9 / 10, key));
        }
    }
    notes.sort_by_key(|n| n.frame);
    notes
}

/// The tempo of the tempo-synced demos.
const DEMO_BPM: f32 = 120.0;

/// The exciter that shows each family's envelope controls.
fn family_exciter(family: ExciterFamily) -> ExciterType {
    match family {
        ExciterFamily::Hit => ExciterType::HandStrike,
        ExciterFamily::Friction => ExciterType::Bow,
        ExciterFamily::Specialty => ExciterType::PneumaticJet,
    }
}

/// The patch every parameter clip starts from: the default patch (Hand
/// Strike) on Plate, with Space off, in Render quality. Chosen by ear by the
/// maintainer from four candidates (task 4.2, candidate D).
pub fn guide_base_patch() -> Preset {
    with_values(
        &Preset::from_params("Guide base", &CorrosionParams::default()),
        &[
            ("object", Object::Plate.to_int() as f32),
            ("space_mode", 0.0),
            ("quality_mode", 3.0),
        ],
    )
}

/// `preset` with parameters set by ID to plain values, through the preset
/// loader (as kit pads do), so values clamp like a loaded file.
pub fn with_values(preset: &Preset, values: &[(&str, f32)]) -> Preset {
    let defaults = CorrosionParams::default();
    let ints: Vec<String> = defaults
        .param_map()
        .into_iter()
        .filter(|(_, p, _)| matches!(p, ParamPtr::IntParam(_) | ParamPtr::EnumParam(_)))
        .map(|(id, _, _)| id)
        .collect();
    let mut json = serde_json::to_value(preset).expect("presets serialize");
    for &(id, value) in values {
        let number = if id == "object" {
            serde_json::to_value(Object::from_int(value.round() as i32)).expect("object")
        } else if ints.iter().any(|i| i == id) {
            Value::from(value.round() as i64)
        } else {
            Value::from(f64::from(value))
        };
        if json.get(id).is_some() {
            json[id] = number;
        } else {
            json["extra"][id] = number;
        }
    }
    Preset::from_json_str(&json.to_string()).expect("edited presets parse")
}

/// Where a model slot's parameter lives: the first model (editor order) that
/// shows it, and the slot's slider range.
fn slot_of(params: &CorrosionParams, ptr: &ParamPtr) -> Option<((&'static str, f32), f32, f32)> {
    let same =
        |param: &FloatParam| matches!(ptr, ParamPtr::FloatParam(f) if std::ptr::eq(*f, param));
    for &(value, _) in exciter_model_items() {
        for spec in exciter_panel(ExciterType::from_int(value)).controls {
            if same(exciter_param_ref(params, spec.id)) {
                return Some((("exciter", value as f32), spec.min, spec.max));
            }
        }
    }
    for value in 0..9 {
        for spec in resonator_panel(Object::from_int(value)).controls {
            if same(resonator_param_ref(params, spec.id)) {
                return Some((("object", value as f32), spec.min, spec.max));
            }
        }
    }
    None
}

/// The settings that make a fixed control audible.
fn fixed_setup(id: &str) -> Vec<(&'static str, f32)> {
    let mut setup = Vec::new();
    match id {
        "space_amount" => setup.push(("space_mode", 1.0)),
        // The loop's stages only matter with the loop on (Forward).
        "loop_start_stage" | "loop_end_stage" => setup.push(("loop_mode", 1.0)),
        _ => {}
    }
    for control in FIXED_CONTROLS.iter().filter(|c| c.id == id).take(1) {
        match control.shown {
            Shown::Always => {}
            Shown::Family(family) => {
                setup.push(("exciter", family_exciter(family).to_int() as f32));
            }
            Shown::SpaceMode(mode) => {
                setup.push(("space_mode", mode as f32));
                setup.push(("space_amount", 0.5));
            }
        }
    }
    if id.starts_with("loop_") {
        setup.push(("exciter", ExciterType::Bow.to_int() as f32));
    }
    setup
}

/// Every parameter's plan entry, in parameter-map order.
pub fn parameter_plans() -> Vec<ParamPlan> {
    let params = CorrosionParams::default();
    params
        .param_map()
        .into_iter()
        .map(|(id, ptr, _)| {
            let demo = if let Some(&(_, reason)) = EXCLUDED.iter().find(|(e, _)| *e == id) {
                Demo::Excluded { reason }
            } else if VELOCITY_PAIRS.contains(&id.as_str()) {
                Demo::VelocityPair
            } else if LONG_NOTES.contains(&id.as_str()) {
                Demo::LongNote
            } else if id == "sync_rate" {
                Demo::Tempo { bpm: DEMO_BPM }
            } else {
                Demo::Triplet
            };
            let setup = match slot_of(&params, &ptr) {
                Some((model, _, _)) => vec![model],
                None if id == "sync_rate" || id == "delay_time" => {
                    vec![("space_mode", 3.0), ("space_amount", 0.5)]
                }
                None => fixed_setup(&id),
            };
            let quiet = QUIET
                .iter()
                .filter(|(q, _, _)| *q == id)
                .map(|&(_, point, reason)| (point, reason))
                .collect();
            ParamPlan {
                id,
                demo,
                setup,
                quiet,
            }
        })
        .collect()
}

/// A parameter's plain value at a point: the host range's ends and middle
/// (in normalized travel), or a slot's slider range.
fn point_value(params: &CorrosionParams, ptr: &ParamPtr, point: Point) -> f32 {
    // SAFETY: `ptr` points into `params`, alive for this call.
    unsafe {
        let (lo, hi) = match slot_of(params, ptr) {
            Some((_, min, max)) => (ptr.preview_normalized(min), ptr.preview_normalized(max)),
            None => (0.0, 1.0),
        };
        let n = match point {
            Point::Min => lo,
            Point::Mid => (lo + hi) / 2.0,
            Point::Max => hi,
        };
        ptr.preview_plain(n)
    }
}

fn plain_note(note_off: f32, note: u8, velocity: f32) -> Vec<OfflineNote> {
    vec![
        OfflineNote::on(0, note, velocity / 127.0),
        OfflineNote::off((SAMPLE_RATE as f32 * note_off) as usize, note),
    ]
}

fn notes_for(demo: Demo) -> (Vec<OfflineNote>, f32) {
    let frame = |s: f32| (SAMPLE_RATE as f32 * s) as usize;
    match demo {
        Demo::VelocityPair => (
            vec![
                OfflineNote::on(0, NOTE, 40.0 / 127.0),
                OfflineNote::off(frame(1.4), NOTE),
                OfflineNote::on(frame(1.5), NOTE, 120.0 / 127.0),
                OfflineNote::off(frame(2.9), NOTE),
            ],
            4.0,
        ),
        Demo::LongNote => (plain_note(4.5, NOTE, VELOCITY), 5.0),
        Demo::Tempo { .. } => (plain_note(2.0, NOTE, VELOCITY), 4.0),
        _ => (plain_note(NOTE_OFF, NOTE, VELOCITY), SECONDS),
    }
}

/// Slug for file names and set items.
fn file_slug(name: &str) -> String {
    factory::slug(name).replace('_', "-")
}

/// A factory preset that shows `matches`: the first curated one, else the
/// experimental grid's `fallback` entry.
fn representative(
    bank: &[factory::FactoryPreset],
    matches: impl Fn(&Preset) -> bool,
    fallback: &str,
) -> Preset {
    bank.iter()
        .filter(|e| factory::CURATED_FOLDERS.contains(&e.folder.as_str()))
        .find(|e| matches(&e.preset))
        .or_else(|| {
            bank.iter()
                .find(|e| e.relative_path().starts_with(fallback))
        })
        .map(|e| e.preset.clone())
        .expect("every model has a factory preset")
}

/// Every clip of the plan.
pub fn plan() -> Vec<ClipSpec> {
    let params = CorrosionParams::default();
    let map = params.param_map();
    let base = guide_base_patch();
    let mut clips = Vec::new();

    for p in parameter_plans() {
        if matches!(p.demo, Demo::Excluded { .. }) {
            continue;
        }
        let ptr = map
            .iter()
            .find(|(id, _, _)| *id == p.id)
            .map(|(_, ptr, _)| *ptr)
            .expect("planned IDs are parameters");
        let (notes, seconds) = notes_for(p.demo);
        for point in Point::ALL {
            let value = point_value(&params, &ptr, point);
            let mut values = p.setup.clone();
            values.push((p.id.as_str(), value));
            // SAFETY: `ptr` points into `params`.
            let shown =
                unsafe { ptr.normalized_value_to_string(ptr.preview_normalized(value), true) };
            clips.push(ClipSpec {
                reference: format!("param:{}:{}", p.id, point.key()),
                file: format!("params/{}/{}", p.id, point.key()),
                set: format!("param:{}", p.id),
                is_reference: point == Point::Mid,
                source: format!("{} = {shown}", p.id),
                preset: with_values(&base, &values),
                notes: notes.clone(),
                seconds,
                tempo_bpm: match p.demo {
                    Demo::Tempo { bpm } => Some(bpm),
                    _ => None,
                },
                quiet: p.quiet.iter().find(|(q, _)| *q == point).map(|(_, r)| *r),
            });
        }
    }

    let plain = |reference: String,
                 file: String,
                 set: &str,
                 first: bool,
                 source: String,
                 preset: Preset| ClipSpec {
        reference,
        file,
        set: set.to_string(),
        is_reference: first,
        source,
        preset,
        notes: plain_note(NOTE_OFF, NOTE, VELOCITY),
        seconds: SECONDS,
        tempo_bpm: None,
        quiet: None,
    };
    for (i, &(value, name)) in exciter_model_items().iter().enumerate() {
        let preset = with_values(
            &base,
            &[
                ("exciter", value as f32),
                ("object", COMPARISON_OBJECT.to_int() as f32),
            ],
        );
        clips.push(plain(
            format!("set:exciters:{name}"),
            format!("exciters/{}", file_slug(name)),
            "set:exciters",
            i == 0,
            format!("{name} on {}", COMPARISON_OBJECT.name()),
            preset,
        ));
    }
    for value in 0..9 {
        let object = Object::from_int(value);
        let preset = with_values(
            &base,
            &[
                ("exciter", COMPARISON_EXCITER.to_int() as f32),
                ("object", value as f32),
            ],
        );
        clips.push(plain(
            format!("set:objects:{}", object.name()),
            format!("objects/{}", file_slug(object.name())),
            "set:objects",
            value == 0,
            format!("{} with {}", object.name(), COMPARISON_EXCITER.name()),
            preset,
        ));
    }

    // Representative presets: levelled one by one (they're already levelled
    // in the bank; this keeps peaks under the ceiling).
    let bank = factory::bank();
    for &(value, name) in exciter_model_items() {
        let exciter_slug = factory::slug(name);
        let preset = representative(
            &bank,
            |p| p.exciter == value && p.kit.is_none(),
            &format!("experimental/{exciter_slug}/"),
        );
        let set = format!("set:exciter-presets:{name}");
        clips.push(plain(
            set.clone(),
            format!("exciter-presets/{}", file_slug(name)),
            &set,
            true,
            format!("preset {}", preset.name),
            preset,
        ));
    }
    for value in 0..9 {
        let object = Object::from_int(value);
        let fallback = format!(
            "experimental/{}/{}",
            factory::slug(COMPARISON_EXCITER.name()),
            factory::slug(object.name())
        );
        let preset = representative(&bank, |p| p.object == object && p.kit.is_none(), &fallback);
        let set = format!("set:object-presets:{}", object.name());
        clips.push(plain(
            set.clone(),
            format!("object-presets/{}", file_slug(object.name())),
            &set,
            true,
            format!("preset {}", preset.name),
            preset,
        ));
    }

    // Drum kits: every pad in turn, a quarter second apart.
    for kit in factory::drum_kits() {
        let entry = bank
            .iter()
            .find(|e| e.relative_path() == kit.relative_path())
            .expect("kits are in the bank");
        let keys: Vec<u8> = entry
            .preset
            .kit
            .as_ref()
            .map(|k| k.pads.iter().map(|p| p.key).collect())
            .unwrap_or_default();
        let step = (SAMPLE_RATE as f32 * 0.25) as usize;
        let mut notes = Vec::new();
        for (i, &key) in keys.iter().enumerate() {
            notes.push(OfflineNote::on(i * step, key, VELOCITY / 127.0));
            notes.push(OfflineNote::off(i * step + step / 2, key));
        }
        notes.sort_by_key(|n| n.frame);
        let set = format!("set:kits:{}", entry.preset.name);
        clips.push(ClipSpec {
            reference: set.clone(),
            file: format!("kits/{}", file_slug(&entry.preset.name)),
            set,
            is_reference: true,
            source: format!("kit {}, {} pads", entry.preset.name, keys.len()),
            preset: entry.preset.clone(),
            seconds: keys.len() as f32 * 0.25 + 1.5,
            notes,
            tempo_bpm: None,
            quiet: None,
        });
    }

    // Home page demos: a preset per curated folder (maintainer's choice,
    // 2026-11-13), each a short phrase from its own root key.
    for &(folder, name, root, phrase) in HOME_DEMOS {
        let entry = bank
            .iter()
            .find(|e| e.folder == folder && e.preset.name == name)
            .unwrap_or_else(|| panic!("no factory preset {folder}/{name}"));
        let notes = demo_notes(root, phrase);
        let set = format!("set:demos:{folder}");
        clips.push(ClipSpec {
            reference: set.clone(),
            file: format!("demos/{folder}"),
            set,
            is_reference: true,
            source: format!("preset {} ({folder})", entry.preset.name),
            preset: entry.preset.clone(),
            notes,
            seconds: 4.0,
            tempo_bpm: None,
            quiet: None,
        });
    }
    for clip in &mut clips {
        let last_off = clip
            .notes
            .iter()
            .filter(|n| n.kind == crate::offline::OfflineNoteKind::Off)
            .map(|n| n.frame)
            .max()
            .unwrap_or(0);
        clip.seconds = clip
            .seconds
            .max(last_off as f32 / SAMPLE_RATE as f32 + RELEASE_TAIL);
    }
    clips
}

/// Cut a rendered clip where its tail falls silent: after the last sample
/// louder than `SILENCE_BELOW_PEAK_DB` under the clip's peak, plus a
/// `TRIM_FADE` fade. A silent clip is cut to the fade's length.
pub fn trim_to_silence(frames: &mut Vec<(f32, f32)>) {
    let fade = (SAMPLE_RATE as f32 * TRIM_FADE) as usize;
    let level = |&(l, r): &(f32, f32)| l.abs().max(r.abs());
    let peak = frames.iter().map(level).fold(0.0, f32::max);
    let threshold = peak * 10f32.powf(SILENCE_BELOW_PEAK_DB / 20.0);
    let last = frames
        .iter()
        .rposition(|f| peak > 0.0 && level(f) > threshold);
    let end = last.map_or(fade, |i| i + 1 + fade).min(frames.len());
    frames.truncate(end);
    let start = end.saturating_sub(fade);
    for (i, frame) in frames[start..].iter_mut().enumerate() {
        let g = 1.0 - (i + 1) as f32 / fade as f32;
        frame.0 *= g;
        frame.1 *= g;
    }
}

/// A stable hash (FNV-1a, 64-bit) of everything the plan renders.
pub fn fingerprint() -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for clip in plan() {
        let text = format!(
            "{}|{}|{}|{}|{:?}|{}|{:?}|{}",
            clip.reference,
            clip.file,
            clip.set,
            clip.is_reference,
            clip.notes,
            clip.seconds,
            clip.tempo_bpm,
            serde_json::to_string(&clip.preset).expect("presets serialize"),
        );
        for byte in text.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// Render one clip through the plugin's output path.
pub fn render_clip(clip: &ClipSpec) -> Vec<(f32, f32)> {
    render_preset_at_tempo(
        &clip.preset,
        &clip.notes,
        SAMPLE_RATE,
        clip.seconds,
        clip.tempo_bpm,
    )
}

#[cfg(test)]
mod tests;

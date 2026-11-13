//! Render the user guide's audio clips (`add-user-guide` Decision 7).
//!
//! Every clip of `corrosion::guide::clip_plan::plan()` is rendered through
//! the plugin's own output path at 48 kHz in Render quality, levelled per set
//! (the set's reference clip at `TARGET_RMS`, every peak at or under
//! −1 dBFS), and written as a 32-bit float WAV under `--out`, with
//! `clips.json` (the manifest). `scripts/build-guide-clips.sh` runs this and
//! encodes the WAVs to Opus in `web/guide/public/clips/`.
//!
//! `--candidates` renders the listening set for choosing the base patch and
//! the comparison sets instead (task 4.2).

use clap::Parser;
use corrosion::guide::clip_plan::{
    self, fingerprint, guide_base_patch, plan, render_clip, with_values, ClipSpec, NOTE,
    PEAK_CEILING_DB, SAMPLE_RATE, TARGET_RMS,
};
use corrosion::offline::OfflineNote;
use corrosion::presets::Preset;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(about = "Render the user guide's audio clips")]
struct Args {
    /// Where the WAVs and clips.json go.
    #[arg(long)]
    out: PathBuf,
    /// The generator's commit, recorded in the manifest.
    #[arg(long, default_value = "unknown")]
    commit: String,
    /// Render only clips whose reference starts with this.
    #[arg(long)]
    only: Option<String>,
    /// Render the candidate base patches and comparison sets instead.
    #[arg(long)]
    candidates: bool,
}

fn main() {
    let args = Args::parse();
    std::fs::create_dir_all(&args.out).expect("create the output folder");
    let clips: Vec<ClipSpec> = if args.candidates {
        candidates()
    } else {
        plan()
            .into_iter()
            .filter(|c| {
                args.only
                    .as_ref()
                    .is_none_or(|o| c.reference.starts_with(o))
            })
            .collect()
    };
    let started = std::time::Instant::now();
    let rendered = render_all(&clips);
    let levelled = level(&clips, rendered);
    let mut entries = Vec::new();
    for (clip, (frames, gain)) in clips.iter().zip(&levelled) {
        let path = args.out.join(format!("{}.wav", clip.file));
        std::fs::create_dir_all(path.parent().expect("clip folders")).expect("create folder");
        write_wav_f32(&path, frames);
        let (rms, peak) = measure(frames);
        let mut entry = json!({
            "ref": clip.reference,
            "file": format!("{}.opus", clip.file),
            "set": clip.set,
            "source": clip.source,
            "gain_db": round1(db(*gain)),
            "rms_db": round1(db(rms)),
            "peak_db": round1(db(peak)),
        });
        if let Some(reason) = clip.quiet {
            entry["quiet"] = json!(reason);
        }
        entries.push(entry);
    }
    let manifest = json!({
        "fingerprint": if args.candidates { "candidates".to_string() } else { fingerprint() },
        "generator_commit": args.commit,
        "sample_rate": SAMPLE_RATE,
        "clips": entries,
    });
    let mut text = serde_json::to_string_pretty(&manifest).expect("manifest serializes");
    text.push('\n');
    std::fs::write(args.out.join("clips.json"), text).expect("write manifest");
    println!(
        "rendered {} clips in {:.1} s into {}",
        clips.len(),
        started.elapsed().as_secs_f32(),
        args.out.display()
    );
}

/// Render every clip, spread over the machine's cores.
fn render_all(clips: &[ClipSpec]) -> Vec<Vec<(f32, f32)>> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(vec![Vec::new(); clips.len()]);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(clip) = clips.get(i) else { break };
                let frames = render_clip(clip);
                results.lock().expect("no panics while holding it")[i] = frames;
            });
        }
    });
    results.into_inner().expect("threads joined")
}

/// One gain per set: its reference clip at `TARGET_RMS`, lowered until every
/// clip of the set peaks at or under `PEAK_CEILING_DB`.
fn level(clips: &[ClipSpec], rendered: Vec<Vec<(f32, f32)>>) -> Vec<(Vec<(f32, f32)>, f32)> {
    let mut sets: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, clip) in clips.iter().enumerate() {
        sets.entry(clip.set.as_str()).or_default().push(i);
    }
    let ceiling = 10f32.powf(PEAK_CEILING_DB / 20.0);
    let mut gains = vec![1.0; clips.len()];
    for members in sets.values() {
        let reference = members
            .iter()
            .copied()
            .find(|&i| clips[i].is_reference)
            .unwrap_or(members[0]);
        let (rms, _) = measure(&rendered[reference]);
        let mut gain = if rms > 1e-6 { TARGET_RMS / rms } else { 1.0 };
        let peak = members
            .iter()
            .map(|&i| measure(&rendered[i]).1)
            .fold(0.0, f32::max);
        if peak * gain > ceiling {
            gain = ceiling / peak;
        }
        for &i in members {
            gains[i] = gain;
        }
    }
    rendered
        .into_iter()
        .zip(gains)
        .map(|(frames, g)| (frames.into_iter().map(|(l, r)| (l * g, r * g)).collect(), g))
        .collect()
}

/// RMS over both channels, and the absolute peak.
fn measure(frames: &[(f32, f32)]) -> (f32, f32) {
    if frames.is_empty() {
        return (0.0, 0.0);
    }
    let mut sum = 0.0f64;
    let mut peak = 0.0f32;
    for &(l, r) in frames {
        sum += f64::from(l * l + r * r);
        peak = peak.max(l.abs()).max(r.abs());
    }
    (((sum / (2 * frames.len()) as f64).sqrt()) as f32, peak)
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-10).log10()
}

fn round1(x: f32) -> f64 {
    (f64::from(x) * 10.0).round() / 10.0
}

/// Interleaved stereo frames as a 32-bit float WAV.
fn write_wav_f32(path: &Path, frames: &[(f32, f32)]) {
    let data: Vec<u8> = frames
        .iter()
        .flat_map(|&(l, r)| [l, r])
        .flat_map(f32::to_le_bytes)
        .collect();
    let mut wav = Vec::with_capacity(44 + data.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 8).to_le_bytes());
    wav.extend_from_slice(&8u16.to_le_bytes());
    wav.extend_from_slice(&32u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);
    std::fs::write(path, wav).expect("write wav");
}

// --- Candidates for the listening step (task 4.2) ---------------------------

/// Base patches to choose from, as (key, label, exciter, object).
const BASES: &[(&str, &str, i32, i32)] = &[
    ("a", "Default patch: Hand Strike on Pipe", 2, 0),
    ("b", "Hard Mallet on Plate", 4, 1),
    ("c", "Felt Mallet on Plate", 3, 1),
    ("d", "Hand Strike on Plate", 2, 1),
];

/// Parameters heard over each candidate base.
const PROBES: &[(&str, f32, f32)] = &[
    ("rust", 0.0, 5.0),
    ("strike_position", 0.05, 0.5),
    ("filter_cutoff", 300.0, 20_000.0),
];

fn candidate(
    reference: String,
    set: &str,
    first: bool,
    source: String,
    preset: Preset,
) -> ClipSpec {
    ClipSpec {
        file: reference.replace(':', "/").replace(' ', "-").to_lowercase(),
        reference,
        set: set.to_string(),
        is_reference: first,
        source,
        preset,
        notes: vec![
            OfflineNote::on(0, NOTE, clip_plan::VELOCITY / 127.0),
            OfflineNote::off((SAMPLE_RATE as f32 * clip_plan::NOTE_OFF) as usize, NOTE),
        ],
        seconds: clip_plan::SECONDS,
        tempo_bpm: None,
        quiet: None,
    }
}

fn candidates() -> Vec<ClipSpec> {
    let base = guide_base_patch();
    let mut clips = Vec::new();
    for &(key, label, exciter, object) in BASES {
        let patch = with_values(
            &base,
            &[("exciter", exciter as f32), ("object", object as f32)],
        );
        let set = format!("base-{key}");
        clips.push(candidate(
            format!("base:{key}:plain"),
            &set,
            true,
            label.to_string(),
            patch.clone(),
        ));
        for &(id, lo, hi) in PROBES {
            for (point, value) in [("min", lo), ("max", hi)] {
                clips.push(candidate(
                    format!("base:{key}:{id}-{point}"),
                    &set,
                    false,
                    format!("{label}, {id} = {value}"),
                    with_values(&patch, &[(id, value)]),
                ));
            }
        }
    }
    let exciters = corrosion::guide::exciter_names();
    for (object, name) in [(1, "Plate"), (0, "Pipe"), (2, "Tank")] {
        let set = format!("exciters-on-{}", name.to_lowercase());
        for (i, (value, exciter)) in exciters.iter().enumerate() {
            clips.push(candidate(
                format!("exciters-on-{}:{}", name.to_lowercase(), exciter),
                &set,
                i == 0,
                format!("{exciter} on {name}"),
                with_values(
                    &base,
                    &[("exciter", *value as f32), ("object", object as f32)],
                ),
            ));
        }
    }
    for (exciter, name) in [(4, "Hard Mallet"), (3, "Felt Mallet"), (2, "Hand Strike")] {
        let key = name.to_lowercase().replace(' ', "-");
        let set = format!("objects-with-{key}");
        for object in 0..9 {
            let object_name = corrosion::Object::from_int(object).name();
            clips.push(candidate(
                format!("objects-with-{key}:{object_name}"),
                &set,
                object == 0,
                format!("{object_name} with {name}"),
                with_values(
                    &base,
                    &[("exciter", exciter as f32), ("object", object as f32)],
                ),
            ));
        }
    }
    clips
}

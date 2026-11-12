//! The web engine plays the plugin's engine (`add-web-app`).

use corrosion::offline::{render_preset, OfflineNote};
use corrosion::{CorrosionParams, Preset};
use corrosion_web_engine::WebEngine;
use nih_plug::prelude::Params;

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// `blocks` AudioWorklet-sized blocks of the left channel.
fn render(engine: &mut WebEngine, blocks: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for _ in 0..blocks {
        engine.process(128);
        out.extend_from_slice(&engine.left()[..128]);
    }
    out
}

fn param_index(id: &str) -> usize {
    CorrosionParams::default()
        .param_map()
        .iter()
        .position(|(param_id, _, _)| param_id == id)
        .unwrap_or_else(|| panic!("no parameter {id}"))
}

#[test]
fn a_note_sounds_and_silence_is_silent() {
    let mut engine = WebEngine::new(48_000.0);
    assert_eq!(rms(&render(&mut engine, 20)), 0.0);
    engine.midi(0x90, 60, 100);
    let played = render(&mut engine, 200);
    assert!(rms(&played) > 1e-3, "rms {}", rms(&played));
}

#[test]
fn a_parameter_change_reaches_the_engine() {
    let output = param_index("output");
    let mut engine = WebEngine::new(48_000.0);
    engine.midi(0x90, 60, 100);
    let loud = rms(&render(&mut engine, 100));
    engine.set_param(output, 0.0);
    assert_eq!(engine.param(output), Some(0.0));
    // Let the output smoother settle, then the note is far quieter.
    render(&mut engine, 50);
    let quiet = rms(&render(&mut engine, 50));
    assert!(quiet < loud * 0.01, "loud {loud}, quiet {quiet}");
}

#[test]
fn plays_like_the_offline_renderer() {
    let mut engine = WebEngine::new(48_000.0);
    engine.midi(0x90, 60, 127);
    let web = render(&mut engine, 375);
    let preset = Preset::from_params("default", &CorrosionParams::default());
    let offline: Vec<f32> = render_preset(&preset, &[OfflineNote::on(0, 60, 1.0)], 48_000, 1.0)
        .into_iter()
        .map(|(left, _)| left)
        .collect();
    let (web, offline) = (rms(&web), rms(&offline));
    assert!(
        (20.0 * (web / offline).log10()).abs() < 0.5,
        "web {web}, offline {offline}"
    );
}

/// The native cost of `scripts/bench-engine.mjs`'s scenario, for the
/// wasm/native ratio (`add-web-app` task 1.3). Run with
/// `cargo test --release -p corrosion-web-engine print_native_block_cost -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_native_block_cost() {
    let quality = param_index("quality_mode");
    for (q, name) in ["Eco", "Normal", "High", "Render"].iter().enumerate() {
        for voices in [1u8, 4, 8, 16] {
            let mut engine = WebEngine::new(48_000.0);
            engine.set_param(quality, q as f32 / 3.0);
            for v in 0..voices {
                engine.midi(0x90, 36 + v * 3, 100);
            }
            for _ in 0..20 {
                engine.process(128);
            }
            let start = std::time::Instant::now();
            for _ in 0..1500 {
                engine.process(128);
            }
            let us = start.elapsed().as_secs_f64() * 1e6 / 1500.0;
            println!(
                "{name:8} {voices:6} {us:8.0} us  voices {}",
                engine.active_voices()
            );
        }
    }
}

fn factory_preset_files() -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("preset folder") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "corrosion-preset") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/factory"),
        &mut out,
    );
    out.sort();
    out
}

/// A preset loaded into the web engine plays as the offline renderer
/// renders it (`add-web-app` task 5.4): one preset from each folder, within
/// 1 dB RMS and with matching spectral centroids (within 2 %).
#[test]
fn loaded_presets_sound_like_their_offline_render() {
    let files = factory_preset_files();
    let mut checked = 0;
    for folder in [
        "bass",
        "lead",
        "chords",
        "drones",
        "percussion",
        "experimental",
        "drums",
    ] {
        let Some(path) = files
            .iter()
            .find(|p| p.components().any(|c| c.as_os_str() == folder))
        else {
            continue;
        };
        let preset = Preset::load(path).expect("preset");
        let mut engine = WebEngine::new(48_000.0);
        engine.load_preset(&preset);
        // Let the parameter smoothers reach the preset.
        render(&mut engine, 100);
        // Kits play on their pads (36 and up); tonal presets on middle C.
        let key = if folder == "drums" { 36 } else { 60 };
        engine.midi(0x90, key, 127);
        let web = render(&mut engine, 375);
        let offline: Vec<f32> =
            render_preset(&preset, &[OfflineNote::on(0, key, 1.0)], 48_000, 1.0)
                .into_iter()
                .map(|(left, _)| left)
                .collect();
        let (web_rms, offline_rms) = (rms(&web), rms(&offline));
        assert!(offline_rms > 1e-4, "{}: silent offline", path.display());
        let db = 20.0 * (web_rms / offline_rms).log10();
        assert!(db.abs() < 1.0, "{}: {db:.2} dB", path.display());
        let (a, b) = (centroid(&web), centroid(&offline));
        assert!(
            ((a - b) / b).abs() < 0.02,
            "{}: centroid {a:.0} vs {b:.0} Hz",
            path.display()
        );
        checked += 1;
    }
    assert_eq!(checked, 7);
}

/// A rough spectral centroid (Hz) of `samples` at 48 kHz: a DFT of four
/// 4096-sample frames, each taken every 4th sample with no filter (so the
/// top of the spectrum folds). Coarse, but the same for both renders, which
/// is all the comparison needs.
fn centroid(samples: &[f32]) -> f32 {
    const N: usize = 4096;
    let (mut weighted, mut total) = (0.0f64, 0.0f64);
    for frame in samples.as_chunks::<N>().0.iter().take(4) {
        for bin in 1..N / 2 {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            let step = std::f64::consts::TAU * bin as f64 / N as f64;
            for (i, &x) in frame.iter().enumerate().step_by(4) {
                let phase = step * i as f64;
                re += x as f64 * phase.cos();
                im -= x as f64 * phase.sin();
            }
            let magnitude = (re * re + im * im).sqrt();
            weighted += magnitude * bin as f64 * 48_000.0 / N as f64;
            total += magnitude;
        }
    }
    (weighted / total.max(1e-12)) as f32
}

/// The factory presets' native cost with a held 16-note chord at their own
/// quality, costliest first (`add-web-app` task 1.3). Run with
/// `cargo test --release -p corrosion-web-engine print_preset_block_cost -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_preset_block_cost() {
    let mut rows = Vec::new();
    for path in factory_preset_files() {
        let preset = Preset::load(&path).expect("preset");
        let mut engine = WebEngine::new(48_000.0);
        engine.load_preset(&preset);
        for v in 0..16u8 {
            engine.midi(0x90, 36 + v * 3, 100);
        }
        for _ in 0..20 {
            engine.process(128);
        }
        let start = std::time::Instant::now();
        for _ in 0..400 {
            engine.process(128);
        }
        let us = start.elapsed().as_secs_f64() * 1e6 / 400.0;
        rows.push((
            us,
            path.strip_prefix(path.ancestors().nth(2).unwrap())
                .unwrap()
                .display()
                .to_string(),
        ));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (us, name) in rows.iter().take(12) {
        println!("{us:8.0} us  {name}");
    }
    println!("median {:.0} us", rows[rows.len() / 2].0);
}

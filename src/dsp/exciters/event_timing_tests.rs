//! Short events land between samples at their true time
//! (`resolve-exciter-tradeoffs`, design Decision 5): one event's area
//! (force × time) and centre of mass are the same at 44.1, 48 and 96 kHz.

use super::*;
use crate::dsp::utils::noise::{note_seed, salted};
use crate::dsp::ContactPrediction;

/// A seed as the voice derives it.
fn seed() -> u32 {
    salted(note_seed(60, 0.8, 0), 1)
}

const RATES: [f32; 3] = [44_100.0, 48_000.0, 96_000.0];

/// A rigid, unmoving surface: the exciter's force doesn't move it.
fn rigid(sample_rate: f32) -> ContactPrediction {
    ContactPrediction {
        free: 0.0,
        compliance: 0.0,
        previous: 0.0,
        velocity_scale: sample_rate / 48_000.0,
        limit: f32::INFINITY,
    }
}

/// (area in force·seconds, centre of mass in seconds) of an output stream.
fn moments(out: &[f32], sample_rate: f32) -> (f64, f64) {
    let area: f64 = out.iter().map(|&x| x as f64).sum::<f64>() / sample_rate as f64;
    let centre: f64 = out
        .iter()
        .enumerate()
        .map(|(i, &x)| x as f64 * i as f64 / sample_rate as f64)
        .sum::<f64>()
        / (area * sample_rate as f64);
    (area, centre)
}

/// Renders `seconds` of `next` at each rate and checks area within 2 % and
/// centre of mass within one 44.1 kHz sample plus 10 µs of 48 kHz.
fn check(name: &str, seconds: f32, mut render: impl FnMut(f32, usize) -> Vec<f32>) {
    let results: Vec<(f32, (f64, f64))> = RATES
        .iter()
        .map(|&fs| (fs, moments(&render(fs, (seconds * fs) as usize), fs)))
        .collect();
    let (_, (area48, centre48)) = results[1];
    assert!(area48.abs() > 0.0, "{name}: no output at 48 kHz");
    for (fs, (area, centre)) in &results {
        let area_err = (area / area48 - 1.0).abs();
        let centre_err = (centre - centre48).abs();
        assert!(
            area_err <= 0.02 && centre_err <= 1.0 / 44_100.0 + 10e-6,
            "{name} @ {fs}: area {area:.4e} vs {area48:.4e} ({:+.2} %), centre {:.4} ms vs {:.4} ms",
            100.0 * (area / area48 - 1.0),
            centre * 1e3,
            centre48 * 1e3
        );
    }
}

#[test]
fn wire_brush_impulse() {
    check("WireBrush", 0.06, |fs, n| {
        let mut brush = WireBrush::new();
        brush.set_sample_rate(fs);
        brush.set_parameters(3, 20.0, 0.5, 0.0);
        brush.reseed(seed());
        brush.trigger(0.8);
        (0..n).map(|_| brush.process_sample(0.0, 0.0)).collect()
    });
}

#[test]
fn tension_rise_break() {
    // One break (at ~39 ms) and its release burst, before the next break.
    check("TensionRise", 0.06, |fs, n| {
        let mut rise = TensionRise::new();
        rise.set_sample_rate(fs);
        rise.set_parameters(0.3, 0.5, 0.0, 0.5);
        rise.reseed(seed());
        rise.trigger(0.8);
        (0..n).map(|_| rise.process_sample(0.0, 0.0)).collect()
    });
}

#[test]
fn tension_snap_release() {
    check("TensionSnap", 0.01, |fs, n| {
        let mut snap = TensionSnap::new();
        snap.set_sample_rate(fs);
        snap.set_parameters(10.0, 2.0, 0.3);
        snap.trigger(0.8);
        (0..n).map(|_| snap.process_sample(rigid(fs))).collect()
    });
}

#[test]
fn metal_chain_impacts() {
    check("MetalChain", 0.3, |fs, n| {
        let mut chain = MetalChain::new();
        chain.set_sample_rate(fs);
        chain.set_parameters(2, 1.0, 100.0, 0.0, 0.5);
        chain.reseed(seed());
        chain.trigger(0.8);
        (0..n).map(|_| chain.process_sample(rigid(fs))).collect()
    });
}

#[test]
fn particle_rain_spawns() {
    check("ParticleRain", 0.05, |fs, n| {
        let mut rain = ParticleRain::new();
        rain.set_sample_rate(fs);
        rain.set_parameters(0.2, 0.5, 0.0);
        rain.reseed(seed());
        rain.trigger(0.8);
        (0..n).map(|_| rain.process_sample(rigid(fs))).collect()
    });
}

#[test]
fn hand_strike_contact() {
    check("HandStrike", 0.002, |fs, n| {
        let mut hand = HandStrike::new();
        hand.set_sample_rate(fs);
        hand.set_parameters(1.7, 0.425, 0.75, 0.9245);
        hand.trigger(0.8);
        (0..n).map(|_| hand.process_sample(rigid(fs))).collect()
    });
}

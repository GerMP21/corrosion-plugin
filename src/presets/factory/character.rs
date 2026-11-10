//! Folder character checks (spec "Folder character"), measured on offline
//! renders of a held note.

use super::measure::{
    cents, db, level_at, mono, note_hz, peak_level, render_held, rms_between, Spectrum,
};
use crate::presets::Preset;

/// What the character checks measure; `None` where a folder's check doesn't
/// need it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharacterReport {
    /// Bass: share of the energy below 500 Hz at C2.
    pub low_fraction: Option<f64>,
    /// Lead, chords: the strongest peak at middle C, in Hz, and its distance
    /// in cents from the nearest of C3, C4, C5.
    pub strongest_hz: Option<f32>,
    pub pitch_error_cents: Option<f32>,
    /// Chords: the first six partials (Hz) and the largest deviation, in
    /// cents, from an integer multiple of the first.
    pub partials: Option<Vec<f32>>,
    pub partial_deviation_cents: Option<f32>,
    /// Chords: level 2 s in relative to the peak (dB).
    pub sustain_db: Option<f32>,
    /// Chords: the largest spectral flatness over the 3 s.
    pub flatness: Option<f64>,
    /// Drones: RMS of the last second relative to the second second (dB).
    pub drone_drift_db: Option<f32>,
    /// Percussion: level 1.5 s after the onset relative to the peak (dB).
    pub decay_db: Option<f32>,
}

/// Bounds from the spec.
pub const BASS_LOW_FRACTION: f64 = 0.70;
pub const PITCH_CENTS: f32 = 50.0;
pub const PARTIAL_CENTS: f32 = 15.0;
pub const CHORD_SUSTAIN_DB: f32 = -20.0;
pub const CHORD_FLATNESS: f64 = 0.1;
pub const DRONE_DRIFT_DB: f32 = 6.0;
pub const PERCUSSION_DECAY_DB: f32 = -30.0;

/// Peaks within this range of the strongest count as partials.
pub const PARTIAL_RANGE_DB: f32 = 40.0;

const MIDDLE_C: u8 = 60;
const C2: u8 = 36;
const FFT_LEN: usize = 65_536;

fn after(seconds: f32) -> usize {
    (seconds * super::measure::SAMPLE_RATE as f32) as usize
}

/// Bass: the energy split of a held C2.
pub fn measure_bass(preset: &Preset, report: &mut CharacterReport) {
    let s = mono(&render_held(preset, C2, 2.0).output);
    report.low_fraction = Some(Spectrum::of(&s, after(0.05), FFT_LEN).fraction_below(500.0));
}

/// Lead and chords: the strongest peak of a held middle C.
pub fn measure_pitch(preset: &Preset, report: &mut CharacterReport) {
    let s = mono(&render_held(preset, MIDDLE_C, 2.0).output);
    let peak = Spectrum::of(&s, after(0.1), FFT_LEN).strongest_peak();
    let error = [MIDDLE_C - 12, MIDDLE_C, MIDDLE_C + 12]
        .iter()
        .map(|&n| cents(peak, note_hz(n)).abs())
        .fold(f32::INFINITY, f32::min);
    report.strongest_hz = Some(peak);
    report.pitch_error_cents = Some(error);
}

/// Chords: partials, sustain and flatness of a 3 s held middle C.
pub fn measure_chord(preset: &Preset, report: &mut CharacterReport) {
    let s = mono(&render_held(preset, MIDDLE_C, 3.0).output);
    let spectrum = Spectrum::of(&s, after(0.1), FFT_LEN);
    let partials: Vec<f32> = spectrum
        .peaks(60.0, PARTIAL_RANGE_DB, 20.0)
        .into_iter()
        .take(6)
        .collect();
    let deviation = partials.first().map_or(f32::INFINITY, |&f0| {
        partials
            .iter()
            .map(|&f| {
                let n = (f / f0).round().max(1.0);
                cents(f, n * f0).abs()
            })
            .fold(0.0, f32::max)
    });
    report.partials = Some(partials);
    report.partial_deviation_cents = Some(deviation);
    let peak = peak_level(&s);
    report.sustain_db = Some(db(level_at(&s, 2.0), peak));
    // Flatness of 8192-sample frames from 0.1 s on, skipping frames more than
    // 60 dB under the peak (silence is not noise).
    let floor = (peak * 1e-3) as f64;
    let frame = 8192;
    let flatness = (after(0.1)..s.len().saturating_sub(frame))
        .step_by(frame)
        .filter(|&start| super::measure::rms(&s[start..start + frame]) as f64 > floor)
        .map(|start| Spectrum::of(&s, start, frame).flatness(0.0))
        .fold(0.0, f64::max);
    report.flatness = Some(flatness);
}

/// Drones: level drift over an 8 s hold.
pub fn measure_drone(preset: &Preset, report: &mut CharacterReport) {
    let s = mono(&render_held(preset, MIDDLE_C, 8.0).output);
    report.drone_drift_db = Some(db(rms_between(&s, 7.0, 8.0), rms_between(&s, 1.0, 2.0)));
}

/// Percussion: level 1.5 s after the onset of a held note.
pub fn measure_percussion(preset: &Preset, report: &mut CharacterReport) {
    let s = mono(&render_held(preset, MIDDLE_C, 2.0).output);
    report.decay_db = Some(db(level_at(&s, 1.5), peak_level(&s)));
}

/// Measure `preset` for `folder`'s checks.
pub fn measure(folder: &str, preset: &Preset) -> CharacterReport {
    let mut report = CharacterReport::default();
    match folder {
        "bass" => measure_bass(preset, &mut report),
        "lead" => measure_pitch(preset, &mut report),
        "chords" => {
            measure_pitch(preset, &mut report);
            measure_chord(preset, &mut report);
        }
        "drones" => measure_drone(preset, &mut report),
        "percussion" => measure_percussion(preset, &mut report),
        _ => {}
    }
    report
}

impl CharacterReport {
    /// Whether every measured value is within its bound, and if not, why.
    pub fn check(&self) -> Result<(), String> {
        let mut failures = Vec::new();
        if let Some(f) = self.low_fraction {
            if f < BASS_LOW_FRACTION {
                failures.push(format!(
                    "{:.0} % of energy below 500 Hz (≥ 70 %)",
                    f * 100.0
                ));
            }
        }
        if let (Some(hz), Some(e)) = (self.strongest_hz, self.pitch_error_cents) {
            if e > PITCH_CENTS {
                failures.push(format!(
                    "strongest peak {hz:.1} Hz is {e:.0} cents off C3/C4/C5"
                ));
            }
        }
        if let Some(d) = self.partial_deviation_cents {
            if d > PARTIAL_CENTS {
                failures.push(format!(
                    "partials {:?} deviate {d:.1} cents (≤ 15)",
                    self.partials.as_deref().unwrap_or(&[])
                ));
            }
        }
        if let Some(s) = self.sustain_db {
            if s < CHORD_SUSTAIN_DB {
                failures.push(format!("level at 2 s is {s:.1} dB (≥ −20 dB)"));
            }
        }
        if let Some(f) = self.flatness {
            if f >= CHORD_FLATNESS {
                failures.push(format!("flatness {f:.3} (< 0.1)"));
            }
        }
        if let Some(d) = self.drone_drift_db {
            if d.abs() > DRONE_DRIFT_DB {
                failures.push(format!("last second {d:+.1} dB from the second (±6 dB)"));
            }
        }
        if let Some(d) = self.decay_db {
            if d > PERCUSSION_DECAY_DB {
                failures.push(format!("level at 1.5 s is {d:.1} dB (≤ −30 dB)"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}

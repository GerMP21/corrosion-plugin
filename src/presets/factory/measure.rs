//! Measurements for the floor and character checks (design decision 6):
//! renders of a held note, RMS windows, a windowed FFT, spectral peaks and
//! flatness. Used by the bank tests and by the seeder's measurement table.

use crate::offline::{OfflineNote, PresetRenderer, RenderedFrames};
use crate::presets::Preset;

/// Rate of every character render.
pub const SAMPLE_RATE: u32 = 48_000;

/// RMS window for level-over-time measurements.
pub const LEVEL_WINDOW_SECONDS: f32 = 0.05;

/// Render `preset` holding `note` (velocity [`super::FLOOR_VELOCITY`]) for
/// `seconds`, never released.
pub fn render_held(preset: &Preset, note: u8, seconds: f32) -> RenderedFrames {
    let frames = (SAMPLE_RATE as f32 * seconds) as usize;
    PresetRenderer::new(preset).render(
        &[OfflineNote::on(0, note, super::FLOOR_VELOCITY)],
        SAMPLE_RATE,
        frames,
    )
}

/// The mono (L + R) / 2 signal of `frames`.
pub fn mono(frames: &[(f32, f32)]) -> Vec<f32> {
    frames.iter().map(|&(l, r)| 0.5 * (l + r)).collect()
}

/// RMS of `samples` (0 for an empty slice).
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&x| (x as f64).powi(2)).sum();
    (sum / samples.len() as f64).sqrt() as f32
}

/// RMS of `samples` between `from` and `to` seconds.
pub fn rms_between(samples: &[f32], from: f32, to: f32) -> f32 {
    let at = |t: f32| ((t * SAMPLE_RATE as f32) as usize).min(samples.len());
    rms(&samples[at(from)..at(to)])
}

/// RMS of consecutive [`LEVEL_WINDOW_SECONDS`] windows.
pub fn window_levels(samples: &[f32]) -> Vec<f32> {
    let window = (LEVEL_WINDOW_SECONDS * SAMPLE_RATE as f32) as usize;
    samples.chunks(window).map(rms).collect()
}

/// The loudest window's RMS.
pub fn peak_level(samples: &[f32]) -> f32 {
    window_levels(samples).into_iter().fold(0.0, f32::max)
}

/// The RMS of the window that contains `seconds`.
pub fn level_at(samples: &[f32], seconds: f32) -> f32 {
    let levels = window_levels(samples);
    let index = (seconds / LEVEL_WINDOW_SECONDS) as usize;
    levels.get(index).copied().unwrap_or(0.0)
}

/// `a / b` in dB (−240 dB for silence).
pub fn db(a: f32, b: f32) -> f32 {
    20.0 * (a.max(1e-12) / b.max(1e-12)).log10()
}

/// `f / reference` in cents.
pub fn cents(f: f32, reference: f32) -> f32 {
    1200.0 * (f / reference).log2()
}

/// Frequency of MIDI note `note` (A4 = 440 Hz).
pub fn note_hz(note: u8) -> f32 {
    440.0 * 2f32.powf((note as f32 - 69.0) / 12.0)
}

/// A power spectrum of `samples[start..start + len]` (`len` a power of two),
/// with a 4-term Blackman–Harris window (side lobes about −92 dB, so a
/// window side lobe is never mistaken for a partial). Bin `k` is
/// `k · rate / len` Hz.
pub struct Spectrum {
    pub power: Vec<f64>,
    pub bin_hz: f32,
}

impl Spectrum {
    pub fn of(samples: &[f32], start: usize, len: usize) -> Self {
        assert!(len.is_power_of_two());
        let mut re = vec![0.0f64; len];
        let mut im = vec![0.0f64; len];
        let n = len as f64;
        for (i, slot) in re.iter_mut().enumerate() {
            let x = samples.get(start + i).copied().unwrap_or(0.0) as f64;
            let p = 2.0 * std::f64::consts::PI * i as f64 / n;
            let w =
                0.35875 - 0.48829 * p.cos() + 0.14128 * (2.0 * p).cos() - 0.01168 * (3.0 * p).cos();
            *slot = x * w;
        }
        fft(&mut re, &mut im);
        let power = re
            .iter()
            .zip(&im)
            .take(len / 2)
            .map(|(r, i)| r * r + i * i)
            .collect();
        Self {
            power,
            bin_hz: SAMPLE_RATE as f32 / len as f32,
        }
    }

    fn bin(&self, hz: f32) -> usize {
        ((hz / self.bin_hz) as usize).min(self.power.len())
    }

    /// Share of the energy between 20 Hz and Nyquist that lies below `hz`.
    pub fn fraction_below(&self, hz: f32) -> f64 {
        let lo = self.bin(20.0);
        let split = self.bin(hz);
        let below: f64 = self.power[lo..split].iter().sum();
        let total: f64 = self.power[lo..].iter().sum();
        if total > 0.0 {
            below / total
        } else {
            0.0
        }
    }

    /// The frequency of a peak at bin `k`, refined by parabolic
    /// interpolation of the log power.
    fn refine(&self, k: usize) -> f32 {
        if k == 0 || k + 1 >= self.power.len() {
            return k as f32 * self.bin_hz;
        }
        let (a, b, c) = (
            self.power[k - 1].max(1e-30).ln(),
            self.power[k].max(1e-30).ln(),
            self.power[k + 1].max(1e-30).ln(),
        );
        let denom = a - 2.0 * b + c;
        let offset = if denom.abs() > 1e-12 {
            (0.5 * (a - c) / denom).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        (k as f64 + offset) as f32 * self.bin_hz
    }

    /// The strongest peak between 20 Hz and 20 kHz, in Hz.
    pub fn strongest_peak(&self) -> f32 {
        let (lo, hi) = (self.bin(20.0).max(1), self.bin(20_000.0));
        let k = (lo..hi)
            .max_by(|&a, &b| self.power[a].total_cmp(&self.power[b]))
            .unwrap_or(lo);
        self.refine(k)
    }

    /// Spectral peaks between `from` Hz and 20 kHz within `range_db` of the
    /// strongest one, lowest first: local maxima of the power, each the
    /// largest within ± `min_spacing_hz`.
    pub fn peaks(&self, from: f32, range_db: f32, min_spacing_hz: f32) -> Vec<f32> {
        let (lo, hi) = (self.bin(from).max(1), self.bin(20_000.0));
        let max = self.power[lo..hi].iter().copied().fold(0.0, f64::max);
        let floor = max * 10f64.powf(-range_db as f64 / 10.0);
        let spacing = ((min_spacing_hz / self.bin_hz) as usize).max(1);
        (lo..hi)
            .filter(|&k| {
                let p = self.power[k];
                p >= floor
                    && (k.saturating_sub(spacing)..=(k + spacing).min(self.power.len() - 1))
                        .all(|j| j == k || self.power[j] < p)
            })
            .map(|k| self.refine(k))
            .collect()
    }

    /// Spectral flatness (geometric over arithmetic mean of the power)
    /// between 50 Hz and 16 kHz: about 1 for white noise, near 0 for tones.
    pub fn flatness(&self, floor_power: f64) -> f64 {
        let (lo, hi) = (self.bin(50.0), self.bin(16_000.0));
        let bins = &self.power[lo..hi];
        let mean = bins.iter().sum::<f64>() / bins.len() as f64;
        if mean <= floor_power {
            return 0.0;
        }
        let log_mean =
            bins.iter().map(|p| p.max(mean * 1e-12).ln()).sum::<f64>() / bins.len() as f64;
        log_mean.exp() / mean
    }
}

impl Spectrum {
    /// Spectral centroid between 20 Hz and 20 kHz, in Hz.
    pub fn centroid(&self) -> f32 {
        let (lo, hi) = (self.bin(20.0), self.bin(20_000.0));
        let (mut weighted, mut total) = (0.0f64, 0.0f64);
        for k in lo..hi {
            weighted += k as f64 * self.power[k];
            total += self.power[k];
        }
        if total > 0.0 {
            (weighted / total) as f32 * self.bin_hz
        } else {
            0.0
        }
    }
}

/// Seconds from the loudest window until the level first falls 30 dB under
/// it (`None` if it never does).
pub fn decay_to_minus_30(samples: &[f32]) -> Option<f32> {
    let levels = window_levels(samples);
    let (peak_index, peak) =
        levels
            .iter()
            .copied()
            .enumerate()
            .fold(
                (0, 0.0f32),
                |best, (i, l)| if l > best.1 { (i, l) } else { best },
            );
    levels
        .iter()
        .enumerate()
        .skip(peak_index)
        .find(|(_, &l)| db(l, peak) <= -30.0)
        .map(|(i, _)| (i - peak_index) as f32 * LEVEL_WINDOW_SECONDS)
}

/// In-place iterative radix-2 FFT.
fn fft(re: &mut [f64], im: &mut [f64]) {
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
        let angle = -2.0 * std::f64::consts::PI / len as f64;
        let (w_re, w_im) = (angle.cos(), angle.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0f64, 0.0f64);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let next = cr * w_re - ci * w_im;
                ci = cr * w_im + ci * w_re;
                cr = next;
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(partials: &[(f32, f32)], seconds: f32) -> Vec<f32> {
        (0..(SAMPLE_RATE as f32 * seconds) as usize)
            .map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                partials
                    .iter()
                    .map(|&(f, a)| a * (2.0 * std::f32::consts::PI * f * t).sin())
                    .sum()
            })
            .collect()
    }

    #[test]
    fn peaks_find_partials_to_a_cent() {
        let s = tone(&[(261.63, 1.0), (523.25, 0.5), (1046.5, 0.1)], 1.5);
        let spectrum = Spectrum::of(&s, 0, 65_536);
        assert!(cents(spectrum.strongest_peak(), 261.63).abs() < 1.0);
        let peaks = spectrum.peaks(60.0, 40.0, 20.0);
        assert_eq!(peaks.len(), 3, "{peaks:?}");
        for (p, f) in peaks.iter().zip([261.63, 523.25, 1046.5]) {
            assert!(cents(*p, f).abs() < 1.0, "{p} vs {f}");
        }
    }

    #[test]
    fn flatness_tells_noise_from_tones() {
        let s = tone(&[(440.0, 1.0)], 0.5);
        assert!(Spectrum::of(&s, 0, 8192).flatness(0.0) < 0.01);
        let mut state = 1u32;
        let noise: Vec<f32> = (0..8192)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 8) as f32 / (1u32 << 24) as f32 - 0.5
            })
            .collect();
        assert!(Spectrum::of(&noise, 0, 8192).flatness(0.0) > 0.5);
    }

    #[test]
    fn low_fraction_and_levels() {
        let s = tone(&[(100.0, 1.0), (2000.0, 1.0)], 1.0);
        let f = Spectrum::of(&s, 0, 32_768).fraction_below(500.0);
        assert!((f - 0.5).abs() < 0.02, "{f}");
        let decaying: Vec<f32> = s
            .iter()
            .enumerate()
            .map(|(i, x)| x * (-(i as f32) / SAMPLE_RATE as f32 * 5.0).exp())
            .collect();
        let drop = db(level_at(&decaying, 0.9), peak_level(&decaying));
        assert!((drop + 38.0).abs() < 2.0, "{drop}");
    }
}

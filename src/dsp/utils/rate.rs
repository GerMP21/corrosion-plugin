//! Converting per-sample constants written for 48 kHz to the host rate.
//!
//! Much of the exciter and voice code integrates and decays in per-sample
//! steps that were tuned at 48 kHz. These helpers re-express such a constant
//! for another sample rate so its effect per *second* stays the same. Each is
//! exactly the identity at 48 kHz (`ratio == 1.0`), so 48 kHz output is
//! bit-identical.

/// Sample rate the per-sample constants were tuned at.
pub const REFERENCE_RATE: f32 = 48_000.0;

/// `48 kHz / fs`: how many reference samples one host sample spans.
#[inline]
pub fn ratio(sample_rate: f32) -> f32 {
    REFERENCE_RATE / sample_rate.max(1.0)
}

/// A per-sample decay (or one-pole pole) factor `c`: `c^ratio`.
#[inline]
pub fn decay(c: f32, ratio: f32) -> f32 {
    if ratio == 1.0 {
        c
    } else {
        c.powf(ratio)
    }
}

/// A one-pole smoother step `x += (t − x)·k`: `1 − (1 − k)^ratio`.
#[inline]
pub fn smoother(k: f32, ratio: f32) -> f32 {
    if ratio == 1.0 {
        k
    } else {
        1.0 - (1.0 - k).powf(ratio)
    }
}

/// A per-sample step or rate (integration step, acceleration, phase
/// increment): `step · ratio`.
#[inline]
pub fn step(step: f32, ratio: f32) -> f32 {
    step * ratio
}

/// A count of reference samples: the same duration in host samples.
#[inline]
pub fn frames(reference_frames: u32, ratio: f32) -> u32 {
    if ratio == 1.0 {
        reference_frames
    } else {
        ((reference_frames as f32 / ratio).round() as u32).max(1)
    }
}

/// A one-sample force spike: `amplitude / ratio`, so its area (force ×
/// time) stays the same.
#[inline]
pub fn impulse(amplitude: f32, ratio: f32) -> f32 {
    if ratio == 1.0 {
        amplitude
    } else {
        amplitude / ratio
    }
}

/// Per-sample white noise: `amplitude / √ratio`, so its spectral density in
/// the audible band stays the same when the band it spreads over widens.
#[inline]
pub fn white_noise(amplitude: f32, ratio: f32) -> f32 {
    if ratio == 1.0 {
        amplitude
    } else {
        amplitude / ratio.sqrt()
    }
}

/// Places short events between samples (`resolve-exciter-tradeoffs`).
///
/// An event of size `amount` that happened `early` (0–1) of a sample before
/// the end of the current sample is emitted as `early·amount` now and
/// `(1 − early)·amount` on the next sample. Its area is kept exactly, and its
/// centre of mass sits one sample after the true time at every rate, so a
/// train of events no longer rounds to the sample grid.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EventSplitter {
    now: f32,
    next: f32,
}

impl EventSplitter {
    /// Add an event that happened `early` (clamped to 0–1) of a sample before
    /// the end of this sample.
    #[inline]
    pub fn add(&mut self, amount: f32, early: f32) {
        let early = early.clamp(0.0, 1.0);
        self.now += amount * early;
        self.next += amount * (1.0 - early);
    }

    /// This sample's share of the events, then move on to the next sample.
    #[inline]
    pub fn take(&mut self) -> f32 {
        let out = self.now;
        self.now = self.next;
        self.next = 0.0;
        out
    }

    /// Whether anything is still waiting to be emitted.
    #[inline]
    pub fn is_pending(&self) -> bool {
        self.now != 0.0 || self.next != 0.0
    }

    /// Drop anything pending.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_conversion_is_exact_at_48k() {
        let r = ratio(48_000.0);
        assert_eq!(r, 1.0);
        for c in [0.0, 0.001, 0.01, 0.5, 0.8, 0.98, 0.99, 0.995, 0.999, 1.0] {
            assert_eq!(decay(c, r).to_bits(), c.to_bits());
            assert_eq!(smoother(c, r).to_bits(), c.to_bits());
            assert_eq!(step(c, r).to_bits(), c.to_bits());
            assert_eq!(impulse(c, r).to_bits(), c.to_bits());
            assert_eq!(white_noise(c, r).to_bits(), c.to_bits());
        }
        assert_eq!(frames(4_800, r), 4_800);
        assert_eq!(frames(50, r), 50);
    }

    #[test]
    fn conversions_keep_the_per_second_effect() {
        // Two samples at 96 kHz equal one at 48 kHz.
        let r = ratio(96_000.0);
        assert!((decay(0.99, r).powi(2) - 0.99).abs() < 1e-6);
        let k = smoother(0.001, r);
        assert!(((1.0 - k).powi(2) - 0.999).abs() < 1e-6);
        assert_eq!(step(0.01, r) * 2.0, 0.01);
        assert_eq!(frames(4_800, r), 9_600);
        assert_eq!(impulse(1.0, r), 2.0);
        assert!((white_noise(1.0, r).powi(2) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn event_splitter_keeps_area_and_centre_of_mass() {
        // An event 30 % of a sample early: 0.3 now, 0.7 next, centre one
        // sample after the event.
        let mut split = EventSplitter::default();
        split.add(2.0, 0.3);
        let (a, b, c) = (split.take(), split.take(), split.take());
        assert!((a - 0.6).abs() < 1e-6 && (b - 1.4).abs() < 1e-6 && c == 0.0);
        let centre = (0.0 * a + 1.0 * b) / (a + b);
        assert!((centre - (1.0 - 0.3)).abs() < 1e-6);
    }
}

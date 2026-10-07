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
}

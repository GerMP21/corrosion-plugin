//! One integer noise generator for the whole engine.
//!
//! Replaces the `fract(sin(phase) · K)` hashes the exciters used. Those
//! grew an f32 phase every call, so their resolution coarsened as a note went
//! on, and faster at higher sample rates: their spectrum drifted with note
//! length and rate (`resolve-exciter-tradeoffs`). xorshift32 is white,
//! allocation-free and deterministic for a given seed.

/// xorshift32 noise source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noise {
    state: u32,
}

impl Noise {
    /// A generator for `seed`. Any seed works: it is mixed first, so nearby
    /// seeds give unrelated sequences, and a zero state is avoided.
    pub fn new(seed: u32) -> Self {
        Self {
            state: mix(seed).max(1),
        }
    }

    /// Restart from `seed`, as `new` would.
    pub fn reseed(&mut self, seed: u32) {
        *self = Self::new(seed);
    }

    /// Next raw 32-bit value.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn unit(&mut self) -> f32 {
        // The top 24 bits fill an f32 mantissa exactly.
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    /// Uniform in `[−1, 1)`.
    #[inline]
    pub fn bipolar(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

impl Default for Noise {
    fn default() -> Self {
        Self::new(0)
    }
}

/// A note's seed: unique per note, velocity and start frame (so per voice),
/// and stable across runs.
pub fn note_seed(note: u8, velocity: f32, start_frame: u64) -> u32 {
    mix(note as u32
        ^ velocity.to_bits().rotate_left(8)
        ^ mix(start_frame as u32 ^ (start_frame >> 32) as u32))
}

/// A distinct stream of the same note seed for each consumer (`salt`).
pub fn salted(seed: u32, salt: u32) -> u32 {
    mix(seed ^ salt.wrapping_mul(0x9e37_79b9))
}

/// Integer hash (lowbias32), used to decorrelate seeds.
fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence_and_different_seeds_differ() {
        let (mut a, mut b, mut c) = (Noise::new(7), Noise::new(7), Noise::new(8));
        let xs: Vec<u32> = (0..64).map(|_| a.next_u32()).collect();
        let ys: Vec<u32> = (0..64).map(|_| b.next_u32()).collect();
        let zs: Vec<u32> = (0..64).map(|_| c.next_u32()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs, zs);
    }

    #[test]
    fn values_stay_in_range_with_zero_mean() {
        let mut noise = Noise::new(42);
        let mut sum = 0.0f64;
        for _ in 0..100_000 {
            let x = noise.bipolar();
            assert!((-1.0..1.0).contains(&x));
            sum += x as f64;
        }
        assert!((sum / 100_000.0).abs() < 0.01, "mean {}", sum / 100_000.0);
    }

    /// Equal power in four octave bands within 1 dB, early and late in a
    /// long note: the noise is white and stays white. Averages the
    /// periodograms of 32 blocks (Welch), so each band's estimate rests on at
    /// least 1 024 bins.
    #[test]
    fn spectrum_is_flat() {
        const N: usize = 1_024;
        for skip in [0usize, 10_000_000] {
            let mut noise = Noise::new(3);
            for _ in 0..skip {
                noise.next_u32();
            }
            let mut power = vec![0.0f64; N / 2];
            for _ in 0..32 {
                let xs: Vec<f64> = (0..N).map(|_| noise.bipolar() as f64).collect();
                for (k, p) in power.iter_mut().enumerate().skip(32) {
                    let (mut re, mut im) = (0.0, 0.0);
                    for (i, &x) in xs.iter().enumerate() {
                        let ph = std::f64::consts::TAU * (k * i) as f64 / N as f64;
                        re += x * ph.cos();
                        im += x * ph.sin();
                    }
                    *p += re * re + im * im;
                }
            }
            let band = |lo: usize, hi: usize| power[lo..hi].iter().sum::<f64>() / (hi - lo) as f64;
            let powers = [band(32, 64), band(64, 128), band(128, 256), band(256, 512)];
            let mean = powers.iter().sum::<f64>() / 4.0;
            for p in &powers {
                let db = 10.0 * (p / mean).log10();
                assert!(
                    db.abs() <= 1.0,
                    "skip {skip}: band {db:+.2} dB from the mean {powers:?}"
                );
            }
        }
    }
}

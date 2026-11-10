//! Half-band polyphase IIR filters for 2× up- and down-sampling.
//!
//! The filter is the classic two-path all-pass structure (as in Laurent de
//! Soras' HIIR): `H(z) = ½·(A0(z²) + z⁻¹·A1(z²))`, where `A0` and `A1` are
//! chains of first-order all-pass sections `(c + z⁻¹)/(1 + c·z⁻¹)` running
//! at the low rate. Coefficients alternate between the two paths. They come
//! from the elliptic half-band design, computed in f64 once, outside the
//! audio thread.
//!
//! The filter is power-complementary around a quarter of the stage's rate:
//! flat to `0.25 − transition`, rejecting from `0.25 + transition` (both as
//! fractions of the high rate). It is not linear-phase: low frequencies are
//! delayed by a few high-rate samples, which the post chain doesn't report.

/// Most all-pass coefficients a stage can hold.
pub const MAX_COEFS: usize = 12;

/// Coefficients of one half-band stage.
#[derive(Clone, Copy, Debug)]
pub struct HalfBandCoefs {
    coefs: [f32; MAX_COEFS],
    len: usize,
}

impl HalfBandCoefs {
    /// Designs the stage with the fewest coefficients that rejects at least
    /// `attenuation_db` from `0.25 + transition` of the high rate up, with
    /// the pass band flat to `0.25 − transition` (`0 < transition < 0.25`).
    ///
    /// # Panics
    /// When the design needs more than [`MAX_COEFS`] coefficients.
    pub fn design(attenuation_db: f64, transition: f64) -> Self {
        let (k, q) = transition_params(transition);
        let order = filter_order(attenuation_db, q);
        let len = (order - 1) / 2;
        assert!(
            len <= MAX_COEFS,
            "half-band design needs {len} coefficients, at most {MAX_COEFS} fit"
        );
        let mut coefs = [0.0f32; MAX_COEFS];
        for (index, coef) in coefs.iter_mut().take(len).enumerate() {
            *coef = coefficient(index, k, q, order) as f32;
        }
        Self { coefs, len }
    }

    /// Number of all-pass coefficients.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the stage has no coefficients (never true for a design).
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The coefficients in use.
    pub fn as_slice(&self) -> &[f32] {
        &self.coefs[..self.len]
    }
}

/// Elliptic modulus `k` and nome `q` for a transition half-width.
fn transition_params(transition: f64) -> (f64, f64) {
    let k = ((1.0 - transition * 2.0) * std::f64::consts::FRAC_PI_4)
        .tan()
        .powi(2);
    let kksqrt = (1.0 - k * k).powf(0.25);
    let e = 0.5 * (1.0 - kksqrt) / (1.0 + kksqrt);
    let e4 = e.powi(4);
    let q = e * (1.0 + e4 * (2.0 + e4 * (15.0 + 150.0 * e4)));
    (k, q)
}

/// Smallest odd order (at least 3) reaching `attenuation_db` for nome `q`.
fn filter_order(attenuation_db: f64, q: f64) -> usize {
    let attn_p2 = 10f64.powf(-attenuation_db / 10.0);
    let a = attn_p2 / (1.0 - attn_p2);
    let mut order = ((a * a / 16.0).ln() / q.ln()).ceil() as usize;
    if order.is_multiple_of(2) {
        order += 1;
    }
    order.max(3)
}

fn coefficient(index: usize, k: f64, q: f64, order: usize) -> f64 {
    let c = (index + 1) as f64;
    let order = order as f64;
    let pi = std::f64::consts::PI;

    let mut num = 0.0;
    let mut i = 0i32;
    let mut sign = 1.0;
    loop {
        let term = q.powi(i * (i + 1)) * (((i * 2 + 1) as f64) * c * pi / order).sin() * sign;
        num += term;
        sign = -sign;
        i += 1;
        if term.abs() <= 1e-100 {
            break;
        }
    }

    let mut den = 0.0;
    let mut i = 1i32;
    let mut sign = -1.0;
    loop {
        let term = q.powi(i * i) * ((i * 2) as f64 * c * pi / order).cos() * sign;
        den += term;
        sign = -sign;
        i += 1;
        if term.abs() <= 1e-100 {
            break;
        }
    }

    let ww = num * q.powf(0.25) / (den + 0.5);
    let wwsq = ww * ww;
    let x = ((1.0 - wwsq * k) * (1.0 - wwsq / k)).sqrt() / (1.0 + wwsq);
    (1.0 - x) / (1.0 + x)
}

/// All-pass memory for both paths of one stage (one channel, one direction).
#[derive(Clone, Copy, Debug, Default)]
pub struct HalfBandState {
    x: [f32; MAX_COEFS],
    y: [f32; MAX_COEFS],
}

impl HalfBandState {
    /// Runs `path_0` through coefficients 0, 2, 4, … and `path_1` through
    /// 1, 3, 5, …, each section `y = (x − y₋₁)·c + x₋₁`.
    #[inline(always)]
    fn run(&mut self, coefs: &HalfBandCoefs, mut path_0: f32, mut path_1: f32) -> (f32, f32) {
        let len = coefs.len;
        let mut i = 0;
        while i + 1 < len {
            let (x0, x1) = (self.x[i], self.x[i + 1]);
            self.x[i] = path_0;
            self.x[i + 1] = path_1;
            path_0 = (path_0 - self.y[i]) * coefs.coefs[i] + x0;
            path_1 = (path_1 - self.y[i + 1]) * coefs.coefs[i + 1] + x1;
            self.y[i] = path_0;
            self.y[i + 1] = path_1;
            i += 2;
        }
        if i < len {
            let x0 = self.x[i];
            self.x[i] = path_0;
            path_0 = (path_0 - self.y[i]) * coefs.coefs[i] + x0;
            self.y[i] = path_0;
        }
        (path_0, path_1)
    }

    /// One low-rate sample in, two high-rate samples out (earlier first),
    /// with unity pass-band gain.
    #[inline(always)]
    pub fn upsample(&mut self, coefs: &HalfBandCoefs, input: f32) -> [f32; 2] {
        let (even, odd) = self.run(coefs, input, input);
        [even, odd]
    }

    /// Two high-rate samples in (earlier first), one low-rate sample out.
    #[inline(always)]
    pub fn downsample(&mut self, coefs: &HalfBandCoefs, input: [f32; 2]) -> f32 {
        let (a, b) = self.run(coefs, input[1], input[0]);
        0.5 * (a + b)
    }

    /// Clears the memory.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Level in dB of `samples` at a bin-centred frequency (`bin` of `n`).
    fn level_db(samples: &[f32], bin: usize) -> f64 {
        let n = samples.len();
        let step = std::f64::consts::TAU / n as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, s) in samples.iter().enumerate() {
            let phase = step * ((bin * i) % n) as f64;
            re += *s as f64 * phase.cos();
            im -= *s as f64 * phase.sin();
        }
        20.0 * (2.0 * (re * re + im * im).sqrt() / n as f64).log10()
    }

    const N: usize = 4_096;

    /// Unit sine at `bin` of a `period`-sample DFT, sample `i`, phase exact.
    fn sine(bin: usize, period: usize, i: usize) -> f32 {
        (std::f64::consts::TAU * ((bin * i) % period) as f64 / period as f64).sin() as f32
    }

    /// Down-samples a high-rate sine at `bin` of `2N` and returns the
    /// low-rate output (N samples after N of warm-up).
    fn downsampled(coefs: &HalfBandCoefs, hi_bin: usize) -> Vec<f32> {
        let mut state = HalfBandState::default();
        let x = |i: usize| sine(hi_bin, 2 * N, i);
        let mut out = Vec::with_capacity(N);
        for frame in 0..2 * N {
            let y = state.downsample(coefs, [x(2 * frame), x(2 * frame + 1)]);
            if frame >= N {
                out.push(y);
            }
        }
        out
    }

    /// Up-samples a low-rate sine at `bin` of `N` and returns the high-rate
    /// output (2N samples after warm-up).
    fn upsampled(coefs: &HalfBandCoefs, lo_bin: usize) -> Vec<f32> {
        let mut state = HalfBandState::default();
        let mut out = Vec::with_capacity(2 * N);
        for frame in 0..2 * N {
            let pair = state.upsample(coefs, sine(lo_bin, N, frame));
            if frame >= N {
                out.extend_from_slice(&pair);
            }
        }
        out
    }

    /// The four stages the clipper uses (pass edge 22/48 of the base rate).
    fn clipper_stages() -> Vec<(f64, HalfBandCoefs)> {
        (1..=4)
            .map(|stage| {
                let transition = 0.25 - (22.0 / 48.0) / f64::from(1u32 << stage);
                (transition, HalfBandCoefs::design(90.0, transition))
            })
            .collect()
    }

    #[test]
    #[ignore = "measurement: prints each clipper stage's pass-band level and rejection"]
    fn print_stage_responses() {
        for (stage, (transition, coefs)) in clipper_stages().into_iter().enumerate() {
            println!(
                "stage {}: transition {transition:.4}, {} coefs {:?}",
                stage + 1,
                coefs.len(),
                coefs.as_slice()
            );
            for f in [0.05, 0.2, 0.25 - transition, 0.25 + transition, 0.4, 0.49] {
                let hi_bin = ((f * (2 * N) as f64).round() as usize).clamp(1, N - 1);
                let (out_bin, label) = if hi_bin < N / 2 {
                    (hi_bin, "kept")
                } else {
                    (N - hi_bin, "folded")
                };
                let level = level_db(&downsampled(&coefs, hi_bin), out_bin);
                println!("  f {f:.4} of high rate: {label} {level:+.6} dB");
            }
        }
    }

    #[test]
    fn clipper_stage_sizes() {
        let lens: Vec<usize> = clipper_stages().iter().map(|(_, c)| c.len()).collect();
        assert_eq!(lens, vec![9, 5, 4, 4]);
    }

    #[test]
    fn each_stage_is_flat_in_its_pass_band_and_rejects_its_stop_band() {
        let mut failures = Vec::new();
        for (stage, (transition, coefs)) in clipper_stages().into_iter().enumerate() {
            // Fractions of the high rate (2N bins span it).
            let pass_edge = 0.25 - transition;
            let stop_edge = 0.25 + transition;
            let hi_bins = 2 * N;
            for frac in [0.01, 0.1, 0.5, 0.9, 1.0] {
                let f = pass_edge * frac;
                let hi_bin = ((f * hi_bins as f64).round() as usize).max(1);
                // Down: high-rate pass-band tone keeps its level.
                let down = level_db(&downsampled(&coefs, hi_bin), hi_bin);
                // Up: low-rate tone keeps its level at the high rate…
                let up_out = upsampled(&coefs, hi_bin);
                let up = level_db(&up_out, hi_bin);
                // …and its image (mirrored about a quarter of the high rate)
                // is rejected.
                let image = level_db(&up_out, N - hi_bin);
                if down.abs() > 0.001 || up.abs() > 0.001 || image > -90.0 {
                    failures.push(format!(
                        "stage {}: pass {f:.4}: down {down:+.5} dB, up {up:+.5} dB, image {image:.1} dB",
                        stage + 1
                    ));
                }
            }
            for frac in [0.0, 0.25, 0.5, 0.75, 0.99] {
                let f = stop_edge + (0.5 - stop_edge) * frac;
                let hi_bin = ((f * hi_bins as f64).round() as usize).min(N - 1);
                // Down: a stop-band tone folds to `N − hi_bin` and must be gone.
                let folded = level_db(&downsampled(&coefs, hi_bin), N - hi_bin);
                if folded > -90.0 {
                    failures.push(format!(
                        "stage {}: stop {f:.4}: folded {folded:.1} dB",
                        stage + 1
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}

//! Four-pole ladder-style filter used in the post chain.
//!
//! Despite the historical `WdfLadderFilter` type name (kept for ABI/preset
//! stability), this stage is **not** a Wave Digital Filter circuit solver
//! and does not perform iterative Newton-Raphson on a non-linear circuit.
//! It is a much cheaper approximation: four cascaded TPT (topology-preserving
//! transform) one-pole low-pass stages with transistor-style soft-saturation
//! between stages, plus zero-delay resonance feedback — i.e. a Moog-flavored
//! topology, not an electrical-domain WDF.
//!
//! Resonance: the feedback `u = x − k·y₄` is solved each sample against the
//! linear cascade (no extra sample of delay), with `k = 4·resonance / 0.9`.
//! The ladder therefore self-oscillates exactly when `k ≥ 4`, i.e. from
//! resonance [`SELF_OSCILLATION_RESONANCE`] (0.9) up, at every cutoff and
//! sample rate; below that, tails always decay. The saturators bound the
//! oscillation.
//!
//! Coefficients: the cutoff is prewarped with `g = tan(π·fc/fs)` and `fc` is
//! clamped to `0.49·fs`, which keeps every stage stable at any host cutoff
//! and sample rate. Each stage has unity DC gain and its −3 dB point at `fc`.
//! (Before 2026-10, `g = tan(2π·fc/fs)` with a 0.5-DC-gain stage made the
//! filter latch to a DC rail for `fc ≥ fs/4`, including the 20 kHz default —
//! review finding C1, OpenSpec change `fix-ladder-filter-prewarp`.)
//!
//! See `docs/original-specs/post-processing.md` for the contractual
//! intended response; this file describes the shipped approximation
//! (P1.7 decision banner in `docs/ARCHITECTURE.md` §9).
#[derive(Clone, Debug)]
/// Four-pole ladder-style filter approximation with resonance and saturation.
/// Not a circuit-solver WDF — see the module docstring above.
pub struct WdfLadderFilter {
    /// Smoothed (audible) cutoff used to compute `g`. Lags `target_cutoff_hz`
    /// by a few ms so rapid host automation never zippers.
    cutoff_hz: f32,
    target_cutoff_hz: f32,
    /// Smoothed (audible) resonance, paired with `cutoff_hz`.
    resonance: f32,
    target_resonance: f32,
    component_tolerance: f32,

    /// Trapezoidal integrator state, one per stage (linear, unsaturated).
    s: [f32; 4],
    /// Noise for the component-tolerance jitter (`resolve-exciter-tradeoffs`).
    tolerance_noise: crate::dsp::utils::noise::Noise,
    /// Slow random drift of the components (unit variance), its smoother's
    /// coefficient and the gain that normalizes the smoothed noise
    /// (`fix-inert-controls`).
    drift: f32,
    drift_coeff: f32,
    drift_norm: f32,

    // Cached coefficients
    g: f32,
    g_comp: f32,
    sample_rate: f32,
    /// Per-sample one-pole smoother coefficient (≈1 - exp(-1/(tau·sr))).
    smoothing_coeff: f32,
    /// Whether at least one `set_parameters` call has happened since
    /// new/reset. The first call snaps; later calls smooth.
    smoothing_primed: bool,
}

/// Resonance at and above which the ladder self-oscillates.
pub const SELF_OSCILLATION_RESONANCE: f32 = 0.9;

impl WdfLadderFilter {
    /// One-pole smoother time constant in seconds. Slow enough to kill zipper
    /// noise without making knob movement feel laggy.
    const SMOOTHING_TAU_SECONDS: f32 = 0.02;

    /// Highest effective cutoff as a fraction of the sample rate.
    const MAX_CUTOFF_RATIO: f32 = 0.49;

    /// How slowly the components drift, and how far the cutoff moves at full
    /// Component Tolerance (one standard deviation).
    const DRIFT_TAU_SECONDS: f32 = 0.3;
    const DRIFT_DEPTH: f32 = 0.3;

    /// Creates the filter with a wide-open cutoff and zero resonance.
    pub fn new() -> Self {
        let mut filter = Self {
            cutoff_hz: 20000.0,
            target_cutoff_hz: 20000.0,
            resonance: 0.0,
            target_resonance: 0.0,
            component_tolerance: 0.0,
            s: [0.0; 4],
            tolerance_noise: crate::dsp::utils::noise::Noise::default(),
            drift: 0.0,
            drift_coeff: 0.0,
            drift_norm: 0.0,
            g: 0.0,
            g_comp: 0.0,
            sample_rate: 48000.0,
            smoothing_coeff: 0.0,
            smoothing_primed: false,
        };
        filter.recompute_smoothing_coeff();
        filter.update_coefficients();
        filter
    }

    /// Sets cutoff, resonance, and component tolerance.
    ///
    /// Cutoff and resonance update *targets*; the smoothers track them at
    /// audio rate inside `process()`. Component tolerance jumps immediately
    /// because it is a noise scale, not a coefficient. The first call after
    /// `new`/`reset` snaps so initial setup reaches the requested response
    /// without an audible ramp.
    pub fn set_parameters(&mut self, cutoff_hz: f32, resonance: f32, tolerance: f32) {
        self.target_cutoff_hz = cutoff_hz.clamp(20.0, 20000.0);
        self.target_resonance = resonance.clamp(0.0, 1.0);
        self.component_tolerance = tolerance.clamp(0.0, 1.0);
        if !self.smoothing_primed {
            self.smoothing_primed = true;
            self.settle();
        }
    }

    /// Snap the cutoff/resonance smoothers immediately to their targets.
    ///
    /// Used after a `reset()` or in static-config contexts (tests, offline
    /// renders) where the audio-rate ramp would obscure parameter response.
    pub fn settle(&mut self) {
        self.cutoff_hz = self.target_cutoff_hz;
        self.resonance = self.target_resonance;
        self.update_coefficients();
    }

    /// Updates the coefficient cache for a new sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.recompute_smoothing_coeff();
        self.update_coefficients();
    }

    fn recompute_smoothing_coeff(&mut self) {
        let sr = self.sample_rate.max(1.0);
        self.smoothing_coeff = 1.0 - (-1.0 / (Self::SMOOTHING_TAU_SECONDS * sr)).exp();
        self.drift_coeff = 1.0 - (-1.0 / (Self::DRIFT_TAU_SECONDS * sr)).exp();
        // A one-pole smoothing uniform noise (variance 1/3) leaves
        // variance k/(2 − k)/3; this brings it back to 1.
        let k = self.drift_coeff;
        self.drift_norm = (3.0 * (2.0 - k) / k).sqrt();
    }

    fn update_coefficients(&mut self) {
        // Bilinear prewarp `g = tan(π·fc/fs)`. Clamping fc below Nyquist here
        // (not only in `set_parameters`) also covers a later `set_sample_rate`
        // that lowers fs; it keeps `g` finite and positive, so every stage's
        // pole `(1 − g)/(1 + g)` stays inside the unit circle.
        let sr = self.sample_rate.max(1.0);
        // Component Tolerance lets the cutoff drift; exactly the set cutoff
        // at 0.
        let drifted = if self.component_tolerance > 0.0 {
            self.cutoff_hz
                * (1.0 + self.component_tolerance * Self::DRIFT_DEPTH * self.drift).max(0.1)
        } else {
            self.cutoff_hz
        };
        let fc = drifted.min(Self::MAX_CUTOFF_RATIO * sr);
        self.g = (std::f32::consts::PI * fc / sr).tan();

        // Compensation gain for resonance
        self.g_comp = 1.0 / (1.0 + self.resonance * 3.5);
    }

    /// Processes one mono sample through the ladder stages.
    pub fn process(&mut self, input: f32) -> f32 {
        // Advance the cutoff/resonance smoothers and rebuild coefficients if
        // either drifted enough to matter. The `0.005`/`5e-4` thresholds skip
        // tan()/divide work on every steady-state sample without making the
        // ramp audibly stepwise.
        let cutoff_changed = (self.target_cutoff_hz - self.cutoff_hz).abs() > 0.005;
        let resonance_changed = (self.target_resonance - self.resonance).abs() > 5e-4;
        if cutoff_changed || resonance_changed {
            self.cutoff_hz += (self.target_cutoff_hz - self.cutoff_hz) * self.smoothing_coeff;
            self.resonance += (self.target_resonance - self.resonance) * self.smoothing_coeff;
        }
        // The components drift slowly, so the cutoff wanders around its
        // setting (`fix-inert-controls`: the old jitter alone, 60 dB under the
        // signal, was not heard).
        let drifting = self.component_tolerance > 0.0;
        if drifting {
            let noise = self.tolerance_noise.bipolar() * self.drift_norm;
            self.drift += self.drift_coeff * (noise - self.drift);
        }
        if cutoff_changed || resonance_changed || drifting {
            self.update_coefficients();
        }

        // Apply component tolerance as subtle noise/jitter. Gated on the
        // input so silence in stays silence out (and a reset stays silent).
        let tolerance_jitter = if self.component_tolerance > 0.0 && input != 0.0 {
            let noise = self.tolerance_noise.bipolar();
            noise * self.component_tolerance * 0.001
        } else {
            0.0
        };

        let x = input + tolerance_jitter;

        // Each TPT stage is y = G·x + (1 − G)·s. Through the linear cascade
        // the last stage gives y₄ = G⁴·u + Σ, with Σ collected from the
        // integrator states. Solving u = x − k·y₄ for u removes the
        // one-sample feedback delay that made the ladder oscillate at low
        // resonance near high cutoffs.
        let big_g = self.g / (1.0 + self.g);
        let k = self.resonance * 4.0 / SELF_OSCILLATION_RESONANCE;
        let sigma = self
            .s
            .iter()
            .fold(0.0, |acc, &state| acc * big_g + (1.0 - big_g) * state);
        let g4 = big_g * big_g * big_g * big_g;
        let u = (x - k * sigma) / (1.0 + k * g4);

        // 4-pole ladder: unity-DC-gain TPT one-pole stages
        // (v = (x − s)·g/(1+g), y = v + s, s ← y + v), each followed by the
        // transistor saturator. The integrator state stays linear so the
        // saturator cannot latch it; only the signal passed on is shaped.
        let mut stage_in = u;
        for state in self.s.iter_mut() {
            let v = (stage_in - *state) * big_g;
            let y = v + *state;
            *state = y + v;
            stage_in = Self::transistor_saturate(y);
        }

        // Output with compensation
        stage_in * self.g_comp
    }

    fn transistor_saturate(x: f32) -> f32 {
        // Soft clipping approximation of transistor junction
        let threshold = 0.7;
        if x.abs() < threshold {
            x
        } else {
            let sign = x.signum();
            let excess = x.abs() - threshold;
            let compressed = threshold + (1.0 - threshold) * (1.0 - (-excess * 2.0).exp());
            sign * compressed.min(1.0)
        }
    }

    /// Clears the filter state history.
    pub fn reset(&mut self) {
        self.s = [0.0; 4];
        self.tolerance_noise.reseed(0);
        self.drift = 0.0;
        // Re-prime so the next `set_parameters` snaps to the requested values.
        self.smoothing_primed = false;
    }
}

impl Default for WdfLadderFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives `filter` with a one-second sine burst then one second of
    /// silence and returns `(max_abs, all_finite, tail_max_abs)`, where the
    /// tail is the final 100 ms of the silent section. A per-sample maximum
    /// (not a mean) so a ringing or oscillating tail can't average out.
    fn burst_then_silence(
        filter: &mut WdfLadderFilter,
        sr: f32,
        freq_hz: f32,
        amplitude: f32,
    ) -> (f32, bool, f32) {
        let n = sr as usize;
        let tail = (sr * 0.1) as usize;
        let mut max_abs = 0.0_f32;
        let mut finite = true;
        let mut tail_max = 0.0_f32;
        for i in 0..2 * n {
            let x = if i < n {
                amplitude * (2.0 * std::f32::consts::PI * freq_hz * i as f32 / sr).sin()
            } else {
                0.0
            };
            let y = filter.process(x);
            finite &= y.is_finite();
            max_abs = max_abs.max(y.abs());
            if i >= 2 * n - tail {
                tail_max = tail_max.max(y.abs());
            }
        }
        (max_abs, finite, tail_max)
    }

    /// Steady-state RMS of the filter output for a sine input, skipping the
    /// first half of the run so the transient has settled.
    fn steady_rms(filter: &mut WdfLadderFilter, sr: f32, freq_hz: f32, amplitude: f32) -> f32 {
        let n = sr as usize / 2;
        let mut sum_sq = 0.0_f64;
        for i in 0..2 * n {
            let x = amplitude * (2.0 * std::f32::consts::PI * freq_hz * i as f32 / sr).sin();
            let y = filter.process(x);
            if i >= n {
                sum_sq += (y as f64) * (y as f64);
            }
        }
        (sum_sq / n as f64).sqrt() as f32
    }

    #[test]
    fn default_cutoff_does_not_rail_at_48k() {
        // Regression for review C1: the default 20 kHz cutoff used to latch
        // the ladder to a ±1 DC rail at 48 kHz.
        let sr = 48_000.0;
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(sr);
        filter.set_parameters(20_000.0, 0.0, 0.0);
        let (max_abs, finite, tail_max) = burst_then_silence(&mut filter, sr, 1_000.0, 0.1);
        assert!(finite, "output must stay finite");
        assert!(max_abs <= 1.0, "output must stay within ±1, got {max_abs}");
        assert!(
            tail_max < 1e-3,
            "output must return to zero after input stops, tail max {tail_max}"
        );
    }

    #[test]
    fn cutoff_sweep_through_quarter_sample_rate_is_stable() {
        for &sr in &[44_100.0_f32, 48_000.0] {
            for &cutoff in &[0.24 * sr, 0.25 * sr, 0.26 * sr, 0.375 * sr, 20_000.0] {
                let mut filter = WdfLadderFilter::new();
                filter.set_sample_rate(sr);
                filter.set_parameters(cutoff, 0.0, 0.0);
                let (_, finite, tail_max) = burst_then_silence(&mut filter, sr, 1_000.0, 0.1);
                assert!(finite, "non-finite output at sr={sr} cutoff={cutoff}");
                assert!(
                    tail_max < 1e-3,
                    "no decay to zero at sr={sr} cutoff={cutoff}: tail max {tail_max}"
                );
            }
        }
    }

    #[test]
    fn cutoff_above_nyquist_is_clamped() {
        // 20 kHz is above Nyquist at 32 kHz.
        let sr = 32_000.0;
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(sr);
        filter.set_parameters(20_000.0, 0.0, 0.0);
        // Strictly below Nyquist: g = tan(π·fc/fs) is finite and positive
        // only for fc < fs/2.
        assert!(filter.g.is_finite() && filter.g > 0.0, "g = {}", filter.g);
        let (_, finite, tail_max) = burst_then_silence(&mut filter, sr, 1_000.0, 0.1);
        assert!(finite && tail_max < 1e-3, "unstable above Nyquist");

        // Cutoff set first, then the sample rate lowered underneath it.
        let mut filter = WdfLadderFilter::new();
        filter.set_parameters(20_000.0, 0.0, 0.0);
        filter.set_sample_rate(sr);
        assert!(filter.g.is_finite() && filter.g > 0.0, "g = {}", filter.g);
        let (_, finite, tail_max) = burst_then_silence(&mut filter, sr, 1_000.0, 0.1);
        assert!(
            finite && tail_max < 1e-3,
            "unstable after lowering the sample rate below 2·cutoff"
        );
    }

    #[test]
    fn cutoff_tracks_displayed_frequency() {
        // Four identical stages with no resonance: if each stage's −3 dB point
        // is within 900–1100 Hz of a 1 kHz setting, the whole ladder's gain at
        // 1 kHz relative to its low-frequency gain is (1/√2)^4 ≈ 0.25, with
        // 0.20 and 0.30 bounding the ±10 % window. Small amplitude keeps the
        // saturator out of the measurement.
        let sr = 48_000.0;
        let amplitude = 0.05;
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(sr);
        filter.set_parameters(1_000.0, 0.0, 0.0);
        let low = steady_rms(&mut filter, sr, 50.0, amplitude);
        filter.reset();
        filter.set_parameters(1_000.0, 0.0, 0.0);
        let at_cutoff = steady_rms(&mut filter, sr, 1_000.0, amplitude);
        let ratio = at_cutoff / low;
        assert!(
            (0.20..=0.30).contains(&ratio),
            "gain at cutoff relative to passband should be ≈0.25, got {ratio}"
        );
    }

    #[test]
    fn passband_has_unity_gain() {
        // Each TPT stage has unity DC gain, so with no resonance a low tone
        // well under the cutoff passes at (nearly) its input level.
        let sr = 48_000.0;
        let amplitude = 0.1;
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(sr);
        filter.set_parameters(20_000.0, 0.0, 0.0);
        let rms = steady_rms(&mut filter, sr, 100.0, amplitude);
        let gain = rms / (amplitude / std::f32::consts::SQRT_2);
        assert!(
            (0.97..=1.01).contains(&gain),
            "passband gain should be ≈1.0, got {gain}"
        );
    }

    #[test]
    fn max_resonance_stays_bounded() {
        for &sr in &[44_100.0_f32, 48_000.0, 96_000.0] {
            for &cutoff in &[100.0, 1_000.0, 5_000.0, 0.25 * sr, 20_000.0] {
                let mut filter = WdfLadderFilter::new();
                filter.set_sample_rate(sr);
                filter.set_parameters(cutoff, 1.0, 0.0);
                let (max_abs, finite, _) = burst_then_silence(&mut filter, sr, 1_000.0, 0.5);
                assert!(finite, "non-finite at sr={sr} cutoff={cutoff}");
                assert!(max_abs <= 1.0, "unbounded at sr={sr} cutoff={cutoff}");
            }
        }
    }

    #[test]
    fn max_resonance_tail_stays_bounded_while_oscillating() {
        // Resonance 1.0 self-oscillates; the saturators must keep the
        // sustained tail bounded too.
        for &sr in &[44_100.0_f32, 48_000.0, 96_000.0] {
            for &cutoff in &[100.0, 1_000.0, 5_000.0, 0.25 * sr, 20_000.0] {
                let mut filter = WdfLadderFilter::new();
                filter.set_sample_rate(sr);
                filter.set_parameters(cutoff, 1.0, 0.0);
                let (_, finite, tail_max) = burst_then_silence(&mut filter, sr, 1_000.0, 0.5);
                assert!(
                    finite && tail_max <= 1.0,
                    "sr={sr} cutoff={cutoff}: {tail_max}"
                );
            }
        }
    }

    #[test]
    fn resonance_below_ninety_percent_decays() {
        // Regression for the verification finding: with one-sample-delayed
        // feedback the ladder rang forever at resonance 0.35 and the 20 kHz
        // default cutoff.
        for &sr in &[44_100.0_f32, 48_000.0, 96_000.0] {
            for &cutoff in &[200.0, 1_000.0, 5_000.0, 0.25 * sr, 20_000.0] {
                for &resonance in &[0.35, 0.6, 0.85] {
                    let mut filter = WdfLadderFilter::new();
                    filter.set_sample_rate(sr);
                    filter.set_parameters(cutoff, resonance, 0.0);
                    let (_, finite, tail_max) = burst_then_silence(&mut filter, sr, 1_000.0, 0.5);
                    assert!(
                        finite && tail_max < 1e-3,
                        "sr={sr} cutoff={cutoff} resonance={resonance}: tail max {tail_max}"
                    );
                }
            }
        }
    }

    #[test]
    fn resonance_above_ninety_percent_self_oscillates() {
        for &sr in &[44_100.0_f32, 48_000.0, 96_000.0] {
            for &cutoff in &[200.0, 1_000.0, 5_000.0, 0.25 * sr, 20_000.0] {
                for &resonance in &[0.95, 1.0] {
                    let mut filter = WdfLadderFilter::new();
                    filter.set_sample_rate(sr);
                    filter.set_parameters(cutoff, resonance, 0.0);
                    let (_, _, tail_max) = burst_then_silence(&mut filter, sr, 1_000.0, 0.5);
                    assert!(
                        tail_max > 0.01,
                        "sr={sr} cutoff={cutoff} resonance={resonance}: no oscillation ({tail_max})"
                    );
                }
            }
        }
    }

    /// Energy of `xs` above `min_hz`, from a direct DFT over every fourth bin.
    fn energy_above(xs: &[f32], sr: f32, min_hz: f32) -> f64 {
        let n = xs.len();
        let first_bin = (min_hz / sr * n as f32).ceil() as usize;
        (first_bin..n / 2)
            .step_by(4)
            .map(|bin| {
                let w = std::f64::consts::TAU * bin as f64 / n as f64;
                let (re, im) = xs.iter().enumerate().fold((0.0, 0.0), |(re, im), (i, &x)| {
                    let phase = w * i as f64;
                    (re + x as f64 * phase.cos(), im - x as f64 * phase.sin())
                });
                re * re + im * im
            })
            .sum()
    }

    #[test]
    fn higher_cutoff_passes_more_high_frequency_energy() {
        // Same broadband (white noise) input through 2 kHz and 8 kHz cutoffs;
        // compare the energy above 4 kHz after the filters have settled.
        let sr = 48_000.0;
        let render = |cutoff: f32| {
            let mut filter = WdfLadderFilter::new();
            filter.set_sample_rate(sr);
            filter.set_parameters(cutoff, 0.0, 0.0);
            let mut seed = 0x2468_ace1_u32;
            let out: Vec<f32> = (0..6_144)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let x = (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
                    filter.process(0.2 * x)
                })
                .collect();
            energy_above(&out[2_048..], sr, 4_000.0)
        };
        let (dark, bright) = (render(2_000.0), render(8_000.0));
        assert!(
            bright > 4.0 * dark,
            "8 kHz cutoff should pass much more energy above 4 kHz ({bright}) than 2 kHz ({dark})"
        );
    }

    #[test]
    fn tolerance_adds_nothing_to_silence() {
        // Component tolerance shapes signal; it must not create output from
        // silence, including after a reset.
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(48_000.0);
        filter.set_parameters(5_000.0, 0.3, 1.0);
        for i in 0..4_800 {
            filter.process((i as f32 * 0.05).sin() * 0.5);
        }
        filter.reset();
        filter.set_parameters(5_000.0, 0.3, 1.0);
        for _ in 0..4_800 {
            assert_eq!(
                filter.process(0.0),
                0.0,
                "tolerance jitter leaked into silence"
            );
        }
    }

    #[test]
    fn reset_clears_state_after_loud_input() {
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(48_000.0);
        filter.set_parameters(20_000.0, 0.0, 0.0);
        for i in 0..4_800 {
            filter.process(if i % 2 == 0 { 1.0 } else { -1.0 });
        }
        filter.reset();
        for _ in 0..4_800 {
            assert_eq!(filter.process(0.0), 0.0, "residual output after reset");
        }
    }

    #[test]
    fn filter_processes_without_nan() {
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(48000.0);
        filter.set_parameters(1000.0, 0.5, 0.0);

        for i in 0..100 {
            let input = (i as f32 * 0.01).sin();
            let output = filter.process(input);
            assert!(output.is_finite(), "Output should be finite");
        }
    }

    #[test]
    fn zero_input_produces_zero() {
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(48000.0);

        for _ in 0..50 {
            let output = filter.process(0.0);
            assert!(
                output.abs() < 1e-6,
                "Zero input should produce near-zero output"
            );
        }
    }

    #[test]
    fn mid_stream_cutoff_change_is_smoothed() {
        // After priming, an abrupt cutoff change must not appear as a single-
        // sample step in the audible response. We compare two filters: one
        // that gets the new cutoff applied with smoothing, and one that
        // settles immediately. The smoothed filter must trail the snapped one
        // for at least one millisecond after the change.
        let sr = 48_000.0;
        let mut smoothed = WdfLadderFilter::new();
        smoothed.set_sample_rate(sr);
        smoothed.set_parameters(20_000.0, 0.0, 0.0);
        // Prime past the first-set snap.
        for _ in 0..512 {
            smoothed.process(0.5);
        }

        let mut snapped = smoothed.clone();
        snapped.set_parameters(200.0, 0.0, 0.0);
        snapped.settle();
        smoothed.set_parameters(200.0, 0.0, 0.0);

        let mut delta = 0.0_f32;
        for i in 0..48 {
            let x = ((i as f32) * 0.3).sin();
            delta += (smoothed.process(x) - snapped.process(x)).abs();
        }
        assert!(
            delta > 0.0,
            "smoothed filter should lag the snapped one immediately after a cutoff change"
        );
    }

    #[test]
    fn low_cutoff_attenuates_high_freqs() {
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(48000.0);
        filter.set_parameters(100.0, 0.0, 0.0);

        // High frequency input
        let mut high_energy = 0.0f32;
        for i in 0..480 {
            let input = (i as f32 * 0.5).sin(); // High freq
            high_energy += filter.process(input).abs();
        }

        filter.reset();
        filter.set_parameters(5000.0, 0.0, 0.0);

        let mut low_energy = 0.0f32;
        for i in 0..480 {
            let input = (i as f32 * 0.5).sin();
            low_energy += filter.process(input).abs();
        }

        assert!(high_energy < low_energy, "Low cutoff should attenuate more");
    }

    /// Spread (dB) between the loudest and quietest 50 ms of 4 s of white
    /// noise through a resonant 1.5 kHz low-pass at `tolerance`.
    fn drift_spread_db(tolerance: f32) -> f32 {
        let mut filter = WdfLadderFilter::new();
        filter.set_sample_rate(48_000.0);
        filter.set_parameters(1500.0, 0.6, tolerance);
        let mut noise = crate::dsp::utils::noise::Noise::default();
        let windows: Vec<f32> = (0..80)
            .map(|_| {
                let energy: f32 = (0..2_400)
                    .map(|_| filter.process(noise.bipolar() * 0.3).powi(2))
                    .sum();
                10.0 * (energy / 2_400.0).log10()
            })
            .collect();
        let (lo, hi) = windows
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        hi - lo
    }

    /// `fix-inert-controls`: Component Tolerance lets the cutoff drift.
    #[test]
    fn component_tolerance_drifts_the_cutoff() {
        let (exact, drifting) = (drift_spread_db(0.0), drift_spread_db(1.0));
        assert!(
            drifting - exact >= 2.0,
            "50 ms level spread at tolerance 0: {exact:.2} dB, at 1: {drifting:.2} dB"
        );
    }
}

//! Oversampled diode stage: the post chain's final clipper.
//!
//! The input is up-sampled through cascaded 2× half-band IIR stages, the
//! diode curve runs on every high-rate sample, and the same stages bring it
//! back down. Normal, High and Render use 2, 3 and 4 stages (4×, 8×, 16×);
//! Eco runs the curve at the host rate. Quality only changes how much the
//! clipping aliases: the curve, its headroom and its DC removal are the same
//! in every mode (`add-asymmetric-diode-stage`).
//!
//! The curve is a tanh knee per polarity. Its limit is the Analog Ceiling ÷
//! `CEILING_HEADROOM`, which leaves room for the down-sampling filters'
//! overshoot under the output limiter. Diode Softness sets the knee's width
//! (hard at 0, starting 14 dB under the limit at 1); Diode Asymmetry lowers
//! the negative limit (6 dB at 1) for even harmonics, and a 2 Hz DC blocker
//! removes the offset that creates. The half-bands delay low frequencies by
//! about 4–5 samples, which isn't reported as latency
//! (`m06-m09-smooth-starvation-and-oversample-clipper`).
use super::half_band::{HalfBandCoefs, HalfBandState};
use std::sync::atomic::{AtomicU32, Ordering};

/// The diode stage's level reduction, shared between the audio thread and
/// the editor (`add-asymmetric-diode-stage`). It holds the largest reduction,
/// in dB, published since the editor last took it. Atomics only: no locks,
/// no allocation.
#[derive(Debug, Default)]
pub struct DiodeMeter {
    /// f32 bits of a non-negative dB value. For non-negative floats the bit
    /// patterns order like the values, so `fetch_max` keeps the larger one.
    reduction_db_bits: AtomicU32,
}

impl DiodeMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a block's reduction (dB, 0 or more), keeping the largest
    /// since the last `take`.
    pub fn publish(&self, reduction_db: f32) {
        let db = if reduction_db.is_finite() {
            reduction_db.max(0.0)
        } else {
            0.0
        };
        self.reduction_db_bits
            .fetch_max(db.to_bits(), Ordering::Relaxed);
    }

    /// The largest reduction published since the last call, then clears it.
    pub fn take(&self) -> f32 {
        f32::from_bits(self.reduction_db_bits.swap(0, Ordering::Relaxed))
    }
}

/// Most 2× stages (16× oversampling).
const MAX_STAGES: usize = 4;
/// Most high-rate samples per input sample.
const MAX_FACTOR: usize = 1 << MAX_STAGES;
/// Stop-band rejection each half-band is designed for.
const STAGE_ATTENUATION_DB: f64 = 90.0;
/// Pass-band edge as a fraction of the host rate (22 kHz at 48 kHz).
const PASS_EDGE: f64 = 22.0 / 48.0;
/// The curve's limit is the ceiling divided by this, in every quality mode,
/// so the down-sampling filters' overshoot (the harmonics they remove were
/// holding the peaks down) and the DC removal's shift stay under the output
/// limiter that follows.
const CEILING_HEADROOM: f32 = 1.3;
/// Knee width as a fraction of the limit at Diode Softness 1: the knee
/// starts 14 dB under the limit.
const KNEE_WIDTH: f32 = 0.8;
/// Smoothing time of Diode Softness and Diode Asymmetry, as the drive's.
const CONTROL_TAU_SECONDS: f32 = 0.02;
/// A smoothed control within this of its target snaps to it, so the
/// settled path stops recomputing the curve's constants.
const CONTROL_SNAP: f32 = 1e-5;
/// The DC blocker's cutoff. Low, because a high-pass tilts the flat tops of
/// a clipped wave: at 2 Hz a hard-clipped 200 Hz tone spans 1.4 % more
/// than the curve's limits (3.4 % at 5 Hz).
const DC_CUTOFF_HZ: f32 = 2.0;
/// Diode Asymmetry at which the DC blocker is fully in; below it the
/// blocker fades in with the asymmetry.
const DC_FULL_ASYMMETRY: f32 = 0.05;

/// Points in the tanh table, over `[0, TANH_TABLE_MAX]`.
const TANH_TABLE_POINTS: usize = 513;
/// Past this the table returns its last point (tanh(8) = 1 − 2.3e-7).
const TANH_TABLE_MAX: f32 = 8.0;
static TANH_TABLE: std::sync::OnceLock<[f32; TANH_TABLE_POINTS]> = std::sync::OnceLock::new();

/// The tanh table, built once (from `OversampledClipper::new`, off the audio
/// thread; no heap allocation).
fn tanh_table() -> &'static [f32; TANH_TABLE_POINTS] {
    TANH_TABLE.get_or_init(|| {
        std::array::from_fn(|i| {
            (f64::from(TANH_TABLE_MAX) * i as f64 / (TANH_TABLE_POINTS - 1) as f64).tanh() as f32
        })
    })
}

/// tanh of `x` (0 or more) by linear interpolation in the table: error under
/// 2.5e-5, monotonic, and the same on every libm. It took 0.63× libm's time
/// for the clipper alone at 16× (`add-asymmetric-diode-stage`, task 2.3).
#[inline(always)]
fn table_tanh(x: f32, table: &[f32; TANH_TABLE_POINTS]) -> f32 {
    let position = x.min(TANH_TABLE_MAX) * ((TANH_TABLE_POINTS - 1) as f32 / TANH_TABLE_MAX);
    let index = (position as usize).min(TANH_TABLE_POINTS - 2);
    let fraction = position - index as f32;
    table[index] + (table[index + 1] - table[index]) * fraction
}

/// One polarity of the curve: linear up to `knee`, then
/// `knee + span · tanh((x − knee) / span)`, which never passes
/// `knee + span`, the limit. A hard knee has `span` 0 (and `inv_span` 0).
#[derive(Clone, Copy, Debug, Default)]
struct Knee {
    knee: f32,
    span: f32,
    inv_span: f32,
}

impl Knee {
    fn new(limit: f32, width: f32) -> Self {
        let span = limit * width.clamp(0.0, 1.0);
        Self {
            knee: limit - span,
            span,
            inv_span: if span > 0.0 { 1.0 / span } else { 0.0 },
        }
    }

    /// The curve on a magnitude `x` (0 or more).
    #[inline(always)]
    fn apply(&self, x: f32, table: &[f32; TANH_TABLE_POINTS]) -> f32 {
        if x <= self.knee {
            x
        } else {
            self.knee + self.span * table_tanh((x - self.knee) * self.inv_span, table)
        }
    }
}

/// The output clipper: the diode curve, oversampled per quality mode.
pub struct OversampledClipper {
    ceiling: f32,
    /// The ceiling ÷ this is the curve's limit (`CEILING_HEADROOM`; tests
    /// sweep it).
    headroom: f32,
    /// Diode Softness and Asymmetry: targets and smoothed values.
    softness_target: f32,
    asymmetry_target: f32,
    softness: f32,
    asymmetry: f32,
    /// Per-host-sample smoothing step for both controls.
    control_coeff: f32,
    /// False until the first sample: settings before it apply at once.
    primed: bool,
    /// The curve's halves at the smoothed settings.
    positive: Knee,
    negative: Knee,
    /// DC blocker: pole, weight (0 bypasses it), and per-channel memory
    /// (previous input and output).
    dc_pole: f32,
    dc_weight: f32,
    dc_x1: [f32; 2],
    dc_y1: [f32; 2],
    /// Peaks of the curve's input and output since the meter last read them.
    peak_in: f32,
    peak_out: f32,
    sample_rate: f32,
    /// Test-only switch: false skips the DC removal, to compare against the
    /// bare curve.
    #[cfg(test)]
    dc_stage_enabled: bool,
    /// 2× stages in use: 0 (1×) to `MAX_STAGES` (16×).
    stages: usize,
    /// Stage `i` runs between `2^i` and `2^(i+1)` times the host rate.
    coefs: [HalfBandCoefs; MAX_STAGES],
    /// Up- and down-sampling memory per channel and stage.
    up: [[HalfBandState; MAX_STAGES]; 2],
    down: [[HalfBandState; MAX_STAGES]; 2],
}

impl OversampledClipper {
    /// Creates the clipper at the default ceiling, Diode Softness 0.5 and
    /// Diode Asymmetry 0.25, 48 kHz and 16× oversampling.
    pub fn new() -> Self {
        let coefs = std::array::from_fn(|stage| {
            // The pass edge stays fixed relative to the host rate, so later
            // stages, at higher rates, get ever wider transitions.
            let transition = 0.25 - PASS_EDGE / f64::from(2u32 << stage);
            HalfBandCoefs::design(STAGE_ATTENUATION_DB, transition)
        });
        // Build the shared tanh table here, not on the audio thread.
        let _ = tanh_table();
        let mut clipper = Self {
            ceiling: 0.9661,
            headroom: CEILING_HEADROOM,
            softness_target: 0.5,
            asymmetry_target: 0.25,
            softness: 0.5,
            asymmetry: 0.25,
            control_coeff: 0.0,
            primed: false,
            positive: Knee::default(),
            negative: Knee::default(),
            dc_pole: 0.0,
            dc_weight: 0.0,
            dc_x1: [0.0; 2],
            dc_y1: [0.0; 2],
            peak_in: 0.0,
            peak_out: 0.0,
            sample_rate: 48_000.0,
            #[cfg(test)]
            dc_stage_enabled: true,
            stages: MAX_STAGES,
            coefs,
            up: [[HalfBandState::default(); MAX_STAGES]; 2],
            down: [[HalfBandState::default(); MAX_STAGES]; 2],
        };
        clipper.set_sample_rate(48_000.0);
        clipper.update_curve();
        clipper
    }

    /// Sets the ceiling, Diode Softness and Diode Asymmetry. Softness and
    /// Asymmetry glide to their new values (20 ms) once audio is running;
    /// the ceiling applies at once.
    pub fn set_parameters(&mut self, ceiling: f32, softness: f32, asymmetry: f32) {
        self.ceiling = ceiling.clamp(0.5, 1.0);
        self.softness_target = softness.clamp(0.0, 1.0);
        self.asymmetry_target = asymmetry.clamp(0.0, 1.0);
        if !self.primed {
            self.snap_controls();
        }
        self.update_curve();
    }

    /// Sets the host sample rate: the controls' smoothing and the DC
    /// blocker's pole.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.control_coeff = 1.0 - (-1.0 / (CONTROL_TAU_SECONDS * self.sample_rate)).exp();
        self.dc_pole = (-std::f32::consts::TAU * DC_CUTOFF_HZ / self.sample_rate).exp();
    }

    /// The curve's positive limit, the same in every quality mode.
    pub fn limit(&self) -> f32 {
        self.ceiling / self.headroom
    }

    /// Test-only: a different headroom, for the sweep that sets
    /// `CEILING_HEADROOM`.
    #[cfg(test)]
    pub(crate) fn set_headroom_for_test(&mut self, headroom: f32) {
        self.headroom = headroom;
        self.update_curve();
    }

    /// Sets the oversampling factor: 1, 2, 4, 8 or 16 (others round down to
    /// a power of two). A change clears the filters' memory.
    pub fn set_oversample_factor(&mut self, factor: usize) {
        let factor = factor.clamp(1, MAX_FACTOR);
        let stages = (usize::BITS - 1 - factor.leading_zeros()) as usize;
        if stages != self.stages {
            self.stages = stages;
            self.clear_filters();
        }
    }

    /// Processes one mono sample.
    pub fn process(&mut self, input: f32) -> f32 {
        self.advance_controls();
        self.process_channel(input, 0)
    }

    /// Processes a stereo frame, each channel through its own filters.
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        self.advance_controls();
        (
            self.process_channel(left, 0),
            self.process_channel(right, 1),
        )
    }

    /// The level reduction since the last call, in dB: the curve input's
    /// peak over its output's peak, 0 when the input stayed under the knee.
    pub fn take_block_reduction_db(&mut self) -> f32 {
        let reduction = if self.peak_out > 0.0 && self.peak_in > self.peak_out {
            20.0 * (self.peak_in / self.peak_out).log10()
        } else {
            0.0
        };
        self.peak_in = 0.0;
        self.peak_out = 0.0;
        reduction
    }

    /// Clears the filters' and the DC blocker's memory and settles the
    /// controls at their targets.
    pub fn reset(&mut self) {
        self.clear_filters();
        self.snap_controls();
        self.update_curve();
        self.peak_in = 0.0;
        self.peak_out = 0.0;
    }

    fn clear_filters(&mut self) {
        for state in self.up.iter_mut().chain(self.down.iter_mut()).flatten() {
            state.reset();
        }
        self.dc_x1 = [0.0; 2];
        self.dc_y1 = [0.0; 2];
    }

    fn snap_controls(&mut self) {
        self.softness = self.softness_target;
        self.asymmetry = self.asymmetry_target;
    }

    /// One host sample of control smoothing. The curve's constants are
    /// recomputed only while a control is still moving.
    #[inline]
    fn advance_controls(&mut self) {
        self.primed = true;
        if self.softness == self.softness_target && self.asymmetry == self.asymmetry_target {
            return;
        }
        self.softness = Self::glide(self.softness, self.softness_target, self.control_coeff);
        self.asymmetry = Self::glide(self.asymmetry, self.asymmetry_target, self.control_coeff);
        self.update_curve();
    }

    fn glide(value: f32, target: f32, coeff: f32) -> f32 {
        let next = value + (target - value) * coeff;
        if (target - next).abs() < CONTROL_SNAP {
            target
        } else {
            next
        }
    }

    /// Rebuilds both halves of the curve, and the DC blocker's weight, from
    /// the ceiling and the smoothed controls.
    fn update_curve(&mut self) {
        let limit = self.limit();
        let width = KNEE_WIDTH * self.softness;
        self.positive = Knee::new(limit, width);
        // One diode against two in series at Asymmetry 1: the negative half
        // conducts 6 dB earlier, with a firmer knee.
        self.negative = Knee::new(
            limit * (-self.asymmetry).exp2(),
            width * (1.0 - 0.5 * self.asymmetry),
        );
        self.dc_weight = (self.asymmetry / DC_FULL_ASYMMETRY).min(1.0);
        if self.dc_weight == 0.0 {
            self.dc_x1 = [0.0; 2];
            self.dc_y1 = [0.0; 2];
        }
    }

    /// Up-samples one channel stage by stage, runs the curve on every
    /// high-rate sample, down-samples back through the stages in reverse,
    /// and removes the DC.
    fn process_channel(&mut self, input: f32, channel: usize) -> f32 {
        let table = tanh_table();
        if self.stages == 0 {
            let clipped = self.clip_and_meter(input, table);
            return self.remove_dc(clipped, channel);
        }

        let mut a = [0.0f32; MAX_FACTOR];
        let mut b = [0.0f32; MAX_FACTOR];
        a[0] = input;
        let mut len = 1;
        for stage in 0..self.stages {
            let (coefs, state) = (&self.coefs[stage], &mut self.up[channel][stage]);
            for i in 0..len {
                let pair = state.upsample(coefs, a[i]);
                b[2 * i] = pair[0];
                b[2 * i + 1] = pair[1];
            }
            len *= 2;
            std::mem::swap(&mut a, &mut b);
        }

        for i in 0..len {
            a[i] = self.clip_and_meter(a[i], table);
        }

        for stage in (0..self.stages).rev() {
            let (coefs, state) = (&self.coefs[stage], &mut self.down[channel][stage]);
            len /= 2;
            for i in 0..len {
                b[i] = state.downsample(coefs, [a[2 * i], a[2 * i + 1]]);
            }
            std::mem::swap(&mut a, &mut b);
        }
        self.remove_dc(a[0], channel)
    }

    /// The curve on one sample, tracking the meter's peaks.
    #[inline(always)]
    fn clip_and_meter(&mut self, x: f32, table: &[f32; TANH_TABLE_POINTS]) -> f32 {
        let y = self.diode_clip(x, table);
        self.peak_in = self.peak_in.max(x.abs());
        self.peak_out = self.peak_out.max(y.abs());
        y
    }

    #[inline(always)]
    fn diode_clip(&self, x: f32, table: &[f32; TANH_TABLE_POINTS]) -> f32 {
        if x >= 0.0 {
            self.positive.apply(x, table)
        } else {
            -self.negative.apply(-x, table)
        }
    }

    /// The 2 Hz DC blocker, faded in by the asymmetry. At Asymmetry 0 it is
    /// out of the path, so a symmetric curve is unfiltered.
    #[inline(always)]
    fn remove_dc(&mut self, y: f32, channel: usize) -> f32 {
        #[cfg(test)]
        if !self.dc_stage_enabled {
            return y;
        }
        if self.dc_weight == 0.0 {
            return y;
        }
        let high_passed = y - self.dc_x1[channel] + self.dc_pole * self.dc_y1[channel];
        self.dc_x1[channel] = y;
        self.dc_y1[channel] = high_passed;
        y + self.dc_weight * (high_passed - y)
    }

    /// The diode curve on one sample at the current settings, without the
    /// DC removal or any filtering.
    #[cfg(test)]
    pub(crate) fn curve(&self, x: f32) -> f32 {
        self.diode_clip(x, tanh_table())
    }
}

impl Default for OversampledClipper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- `diode-stage` (`add-asymmetric-diode-stage`) ---

    #[test]
    fn table_tanh_is_accurate_and_monotonic() {
        // Decision 7: error under 1e-4 and monotonic, checked densely past
        // the table's end.
        let table = tanh_table();
        let mut worst = 0.0f64;
        let mut previous = 0.0f32;
        for i in 0..=1_200_000 {
            let x = i as f32 * 1e-5;
            let y = table_tanh(x, table);
            worst = worst.max((f64::from(y) - f64::from(x).tanh()).abs());
            assert!(y >= previous, "not monotonic at {x}");
            previous = y;
        }
        assert!(worst < 1e-4, "worst error {worst:.2e}");
    }

    /// Eco, Normal, High and Render.
    const FACTORS: [usize; 4] = [1, 4, 8, 16];
    const CEILING: f32 = 0.9661;
    const RATE: f32 = 48_000.0;
    /// One analysis window: bins 10 Hz apart at 48 kHz.
    const WINDOW: usize = 4_800;

    fn diode(factor: usize, softness: f32, asymmetry: f32) -> OversampledClipper {
        let mut c = OversampledClipper::new();
        c.set_sample_rate(RATE);
        c.set_oversample_factor(factor);
        c.set_parameters(CEILING, softness, asymmetry);
        c.reset();
        c
    }

    /// Runs `seconds` of a sine at `hz` and `amplitude` and returns the
    /// output's last `WINDOW` samples (or every sample with `keep_all`).
    fn run_sine(c: &mut OversampledClipper, hz: f32, amplitude: f32, seconds: f32) -> Vec<f32> {
        let total = (seconds * RATE) as usize;
        let step = std::f64::consts::TAU * f64::from(hz) / f64::from(RATE);
        let out: Vec<f32> = (0..total)
            .map(|i| c.process((f64::from(amplitude) * (step * i as f64).sin()) as f32))
            .collect();
        out[total - WINDOW..].to_vec()
    }

    /// Magnitude of `bin` in `samples` (f64 DFT, exact phase reduction).
    fn magnitude(samples: &[f32], bin: usize) -> f64 {
        let n = samples.len();
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, x) in samples.iter().enumerate() {
            let phase = std::f64::consts::TAU * ((bin * i) % n) as f64 / n as f64;
            re += f64::from(*x) * phase.cos();
            im -= f64::from(*x) * phase.sin();
        }
        (re * re + im * im).sqrt()
    }

    /// The output level of a sine at `amplitude` relative to its input, in
    /// dB, at its fundamental.
    fn level_db(c: &mut OversampledClipper, hz: f32, amplitude: f32) -> f64 {
        let out = run_sine(c, hz, amplitude, 0.5);
        let bin = (hz * WINDOW as f32 / RATE).round() as usize;
        let input = f64::from(amplitude) * WINDOW as f64 / 2.0;
        20.0 * (magnitude(&out, bin) / input).log10()
    }

    #[test]
    fn hard_knee_at_zero() {
        for factor in FACTORS {
            let mut c = diode(factor, 0.0, 0.0);
            let amplitude = 0.9 * c.limit();
            let db = level_db(&mut c, 100.0, amplitude);
            assert!(db.abs() < 0.1, "{factor}x: {db:+.3} dB");
        }
    }

    #[test]
    fn wide_knee_at_one() {
        for factor in FACTORS {
            let mut soft = diode(factor, 1.0, 0.0);
            let amplitude = 0.6 * soft.limit();
            let soft_db = level_db(&mut soft, 100.0, amplitude);
            let hard_db = level_db(&mut diode(factor, 0.0, 0.0), 100.0, amplitude);
            assert!(
                soft_db <= -0.2 && hard_db.abs() < 0.1,
                "{factor}x: Softness 1 {soft_db:+.3} dB, Softness 0 {hard_db:+.3} dB"
            );
        }
    }

    #[test]
    fn softer_knees_add_fewer_high_harmonics() {
        let high_share = |softness: f32| {
            let mut c = diode(16, softness, 0.0);
            let amplitude = 2.0 * c.limit();
            let out = run_sine(&mut c, 1_000.0, amplitude, 0.5);
            let fundamental = magnitude(&out, 100).powi(2);
            let high: f64 = (7..24).map(|h| magnitude(&out, 100 * h).powi(2)).sum();
            10.0 * (high / fundamental).log10()
        };
        let shares: Vec<f64> = [0.0, 0.5, 1.0].map(high_share).to_vec();
        assert!(
            shares[0] > shares[1] && shares[1] > shares[2],
            "7th+ harmonics vs fundamental (dB) at Softness 0, 0.5, 1: {shares:?}"
        );
    }

    #[test]
    fn limits_per_polarity() {
        // Measured in Eco: the curve's limits without the half-bands'
        // ringing on the clipped edges.
        for asymmetry in [0.0f32, 0.25, 1.0] {
            let mut c = diode(1, 0.0, asymmetry);
            let limit = c.limit();
            let out = run_sine(&mut c, 200.0, 4.0 * limit, 1.0);
            let span = out.iter().copied().fold(f32::MIN, f32::max)
                - out.iter().copied().fold(f32::MAX, f32::min);
            let want = limit * (1.0 + 2f32.powf(-asymmetry));
            assert!(
                (span / want - 1.0).abs() < 0.02,
                "asymmetry {asymmetry}: peak-to-peak {span:.4}, want {want:.4}"
            );
        }
    }

    #[test]
    #[ignore = "measurement: peak-to-peak vs the curve's limits, with and without DC removal"]
    fn print_limits_per_polarity() {
        for asymmetry in [0.0f32, 0.25, 0.5, 1.0] {
            for (label, dc, cutoff) in [
                ("no DC", false, DC_CUTOFF_HZ),
                ("5 Hz", true, 5.0),
                ("2 Hz", true, 2.0),
            ] {
                let mut c = diode(1, 0.0, asymmetry);
                c.dc_stage_enabled = dc;
                c.dc_pole = (-std::f32::consts::TAU * cutoff / RATE).exp();
                let limit = c.limit();
                let out = run_sine(&mut c, 200.0, 4.0 * limit, 1.0);
                let span = out.iter().copied().fold(f32::MIN, f32::max)
                    - out.iter().copied().fold(f32::MAX, f32::min);
                let want = limit * (1.0 + 2f32.powf(-asymmetry));
                print!(
                    "a {asymmetry}: {label} {:+.2} %;  ",
                    100.0 * (span / want - 1.0)
                );
            }
            println!();
        }
    }

    #[test]
    fn even_harmonics_follow_asymmetry() {
        let second = |asymmetry: f32| {
            let mut c = diode(16, 0.5, asymmetry);
            let amplitude = 2.0 * c.limit();
            let out = run_sine(&mut c, 200.0, amplitude, 1.0);
            20.0 * (magnitude(&out, 40) / magnitude(&out, 20)).log10()
        };
        let levels: Vec<f64> = [0.0, 0.25, 1.0].map(second).to_vec();
        assert!(
            levels[0] <= -60.0 && levels[0] < levels[1] && levels[1] < levels[2],
            "2nd harmonic vs fundamental (dB) at Asymmetry 0, 0.25, 1: {levels:?}"
        );
    }

    #[test]
    fn asymmetric_clipping_is_centred() {
        for factor in FACTORS {
            let mut c = diode(factor, 0.5, 1.0);
            let amplitude = 2.0 * c.limit();
            let total = (2.0 * RATE) as usize;
            let step = std::f64::consts::TAU * 200.0 / f64::from(RATE);
            let out: Vec<f32> = (0..total)
                .map(|i| c.process((f64::from(amplitude) * (step * i as f64).sin()) as f32))
                .collect();
            let last = &out[total / 2..];
            let mean = last.iter().map(|x| f64::from(*x)).sum::<f64>() / last.len() as f64;
            assert!(
                mean.abs() < 0.001 * f64::from(CEILING),
                "{factor}x: mean of the last second {mean:.5}"
            );
        }
    }

    #[test]
    fn symmetric_curve_is_unfiltered() {
        // At Asymmetry 0 the DC removal is out of the path: the output is
        // bit-identical to the stage without it, in every mode.
        for factor in FACTORS {
            let mut with_dc = diode(factor, 0.5, 0.0);
            let mut without = diode(factor, 0.5, 0.0);
            without.dc_stage_enabled = false;
            for i in 0..4_800 {
                let x = 2.0 * (i as f32 * 0.37).sin() * (i as f32 * 0.0011).cos();
                assert_eq!(
                    with_dc.process(x).to_bits(),
                    without.process(x).to_bits(),
                    "{factor}x, sample {i}"
                );
            }
        }
    }

    #[test]
    fn peaks_stay_under_the_ceiling() {
        let mut over = Vec::new();
        for factor in FACTORS {
            for asymmetry in [0.0f32, 0.25, 1.0] {
                let mut c = diode(factor, 0.5, asymmetry);
                let total = RATE as usize;
                let step = std::f64::consts::TAU * 5_000.0 / f64::from(RATE);
                let peak = (0..total)
                    .map(|i| {
                        let x = 2.0 * f64::from(CEILING) * (step * i as f64).sin();
                        c.process(x as f32).abs()
                    })
                    .fold(0.0f32, f32::max);
                if peak > CEILING {
                    over.push((factor, asymmetry, peak));
                }
            }
        }
        assert!(over.is_empty(), "peaks over {CEILING}: {over:?}");
    }

    #[test]
    fn eco_sounds_like_render() {
        let measure = |factor: usize| {
            let mut c = diode(factor, 0.5, 0.25);
            let out = run_sine(&mut c, 200.0, 2.0 * CEILING, 1.0);
            [20, 40, 60].map(|bin| 20.0 * magnitude(&out, bin).log10())
        };
        let (eco, render) = (measure(1), measure(16));
        assert!(
            (eco[0] - render[0]).abs() < 0.1
                && (eco[1] - render[1]).abs() < 0.5
                && (eco[2] - render[2]).abs() < 0.5,
            "fundamental, 2nd, 3rd (dB): Eco {eco:?}, Render {render:?}"
        );
    }

    /// The largest sample-to-sample step of a 200 Hz sine at twice the
    /// ceiling, with Diode Asymmetry following `asymmetry_at(block)`
    /// (blocks of 256 samples).
    fn largest_step(factor: usize, asymmetry_at: impl Fn(usize) -> f32) -> f32 {
        let mut c = diode(factor, 0.5, asymmetry_at(0));
        let step = std::f64::consts::TAU * 200.0 / f64::from(RATE);
        let mut previous = 0.0f32;
        let mut largest = 0.0f32;
        for i in 0..(RATE as usize) {
            if i % 256 == 0 {
                c.set_parameters(CEILING, 0.5, asymmetry_at(i / 256));
            }
            let y = c.process((2.0 * f64::from(CEILING) * (step * i as f64).sin()) as f32);
            // Skip the first 10 ms, where the sine itself starts.
            if i > 480 {
                largest = largest.max((y - previous).abs());
            }
            previous = y;
        }
        largest
    }

    #[test]
    fn asymmetry_jumps_do_not_click() {
        for factor in FACTORS {
            let held = largest_step(factor, |_| 1.0);
            // 0 for 50 blocks, 1 for 50 blocks, then 0 again.
            let jumping = largest_step(
                factor,
                |block| if (50..100).contains(&block) { 1.0 } else { 0.0 },
            );
            assert!(
                jumping <= 1.5 * held,
                "{factor}x: largest step {jumping:.4} jumping, {held:.4} held at 1"
            );
        }
    }

    #[test]
    #[ignore = "measurement: prints the clipper's peak output per quality"]
    fn print_peaks() {
        for factor in [1, 4, 8, 16] {
            let mut step = OversampledClipper::new();
            step.set_parameters(0.8, 0.0, 0.0);
            step.set_oversample_factor(factor);
            let step_peak = (0..100)
                .map(|_| step.process(2.0).abs())
                .fold(0.0, f32::max);
            print!("{factor:>2}x: step 0->2 at ceiling 0.8 peaks {step_peak:.4};");
            for hz in [100.0, 1_000.0, 5_000.0, 10_000.0, 15_000.0] {
                let mut c = OversampledClipper::new();
                c.set_oversample_factor(factor);
                let peak = (0..48_000)
                    .map(|i| {
                        let x =
                            2.0 * 0.9661 * (std::f32::consts::TAU * hz * i as f32 / 48_000.0).sin();
                        c.process(x).abs()
                    })
                    .skip(4_800)
                    .fold(0.0, f32::max);
                print!(" {hz} Hz at 2x ceiling 0.9661 peaks {peak:.4};");
            }
            println!();
        }
    }

    #[test]
    #[ignore = "measurement: worst peak over the curve's threshold, per factor"]
    fn print_overshoot_sweep() {
        let threshold = 0.8f32;
        for factor in [4, 8, 16] {
            let mut worst = (0.0f32, 0.0f32, 0.0f32);
            for over in [1.0f32, 1.1, 1.25, 1.5, 2.0, 3.0, 4.0, 8.0, 16.0] {
                for hz in [
                    50.0f32, 100.0, 300.0, 1_000.0, 2_000.0, 3_000.0, 5_000.0, 7_000.0, 10_000.0,
                    13_000.0, 15_000.0, 19_000.0,
                ] {
                    for softness in [0.0f32, 0.5, 1.0] {
                        let mut c = OversampledClipper::new();
                        c.set_parameters(threshold, softness, 0.0);
                        c.set_oversample_factor(factor);
                        let peak = (0..24_000)
                            .map(|i| {
                                let x = over
                                    * threshold
                                    * (std::f32::consts::TAU * hz * i as f32 / 48_000.0).sin();
                                c.process(x).abs()
                            })
                            .skip(2_400)
                            .fold(0.0, f32::max);
                        if peak / threshold > worst.0 {
                            worst = (peak / threshold, over, hz);
                        }
                    }
                }
            }
            println!(
                "{factor:>2}x: worst peak {:.4}x threshold at {}x, {} Hz",
                worst.0, worst.1, worst.2
            );
        }
    }

    #[test]
    fn never_exceeds_ceiling() {
        let mut clipper = OversampledClipper::new();
        clipper.set_parameters(0.8, 0.0, 0.0);

        for _ in 0..100 {
            let input = 2.0;
            let output = clipper.process(input);
            assert!(
                output.abs() <= 0.81,
                "Output should not exceed ceiling significantly"
            );
        }
    }

    #[test]
    fn preserves_low_levels() {
        // A slow sine under the threshold keeps its peak level in every mode
        // (the half-bands delay it by a few samples, so compare peaks, not
        // samples; `output-clipper` checks gain and delay precisely).
        for factor in [1, 4, 8, 16] {
            let mut clipper = OversampledClipper::new();
            clipper.set_parameters(0.9661, 0.0, 0.0);
            clipper.set_oversample_factor(factor);
            let peak = (0..2_000)
                .map(|i| clipper.process((i as f32 * 0.01).sin() * 0.3).abs())
                .skip(1_000)
                .fold(0.0, f32::max);
            assert!(
                (peak - 0.3).abs() < 0.003,
                "{factor}x: low-level peak {peak}, expected 0.3"
            );
        }
    }

    #[test]
    fn oversampling_factor_changes_output_on_transients() {
        // Drive both clippers with the same signal that swings past the ceiling.
        // With real interpolation across inputs, a higher factor band-limits the
        // clip differently, so the outputs must diverge on the transients.
        let inputs = [0.0, 2.0, -2.0, 1.5, -1.8, 0.4, -0.4, 2.5];

        let mut c1 = OversampledClipper::new();
        c1.set_parameters(0.8, 0.5, 0.0);
        c1.set_oversample_factor(1);

        let mut c16 = OversampledClipper::new();
        c16.set_parameters(0.8, 0.5, 0.0);
        c16.set_oversample_factor(16);

        let mut total_diff = 0.0f32;
        for &x in inputs.iter() {
            total_diff += (c1.process(x) - c16.process(x)).abs();
        }

        assert!(
            total_diff > 0.001,
            "1x and 16x oversampling must differ on transients, diff={total_diff}"
        );
    }

    #[test]
    fn stereo_channels_have_independent_state() {
        // The two channels must not share interpolation state.
        let mut clipper = OversampledClipper::new();
        clipper.set_parameters(0.8, 0.5, 0.0);
        clipper.set_oversample_factor(8);

        let (l1, r1) = clipper.process_stereo(2.0, -2.0);
        assert!(l1.is_finite() && r1.is_finite());
        // Left rising from 0, right falling from 0 — opposite-signed inputs
        // should yield opposite-signed clipped outputs.
        assert!(
            l1 > 0.0 && r1 < 0.0,
            "channels must track their own input sign"
        );
    }

    /// `output-clipper`: in Eco with Diode Asymmetry 0, each output sample
    /// is the diode curve on the input sample, unfiltered.
    #[test]
    fn eco_is_plain_clipping() {
        let mut clipper = diode(1, 0.5, 0.0);
        for i in 0..4_800 {
            let x = 2.0 * (i as f32 * 0.37).sin() * (i as f32 * 0.0011).cos();
            assert_eq!(
                clipper.process(x).to_bits(),
                clipper.curve(x).to_bits(),
                "sample {i}"
            );
        }
    }

    #[test]
    fn no_nan_produced() {
        let mut clipper = OversampledClipper::new();

        for i in 0..1000 {
            let input = (i as f32 * 0.1).sin() * 2.0;
            let output = clipper.process(input);
            assert!(output.is_finite(), "Output should be finite");
        }
    }
}

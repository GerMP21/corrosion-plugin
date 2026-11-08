//! Second-order modal resonator core used by the voice layer.
//!
//! The voice module feeds excitation into these modes per sample, while the
//! interaction bus updates coupling and strike-position coefficients without
//! allocating on the audio thread.

use crate::dsp::utils::rate;
use std::f32::consts::PI;

use crate::dsp::resonators::{
    ChainResonator, CoilSpringResonator, IBeamResonator, IndustrialCogResonator, PipeResonator,
    PlateResonator, ResonatorAlgorithm, SheetMetalResonator, TankResonator, TautCableResonator,
};

/// Curated per-object "character" controls exposed to the user.
///
/// Each field maps to one algorithmic-resonator generator parameter (see
/// `docs/backlog.md` → "Algorithmic resonator engine"). Only the field that
/// matches the active object is consulted at note-on; the rest are ignored.
/// Size/decay/brightness-like behavior stays covered by the global controls.
#[derive(Clone, Copy, Debug)]
pub struct CharacterParams {
    pub pipe_diameter: f32,
    pub plate_aspect: f32,
    pub plate_stiffness: f32,
    pub tank_volume: f32,
    pub tank_cavity_mix: f32,
    pub chain_link_mass: f32,
    pub chain_instability: f32,
    pub beam_shear: f32,
    pub cable_braid: f32,
    pub cable_tension_drop: f32,
    pub spring_dispersion: f32,
    pub spring_slosh: f32,
    pub sheet_thinness: f32,
    pub cog_dissonance: f32,
}

impl Default for CharacterParams {
    fn default() -> Self {
        Self {
            pipe_diameter: 0.5,
            plate_aspect: 1.0,
            plate_stiffness: 1.0,
            tank_volume: 0.5,
            tank_cavity_mix: 0.6,
            chain_link_mass: 0.5,
            chain_instability: 0.3,
            beam_shear: 0.5,
            cable_braid: 0.3,
            cable_tension_drop: 0.4,
            spring_dispersion: 0.5,
            spring_slosh: 0.3,
            sheet_thinness: 0.4,
            cog_dissonance: 0.1,
        }
    }
}

/// Per-sample dynamic behavior retained from the object's algorithm so the
/// resonator can modulate mode frequencies while it rings.
#[derive(Clone, Debug)]
enum Dynamics {
    None,
    /// Cable tension drop: amplitude raises pitch, which falls back as it decays.
    TautCable(TautCableResonator),
    /// Sheet-metal buckling: low-frequency displacement warps every mode.
    SheetMetal(SheetMetalResonator),
}

/// Every object profile, in `ModalProfileId` order.
const ALL_PROFILES: [crate::dsp::ModalProfileId; 9] = [
    crate::dsp::ModalProfileId::Pipe,
    crate::dsp::ModalProfileId::Plate,
    crate::dsp::ModalProfileId::Tank,
    crate::dsp::ModalProfileId::Chain,
    crate::dsp::ModalProfileId::IBeam,
    crate::dsp::ModalProfileId::TautCable,
    crate::dsp::ModalProfileId::CoilSpring,
    crate::dsp::ModalProfileId::SheetMetal,
    crate::dsp::ModalProfileId::IndustrialCog,
];

/// Largest base mode count across the object profile tables.
const MAX_BASE_MODE_COUNT: usize = {
    let mut max = 0;
    let mut i = 0;
    while i < ALL_PROFILES.len() {
        let count = crate::dsp::ModalProfile::from_id(ALL_PROFILES[i]).mode_count();
        if count > max {
            max = count;
        }
        i += 1;
    }
    max
};

/// Upper clamp on the quality-mode `mode_count_scale`. An integer, so
/// `MAX_MODES` below is exact: `round(base · scale) ≤ base · this`.
const MAX_MODE_COUNT_SCALE_INT: usize = 4;
const MAX_MODE_COUNT_SCALE: f32 = MAX_MODE_COUNT_SCALE_INT as f32;

/// Most raw (pre-damage) modes one note-on can generate.
const MAX_RAW_MODES: usize = MAX_BASE_MODE_COUNT * MAX_MODE_COUNT_SCALE_INT;

/// Worst-case number of modes in one note-on bank: the largest profile at the
/// `mode_count_scale` cap, doubled for damage companion modes. Voices reserve
/// this much up front so note-on never allocates (review C3).
pub const MAX_MODES: usize = MAX_RAW_MODES * 2;

#[derive(Clone, Copy)]
struct AlgorithmTransformControls {
    rust_amount: crate::dsp::RustAmount,
    damage_amount: crate::dsp::DamageAmount,
    thickness_amount: crate::dsp::ThicknessAmount,
    heat_amount: crate::dsp::HeatAmount,
    sludge_amount: crate::dsp::SludgeAmount,
}

#[derive(Clone, Copy, Debug)]
/// Coefficients for a damped second-order modal resonator.
pub struct ResonatorCoefficients {
    /// Input gain.
    pub b0: f32,
    /// Feedback coefficient for the first delay.
    pub a1: f32,
    /// Feedback coefficient for the second delay.
    pub a2: f32,
}

/// Sample rate at which mode gains are specified (and at which the input
/// gain scaling below is exactly 1).
pub const REFERENCE_SAMPLE_RATE: f32 = 48_000.0;

impl ResonatorCoefficients {
    /// Compute resonator coefficients for a modal specification.
    ///
    /// The input gain is `gain · (48 kHz / fs)²`. Each mode stands in for an
    /// oscillator driven by a force, whose discretized force term scales
    /// with `dt²`; without this, a force lasting a fixed time rings the mode
    /// with a level proportional to fs² (+12 dB at 96 kHz). At 48 kHz the
    /// factor is exactly 1, so output there is unchanged.
    pub fn for_mode(mode: crate::dsp::ModalModeSpec, sample_rate: u32) -> Self {
        let safe_sample_rate = sample_rate.max(1) as f32;
        let decay_seconds = mode.decay_seconds.max(f32::EPSILON);
        let omega = 2.0 * PI * mode.frequency_hz / safe_sample_rate;
        let r = (-1.0 / (decay_seconds * safe_sample_rate)).exp();
        let rate_ratio = REFERENCE_SAMPLE_RATE / safe_sample_rate;

        Self {
            b0: mode.gain * (rate_ratio * rate_ratio),
            a1: -2.0 * r * omega.cos(),
            a2: r * r,
        }
    }
}

/// Top of the audible band, in Hz: modes fade out below it and are silent
/// at and above it (`resolve-exciter-tradeoffs`).
pub const BAND_TOP_HZ: f32 = 20_000.0;

/// Output weight of a mode at `frequency_hz` and `sample_rate`: 1 below the
/// fade, 0 at or above `min(BAND_TOP_HZ, 0.45·fs)`, raised-cosine over the
/// top 10 % (18–20 kHz at 44.1 kHz and above). A mode retuned across it
/// fades rather than switching off, and nothing near or above Nyquist is
/// heard as an alias.
#[inline]
fn band_fade(frequency_hz: f32, sample_rate: f32) -> f32 {
    let end = BAND_TOP_HZ.min(0.45 * sample_rate);
    let start = 0.9 * end;
    if frequency_hz <= start {
        1.0
    } else if frequency_hz >= end {
        0.0
    } else {
        0.5 * (1.0 + (std::f32::consts::PI * (frequency_hz - start) / (end - start)).cos())
    }
}

/// The voice's brightness high-pass, `s ← a·s + (1 − a)·x`, `hp = x − s`,
/// with `a` this pole at 48 kHz (`crate::voice`). The ring bound weights each
/// mode by its response, since the voice adds the high-passed signal to the
/// output before its clamp.
pub(crate) const OUTPUT_HIGHPASS_POLE: f32 = 0.8;

/// `|H(e^{jω})| = 2a·|sin(ω/2)| / |1 − a·e^{−jω}|` of the voice's high-pass,
/// from `cos ω` (`|sin(ω/2)| = √((1 − cos ω)/2)`), so a retune that already
/// has `cos ω` needs no further transcendental
/// (`h12-skip-redundant-mode-retunes`).
fn highpass_response(cos_omega: f64, sample_rate: u32) -> f32 {
    let ratio = crate::dsp::utils::rate::ratio(sample_rate.max(1) as f32);
    let a = crate::dsp::utils::rate::decay(OUTPUT_HIGHPASS_POLE, ratio) as f64;
    let denominator = (1.0 - 2.0 * a * cos_omega + a * a).max(1e-12).sqrt();
    let half_sin = (0.5 * (1.0 - cos_omega)).max(0.0).sqrt();
    (2.0 * a * half_sin / denominator) as f32
}

/// A mode's angular frequency per sample, `ω = 2π·f / fs`, in f64.
#[inline]
fn mode_omega(frequency_hz: f32, sample_rate: u32) -> f64 {
    std::f64::consts::TAU * frequency_hz as f64 / sample_rate.max(1) as f64
}

/// What a retune needs about one mode, kept off the modes so the
/// per-sample loop's data stays small (`h12-skip-redundant-mode-retunes`):
/// the pole radius, and the angle the coefficients were last built for with
/// its sine and cosine, so a small retune can rotate them instead of calling
/// `sin_cos`.
#[derive(Clone, Copy, Debug, Default)]
struct RetuneCache {
    radius: f64,
    /// The input gain `b0 = gain·(48 000/fs)²`.
    b0: f64,
    omega: f64,
    /// `2π / fs`, so a small retune gets `ω` from the frequency without a
    /// division.
    omega_per_hz: f64,
    /// `1/ω`, updated with the angle so a small retune needs no division.
    inv_omega: f64,
    sin: f64,
    cos: f64,
    /// Incremental rotations since the last exact `sin_cos`.
    steps: u32,
    /// The angle the ring-bound terms (`gain_norm`, `highpass_response`)
    /// were last computed for.
    omega_full: f64,
}

/// Incremental rotations allowed before an exact `sin_cos` refresh. Each
/// rotation adds about 1e-16 of rounding, so this keeps the drift far below
/// anything audible while making the refresh's cost negligible.
const RETUNE_EXACT_EVERY: u32 = 256;
/// The largest angle step rotated incrementally; a bigger jump (a fast bend)
/// is recomputed exactly. The fifth-order rotation's error at this step is
/// about 1e-24.
const RETUNE_MAX_ROTATION: f64 = 1e-3;
/// How far (relative) the angle may move before an incremental retune also
/// recomputes the ring-bound terms, `|g|` and the high-pass response: about
/// 1 cent. They only feed the drive governor's bound, which keeps a 10 %
/// margin under full scale, and move by about this fraction, so a bound up
/// to 0.06 % stale is far inside it.
const RETUNE_BOUND_REFRESH: f64 = 6e-4;

/// A mode's input gain `b0 = gain·(48 000 / fs)²`, in f64.
#[inline]
fn input_gain(mode: crate::dsp::ModalModeSpec, sample_rate: u32) -> f64 {
    let rate_ratio = REFERENCE_SAMPLE_RATE as f64 / sample_rate.max(1) as f64;
    mode.gain as f64 * rate_ratio * rate_ratio
}

/// A mode's pole radius `r = exp(−1/(τ·fs))`, in f64. It depends only on
/// the decay and the sample rate, so a retune reuses it.
fn pole_radius(mode: crate::dsp::ModalModeSpec, sample_rate: u32) -> f64 {
    let fs = sample_rate.max(1) as f64;
    let decay_seconds = (mode.decay_seconds.max(f32::EPSILON)) as f64;
    (-1.0 / (decay_seconds * fs)).exp()
}

/// Coefficients of one mode in coupled (complex one-pole) form:
/// `z[n] = p·z[n−1] + g·x[n]`, output `fade·Im z[n]`, with `p = r·e^{iω}`
/// and `g = b0·(sin ω / ω)·(cot ω + i)`.
///
/// Its impulse response is `b0·rⁿ·sin(ω(n+1)) / ω`: the direct-form
/// biquad's (`ResonatorCoefficients`) times `sin ω / ω`. That makes the ring
/// amplitude `b0/ω`, the continuous oscillator's and the same at every rate,
/// where the discrete `b0/sin ω` grew without bound near Nyquist
/// (`resolve-exciter-tradeoffs`). `fade` is `band_fade` at the mode's
/// current frequency. Two reasons to run it in coupled form
/// (`normalize-object-loudness`):
/// - A frequency change rotates the state without changing its magnitude,
///   so per-sample retuning (sheet warp, cable drop, pitch bend) doesn't
///   pump energy into the mode. In direct form the sheet warp diverged.
/// - The frequency is set by `r·sin ω` rather than `−2r·cos ω ≈ −2`, which
///   keeps low, long-ringing modes accurate in f32 at high sample rates.
///   The coefficients are computed in f64 and rounded once.
#[derive(Clone, Copy, Debug)]
struct CoupledCoefficients {
    pole_re: f32,
    pole_im: f32,
    gain_re: f32,
    gain_im: f32,
    /// Output weight at the top of the band (`band_fade`).
    fade: f32,
    /// `|g|`, how far one unit of force moves the mode's state.
    gain_norm: f32,
    /// `|H(ω)|` of the voice's brightness high-pass at this mode's
    /// frequency (`OUTPUT_HIGHPASS_POLE`), for the ring bound.
    highpass_response: f32,
}

impl CoupledCoefficients {
    fn for_mode(mode: crate::dsp::ModalModeSpec, sample_rate: u32) -> Self {
        Self::with_radius(mode, sample_rate, pole_radius(mode, sample_rate))
    }

    /// `for_mode` with the pole radius already known: the frequency-dependent
    /// terms only. Bit-identical to `for_mode` for the same radius.
    fn with_radius(mode: crate::dsp::ModalModeSpec, sample_rate: u32, r: f64) -> Self {
        let omega = mode_omega(mode.frequency_hz, sample_rate);
        let (sin, cos) = omega.sin_cos();
        Self::with_phasor(mode, sample_rate, r, omega, sin, cos)
    }

    /// `with_radius` with `sin ω` and `cos ω` already known too.
    fn with_phasor(
        mode: crate::dsp::ModalModeSpec,
        sample_rate: u32,
        r: f64,
        omega: f64,
        sin: f64,
        cos: f64,
    ) -> Self {
        let fs = sample_rate.max(1) as f64;
        let b0 = input_gain(mode, sample_rate);
        // b0·(sin ω / ω)·(cot ω + i); a 0 Hz mode keeps a finite gain.
        let scale = b0 / omega.max(1e-9);
        Self {
            pole_re: (r * cos) as f32,
            pole_im: (r * sin) as f32,
            gain_re: (scale * cos) as f32,
            gain_im: (scale * sin) as f32,
            fade: band_fade(mode.frequency_hz, fs as f32),
            gain_norm: scale.abs() as f32,
            highpass_response: highpass_response(cos, sample_rate),
        }
    }
}

#[derive(Clone, Debug)]
/// Cached second-order state for one modal peak.
pub struct SecondOrderMode {
    pub(crate) spec: crate::dsp::ModalModeSpec,
    /// Note-on frequency, kept immutable so per-sample dynamic pitch effects
    /// (cable tension drop, sheet-metal warp) scale from a fixed base instead
    /// of compounding their own prior output.
    base_frequency_hz: f32,
    /// Note-on decay before damping/brightness are applied; lets held-note
    /// damping automation re-derive `spec.decay_seconds` instead of compounding.
    base_decay_seconds: f32,
    /// Note-on gain before damping/brightness are applied; mirror of
    /// `base_decay_seconds` for brightness automation.
    base_gain: f32,
    coefficients: CoupledCoefficients,
    cached_sample_rate: Option<u32>,
    /// Complex state `z`; the mode's output is its imaginary part.
    z_re: f32,
    z_im: f32,
}

impl SecondOrderMode {
    /// Create a modal state from a modal specification.
    pub fn new(spec: crate::dsp::ModalModeSpec) -> Self {
        Self {
            spec,
            base_frequency_hz: spec.frequency_hz,
            base_decay_seconds: spec.decay_seconds,
            base_gain: spec.gain,
            coefficients: CoupledCoefficients::for_mode(spec, 48_000),
            cached_sample_rate: None,
            z_re: 0.0,
            z_im: 0.0,
        }
    }

    /// Override the mode frequency and rebuild coefficients in place.
    ///
    /// Used by the per-sample dynamic hooks. Allocation-free.
    pub fn set_frequency(&mut self, frequency_hz: f32, sample_rate: u32) {
        self.spec.frequency_hz = frequency_hz;
        self.coefficients = CoupledCoefficients::for_mode(self.spec, sample_rate);
        self.cached_sample_rate = Some(sample_rate);
    }

    /// Retune the mode to `frequency_hz` exactly, reusing the cached pole
    /// radius (no `exp`). Bit-identical to `set_frequency`
    /// (`h12-skip-redundant-mode-retunes`). Allocation-free.
    fn retune_exact(&mut self, frequency_hz: f32, sample_rate: u32, cache: &mut RetuneCache) {
        self.spec.frequency_hz = frequency_hz;
        let omega = mode_omega(frequency_hz, sample_rate);
        let (sin, cos) = omega.sin_cos();
        *cache = RetuneCache {
            omega,
            inv_omega: 1.0 / omega.max(1e-9),
            sin,
            cos,
            steps: 0,
            omega_full: omega,
            ..*cache
        };
        self.coefficients =
            CoupledCoefficients::with_phasor(self.spec, sample_rate, cache.radius, omega, sin, cos);
        self.cached_sample_rate = Some(sample_rate);
    }

    /// Retune the mode to `frequency_hz` for a small, moving pitch change:
    /// the pole is rotated by the angle step (a fifth-order Taylor rotation,
    /// no transcendental call) rather than rebuilt with `sin_cos`. The angle
    /// itself is recomputed exactly from the frequency, so it doesn't drift;
    /// its sine and cosine are refreshed exactly every `RETUNE_EXACT_EVERY`
    /// rotations or on a jump above `RETUNE_MAX_ROTATION`. Allocation-free.
    fn retune_incremental(&mut self, frequency_hz: f32, sample_rate: u32, cache: &mut RetuneCache) {
        let omega = frequency_hz as f64 * cache.omega_per_hz;
        let step = omega - cache.omega;
        if cache.steps >= RETUNE_EXACT_EVERY || step.abs() > RETUNE_MAX_ROTATION {
            self.retune_exact(frequency_hz, sample_rate, cache);
            return;
        }
        self.spec.frequency_hz = frequency_hz;
        // Multiplications by the reciprocals: a float division by a constant
        // isn't turned into one, and this runs for every mode every sample.
        const SIXTH: f64 = 1.0 / 6.0;
        const TWENTIETH: f64 = 1.0 / 20.0;
        const TWENTY_FOURTH: f64 = 1.0 / 24.0;
        let step2 = step * step;
        let cos_step = 1.0 - step2 * (0.5 - step2 * TWENTY_FOURTH);
        let sin_step = step * (1.0 - step2 * SIXTH * (1.0 - step2 * TWENTIETH));
        let (sin, cos) = (
            cache.sin * cos_step + cache.cos * sin_step,
            cache.cos * cos_step - cache.sin * sin_step,
        );
        // `1/(ω + Δ) = (1/ω)·(1 − δ + δ²)` with `δ = Δ/ω`, to O(δ³) ≈ 1e-14.
        let delta = step * cache.inv_omega;
        let inv_omega = cache.inv_omega * (1.0 - delta * (1.0 - delta));
        *cache = RetuneCache {
            omega,
            inv_omega,
            sin,
            cos,
            steps: cache.steps + 1,
            ..*cache
        };
        if (omega - cache.omega_full).abs() > RETUNE_BOUND_REFRESH * cache.omega_full {
            cache.omega_full = omega;
            self.coefficients = CoupledCoefficients::with_phasor(
                self.spec,
                sample_rate,
                cache.radius,
                omega,
                sin,
                cos,
            );
        } else {
            // The pole and the input gain, as `with_phasor` builds them; the
            // ring-bound terms keep their recent values.
            let r = cache.radius;
            let scale = cache.b0 * cache.inv_omega;
            let c = &mut self.coefficients;
            c.pole_re = (r * cos) as f32;
            c.pole_im = (r * sin) as f32;
            c.gain_re = (scale * cos) as f32;
            c.gain_im = (scale * sin) as f32;
            c.fade = band_fade(frequency_hz, sample_rate.max(1) as f32);
        }
        self.cached_sample_rate = Some(sample_rate);
    }

    /// Re-derive `decay_seconds`/`gain` from the note-on base values using new
    /// damping/brightness scalars, then rebuild coefficients. Allocation-free.
    ///
    /// Mirrors `ModalModeSpec::adjusted_for_controls` so live automation
    /// matches what would have happened if the same controls were captured at
    /// note-on.
    pub fn set_damping_brightness(
        &mut self,
        damping: f32,
        brightness: f32,
        mode_weight: f32,
        sample_rate: u32,
    ) {
        let damping_scale = (1.0 + (0.5 - damping.clamp(0.0, 1.0)) * 1.2).max(0.1);
        let brightness_tilt = 1.0 + (brightness.clamp(0.0, 1.0) - 0.5) * (0.5 + 1.0 * mode_weight);
        self.spec.decay_seconds = (self.base_decay_seconds * damping_scale).max(f32::EPSILON);
        self.spec.gain = (self.base_gain * brightness_tilt.max(0.1)).max(f32::EPSILON);
        self.coefficients = CoupledCoefficients::for_mode(self.spec, sample_rate);
        self.cached_sample_rate = Some(sample_rate);
    }

    /// Rebuild the coefficients if they were derived for another rate.
    ///
    /// Runs for every mode every sample, so only the comparison is inlined.
    /// The rebuild is kept out of line: inlined into the per-sample loops,
    /// its size (which changes with any edit to the coefficient code) moved
    /// plain notes by up to 30 % with bit-identical output
    /// (`h12-skip-redundant-mode-retunes`).
    #[inline]
    fn ensure_rate(&mut self, sample_rate: u32) {
        if self.cached_sample_rate != Some(sample_rate) {
            self.rebuild_for_rate(sample_rate);
        }
    }

    #[cold]
    #[inline(never)]
    fn rebuild_for_rate(&mut self, sample_rate: u32) {
        self.coefficients = CoupledCoefficients::for_mode(self.spec, sample_rate);
        self.cached_sample_rate = Some(sample_rate);
    }

    /// Process one excitation sample through the resonator.
    pub fn process(&mut self, excitation: f32, sample_rate: u32) -> f32 {
        self.ensure_rate(sample_rate);

        let c = self.coefficients;
        let z_re = c.pole_re * self.z_re - c.pole_im * self.z_im + c.gain_re * excitation;
        let z_im = c.pole_re * self.z_im + c.pole_im * self.z_re + c.gain_im * excitation;
        self.z_re = z_re;
        self.z_im = z_im;
        z_im * c.fade
    }
}

/// This sample's contact displacement as a function of this sample's force;
/// see `ModalResonator::contact_prediction`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactPrediction {
    /// Displacement with no force this sample.
    pub free: f32,
    /// Displacement per unit of force this sample.
    pub compliance: f32,
    /// Last sample's contact displacement.
    pub previous: f32,
    /// `fs / 48 kHz`: converts a per-sample displacement change into the
    /// 48 kHz-sample velocity units the exciters use.
    pub velocity_scale: f32,
    /// The most force (either sign) that will actually reach the resonator
    /// this sample: the voice's drive governor caps it (`f32::INFINITY` when
    /// nothing does). The prediction saturates there, so a contact solve
    /// sees the force that is really applied (`resolve-exciter-tradeoffs`).
    pub limit: f32,
}

impl ContactPrediction {
    /// The prediction for a force scaled by `scale` after the exciter (force
    /// envelope, drive trim), as the exciter sees it.
    pub fn scaled(self, scale: f32) -> Self {
        Self {
            compliance: self.compliance * scale,
            limit: self.limit / scale.abs(),
            ..self
        }
    }

    /// The prediction in an exciter's own length units, `scale` resonator
    /// displacement units each: an exciter that moves through larger
    /// distances meets the same surface it met at a lower drive
    /// (`resolve-exciter-tradeoffs`).
    pub fn in_units(self, scale: f32) -> Self {
        Self {
            free: self.free / scale,
            compliance: self.compliance / scale,
            previous: self.previous / scale,
            ..self
        }
    }

    /// The prediction with the force capped at `limit` (in the units the
    /// exciter sees, after `scaled`).
    pub fn limited(self, limit: f32) -> Self {
        Self {
            limit: self.limit.min(limit),
            ..self
        }
    }

    /// Contact displacement if the exciter applies `force` this sample.
    #[inline]
    pub fn displacement(&self, force: f32) -> f32 {
        self.free + self.compliance * force.clamp(-self.limit, self.limit)
    }

    /// Contact velocity (48 kHz-sample units) if the exciter applies `force`.
    #[inline]
    pub fn velocity(&self, force: f32) -> f32 {
        (self.displacement(force) - self.previous) * self.velocity_scale
    }
}

/// A bound on the voice's signal before its clamp, from the modes' ring
/// amplitudes (`resolve-exciter-tradeoffs`, drive governor).
///
/// Each mode rings at `|z_i|` and reaches the output through its band fade
/// and the voice's brightness boost, so no later sample (without more force)
/// can exceed `level = Σ w_i·|z_i|`, `w_i = fade_i·(1 + boost·|H(ω_i)|)`.
/// That holds for each stereo channel too: a mode's pan gain is at most 1.
/// `per_force` is how much one unit of force can raise it, `Σ w_i·|g_i|·|b_i|`
/// (`b_i` the strike-position blend, negative past a node). Measured over the 27-note sweep, the
/// bound is 1.0–1.5× the actual peak for most notes and at most about 3×.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RingBound {
    /// The bound after the last processed sample.
    pub level: f32,
    /// Its rise per unit of force.
    pub per_force: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct RingTracking {
    enabled: bool,
    boost: f32,
    bound: RingBound,
    /// Samples until the bound is next summed over the modes; in between it
    /// grows by `per_force·|force|`, still an upper bound (the modes only
    /// decay), so the square root per mode runs on one sample in
    /// `RING_EXACT_INTERVAL`.
    countdown: u32,
}

/// See `RingTracking::countdown`.
const RING_EXACT_INTERVAL: u32 = 8;

#[derive(Clone, Debug)]
/// Modal resonator assembled from a profile and control transforms.
pub struct ModalResonator {
    pub(crate) profile: crate::dsp::ModalProfileId,
    #[allow(dead_code)]
    pub(crate) size_scale: crate::dsp::SizeScale,
    #[allow(dead_code)]
    pub(crate) rust_amount: crate::dsp::RustAmount,
    #[allow(dead_code)]
    pub(crate) damage_amount: crate::dsp::DamageAmount,
    #[allow(dead_code)]
    pub(crate) thickness_amount: crate::dsp::ThicknessAmount,
    #[allow(dead_code)]
    pub(crate) heat_amount: crate::dsp::HeatAmount,
    #[allow(dead_code)]
    pub(crate) sludge_amount: crate::dsp::SludgeAmount,
    pub(crate) modes: Vec<SecondOrderMode>,
    interaction_bus: crate::dsp::BidirectionalInteractionBus,
    /// Per-sample dynamic behavior (cable/sheet); `None` for static objects.
    dynamics: Dynamics,
    /// Smoothed running amplitude estimate driving the cable tension drop.
    dyn_amplitude: f32,
    /// Smoothed low-frequency displacement driving the sheet-metal warp.
    dyn_lf_displacement: f32,
    /// Cached (sample rate, amplitude pole, LF pole) for non-48 kHz rates.
    dyn_rate: (u32, f32, f32),
    /// Multiplicative pitch-bend factor applied on top of mode base frequencies.
    /// `1.0` is neutral; written by `set_pitch_bend_factor` from MIDI pitch bend
    /// or poly tuning events. Layered with object dynamics in `apply_dynamics`.
    pitch_bend_factor: f32,
    /// Whether `contact_prediction` already ran this sample's preparation
    /// (strike-position update and dynamic retune), so `process_sample`
    /// must not run it again.
    prepared: bool,
    /// Ring-bound tracking for the voice's drive governor (`RingBound`).
    ring: RingTracking,
    /// Note-on scratch for raw generator output. Keeps `MAX_MODES` capacity
    /// so `rebuild_in_place` never allocates.
    raw_specs: Vec<crate::dsp::ModalModeSpec>,
    /// Note-on scratch for the rust/thickness/heat/sludge/damage-transformed
    /// specs, also kept at `MAX_MODES` capacity.
    transformed_specs: Vec<crate::dsp::ModalModeSpec>,
    /// Each mode's brightness weight (0 for the first mode of the unpruned
    /// bank, 1 for the last), kept from before the prune so dropping modes
    /// doesn't re-tilt the ones that stay. `MAX_MODES` capacity.
    mode_weights: Vec<f32>,
    /// Whether note-on drops the modes that can never sound. Always on in
    /// use; tests turn it off to compare with the full bank
    /// (`h11-prune-unsoundable-modes`).
    pub(crate) prune_enabled: bool,
    /// The pitch factor (dynamics × bend) the modes' coefficients were last
    /// built for (`h12-skip-redundant-mode-retunes`).
    applied_factor: f32,
    /// Whether `apply_dynamics` may skip a rebuild the factor doesn't need.
    /// Always on in use; tests turn it off to rebuild every mode every
    /// sample, the old behaviour.
    pub(crate) retune_skip_enabled: bool,
    /// Each mode's `RetuneCache`, in mode order; `MAX_MODES` capacity.
    retune_cache: Vec<RetuneCache>,
    /// The sample rate `retune_cache` was built for, or 0 when it must be
    /// rebuilt (a new bank at note-on).
    retune_cache_rate: u32,
    /// Whether the decays and gains changed since the cache was built (live
    /// damping and brightness), so its radii and input gains must be
    /// recomputed; the angles are still valid.
    retune_radii_stale: bool,
}

/// A mode at or above this frequency can't reach the audible band at any
/// pitch a note can be tuned to: `BAND_TOP_HZ` at the lowest per-note
/// tuning (×0.25, −24 semitones). Object dynamics only raise pitch. Fixed in
/// hertz, so the bank is the same at every sample rate
/// (`h11-prune-unsoundable-modes`).
pub const PRUNE_MAX_FREQUENCY_HZ: f32 = BAND_TOP_HZ / 0.25;
/// A mode whose decay time constant is under this is gone within about five
/// samples at 48 kHz: a feedthrough tick, not a resonance.
pub const PRUNE_MIN_DECAY_SECONDS: f32 = 1e-4;

/// How far the pitch factor may move before `apply_dynamics` rebuilds the
/// modes: just under 0.05 cents (`2^(0.05/1200) − 1 ≈ 2.888e-5`), leaving
/// room for f32 rounding, so every mode stays within 0.05 cents of its
/// target (`h12-skip-redundant-mode-retunes`).
const RETUNE_TOLERANCE: f32 = 2.8e-5;

/// Whether a mode can ever contribute to the sound.
fn mode_can_sound(spec: &crate::dsp::ModalModeSpec) -> bool {
    spec.frequency_hz < PRUNE_MAX_FREQUENCY_HZ && spec.decay_seconds >= PRUNE_MIN_DECAY_SECONDS
}

/// Brightness weight of mode `index` of `count`: 0 for the first, 1 for the
/// last.
fn brightness_weight(index: usize, count: usize) -> f32 {
    if count <= 1 {
        0.0
    } else {
        index as f32 / (count - 1) as f32
    }
}

/// Strike-level estimate of the default patch's bank (Pipe, C4, default
/// controls, after the material transforms). Every note-on bank is scaled to
/// this level; see `ModalResonator::normalize_strike_level`.
pub const STRIKE_LEVEL_REF: f32 = 0.594_266_06;

impl ModalResonator {
    const REFERENCE_PITCH_HZ: f32 = 220.0;

    /// Estimated strike level of the bank: the peak response to a
    /// one-sample unit impulse, with each mode's force weighted as
    /// `process_sample` weights it (`force_weights`, from strike position
    /// and coupling).
    ///
    /// Mode i rings at amplitude `A_i = |w_i · fade_i · b0_i / ω_i|`. The peak of
    /// the sum lies between the incoherent `‖A‖₂` and the fully coherent
    /// `‖A‖₁` (reached when beating modes come into phase). The `‖A‖_p` norm
    /// with p = 1.3 tracks the measured peak within about 4.3 dB over every
    /// object, C2–C6, Eco–Render and damage 0–1 (design.md findings).
    /// O(modes) and allocation-free.
    fn strike_level_estimate(
        modes: &[SecondOrderMode],
        force_weights: impl Fn(usize) -> f32,
    ) -> f32 {
        const P: f32 = 1.3;
        let sum: f32 = modes
            .iter()
            .enumerate()
            .map(|(index, mode)| {
                let omega = std::f32::consts::TAU * mode.spec.frequency_hz / REFERENCE_SAMPLE_RATE;
                let fade = band_fade(mode.spec.frequency_hz, REFERENCE_SAMPLE_RATE);
                (force_weights(index) * fade * mode.spec.gain / omega.max(1e-6))
                    .abs()
                    .powf(P)
            })
            .sum();
        sum.powf(1.0 / P)
    }

    /// Scale the bank so its strike level is `STRIKE_LEVEL_REF`, whatever
    /// the object, pitch, quality mode, damage or character settings.
    /// Relative mode gains, and so the timbre, are kept
    /// (`normalize-object-loudness`). Runs at note-on, before the live
    /// damping/brightness controls and at the bus's note-on strike position
    /// (the default; a later `set_interaction_params` moves it, which
    /// changes level and timbre as a strike position should).
    /// Allocation-free.
    fn normalize_strike_level(&mut self) {
        let coupling = self
            .interaction_bus
            .state
            .coupling_stiffness
            .clamp(0.0, 1.0);
        let weights = &self.interaction_bus.mode_coefficients;
        let weight = |index: usize| {
            let coeff = weights.get(index).copied().unwrap_or(1.0);
            (1.0 - coupling) + coeff * coupling
        };
        let level = Self::strike_level_estimate(&self.modes, weight);
        if !(level.is_finite() && level > 0.0) {
            return;
        }
        let scale = STRIKE_LEVEL_REF / level;
        for mode in &mut self.modes {
            mode.base_gain *= scale;
            mode.spec.gain *= scale;
        }
    }

    /// An empty resonator whose mode, scratch, and interaction buffers are
    /// preallocated for the worst-case bank (`MAX_MODES`).
    fn with_note_capacity(profile_id: crate::dsp::ModalProfileId) -> Self {
        Self {
            profile: profile_id,
            size_scale: crate::dsp::SizeScale::default(),
            rust_amount: crate::dsp::RustAmount::default(),
            damage_amount: crate::dsp::DamageAmount::default(),
            thickness_amount: crate::dsp::ThicknessAmount::default(),
            heat_amount: crate::dsp::HeatAmount::default(),
            sludge_amount: crate::dsp::SludgeAmount::default(),
            modes: Vec::with_capacity(MAX_MODES),
            interaction_bus: crate::dsp::BidirectionalInteractionBus::with_capacity(MAX_MODES),
            dynamics: Dynamics::None,
            dyn_amplitude: 0.0,
            dyn_lf_displacement: 0.0,
            dyn_rate: (48_000, 0.99, 0.95),
            pitch_bend_factor: 1.0,
            prepared: false,
            ring: RingTracking::default(),
            raw_specs: Vec::with_capacity(MAX_MODES),
            transformed_specs: Vec::with_capacity(MAX_MODES),
            mode_weights: Vec::with_capacity(MAX_MODES),
            prune_enabled: true,
            applied_factor: 1.0,
            retune_skip_enabled: true,
            retune_cache: Vec::with_capacity(MAX_MODES),
            retune_cache_rate: 0,
            retune_radii_stale: false,
        }
    }

    /// Apply the material transforms to `modes`, replacing the contents of
    /// `out`. Stops at `MAX_MODES` so a preallocated `out` never grows.
    fn transform_algorithm_modes_into(
        modes: &[crate::dsp::ModalModeSpec],
        controls: AlgorithmTransformControls,
        out: &mut Vec<crate::dsp::ModalModeSpec>,
    ) {
        out.clear();
        let mode_count = modes.len().max(1);

        for (index, &mode) in modes.iter().enumerate() {
            let rusted = mode.corroded(controls.rust_amount, index, mode_count);
            let thickened = rusted.thickened(controls.thickness_amount, index);
            let heated = thickened.heated(controls.heat_amount);
            let sludge_loaded = heated.sludge_loaded(controls.sludge_amount);
            let damaged = sludge_loaded.damaged(controls.damage_amount, index, mode_count);

            for damaged_mode in damaged {
                if out.len() == MAX_MODES {
                    return;
                }
                out.push(damaged_mode);
            }
        }
    }

    /// Generate the algorithmic modal bank for an object, configured by the
    /// curated character controls. The profile tables now contribute only
    /// `mode_count` (budget/test metadata); they no longer drive the sound.
    ///
    /// `mode_count_scale` multiplies the base mode count; QualityMode supplies
    /// 0.5/1.0/1.5/2.0 so Eco runs leaner banks and Render builds richer ones.
    ///
    /// Replaces the contents of `out` with the raw modes and returns any
    /// per-sample dynamic behavior the object retains (cable tension drop,
    /// sheet-metal warp).
    fn generate_algorithm_modes_into(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        target_frequency_hz: f32,
        character: CharacterParams,
        mode_count_scale: f32,
        out: &mut Vec<crate::dsp::ModalModeSpec>,
    ) -> Dynamics {
        out.clear();
        let base = crate::dsp::ModalProfile::from_id(profile_id)
            .mode_count()
            .max(1);
        let scale = if mode_count_scale.is_finite() && mode_count_scale > 0.0 {
            mode_count_scale.clamp(0.25, MAX_MODE_COUNT_SCALE)
        } else {
            1.0
        };
        let mode_count = ((base as f32 * scale).round() as usize).max(1);

        match profile_id {
            crate::dsp::ModalProfileId::Pipe => {
                let resonator = PipeResonator {
                    tube_diameter: character.pipe_diameter,
                    sustain_time: 1.0,
                };
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::Plate => {
                let resonator = PlateResonator {
                    aspect_ratio: character.plate_aspect,
                    metal_stiffness: character.plate_stiffness,
                };
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::Tank => {
                let resonator = TankResonator {
                    tank_volume: character.tank_volume,
                    wall_thickness: 0.5,
                    cavity_mix: character.tank_cavity_mix,
                };
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::Chain => {
                let resonator = ChainResonator {
                    link_mass: character.chain_link_mass,
                    chain_length: mode_count,
                    instability: character.chain_instability,
                    friction_decay: 2.0,
                };
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::IBeam => {
                let resonator = IBeamResonator::with_character(character.beam_shear);
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::TautCable => {
                let resonator = TautCableResonator::with_character(
                    character.cable_braid,
                    character.cable_tension_drop,
                );
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::TautCable(resonator)
            }
            crate::dsp::ModalProfileId::CoilSpring => {
                let resonator = CoilSpringResonator::with_character(
                    character.spring_dispersion,
                    character.spring_slosh,
                );
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
            crate::dsp::ModalProfileId::SheetMetal => {
                let resonator = SheetMetalResonator::with_character(character.sheet_thinness);
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::SheetMetal(resonator)
            }
            crate::dsp::ModalProfileId::IndustrialCog => {
                let resonator = IndustrialCogResonator::with_character(character.cog_dissonance);
                resonator.generate_modes_into(target_frequency_hz, mode_count, size_scale, out);
                Dynamics::None
            }
        }
    }

    // Internal constructor intentionally mirrors all modal control transforms.
    #[allow(clippy::too_many_arguments)]
    fn from_profile(
        profile: crate::dsp::ModalProfile,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
        target_frequency_hz: Option<f32>,
        damping: f32,
        brightness: f32,
    ) -> Self {
        let pitch_scale = target_frequency_hz
            .map(|target| (target / Self::REFERENCE_PITCH_HZ.max(f32::EPSILON)).max(f32::EPSILON));
        let mode_count = profile.mode_count().max(1);
        let mut modes: Vec<SecondOrderMode> = profile
            .scaled_mode_specs_extended(
                size_scale,
                rust_amount,
                damage_amount,
                thickness_amount,
                heat_amount,
                sludge_amount,
            )
            .into_iter()
            .enumerate()
            .map(|(index, mode)| {
                let mode_weight = if mode_count <= 1 {
                    0.0
                } else {
                    index as f32 / (mode_count - 1) as f32
                };
                let mode = if let Some(scale) = pitch_scale {
                    mode.retuned(scale)
                } else {
                    mode
                };
                mode.adjusted_for_controls(damping, brightness, mode_weight)
            })
            .map(SecondOrderMode::new)
            .collect();
        // Reserve the worst-case bank so a later `rebuild_in_place` (note-on
        // on the audio thread) never has to grow these buffers.
        modes.reserve(MAX_MODES.saturating_sub(modes.len()));

        let mut interaction_bus = crate::dsp::BidirectionalInteractionBus::with_capacity(MAX_MODES);
        interaction_bus.initialize(modes.len());

        Self {
            profile: profile.id,
            size_scale,
            rust_amount,
            damage_amount,
            thickness_amount,
            heat_amount,
            sludge_amount,
            modes,
            interaction_bus,
            dynamics: Dynamics::None,
            dyn_amplitude: 0.0,
            dyn_lf_displacement: 0.0,
            dyn_rate: (48_000, 0.99, 0.95),
            pitch_bend_factor: 1.0,
            prepared: false,
            ring: RingTracking::default(),
            raw_specs: Vec::with_capacity(MAX_MODES),
            transformed_specs: Vec::with_capacity(MAX_MODES),
            mode_weights: Vec::with_capacity(MAX_MODES),
            prune_enabled: true,
            applied_factor: 1.0,
            retune_skip_enabled: true,
            retune_cache: Vec::with_capacity(MAX_MODES),
            retune_cache_rate: 0,
            retune_radii_stale: false,
        }
    }

    /// Create a resonator from the default profile.
    pub fn with_profile(profile_id: crate::dsp::ModalProfileId) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            crate::dsp::SizeScale::default(),
            crate::dsp::RustAmount::default(),
            crate::dsp::DamageAmount::default(),
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }

    /// Create a resonator with a profile and size control.
    pub fn with_profile_and_size(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
    ) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            size_scale,
            crate::dsp::RustAmount::default(),
            crate::dsp::DamageAmount::default(),
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }

    /// Create a resonator with profile, size, and rust controls.
    pub fn with_profile_size_and_rust(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
    ) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            size_scale,
            rust_amount,
            crate::dsp::DamageAmount::default(),
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }

    /// Create a resonator with size, rust, and damage controls.
    pub fn with_profile_size_rust_and_damage(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
    ) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            size_scale,
            rust_amount,
            damage_amount,
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }

    /// Create a resonator with the full control surface and pitch target.
    // Public full-control constructor preserves the existing voice call shape.
    #[allow(clippy::too_many_arguments)]
    pub fn with_profile_controls_and_note(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
        target_frequency_hz: f32,
        damping: f32,
        brightness: f32,
    ) -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::from_id(profile_id),
            size_scale,
            rust_amount,
            damage_amount,
            thickness_amount,
            heat_amount,
            sludge_amount,
            Some(target_frequency_hz),
            damping,
            brightness,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_algorithm_controls_and_note(
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
        target_frequency_hz: f32,
        damping: f32,
        brightness: f32,
        character: CharacterParams,
        mode_count_scale: f32,
    ) -> Self {
        let mut resonator = Self::with_note_capacity(profile_id);
        resonator.rebuild_in_place(
            profile_id,
            size_scale,
            rust_amount,
            damage_amount,
            thickness_amount,
            heat_amount,
            sludge_amount,
            target_frequency_hz,
            damping,
            brightness,
            character,
            mode_count_scale,
            48_000,
        );
        resonator
    }

    /// Rebuild this resonator's modal bank for a new note, reusing its
    /// preallocated buffers. Produces the same bank as
    /// `with_algorithm_controls_and_note` without allocating or freeing, so
    /// the voice can call it from note-on on the audio thread (review C3).
    #[allow(clippy::too_many_arguments)]
    pub fn rebuild_in_place(
        &mut self,
        profile_id: crate::dsp::ModalProfileId,
        size_scale: crate::dsp::SizeScale,
        rust_amount: crate::dsp::RustAmount,
        damage_amount: crate::dsp::DamageAmount,
        thickness_amount: crate::dsp::ThicknessAmount,
        heat_amount: crate::dsp::HeatAmount,
        sludge_amount: crate::dsp::SludgeAmount,
        target_frequency_hz: f32,
        damping: f32,
        brightness: f32,
        character: CharacterParams,
        mode_count_scale: f32,
        sample_rate: u32,
    ) {
        let dynamics = Self::generate_algorithm_modes_into(
            profile_id,
            size_scale,
            target_frequency_hz,
            character,
            mode_count_scale,
            &mut self.raw_specs,
        );
        // Generators must stay within the preallocated raw buffer; growing
        // it here would allocate on the audio thread.
        debug_assert!(
            self.raw_specs.len() <= MAX_RAW_MODES,
            "{profile_id:?} generated {} raw modes > {MAX_RAW_MODES}",
            self.raw_specs.len()
        );
        Self::transform_algorithm_modes_into(
            &self.raw_specs,
            AlgorithmTransformControls {
                rust_amount,
                damage_amount,
                thickness_amount,
                heat_amount,
                sludge_amount,
            },
            &mut self.transformed_specs,
        );
        // Drop the modes that can never sound, in place, keeping each kept
        // mode's brightness weight from the full bank
        // (`h11-prune-unsoundable-modes`). Allocation-free: both buffers
        // keep `MAX_MODES` capacity.
        let full_count = self.transformed_specs.len();
        self.mode_weights.clear();
        let mut kept = 0;
        for index in 0..full_count {
            let spec = self.transformed_specs[index];
            if self.prune_enabled && !mode_can_sound(&spec) {
                continue;
            }
            self.transformed_specs[kept] = spec;
            self.mode_weights.push(brightness_weight(index, full_count));
            kept += 1;
        }
        self.transformed_specs.truncate(kept);
        self.modes.clear();
        self.modes.extend(
            self.transformed_specs
                .iter()
                .copied()
                .map(SecondOrderMode::new),
        );

        self.profile = profile_id;
        self.size_scale = crate::dsp::SizeScale::default();
        self.rust_amount = crate::dsp::RustAmount::default();
        self.damage_amount = crate::dsp::DamageAmount::default();
        self.thickness_amount = crate::dsp::ThicknessAmount::default();
        self.heat_amount = crate::dsp::HeatAmount::default();
        self.sludge_amount = crate::dsp::SludgeAmount::default();
        self.interaction_bus.reset(self.modes.len());
        // The modes start at rest.
        self.ring = RingTracking::default();
        self.normalize_strike_level();
        self.dynamics = dynamics;
        self.dyn_amplitude = 0.0;
        self.dyn_lf_displacement = 0.0;
        self.pitch_bend_factor = 1.0;
        // The fill built every mode at its base frequency, with new decays.
        self.applied_factor = 1.0;
        self.retune_cache_rate = 0;
        self.prepared = false;

        // Damping/brightness are kept off `base_decay_seconds`/`base_gain` so
        // held-note automation can re-derive them; apply them here at note-on,
        // building the coefficients once at the voice's rate (it used to be a
        // fixed 48 kHz, rebuilt again on the first sample at any other rate:
        // `m03-m20-start-at-host-rate-and-report-latency`).
        self.set_live_resonator_controls(damping, brightness, sample_rate);
    }
}

impl Default for ModalResonator {
    /// Construct the default pipe resonator.
    fn default() -> Self {
        Self::from_profile(
            crate::dsp::ModalProfile::pipe(),
            crate::dsp::SizeScale::default(),
            crate::dsp::RustAmount::default(),
            crate::dsp::DamageAmount::default(),
            crate::dsp::ThicknessAmount::default(),
            crate::dsp::HeatAmount::default(),
            crate::dsp::SludgeAmount::default(),
            None,
            0.5,
            0.5,
        )
    }
}

/// Audio-rate resonator API used by the voice module.
pub trait ResonatorCore {
    /// Process one mono excitation sample.
    fn process_sample(&mut self, excitation: f32, sample_rate: u32) -> f32;
    /// Process one sample and return a stereo pair.
    fn process_sample_stereo(
        &mut self,
        excitation: f32,
        sample_rate: u32,
        width: f32,
    ) -> (f32, f32);
}

impl ModalResonator {
    /// Silence the resonator in place: zero every mode's filter history and
    /// the dynamics trackers. The bank itself is kept; the next note-on
    /// rebuilds it. Allocation-free.
    pub fn clear_state(&mut self) {
        for mode in &mut self.modes {
            mode.z_re = 0.0;
            mode.z_im = 0.0;
        }
        self.dyn_amplitude = 0.0;
        self.dyn_lf_displacement = 0.0;
        self.interaction_bus.reset(self.modes.len());
        // The modes start at rest.
        self.ring = RingTracking::default();
    }

    /// Apply live damping/brightness updates to every mode.
    ///
    /// Modes keep their note-on `base_decay_seconds` / `base_gain`; this method
    /// re-derives `spec.decay_seconds` / `spec.gain` and rebuilds biquad
    /// coefficients in place. Allocation-free; intended to be called once per
    /// buffer from the host layer for held-note automation.
    pub fn set_live_resonator_controls(&mut self, damping: f32, brightness: f32, sample_rate: u32) {
        // Damping changes the decays, so the retune radii must be rebuilt.
        self.retune_radii_stale = true;
        let mode_count = self.modes.len();
        if mode_count == 0 {
            return;
        }
        for (index, mode) in self.modes.iter_mut().enumerate() {
            // Note-on banks carry their weights from before the prune; a
            // bank built another way weights by position.
            let mode_weight = if self.mode_weights.len() == mode_count {
                self.mode_weights[index]
            } else {
                brightness_weight(index, mode_count)
            };
            mode.set_damping_brightness(damping, brightness, mode_weight, sample_rate);
        }
    }

    /// Update interaction parameters from the voice layer. `wander_rate` is
    /// the position wander's rate in hertz (Wander Rate, 0.1–5 Hz).
    pub fn set_interaction_params(
        &mut self,
        strike_position: f32,
        coupling_stiffness: f32,
        position_wander: f32,
        wander_rate: f32,
        fundamental_anchor: f32,
        sample_rate: f32,
    ) {
        self.interaction_bus
            .state
            .set_strike_position(strike_position);
        self.interaction_bus.state.coupling_stiffness = coupling_stiffness.clamp(0.0, 1.0);
        self.interaction_bus.state.fundamental_lock = fundamental_anchor > 0.0;
        self.interaction_bus.state.fundamental_minimum = fundamental_anchor.clamp(0.0, 1.0);
        self.interaction_bus.state.set_position_wander(
            position_wander,
            wander_rate.clamp(0.1, 5.0),
            sample_rate.max(1.0),
        );
        self.interaction_bus.update();
    }

    /// Move the strike position with the note's force envelope this sample:
    /// `amount · envelope · 0.5` is added to the base position before the
    /// wander and the clamp (`h09-route-envelope-to-strike-position`). Call
    /// it once per sample, before the sample is prepared. At amount 0 the
    /// position, and so the output, is unchanged. Allocation-free.
    #[inline]
    pub fn set_envelope_offset(&mut self, amount: f32, envelope: f32) {
        self.interaction_bus
            .state
            .set_envelope_offset(amount, envelope);
    }

    /// The effective strike position after the last update (base position,
    /// envelope offset and wander, clamped to 0..1).
    pub fn effective_strike_position(&self) -> f32 {
        self.interaction_bus.state.effective_position
    }

    /// The force envelope's offset on the strike position this sample.
    pub fn strike_envelope_offset(&self) -> f32 {
        self.interaction_bus.state.envelope_offset
    }

    /// This sample's preparation, before any force is applied: advance the
    /// strike-position weights and apply the object's dynamic retune. Runs
    /// once per sample, from `contact_prediction` or `process_sample`.
    fn prepare_sample(&mut self, sample_rate: u32) {
        self.interaction_bus.update();
        if self.has_dynamics() {
            self.apply_dynamics(sample_rate);
        }
    }

    /// How this sample's contact displacement depends on this sample's
    /// exciter force, so an exciter can solve its contact without the
    /// one-sample lag (`stabilize-mallet-and-grinding-contact`).
    ///
    /// The displacement is affine in the force `F` fed to `process_sample`:
    /// `D = free + compliance·F`. `free` is where the resonator goes on its
    /// own (`Σ w_i·Im(p_i·z_i)`) and `compliance` is `Σ w_i·b_i·b0_i`, with
    /// `w_i` the strike-position weight and `b_i` the coupling blend that
    /// `process_sample` applies. Runs this sample's preparation, so call it
    /// at most once per sample, right before `process_sample`.
    /// Allocation-free.
    pub fn contact_prediction(&mut self, sample_rate: u32) -> ContactPrediction {
        if !self.prepared {
            self.prepare_sample(sample_rate);
            self.prepared = true;
        }
        let coupling = self
            .interaction_bus
            .state
            .coupling_stiffness
            .clamp(0.0, 1.0);
        let (mut free, mut compliance) = (0.0f32, 0.0f32);
        for (index, mode) in self.modes.iter_mut().enumerate() {
            mode.ensure_rate(sample_rate);
            let weight = self
                .interaction_bus
                .mode_coefficients
                .get(index)
                .copied()
                .unwrap_or(1.0);
            let blend = (1.0 - coupling) + weight * coupling;
            let c = mode.coefficients;
            free += weight * c.fade * (c.pole_re * mode.z_im + c.pole_im * mode.z_re);
            compliance += weight * blend * c.fade * c.gain_im;
        }
        ContactPrediction {
            free,
            compliance,
            previous: self.interaction_bus.state.resonator_displacement,
            velocity_scale: sample_rate as f32 / REFERENCE_SAMPLE_RATE,
            limit: f32::INFINITY,
        }
    }

    /// Track the ring bound in `process_sample` (only while the voice's
    /// governor needs it, since it costs a square root per mode), with the
    /// voice's brightness boost amount. A stale bound only overestimates:
    /// without force the modes only decay. Allocation-free.
    pub fn set_ring_tracking(&mut self, enabled: bool, boost: f32) {
        self.ring.enabled = enabled;
        self.ring.boost = boost;
    }

    /// The ring bound after the last tracked sample. Before the first one
    /// (a fresh note, or tracking just switched on) it is computed from the
    /// current state, so the first sample's force is bounded too.
    /// Allocation-free.
    pub fn ring_bound(&mut self, sample_rate: u32) -> RingBound {
        if self.ring.bound.per_force == 0.0 && !self.modes.is_empty() {
            if !self.prepared {
                self.prepare_sample(sample_rate);
                self.prepared = true;
            }
            let coupling = self
                .interaction_bus
                .state
                .coupling_stiffness
                .clamp(0.0, 1.0);
            let boost = self.ring.boost;
            let (mut level, mut per_force) = (0.0f32, 0.0f32);
            for (index, mode) in self.modes.iter_mut().enumerate() {
                mode.ensure_rate(sample_rate);
                let coeff = self
                    .interaction_bus
                    .mode_coefficients
                    .get(index)
                    .copied()
                    .unwrap_or(1.0);
                let c = &mode.coefficients;
                let w = c.fade * (1.0 + boost * c.highpass_response);
                level += w * (mode.z_re * mode.z_re + mode.z_im * mode.z_im).sqrt();
                per_force += w * c.gain_norm * ((1.0 - coupling) + coeff * coupling).abs();
            }
            self.ring.bound = RingBound { level, per_force };
        }
        self.ring.bound
    }

    /// The pitch factor this sample's `apply_dynamics` aims for: the
    /// object's dynamic factor times the pitch bend.
    pub(crate) fn target_pitch_factor(&self) -> f32 {
        let dyn_factor = match &self.dynamics {
            Dynamics::None => 1.0,
            Dynamics::TautCable(inst) => inst.dynamic_pitch_factor(self.dyn_amplitude),
            Dynamics::SheetMetal(inst) => inst.warp_factor(self.dyn_lf_displacement),
        };
        dyn_factor * self.pitch_bend_factor
    }

    /// Apply the object's per-sample dynamic pitch behavior, recomputing the
    /// affected mode coefficients from their immutable base frequency. Layers
    /// the current `pitch_bend_factor` on top so MIDI pitch bend tracks the
    /// dynamic shift. No-op for static objects without active pitch bend.
    /// Allocation-free.
    // Kept out of line: it only runs for bent or dynamic notes, and inlined
    // into the per-sample path it slowed plain notes by about 25 %
    // (`h12-skip-redundant-mode-retunes`).
    #[inline(never)]
    fn apply_dynamics(&mut self, sample_rate: u32) {
        let factor = self.target_pitch_factor();
        if !factor.is_finite() {
            return;
        }
        if self.retune_skip_enabled {
            // A held bend gives the same factor every sample, and a slowly
            // moving one barely moves: skip until it has moved by more than
            // `RETUNE_TOLERANCE` (`h12-skip-redundant-mode-retunes`).
            if (factor / self.applied_factor - 1.0).abs() <= RETUNE_TOLERANCE {
                return;
            }
            self.refresh_retune_cache(sample_rate);
            for (mode, cache) in self.modes.iter_mut().zip(&mut self.retune_cache) {
                let base = mode.base_frequency_hz;
                mode.retune_incremental(base * factor, sample_rate, cache);
            }
        } else {
            // The reference: rebuild every mode from scratch every sample.
            for mode in &mut self.modes {
                let base = mode.base_frequency_hz;
                mode.set_frequency(base * factor, sample_rate);
            }
        }
        self.applied_factor = factor;
    }

    #[inline]
    fn has_dynamics(&self) -> bool {
        // The dynamics-aware sample path is hot enough that we only re-enter it
        // when there is a real-time pitch effect to apply: a cable/sheet hook,
        // or a non-unity pitch bend factor.
        !matches!(self.dynamics, Dynamics::None) || (self.pitch_bend_factor - 1.0).abs() > 1e-6
    }

    /// Apply a multiplicative pitch-bend factor to the modal bank and rebuild
    /// coefficients once. Routed from `Voice::set_pitch_bend` so MIDI pitch bend
    /// and poly tuning can re-pitch a held note without re-allocating modes.
    pub fn set_pitch_bend_factor(&mut self, factor: f32, sample_rate: u32) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let factor = factor.clamp(0.25, 4.0);
        if (factor - self.pitch_bend_factor).abs() < 1e-6 {
            return;
        }
        self.pitch_bend_factor = factor;
        // For static objects there is no per-sample apply_dynamics callback, so
        // rebuild biquads here. For dynamic objects the next sample will pick
        // it up via apply_dynamics — but doing the rebuild once now keeps the
        // response immediate.
        self.refresh_retune_cache(sample_rate);
        for (mode, cache) in self.modes.iter_mut().zip(&mut self.retune_cache) {
            let base = mode.base_frequency_hz;
            mode.retune_exact(base * factor, sample_rate, cache);
        }
        self.applied_factor = factor;
    }

    /// Make `retune_cache` describe every mode at `sample_rate`: rebuilt in
    /// full for a new bank or rate (radius, angle, sine and cosine of each
    /// mode's current frequency), or just its radii after live damping.
    /// Allocation-free.
    fn refresh_retune_cache(&mut self, sample_rate: u32) {
        if self.retune_cache_rate != sample_rate || self.retune_cache.len() != self.modes.len() {
            self.retune_cache.clear();
            self.retune_cache.extend(self.modes.iter().map(|mode| {
                let omega = mode_omega(mode.spec.frequency_hz, sample_rate);
                let (sin, cos) = omega.sin_cos();
                RetuneCache {
                    radius: pole_radius(mode.spec, sample_rate),
                    b0: input_gain(mode.spec, sample_rate),
                    omega_per_hz: std::f64::consts::TAU / sample_rate.max(1) as f64,
                    omega,
                    inv_omega: 1.0 / omega.max(1e-9),
                    sin,
                    cos,
                    steps: 0,
                    omega_full: omega,
                }
            }));
            self.retune_cache_rate = sample_rate;
            self.retune_radii_stale = false;
        } else if self.retune_radii_stale {
            for (mode, cache) in self.modes.iter().zip(&mut self.retune_cache) {
                cache.radius = pole_radius(mode.spec, sample_rate);
                cache.b0 = input_gain(mode.spec, sample_rate);
            }
            self.retune_radii_stale = false;
        }
    }

    /// Current multiplicative pitch-bend factor (1.0 = no bend).
    #[inline]
    pub fn pitch_bend_factor(&self) -> f32 {
        self.pitch_bend_factor
    }

    /// Update the smoothed amplitude / low-frequency estimates that drive the
    /// dynamic hooks. `lf_sum` is the summed displacement of the lowest modes.
    #[inline]
    fn track_dynamic_state(&mut self, amp_sum: f32, lf_sum: f32, sample_rate: u32) {
        // One-pole trackers tuned per 48 kHz sample (poles 0.99 and 0.95),
        // converted for other rates; the 48 kHz path keeps the exact
        // literals so its output is unchanged.
        if sample_rate == 48_000 {
            self.dyn_amplitude = self.dyn_amplitude * 0.99 + amp_sum * 0.01;
            self.dyn_lf_displacement = self.dyn_lf_displacement * 0.95 + lf_sum * 0.05;
        } else {
            if self.dyn_rate.0 != sample_rate {
                let r = rate::ratio(sample_rate as f32);
                self.dyn_rate = (sample_rate, rate::decay(0.99, r), rate::decay(0.95, r));
            }
            let (_, amp_pole, lf_pole) = self.dyn_rate;
            self.dyn_amplitude = self.dyn_amplitude * amp_pole + amp_sum * (1.0 - amp_pole);
            self.dyn_lf_displacement =
                self.dyn_lf_displacement * lf_pole + lf_sum * (1.0 - lf_pole);
        }
    }

    /// Return the current summed displacement.
    pub fn get_displacement(&self) -> f32 {
        self.interaction_bus.state.resonator_displacement
    }

    /// Return the current summed velocity estimate.
    pub fn get_velocity(&self) -> f32 {
        self.interaction_bus.state.resonator_velocity
    }
}

impl ResonatorCore for ModalResonator {
    fn process_sample(&mut self, excitation: f32, sample_rate: u32) -> f32 {
        let dynamic = self.has_dynamics();
        if !std::mem::take(&mut self.prepared) {
            self.prepare_sample(sample_rate);
        }
        let coupling = self
            .interaction_bus
            .state
            .coupling_stiffness
            .clamp(0.0, 1.0);
        self.interaction_bus.state.set_exciter_force(excitation);
        let track_any = self.ring.enabled;
        let track = track_any && self.ring.countdown == 0;
        let boost = self.ring.boost;
        let (mut level, mut per_force) = (0.0f32, 0.0f32);

        let mut mode_sum = 0.0f32;
        let mut interaction_displacement = 0.0f32;
        let mut amp_sum = 0.0f32;
        let mut lf_sum = 0.0f32;

        for (index, mode) in self.modes.iter_mut().enumerate() {
            let coeff = self
                .interaction_bus
                .mode_coefficients
                .get(index)
                .copied()
                .unwrap_or(1.0);
            let weighted_force = excitation * coeff;
            let blended_force = excitation * (1.0 - coupling) + weighted_force * coupling;
            let mode_output = mode.process(blended_force, sample_rate);
            if track {
                let c = &mode.coefficients;
                let w = c.fade * (1.0 + boost * c.highpass_response);
                level += w * (mode.z_re * mode.z_re + mode.z_im * mode.z_im).sqrt();
                per_force += w * c.gain_norm * ((1.0 - coupling) + coeff * coupling).abs();
            }
            mode_sum += mode_output;
            interaction_displacement += mode_output * coeff;
            if dynamic {
                amp_sum += mode_output.abs();
                if index < 3 {
                    lf_sum += mode_output;
                }
            }
        }

        if track {
            self.ring.bound = RingBound { level, per_force };
            self.ring.countdown = RING_EXACT_INTERVAL - 1;
        } else if track_any {
            self.ring.bound.level += self.ring.bound.per_force * excitation.abs();
            self.ring.countdown -= 1;
        }
        if dynamic {
            self.track_dynamic_state(amp_sum, lf_sum, sample_rate);
        }

        // Velocity in displacement per 48 kHz sample, the unit the exciters'
        // own velocities use (exactly the plain difference at 48 kHz).
        self.interaction_bus.state.resonator_velocity = (interaction_displacement
            - self.interaction_bus.state.resonator_displacement)
            * (sample_rate as f32 / REFERENCE_SAMPLE_RATE);
        self.interaction_bus.state.resonator_displacement = interaction_displacement;

        match self.profile {
            crate::dsp::ModalProfileId::Pipe => mode_sum,
            crate::dsp::ModalProfileId::Plate => mode_sum,
            crate::dsp::ModalProfileId::Tank => mode_sum,
            crate::dsp::ModalProfileId::Chain => mode_sum,
            crate::dsp::ModalProfileId::IBeam => mode_sum,
            crate::dsp::ModalProfileId::TautCable => mode_sum,
            crate::dsp::ModalProfileId::CoilSpring => mode_sum,
            crate::dsp::ModalProfileId::SheetMetal => mode_sum,
            crate::dsp::ModalProfileId::IndustrialCog => mode_sum,
        }
    }

    fn process_sample_stereo(
        &mut self,
        excitation: f32,
        sample_rate: u32,
        width: f32,
    ) -> (f32, f32) {
        let mode_count = self.modes.len();
        if mode_count == 0 {
            self.prepared = false;
            return (0.0, 0.0);
        }

        let dynamic = self.has_dynamics();
        if !std::mem::take(&mut self.prepared) {
            self.prepare_sample(sample_rate);
        }
        let coupling = self
            .interaction_bus
            .state
            .coupling_stiffness
            .clamp(0.0, 1.0);
        self.interaction_bus.state.set_exciter_force(excitation);
        let track_any = self.ring.enabled;
        let track = track_any && self.ring.countdown == 0;
        let boost = self.ring.boost;
        let (mut level, mut per_force) = (0.0f32, 0.0f32);

        let mut left_sum = 0.0f32;
        let mut right_sum = 0.0f32;
        let mut interaction_displacement = 0.0f32;
        let mut amp_sum = 0.0f32;
        let mut lf_sum = 0.0f32;

        for (index, mode) in self.modes.iter_mut().enumerate() {
            let coeff = self
                .interaction_bus
                .mode_coefficients
                .get(index)
                .copied()
                .unwrap_or(1.0);
            let weighted_force = excitation * coeff;
            let blended_force = excitation * (1.0 - coupling) + weighted_force * coupling;
            let sample = mode.process(blended_force, sample_rate);
            if track {
                let c = &mode.coefficients;
                let w = c.fade * (1.0 + boost * c.highpass_response);
                level += w * (mode.z_re * mode.z_re + mode.z_im * mode.z_im).sqrt();
                per_force += w * c.gain_norm * ((1.0 - coupling) + coeff * coupling).abs();
            }
            let mode_position = index as f32 / mode_count.max(1) as f32;
            let pan_spread = width * mode_position;
            let pan_direction = if index.is_multiple_of(2) { 1.0 } else { -1.0 };
            let pan = (pan_spread * pan_direction).clamp(-1.0, 1.0);
            let left_gain = 0.5 * (1.0 - pan);
            let right_gain = 0.5 * (1.0 + pan);

            left_sum += sample * left_gain;
            right_sum += sample * right_gain;
            interaction_displacement += sample * coeff;
            if dynamic {
                amp_sum += sample.abs();
                if index < 3 {
                    lf_sum += sample;
                }
            }
        }

        if track {
            self.ring.bound = RingBound { level, per_force };
            self.ring.countdown = RING_EXACT_INTERVAL - 1;
        } else if track_any {
            self.ring.bound.level += self.ring.bound.per_force * excitation.abs();
            self.ring.countdown -= 1;
        }
        if dynamic {
            self.track_dynamic_state(amp_sum, lf_sum, sample_rate);
        }

        // Velocity in displacement per 48 kHz sample, the unit the exciters'
        // own velocities use (exactly the plain difference at 48 kHz).
        self.interaction_bus.state.resonator_velocity = (interaction_displacement
            - self.interaction_bus.state.resonator_displacement)
            * (sample_rate as f32 / REFERENCE_SAMPLE_RATE);
        self.interaction_bus.state.resonator_displacement = interaction_displacement;
        (left_sum, right_sum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::ModalProfileId;

    const PROFILES: [ModalProfileId; 9] = [
        ModalProfileId::Pipe,
        ModalProfileId::Plate,
        ModalProfileId::Tank,
        ModalProfileId::Chain,
        ModalProfileId::IBeam,
        ModalProfileId::TautCable,
        ModalProfileId::CoilSpring,
        ModalProfileId::SheetMetal,
        ModalProfileId::IndustrialCog,
    ];
    /// QualityMode Eco / Normal / High / Render mode-count scales.
    const QUALITY_SCALES: [f32; 4] = [0.5, 1.0, 1.5, 2.0];
    const DAMAGE: [f32; 3] = [0.0, 0.5, 1.0];
    const FIXTURE: &str = include_str!("../../../tests/fixtures/mode_bank_hashes.txt");

    /// Builds a note-on bank the way `Voice::note_on_with_controls` does,
    /// with every transform engaged so they all reach the fingerprint.
    fn note_on_bank(profile: ModalProfileId, scale: f32, damage: f32) -> ModalResonator {
        ModalResonator::with_algorithm_controls_and_note(
            profile,
            crate::dsp::SizeScale::new(1.2),
            crate::dsp::RustAmount::new(0.4),
            crate::dsp::DamageAmount::new(damage),
            crate::dsp::ThicknessAmount::new(0.3),
            crate::dsp::HeatAmount::new(0.2),
            crate::dsp::SludgeAmount::new(0.1),
            261.63,
            0.45,
            0.6,
            CharacterParams::default(),
            scale,
        )
    }

    /// FNV-1a over the mode count and every mode's spec and base values.
    fn bank_hash(resonator: &ModalResonator) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut feed = |bits: u32| {
            for byte in bits.to_le_bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        };
        feed(resonator.modes.len() as u32);
        for mode in &resonator.modes {
            for value in [
                mode.spec.frequency_hz,
                mode.spec.decay_seconds,
                mode.spec.gain,
                mode.base_frequency_hz,
                mode.base_decay_seconds,
                mode.base_gain,
            ] {
                feed(value.to_bits());
            }
        }
        hash
    }

    fn current_fingerprints() -> Vec<String> {
        let mut lines = Vec::new();
        for profile in PROFILES {
            for scale in QUALITY_SCALES {
                for damage in DAMAGE {
                    let bank = note_on_bank(profile, scale, damage);
                    lines.push(format!(
                        "{profile:?} scale={scale} damage={damage} modes={} {:#018x}",
                        bank.modes.len(),
                        bank_hash(&bank)
                    ));
                }
            }
        }
        lines
    }

    /// `MAX_MODES` must cover every bank a note-on can build. The transform
    /// truncates at `MAX_MODES`, so checking the final bank would pass
    /// vacuously; check the generator's raw count and the untruncated
    /// damage expansion instead.
    #[test]
    fn note_on_banks_fit_max_modes() {
        let extremes = [0.0, 1.0, 2.0];
        let mut raw = Vec::new();
        for profile in PROFILES {
            for scale in [0.25, 0.5, 1.0, 1.5, 2.0, 4.0, 100.0] {
                for value in extremes {
                    let character = CharacterParams {
                        pipe_diameter: value,
                        plate_aspect: value,
                        plate_stiffness: value,
                        tank_volume: value,
                        tank_cavity_mix: value,
                        chain_link_mass: value,
                        chain_instability: value,
                        beam_shear: value,
                        cable_braid: value,
                        cable_tension_drop: value,
                        spring_dispersion: value,
                        spring_slosh: value,
                        sheet_thinness: value,
                        cog_dissonance: value,
                    };
                    ModalResonator::generate_algorithm_modes_into(
                        profile,
                        crate::dsp::SizeScale::new(1.0),
                        261.63,
                        character,
                        scale,
                        &mut raw,
                    );
                    let name = format!("{profile:?} scale={scale} character={value}");
                    assert!(
                        raw.len() <= MAX_RAW_MODES,
                        "{name}: {} raw modes > {MAX_RAW_MODES}",
                        raw.len()
                    );
                    let mode_count = raw.len().max(1);
                    let damaged: usize = raw
                        .iter()
                        .enumerate()
                        .map(|(index, mode)| {
                            mode.damaged(crate::dsp::DamageAmount::new(10.0), index, mode_count)
                                .count()
                        })
                        .sum();
                    assert!(
                        damaged <= MAX_MODES,
                        "{name}: {damaged} damaged modes > MAX_MODES {MAX_MODES}"
                    );
                }
            }
        }
    }

    #[test]
    fn rebuilding_a_reused_resonator_matches_a_fresh_one() {
        // One long-lived resonator, rebuilt for every case in turn (growing
        // and shrinking the bank), must match a freshly built bank exactly.
        let mut reused = ModalResonator::with_profile(ModalProfileId::Pipe);
        for profile in PROFILES {
            for scale in QUALITY_SCALES {
                for damage in DAMAGE {
                    reused.rebuild_in_place(
                        profile,
                        crate::dsp::SizeScale::new(1.2),
                        crate::dsp::RustAmount::new(0.4),
                        crate::dsp::DamageAmount::new(damage),
                        crate::dsp::ThicknessAmount::new(0.3),
                        crate::dsp::HeatAmount::new(0.2),
                        crate::dsp::SludgeAmount::new(0.1),
                        261.63,
                        0.45,
                        0.6,
                        CharacterParams::default(),
                        scale,
                        48_000,
                    );
                    let fresh = note_on_bank(profile, scale, damage);
                    assert_eq!(
                        bank_hash(&reused),
                        bank_hash(&fresh),
                        "{profile:?} scale={scale} damage={damage}"
                    );
                    assert_eq!(
                        reused.interaction_bus.mode_coefficients,
                        fresh.interaction_bus.mode_coefficients
                    );
                }
            }
        }
    }

    /// Note-on mode banks must not change when allocation is removed from the
    /// note-on path (review C3). The fixture was captured from the allocating
    /// implementation; regenerate it only for an intended sound change with
    /// `CORROSION_UPDATE_GOLDEN=1 cargo test mode_banks_match_golden`.
    /// Re-captured for `normalize-object-loudness`, which scales every bank's
    /// gains to one strike level at note-on (the mode frequencies and decays
    /// are unchanged).
    /// Re-captured again for `resolve-exciter-tradeoffs`: the note-on level
    /// estimate uses the modes' true `b0/ω` amplitude and the top-of-band
    /// fade, so the normalized gains moved slightly.
    /// Re-captured for `h11-prune-unsoundable-modes`: note-on drops modes at
    /// or above 80 kHz or with a decay under 0.1 ms. Only the three Pipe
    /// Render cases lose modes (24 → 18, 24 → 18, 12 → 9); the other 105
    /// cases are unchanged.
    #[test]
    fn mode_banks_match_golden_fixture() {
        let current = current_fingerprints();
        if std::env::var_os("CORROSION_UPDATE_GOLDEN").is_some() {
            let path = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/mode_bank_hashes.txt"
            );
            std::fs::write(path, current.join("\n") + "\n").expect("write fixture");
            return;
        }
        let golden: Vec<&str> = FIXTURE.lines().collect();
        assert_eq!(golden.len(), current.len(), "fixture case count changed");
        for (expected, actual) in golden.iter().zip(&current) {
            assert_eq!(*expected, actual.as_str());
        }
    }

    // --- Sample-rate-independent level (fix-resonator-sample-rate-gain) ---

    const LEVEL_PROFILES: [ModalProfileId; 4] = [
        ModalProfileId::Tank,
        ModalProfileId::Pipe,
        ModalProfileId::Plate,
        ModalProfileId::IndustrialCog,
    ];

    fn c4_bank(profile: ModalProfileId) -> ModalResonator {
        ModalResonator::with_algorithm_controls_and_note(
            profile,
            crate::dsp::SizeScale::new(1.0),
            crate::dsp::RustAmount::new(0.0),
            crate::dsp::DamageAmount::new(0.0),
            crate::dsp::ThicknessAmount::new(0.0),
            crate::dsp::HeatAmount::new(0.0),
            crate::dsp::SludgeAmount::new(0.0),
            261.63,
            0.5,
            0.5,
            CharacterParams::default(),
            1.0,
        )
    }

    /// Peak output of a 1 ms unit force pulse, over one second.
    fn pulse_peak(profile: ModalProfileId, sample_rate: u32) -> f32 {
        let mut bank = c4_bank(profile);
        let pulse = (sample_rate as f32 * 0.001).round() as usize;
        (0..sample_rate as usize)
            .map(|i| {
                bank.process_sample(if i < pulse { 1.0 } else { 0.0 }, sample_rate)
                    .abs()
            })
            .fold(0.0, f32::max)
    }

    /// RMS over the second half of a one-second sine drive at the bank's
    /// lowest mode, at the frequency its f32 coefficients actually resonate
    /// at for this sample rate, so the test measures gain, not tuning.
    fn sine_rms(profile: ModalProfileId, sample_rate: u32) -> f32 {
        let mut bank = c4_bank(profile);
        let lowest = bank
            .modes
            .iter()
            .min_by(|a, b| a.spec.frequency_hz.total_cmp(&b.spec.frequency_hz))
            .map(|m| m.spec)
            .unwrap();
        let c = CoupledCoefficients::for_mode(lowest, sample_rate);
        let omega = (c.pole_im as f64).atan2(c.pole_re as f64);
        let f = (omega * sample_rate as f64 / std::f64::consts::TAU) as f32;
        let n = sample_rate as usize;
        let sum: f64 = (0..n)
            .map(|i| {
                let x = 0.01 * (std::f32::consts::TAU * f * i as f32 / sample_rate as f32).sin();
                let y = bank.process_sample(x, sample_rate);
                if i >= n / 2 {
                    (y * y) as f64
                } else {
                    0.0
                }
            })
            .sum();
        (sum / (n / 2) as f64).sqrt() as f32
    }

    fn db(ratio: f32) -> f32 {
        20.0 * ratio.log10()
    }

    #[test]
    fn short_strike_level_does_not_depend_on_sample_rate() {
        for profile in LEVEL_PROFILES {
            let reference = pulse_peak(profile, 48_000);
            for sample_rate in [44_100, 96_000, 192_000] {
                let diff = db(pulse_peak(profile, sample_rate) / reference);
                assert!(
                    diff.abs() <= 0.5,
                    "{profile:?} at {sample_rate} Hz: {diff:+.2} dB vs 48 kHz"
                );
            }
        }
    }

    /// Includes 192 kHz since `normalize-object-loudness` moved the modes to
    /// the coupled form. In the direct form, f32 rounding (a1 ≈ −2, a2 ≈ 1)
    /// cost Tank's low, long-ringing 87 Hz cavity about 1.5 dB at 192 kHz.
    #[test]
    fn sustained_level_does_not_depend_on_sample_rate() {
        for profile in LEVEL_PROFILES {
            let reference = sine_rms(profile, 48_000);
            for sample_rate in [44_100, 96_000, 192_000] {
                let diff = db(sine_rms(profile, sample_rate) / reference);
                assert!(
                    diff.abs() <= 0.5,
                    "{profile:?} at {sample_rate} Hz: {diff:+.2} dB vs 48 kHz"
                );
            }
        }
    }

    /// 48 kHz output must not change (fix-resonator-sample-rate-gain). Only
    /// the input gain picked up a sample-rate factor, so it is enough that
    /// `b0` equals the mode gain exactly at 48 kHz, for every mode of every
    /// object. (A render hash would also prove it, but libm differs between
    /// the musl and glibc targets CI runs, so a hash isn't portable.)
    #[test]
    fn input_gain_is_unchanged_at_48k() {
        for profile in PROFILES {
            for mode in &c4_bank(profile).modes {
                let c = ResonatorCoefficients::for_mode(mode.spec, 48_000);
                assert_eq!(c.b0.to_bits(), mode.spec.gain.to_bits(), "{profile:?}");
            }
        }
    }

    // --- Loudness across objects, pitch, quality and damage
    // (normalize-object-loudness) ---

    fn level_bank(
        profile: ModalProfileId,
        note_hz: f32,
        scale: f32,
        damage: f32,
    ) -> ModalResonator {
        ModalResonator::with_algorithm_controls_and_note(
            profile,
            crate::dsp::SizeScale::new(1.0),
            crate::dsp::RustAmount::new(0.0),
            crate::dsp::DamageAmount::new(damage),
            crate::dsp::ThicknessAmount::new(0.0),
            crate::dsp::HeatAmount::new(0.0),
            crate::dsp::SludgeAmount::new(0.0),
            note_hz,
            0.5,
            0.5,
            CharacterParams::default(),
            scale,
        )
    }

    /// Peak of a bank's response to a one-sample unit impulse at 48 kHz,
    /// over 1 s. (A 1 ms pulse would itself filter the strike: it nearly
    /// cancels any mode near 1 kHz, the C6 fundamental.)
    fn strike_peak(mut bank: ModalResonator) -> f32 {
        (0..48_000)
            .map(|i| {
                bank.process_sample(if i == 0 { 1.0 } else { 0.0 }, 48_000)
                    .abs()
            })
            .fold(0.0, f32::max)
    }

    const C2: f32 = 65.41;
    const C4: f32 = 261.63;
    const C6: f32 = 1046.5;

    #[test]
    fn loudness_does_not_depend_on_pitch() {
        let mut failures = Vec::new();
        for profile in PROFILES {
            let c4 = strike_peak(level_bank(profile, C4, 1.0, 0.0));
            for (name, hz) in [("C2", C2), ("C6", C6)] {
                let diff = db(strike_peak(level_bank(profile, hz, 1.0, 0.0)) / c4);
                if diff.abs() > 3.0 || !diff.is_finite() {
                    failures.push(format!("{profile:?} {name}: {diff:+.2} dB vs C4"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn objects_are_comparably_loud() {
        let peaks: Vec<(ModalProfileId, f32)> = PROFILES
            .iter()
            .map(|&p| (p, strike_peak(level_bank(p, C4, 1.0, 0.0))))
            .collect();
        let (loud, max) = peaks
            .iter()
            .copied()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        let (quiet, min) = peaks
            .iter()
            .copied()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        let spread = db(max / min);
        assert!(
            spread <= 6.0,
            "{loud:?} {max:.3} vs {quiet:?} {min:.3}: {spread:.2} dB spread\n{peaks:?}"
        );
    }

    #[test]
    fn quality_mode_and_damage_do_not_change_loudness() {
        let mut failures = Vec::new();
        for profile in PROFILES {
            let eco = strike_peak(level_bank(profile, C4, 0.5, 0.0));
            let render = strike_peak(level_bank(profile, C4, 2.0, 0.0));
            let diff = db(render / eco);
            if diff.abs() > 2.0 || !diff.is_finite() {
                failures.push(format!("{profile:?} Render vs Eco: {diff:+.2} dB"));
            }
            let clean = strike_peak(level_bank(profile, C4, 1.0, 0.0));
            let damaged = strike_peak(level_bank(profile, C4, 1.0, 1.0));
            let diff = db(damaged / clean);
            if diff.abs() > 2.0 || !diff.is_finite() {
                failures.push(format!("{profile:?} damage 1 vs 0: {diff:+.2} dB"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    // --- Contact prediction (stabilize-mallet-and-grinding-contact) ---

    /// The displacement `contact_prediction` predicts for a force matches
    /// what the next `process_sample` produces with that force, for a
    /// coupled and an uncoupled strike position, on a static and a dynamic
    /// (sheet-warp) object.
    #[test]
    fn contact_prediction_matches_the_next_sample() {
        for profile in [ModalProfileId::Pipe, ModalProfileId::SheetMetal] {
            for (position, coupling) in [(0.5, 1.0), (0.3, 0.6), (0.5, 0.0)] {
                let mut bank = level_bank(profile, C4, 1.0, 0.0);
                bank.set_interaction_params(position, coupling, 0.0, 0.0, 0.0, 48_000.0);
                for n in 0..2_000 {
                    let force = if n < 48 {
                        0.3
                    } else {
                        ((n as f32) * 0.37).sin() * 0.01
                    };
                    let prediction = bank.contact_prediction(48_000);
                    bank.process_sample(force, 48_000);
                    let actual = bank.get_displacement();
                    let predicted = prediction.displacement(force);
                    // f32 rounding: the two paths sum the same terms in a different order.
                    let tolerance = 1e-4 * actual.abs().max(1e-3);
                    assert!(
                        (predicted - actual).abs() <= tolerance,
                        "{profile:?} position {position} coupling {coupling} sample {n}: predicted {predicted} actual {actual}"
                    );
                }
            }
        }
    }

    /// Calling `contact_prediction` before `process_sample` doesn't change
    /// the output: the per-sample preparation still runs exactly once.
    #[test]
    fn contact_prediction_does_not_change_the_output() {
        for profile in [
            ModalProfileId::Pipe,
            ModalProfileId::TautCable,
            ModalProfileId::SheetMetal,
        ] {
            let mut plain = level_bank(profile, C4, 1.0, 0.0);
            let mut predicted = level_bank(profile, C4, 1.0, 0.0);
            for bank in [&mut plain, &mut predicted] {
                bank.set_interaction_params(0.4, 0.8, 0.3, 2.55, 0.0, 48_000.0);
            }
            for n in 0..4_000 {
                let force = if n < 48 { 0.5 } else { 0.0 };
                let _ = predicted.contact_prediction(48_000);
                let a = plain.process_sample(force, 48_000);
                let b = predicted.process_sample(force, 48_000);
                assert_eq!(a.to_bits(), b.to_bits(), "{profile:?} sample {n}");
            }
        }
    }

    // --- Top of the band (resolve-exciter-tradeoffs) ---

    /// A resonator holding one mode, with uncoupled (unit) strike weights.
    fn single_mode_bank(frequency_hz: f32) -> ModalResonator {
        let mut bank = ModalResonator::default();
        bank.modes.clear();
        bank.modes
            .push(SecondOrderMode::new(crate::dsp::ModalModeSpec::new(
                frequency_hz,
                0.5,
                1.0,
            )));
        bank.interaction_bus.reset(1);
        bank.set_interaction_params(0.5, 0.0, 0.0, 0.0, 0.0, 48_000.0);
        bank
    }

    #[test]
    fn ring_bound_rises_by_at_most_per_force() {
        // The voice's drive governor relies on `per_force` bounding how far
        // one sample's force can raise the ring bound.
        for profile in PROFILES {
            let mut bank = note_on_bank(profile, 1.0, 0.0);
            bank.set_ring_tracking(true, 1.5);
            for force in [1.0f32, -0.5, 0.25, 0.0, 2.0] {
                let before = bank.ring_bound(48_000);
                bank.process_sample(force, 48_000);
                let after = bank.ring_bound(48_000);
                assert!(
                    after.level <= before.level + before.per_force * force.abs() + 1e-5,
                    "{profile:?}: {before:?} + {force} -> {after:?}"
                );
            }
        }
    }

    #[test]
    fn a_high_mode_keeps_its_level_at_every_sample_rate() {
        let peak_at = |sample_rate: u32| {
            let mut bank = single_mode_bank(15_000.0);
            // One sample at the reference rate's force × time.
            let impulse = sample_rate as f32 / 48_000.0;
            (0..sample_rate as usize / 50)
                .map(|n| {
                    bank.process_sample(if n == 0 { impulse } else { 0.0 }, sample_rate)
                        .abs()
                })
                .fold(0.0, f32::max)
        };
        let reference = peak_at(48_000);
        for sample_rate in [44_100, 96_000] {
            let diff = db(peak_at(sample_rate) / reference);
            assert!(
                diff.abs() <= 1.0,
                "15 kHz mode at {sample_rate} Hz: {diff:+.2} dB vs 48 kHz"
            );
        }
    }

    /// A ringing mode bent from 16 kHz to 22 kHz over 100 ms fades without a
    /// step: no sample-to-sample change exceeds twice the largest one in the
    /// first 10 ms of the sweep.
    #[test]
    fn a_mode_swept_through_the_top_of_the_band_does_not_click() {
        let mut bank = single_mode_bank(16_000.0);
        bank.process_sample(1.0, 48_000);
        let mut previous = bank.process_sample(0.0, 48_000);
        let (mut early, mut largest) = (0.0f32, 0.0f32);
        for n in 0..4_800 {
            let factor = 1.0 + 0.375 * n as f32 / 4_800.0;
            bank.set_pitch_bend_factor(factor, 48_000);
            let y = bank.process_sample(0.0, 48_000);
            let step = (y - previous).abs();
            previous = y;
            if n < 480 {
                early = early.max(step);
            }
            largest = largest.max(step);
        }
        assert!(
            largest <= 2.0 * early,
            "largest step {largest:.4e} vs {early:.4e} in the first 10 ms"
        );
    }

    // --- Unsoundable modes (h11-prune-unsoundable-modes) ---

    /// The spec's limits: the top of the audible band at the lowest pitch a
    /// note can be tuned to (20 kHz / 0.25), and a 0.1 ms decay.
    const SPEC_MAX_HZ: f32 = 80_000.0;
    const SPEC_MIN_DECAY_S: f32 = 1e-4;

    fn prune_bank(
        profile: ModalProfileId,
        note_hz: f32,
        size: f32,
        scale: f32,
        damage: f32,
    ) -> ModalResonator {
        ModalResonator::with_algorithm_controls_and_note(
            profile,
            crate::dsp::SizeScale::new(size),
            crate::dsp::RustAmount::new(0.0),
            crate::dsp::DamageAmount::new(damage),
            crate::dsp::ThicknessAmount::new(0.0),
            crate::dsp::HeatAmount::new(0.0),
            crate::dsp::SludgeAmount::new(0.0),
            note_hz,
            0.5,
            0.5,
            CharacterParams::default(),
            scale,
        )
    }

    /// The bank's modes that break the spec's limits, as (index, Hz, decay).
    fn unsoundable(bank: &ModalResonator) -> Vec<(usize, f32, f32)> {
        bank.modes
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                m.spec.frequency_hz >= SPEC_MAX_HZ || m.spec.decay_seconds < SPEC_MIN_DECAY_S
            })
            .map(|(i, m)| (i, m.spec.frequency_hz, m.spec.decay_seconds))
            .collect()
    }

    #[test]
    fn small_high_pipe_has_only_soundable_modes() {
        let bank = prune_bank(ModalProfileId::Pipe, C6, 0.05, 2.0, 0.0);
        let bad = unsoundable(&bank);
        assert!(
            bad.is_empty(),
            "Pipe C6, smallest size, Render: {} of {} modes can't sound, e.g. {:?}",
            bad.len(),
            bank.modes.len(),
            &bad[..bad.len().min(3)]
        );
    }

    #[test]
    fn every_bank_has_only_soundable_modes() {
        let mut failures = Vec::new();
        for profile in PROFILES {
            for (name, hz) in [("C2", C2), ("C4", C4), ("C6", C6)] {
                for scale in QUALITY_SCALES {
                    for damage in [0.0, 1.0] {
                        let bank = prune_bank(profile, hz, 1.0, scale, damage);
                        let bad = unsoundable(&bank).len();
                        if bad > 0 {
                            failures.push(format!(
                                "{profile:?} {name} scale {scale} damage {damage}: {bad} of {}",
                                bank.modes.len()
                            ));
                        }
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "banks with unsoundable modes:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn retuning_down_keeps_the_bank() {
        for profile in PROFILES {
            let mut bank = prune_bank(profile, C4, 1.0, 2.0, 0.0);
            let before: Vec<u32> = bank
                .modes
                .iter()
                .map(|m| m.base_frequency_hz.to_bits())
                .collect();
            bank.set_pitch_bend_factor(0.25, 48_000);
            let after: Vec<u32> = bank
                .modes
                .iter()
                .map(|m| m.base_frequency_hz.to_bits())
                .collect();
            assert_eq!(before, after, "{profile:?}: the bank changed on a retune");
            for mode in &bank.modes {
                let expected = mode.base_frequency_hz * 0.25;
                assert!(
                    (mode.spec.frequency_hz - expected).abs() <= expected * 1e-6,
                    "{profile:?}: {} Hz, expected {expected} Hz",
                    mode.spec.frequency_hz
                );
            }
        }
    }

    /// Prints how many modes each object's bank holds that can never sound,
    /// over C2/C4/C6 at every quality and damage 0 and 1.
    #[test]
    #[ignore]
    fn print_unsoundable_mode_counts() {
        for profile in PROFILES {
            let (mut bad, mut total) = (0usize, 0usize);
            let mut worst = String::new();
            let mut worst_bad = 0usize;
            for (name, hz) in [("C2", C2), ("C4", C4), ("C6", C6)] {
                for scale in QUALITY_SCALES {
                    for damage in [0.0, 1.0] {
                        let bank = prune_bank(profile, hz, 1.0, scale, damage);
                        let n = unsoundable(&bank).len();
                        bad += n;
                        total += bank.modes.len();
                        if n > worst_bad {
                            worst_bad = n;
                            worst = format!(
                                "{name} scale {scale} damage {damage}: {n} of {}",
                                bank.modes.len()
                            );
                        }
                    }
                }
            }
            let small = prune_bank(profile, C6, 0.05, 2.0, 0.0);
            println!(
                "UNSOUNDABLE {profile:?}: {bad} of {total} modes over the sweep; worst {worst}; \
                 C6 smallest size Render: {} of {}",
                unsoundable(&small).len(),
                small.modes.len()
            );
        }
    }

    // --- Retune cost (h12-skip-redundant-mode-retunes) ---

    /// A ringing bank's output over one second: struck once, then with
    /// `bend` held, rebuilding every mode every sample unless `skip`.
    /// TautCable's tension drop is 0, so only the bend sets its pitch.
    fn bent_output(profile: ModalProfileId, bend: f32, skip: bool) -> Vec<u32> {
        let mut bank = ModalResonator::with_algorithm_controls_and_note(
            profile,
            crate::dsp::SizeScale::new(1.0),
            crate::dsp::RustAmount::new(0.0),
            crate::dsp::DamageAmount::new(0.5),
            crate::dsp::ThicknessAmount::new(0.0),
            crate::dsp::HeatAmount::new(0.0),
            crate::dsp::SludgeAmount::new(0.0),
            C4,
            0.5,
            0.5,
            CharacterParams {
                cable_tension_drop: 0.0,
                ..CharacterParams::default()
            },
            2.0,
        );
        bank.retune_skip_enabled = skip;
        bank.set_pitch_bend_factor(bend, 48_000);
        (0..48_000)
            .map(|i| {
                bank.process_sample(if i == 0 { 1.0 } else { 0.0 }, 48_000)
                    .to_bits()
            })
            .collect()
    }

    #[test]
    fn held_bend_is_bit_identical_to_rebuilding_every_sample() {
        // A constant pitch: every static object, and TautCable with its
        // tension drop at 0. SheetMetal's warp always moves its pitch, so
        // it's covered by `moving_dynamics_stay_in_tune`'s rule instead.
        let bend = 2.0f32.powf(2.0 / 12.0);
        for profile in PROFILES
            .into_iter()
            .filter(|&p| p != ModalProfileId::SheetMetal)
        {
            assert!(
                bent_output(profile, bend, true) == bent_output(profile, bend, false),
                "{profile:?}: a held +2-semitone bend changes the output"
            );
        }
    }

    #[test]
    fn moving_dynamics_stay_in_tune() {
        // A struck TautCable: its pitch factor moves with the ring's
        // amplitude every sample. Every mode must stay within 0.05 cents of
        // `base × factor` at every sample.
        let tolerance = (0.05f64 / 1200.0 * std::f64::consts::LN_2) as f32;
        for profile in [ModalProfileId::TautCable, ModalProfileId::SheetMetal] {
            let mut bank = prune_bank(profile, C4, 1.0, 2.0, 0.0);
            let mut moved = 0usize;
            let mut last = bank.target_pitch_factor();
            for i in 0..48_000 {
                let factor = bank.target_pitch_factor();
                if factor != last {
                    moved += 1;
                    last = factor;
                }
                bank.process_sample(if i < 48 { 1.0 } else { 0.0 }, 48_000);
                for (index, mode) in bank.modes.iter().enumerate() {
                    let target = mode.base_frequency_hz * factor;
                    let cents_ln = (mode.spec.frequency_hz / target).ln().abs();
                    assert!(
                        cents_ln <= tolerance,
                        "{profile:?} sample {i}, mode {index}: {} Hz vs {target} Hz ({:.4} cents)",
                        mode.spec.frequency_hz,
                        cents_ln / std::f32::consts::LN_2 * 1200.0
                    );
                    // The pole the mode actually rings at, which a small
                    // retune rotates rather than rebuilds, must be in tune
                    // too.
                    let c = mode.coefficients;
                    let angle = (c.pole_im as f64).atan2(c.pole_re as f64);
                    let expected = std::f64::consts::TAU * target as f64 / 48_000.0;
                    if expected < std::f64::consts::PI {
                        let pole_ln = (angle / expected).ln().abs() as f32;
                        assert!(
                            pole_ln <= tolerance,
                            "{profile:?} sample {i}, mode {index}: pole angle {angle} vs \
                             {expected} ({:.4} cents)",
                            pole_ln / std::f32::consts::LN_2 * 1200.0
                        );
                    }
                }
            }
            assert!(
                moved > 1_000,
                "{profile:?}: the factor only moved {moved} times"
            );
        }
    }
}

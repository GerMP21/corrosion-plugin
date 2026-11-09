//! Real-time post-processing chain.
//!
//! Each channel runs its own filter and drive (dual-mono, so the voice
//! engine's stereo image survives). The body resonator runs in parallel on
//! the mono sum and is mixed back into both channels at the BODY amount.
//! The result goes through HRTF spread, the selected space mode, and an
//! oversampled clipper. Every quality mode runs every stage; modes only
//! change oversampling (and, at the voice layer, modal density).
use super::{
    FactoryEcho, FactoryReverb, FemBodyResonator, HrtfSpread, LorenzDrive, OversampledClipper,
    SpaceMode, SpringReverb, WdfLadderFilter,
};

/// CPU/quality tradeoff level for the post-processing chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostQualityMode {
    Eco,
    Normal,
    High,
    Render,
}

impl PostQualityMode {
    /// The final clipper's oversampling factor in this mode.
    pub fn clipper_oversample_factor(self) -> usize {
        match self {
            PostQualityMode::Eco => 1,
            PostQualityMode::Normal => 4,
            PostQualityMode::High => 8,
            PostQualityMode::Render => 16,
        }
    }
}

/// Ordered post-processing pipeline for the plugin output.
pub struct PostProcessingChain {
    filter_left: WdfLadderFilter,
    filter_right: WdfLadderFilter,
    drive_left: LorenzDrive,
    drive_right: LorenzDrive,
    body: FemBodyResonator,
    spread: HrtfSpread,
    factory_reverb: FactoryReverb,
    /// One mono spring per channel, so each tail comes from its own input.
    spring_left: SpringReverb,
    spring_right: SpringReverb,
    factory_echo: FactoryEcho,
    clipper: OversampledClipper,

    space_mode: SpaceMode,
    space_amount: f32,
    /// Smoothed body mix (0–1) and its target; the first set snaps.
    body_amount: f32,
    body_amount_target: f32,
    body_amount_primed: bool,
    /// Per-sample one-pole coefficient for the body-amount smoother.
    body_smoothing_coeff: f32,
    quality_mode: PostQualityMode,

    sample_rate: f32,
}

impl PostProcessingChain {
    /// Time constant of the BODY-amount smoother, matching the filter's.
    const BODY_SMOOTHING_TAU_SECONDS: f32 = 0.02;

    fn smoothing_coeff(sample_rate: f32) -> f32 {
        1.0 - (-1.0 / (Self::BODY_SMOOTHING_TAU_SECONDS * sample_rate.max(1.0))).exp()
    }

    /// Creates the chain with default stage instances and sample rate.
    pub fn new() -> Self {
        Self {
            filter_left: WdfLadderFilter::new(),
            filter_right: WdfLadderFilter::new(),
            drive_left: LorenzDrive::new(),
            drive_right: LorenzDrive::new(),
            body: FemBodyResonator::new(),
            spread: HrtfSpread::new(),
            factory_reverb: FactoryReverb::new(),
            spring_left: SpringReverb::new(),
            spring_right: SpringReverb::new(),
            factory_echo: FactoryEcho::new(),
            clipper: OversampledClipper::new(),
            space_mode: SpaceMode::Off,
            space_amount: 0.0,
            body_amount: 0.0,
            body_amount_target: 0.0,
            body_amount_primed: false,
            body_smoothing_coeff: Self::smoothing_coeff(48000.0),
            quality_mode: PostQualityMode::Normal,
            sample_rate: 48000.0,
        }
    }

    /// Sets the CPU/quality tradeoff mode for the post-processing chain.
    pub fn set_quality_mode(&mut self, mode: PostQualityMode) {
        self.quality_mode = mode;
        self.clipper
            .set_oversample_factor(mode.clipper_oversample_factor());
    }

    /// Updates the sample rate for every stage in the chain.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.filter_left.set_sample_rate(sample_rate);
        self.filter_right.set_sample_rate(sample_rate);
        self.drive_left.set_sample_rate(sample_rate);
        self.drive_right.set_sample_rate(sample_rate);
        self.body_smoothing_coeff = Self::smoothing_coeff(sample_rate);
        self.body.set_sample_rate(sample_rate);
        self.spread.set_sample_rate(sample_rate);
        self.factory_reverb.set_sample_rate(sample_rate);
        self.spring_left.set_sample_rate(sample_rate);
        self.spring_right.set_sample_rate(sample_rate);
        self.factory_echo.set_sample_rate(sample_rate);
    }

    /// Sets the WDF filter controls.
    pub fn set_filter_params(&mut self, cutoff: f32, resonance: f32, tolerance: f32) {
        self.filter_left
            .set_parameters(cutoff, resonance, tolerance);
        self.filter_right
            .set_parameters(cutoff, resonance, tolerance);
    }

    /// Sets the chaotic drive controls.
    pub fn set_drive_params(&mut self, amount: f32, starvation: f32, chaos: f32) {
        self.drive_left.set_parameters(amount, starvation, chaos);
        self.drive_right.set_parameters(amount, starvation, chaos);
    }

    /// Sets the body resonance controls.
    pub fn set_body_params(&mut self, material: f32, volume: f32) {
        self.body.set_parameters(material, volume);
    }

    /// Sets how much body resonance is mixed into both channels (0–1).
    /// Smoothed at audio rate; the first call after `new`/`reset` snaps.
    pub fn set_body_amount(&mut self, amount: f32) {
        self.body_amount_target = amount.clamp(0.0, 1.0);
        if !self.body_amount_primed {
            self.body_amount_primed = true;
            self.body_amount = self.body_amount_target;
        }
    }

    /// Sets stereo spread and listener proximity controls.
    pub fn set_spread_params(&mut self, width: f32, proximity: f32) {
        self.spread.set_parameters(width, proximity);
    }

    /// Chooses which space algorithm is active.
    pub fn set_space_mode(&mut self, mode: SpaceMode) {
        self.space_mode = mode;
    }

    /// Sets the wet mix for the active space algorithm.
    pub fn set_space_amount(&mut self, amount: f32) {
        self.space_amount = amount.clamp(0.0, 1.0);
    }

    /// Sets the factory room parameters.
    pub fn set_factory_params(&mut self, size: f32, clutter: f32, impedance: f32) {
        self.factory_reverb.set_parameters(size, clutter, impedance);
    }

    /// Sets the spring reverb parameters.
    pub fn set_spring_params(&mut self, tension: f32, stiffness: f32, tank_size: f32) {
        self.spring_left
            .set_parameters(tension, stiffness, tank_size);
        self.spring_right
            .set_parameters(tension, stiffness, tank_size);
    }

    /// Sets the factory echo parameters.
    pub fn set_echo_params(
        &mut self,
        delay_time: f32,
        machinery_movement: f32,
        high_frequency_damping: f32,
    ) {
        self.factory_echo
            .set_parameters(delay_time, machinery_movement, high_frequency_damping);
    }

    /// Sets the final clipper ceiling and knee softness.
    pub fn set_clipper_params(&mut self, ceiling: f32, softness: f32) {
        self.clipper.set_parameters(ceiling, softness);
    }

    /// Processes one stereo frame through the full stage order.
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        // 1–2. Filter and drive, per channel.
        let driven_left = self.drive_left.process(self.filter_left.process(left));
        let driven_right = self.drive_right.process(self.filter_right.process(right));

        // 3. Body resonance in parallel on the mono sum, mixed at BODY.
        let diff = self.body_amount_target - self.body_amount;
        self.body_amount = if diff.abs() < 1e-6 {
            self.body_amount_target
        } else {
            self.body_amount + diff * self.body_smoothing_coeff
        };
        let body_wet = self.body.process((driven_left + driven_right) * 0.5) * self.body_amount;
        let bodied_left = driven_left + body_wet;
        let bodied_right = driven_right + body_wet;

        // 4. HRTF Stereo Spread
        let (spread_left, spread_right) = self.spread.process(bodied_left, bodied_right);

        // 5. Space, added at the space amount
        let (space_left, space_right) = match self.space_mode {
            SpaceMode::Off => (spread_left, spread_right),
            _ => {
                let (wet_l, wet_r) = self.process_space(spread_left, spread_right);
                (
                    spread_left + wet_l * self.space_amount,
                    spread_right + wet_r * self.space_amount,
                )
            }
        };

        // 6. Oversampled Clipper
        self.clipper.process_stereo(space_left, space_right)
    }

    /// Runs the selected space algorithm and returns its wet (left, right)
    /// output, before the space amount is applied.
    fn process_space(&mut self, left: f32, right: f32) -> (f32, f32) {
        match self.space_mode {
            SpaceMode::Off => (0.0, 0.0),
            SpaceMode::Factory => {
                let wet = self.factory_reverb.process((left + right) * 0.5);
                (wet, wet)
            }
            SpaceMode::Spring => (
                self.spring_left.process(left),
                self.spring_right.process(right),
            ),
            SpaceMode::Echo => self.factory_echo.process_stereo(left, right),
        }
    }

    /// Clears internal state for all stage buffers and filters.
    pub fn reset(&mut self) {
        self.filter_left.reset();
        self.filter_right.reset();
        self.drive_left.reset();
        self.drive_right.reset();
        self.body_amount_primed = false;
        self.body.reset();
        self.spread.reset();
        self.factory_reverb.reset();
        self.spring_left.reset();
        self.spring_right.reset();
        self.factory_echo.reset();
        self.clipper.reset();
    }
}

impl Default for PostProcessingChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    /// A chain with spread bypassed, space off and the given body amount.
    fn routing_chain(body_amount: f32, chassis_volume: f32) -> PostProcessingChain {
        routing_chain_with_cutoff(body_amount, chassis_volume, 20_000.0)
    }

    /// Like `routing_chain`, with the cutoff set on the first (snapping)
    /// call so the test doesn't measure the smoother's ramp.
    fn routing_chain_with_cutoff(
        body_amount: f32,
        chassis_volume: f32,
        cutoff: f32,
    ) -> PostProcessingChain {
        let mut chain = PostProcessingChain::new();
        chain.set_sample_rate(SR);
        chain.set_filter_params(cutoff, 0.0, 0.0);
        chain.set_drive_params(0.0, 0.0, 0.0);
        chain.set_body_params(0.5, chassis_volume);
        chain.set_body_amount(body_amount);
        chain.set_spread_params(0.0, 0.5);
        chain.set_space_mode(SpaceMode::Off);
        chain
    }

    fn tone(i: usize, freq: f32, amp: f32) -> f32 {
        amp * (std::f32::consts::TAU * freq * i as f32 / SR).sin()
    }

    fn rms(xs: &[f32]) -> f32 {
        (xs.iter().map(|x| x * x).sum::<f32>() / xs.len() as f32).sqrt()
    }

    #[test]
    fn hard_left_input_stays_left() {
        let mut chain = routing_chain(0.0, 0.5);
        let (mut left, mut right) = (Vec::new(), Vec::new());
        for i in 0..9_600 {
            let (l, r) = chain.process(tone(i, 440.0, 0.2), 0.0);
            if i >= 4_800 {
                left.push(l);
                right.push(r);
            }
        }
        let ratio_db = 20.0 * (rms(&right).max(1e-12) / rms(&left)).log10();
        assert!(
            ratio_db <= -20.0,
            "right channel only {ratio_db:.1} dB below left"
        );
    }

    #[test]
    fn body_zero_ignores_chassis_volume() {
        let render = |volume: f32| {
            let mut chain = routing_chain(0.0, volume);
            (0..4_800)
                .map(|i| {
                    let x = tone(i, 300.0, 0.2);
                    chain.process(x, x).0
                })
                .collect::<Vec<_>>()
        };
        let reference = render(0.0);
        for volume in [0.25, 0.5, 0.75, 1.0] {
            let out = render(volume);
            let max_diff = reference
                .iter()
                .zip(&out)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f32, f32::max);
            assert!(
                max_diff <= 1e-6,
                "chassis volume {volume} changed BODY-0 output by {max_diff}"
            );
        }
    }

    #[test]
    fn mono_input_stays_mono() {
        let mut chain = routing_chain(0.6, 0.5);
        chain.set_drive_params(0.8, 0.3, 0.5);
        for i in 0..4_800 {
            let x = tone(i, 220.0, 0.3);
            let (l, r) = chain.process(x, x);
            assert_eq!(l, r, "sample {i}");
        }
    }

    #[test]
    fn body_zero_matches_chain_without_body() {
        // Hand-built filter → drive per channel → (spread bypassed, space off)
        // → clipper, i.e. the chain minus the body stage.
        let mut chain = routing_chain(0.0, 0.5);
        let mut filters = [WdfLadderFilter::new(), WdfLadderFilter::new()];
        let mut drives = [LorenzDrive::new(), LorenzDrive::new()];
        for (f, d) in filters.iter_mut().zip(drives.iter_mut()) {
            f.set_sample_rate(SR);
            f.set_parameters(20_000.0, 0.0, 0.0);
            d.set_sample_rate(SR);
            d.set_parameters(0.0, 0.0, 0.0);
        }
        let mut clipper = OversampledClipper::new();
        for i in 0..4_800 {
            let (x_l, x_r) = (tone(i, 300.0, 0.2), tone(i, 450.0, 0.1));
            let expected = clipper.process_stereo(
                drives[0].process(filters[0].process(x_l)),
                drives[1].process(filters[1].process(x_r)),
            );
            let (l, r) = chain.process(x_l, x_r);
            assert!(
                (l - expected.0).abs() <= 1e-6 && (r - expected.1).abs() <= 1e-6,
                "sample {i}"
            );
        }
    }

    #[test]
    fn body_level_tracks_body_amount() {
        // Body contribution = output(amount) − output(0). Low amplitude and
        // zero drive keep every other stage linear: at amount 0.8 the body
        // peaks near 37× the input, so 0.01 stays under the oversampled
        // clipper's threshold (0.9661 / 1.3 ≈ 0.743).
        let render = |amount: f32| {
            let mut chain = routing_chain(amount, 0.5);
            (0..9_600)
                .map(|i| {
                    let x = tone(i, 250.0, 0.01);
                    chain.process(x, x).0
                })
                .collect::<Vec<_>>()
        };
        let dry = render(0.0);
        let contribution = |amount: f32| {
            let out = render(amount);
            rms(&out.iter().zip(&dry).map(|(a, b)| a - b).collect::<Vec<_>>())
        };
        // BODY 1.0 and 4.0 → amounts 0.2 and 0.8.
        let ratio = contribution(0.8) / contribution(0.2);
        assert!(
            (ratio - 4.0).abs() <= 0.2,
            "body contribution ratio {ratio}, expected 4 ± 5 %"
        );
    }

    /// Energy above ~4 kHz: four cascaded one-pole high-passes (−24 dB/oct).
    fn high_band_energy(xs: &[f32]) -> f64 {
        let a = (-std::f32::consts::TAU * 4_000.0 / SR).exp();
        let mut stages = [(0.0_f32, 0.0_f32); 4];
        let mut energy = 0.0_f64;
        for &x in xs {
            let mut v = x;
            for (prev_in, prev_out) in stages.iter_mut() {
                let y = a * (*prev_out + v - *prev_in);
                *prev_in = v;
                *prev_out = y;
                v = y;
            }
            energy += (v * v) as f64;
        }
        energy
    }

    #[test]
    fn cutoff_shapes_broadband_output() {
        // At the factory BODY (default 2.62 → mix 0.524), so the parallel
        // body path is in the measurement too.
        let factory_body = crate::CorrosionParams::default().body.value() / 5.0;
        assert!(factory_body > 0.5, "factory BODY mix {factory_body}");
        let render = |cutoff: f32| {
            let mut chain = routing_chain_with_cutoff(factory_body, 0.5, cutoff);
            let mut seed = 0x1234_5678_u32;
            (0..24_000)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let x = ((seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * 0.2;
                    chain.process(x, x).0
                })
                .collect::<Vec<_>>()
        };
        let open = high_band_energy(&render(20_000.0));
        let closed = high_band_energy(&render(1_000.0));
        let drop_db = 10.0 * (open / closed).log10();
        assert!(
            drop_db >= 20.0,
            "1 kHz cutoff cut > 4 kHz energy by only {drop_db:.1} dB"
        );
    }

    /// Frequency (Hz) of the strongest component of `xs` between `lo` and
    /// `hi`, by a direct DFT in 1 Hz steps.
    fn strongest_frequency(xs: &[f32], lo: usize, hi: usize) -> usize {
        (lo..=hi)
            .max_by(|&a, &b| {
                let power = |hz: usize| {
                    let w = std::f64::consts::TAU * hz as f64 / SR as f64;
                    let (re, im) = xs.iter().enumerate().fold((0.0, 0.0), |(re, im), (i, &x)| {
                        (
                            re + x as f64 * (w * i as f64).cos(),
                            im + x as f64 * (w * i as f64).sin(),
                        )
                    });
                    re * re + im * im
                };
                power(a).total_cmp(&power(b))
            })
            .unwrap()
    }

    #[test]
    fn larger_chassis_moves_the_body_peak_lower_in_the_output() {
        // Measured on the chain's output at full BODY, not on the body's
        // internal tuning: an impulse excites the body, whose first mode
        // dominates 100–400 Hz (≈314 Hz at volume 0.2, ≈169 Hz at 0.8).
        let peak_hz = |chassis_volume: f32| {
            let mut chain = routing_chain(1.0, chassis_volume);
            let response: Vec<f32> = (0..24_000)
                .map(|i| chain.process(if i == 0 { 0.5 } else { 0.0 }, 0.0).0)
                .collect();
            strongest_frequency(&response, 100, 400)
        };
        let (small, large) = (peak_hz(0.2), peak_hz(0.8));
        assert!(
            large + 50 < small,
            "larger chassis should resonate lower: {large} Hz vs {small} Hz"
        );
    }

    #[test]
    fn chain_processes_stereo() {
        let mut chain = PostProcessingChain::new();
        chain.set_sample_rate(48000.0);

        let (left, right) = chain.process(0.5, 0.3);
        assert!(left.is_finite());
        assert!(right.is_finite());
    }

    #[test]
    fn output_bounded() {
        let mut chain = PostProcessingChain::new();
        chain.set_sample_rate(48000.0);
        chain.set_drive_params(5.0, 1.0, 1.0);

        for i in 0..100 {
            let input = (i as f32 * 0.1).sin() * 2.0;
            let (left, right) = chain.process(input, input);
            assert!(left.abs() <= 1.0, "Left should be bounded by clipper");
            assert!(right.abs() <= 1.0, "Right should be bounded by clipper");
        }
    }

    fn quality_chain(
        mode: PostQualityMode,
        space: SpaceMode,
        body_amount: f32,
    ) -> PostProcessingChain {
        let mut chain = routing_chain(body_amount, 0.5);
        chain.set_quality_mode(mode);
        chain.set_space_mode(space);
        chain.set_space_amount(0.5);
        chain.set_factory_params(0.7, 0.3, 0.5);
        chain
    }

    /// Energy of the tail after a 0.1 s burst stops (reverb only once the
    /// dry signal is gone).
    fn tail_energy(chain: &mut PostProcessingChain) -> f64 {
        let burst = 4_800;
        (0..48_000)
            .map(|i| {
                let x = if i < burst { tone(i, 330.0, 0.2) } else { 0.0 };
                chain.process(x, x).0
            })
            .skip(burst + 480)
            .map(|y| (y * y) as f64)
            .sum()
    }

    #[test]
    fn eco_keeps_the_reverb() {
        // C5: Eco used to skip body, spread and space entirely.
        let eco = tail_energy(&mut quality_chain(
            PostQualityMode::Eco,
            SpaceMode::Factory,
            0.0,
        ));
        let normal = tail_energy(&mut quality_chain(
            PostQualityMode::Normal,
            SpaceMode::Factory,
            0.0,
        ));
        assert!(eco > 0.0, "Eco render has no reverb tail");
        let diff_db = 10.0 * (eco / normal).log10();
        assert!(
            diff_db.abs() <= 3.0,
            "Eco tail is {diff_db:.1} dB from Normal"
        );
    }

    #[test]
    fn eco_keeps_the_body() {
        // Body contribution = output(BODY) − output(no body), per mode. Low
        // amplitude keeps the clipper linear, so the two modes must agree.
        let contribution = |mode: PostQualityMode| {
            let render = |amount: f32| {
                let mut chain = quality_chain(mode, SpaceMode::Off, amount);
                (0..9_600)
                    .map(|i| {
                        let x = tone(i, 250.0, 0.02);
                        chain.process(x, x).0
                    })
                    .collect::<Vec<_>>()
            };
            let (with, without) = (render(0.5), render(0.0));
            rms(&with
                .iter()
                .zip(&without)
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>())
        };
        let (eco, normal) = (
            contribution(PostQualityMode::Eco),
            contribution(PostQualityMode::Normal),
        );
        assert!(eco > 0.0, "Eco output has no body resonance");
        assert!(
            (eco / normal - 1.0).abs() < 0.01,
            "Eco body {eco} vs Normal {normal}"
        );
    }

    #[test]
    fn quality_mode_changes_oversample_factor() {
        // Every quality mode runs the same stages, so the only difference
        // between these chains is the clipper's oversampling factor. Feeding
        // alternating transients exercises the interpolated upsampling.
        let mut normal = PostProcessingChain::new();
        normal.set_sample_rate(48000.0);
        normal.set_quality_mode(PostQualityMode::Normal);

        let mut render = PostProcessingChain::new();
        render.set_sample_rate(48000.0);
        render.set_quality_mode(PostQualityMode::Render);

        let mut total_diff = 0.0f32;
        for i in 0..64 {
            let input = if i % 2 == 0 { 2.0 } else { -2.0 };
            let (n_l, _) = normal.process(input, input);
            let (r_l, _) = render.process(input, input);
            total_diff += (n_l - r_l).abs();
        }

        assert!(
            total_diff > 0.001,
            "4x and 16x oversampling should diverge on transients, diff={total_diff}"
        );
    }

    /// Sample rate and spring settings for `spring_chain`, all away from the
    /// `SpringReverb::new()` defaults (48 kHz, 0.5/0.5/0.5), so a setter
    /// that doesn't reach the right-hand spring shows up in the tests.
    const SPRING_SR: f32 = 44_100.0;
    const SPRING_TENSION: f32 = 0.8;
    const SPRING_STIFFNESS: f32 = 0.2;
    const SPRING_TANK: f32 = 0.25;
    /// 80 ms at 44.1 kHz (3,528 samples) × (0.5 + tank size 0.25).
    const SPRING_DELAY: usize = 2_646;
    /// Spring output gain: tank size × 0.7.
    const SPRING_ECHO_GAIN: f32 = SPRING_TANK * 0.7;

    /// A chain in Spring mode with a known delay length.
    fn spring_chain() -> PostProcessingChain {
        let mut chain = PostProcessingChain::new();
        chain.set_sample_rate(SPRING_SR);
        chain.set_spring_params(SPRING_TENSION, SPRING_STIFFNESS, SPRING_TANK);
        chain.set_space_mode(SpaceMode::Spring);
        chain.set_space_amount(1.0);
        chain
    }

    /// Feeds `frames` frames of `input` to the Space stage alone and returns
    /// the wet (left, right) outputs.
    fn space_wet(
        chain: &mut PostProcessingChain,
        frames: usize,
        input: impl Fn(usize) -> (f32, f32),
    ) -> (Vec<f32>, Vec<f32>) {
        (0..frames)
            .map(|i| {
                let (l, r) = input(i);
                chain.process_space(l, r)
            })
            .unzip()
    }

    /// Deterministic white noise in [-0.5, 0.5).
    fn noise(i: usize) -> f32 {
        let x = (i as u32)
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (x >> 8) as f32 / (1u32 << 24) as f32 - 0.5
    }

    #[test]
    fn spring_left_only_input_leaves_right_silent() {
        let mut chain = spring_chain();
        let impulse = |i: usize| (if i == 0 { 1.0 } else { 0.0 }, 0.0);
        let (left, right) = space_wet(&mut chain, SPRING_DELAY * 6, impulse);
        assert!(left.iter().any(|&x| x != 0.0), "left channel has no tail");
        let first_leak = right.iter().position(|&x| x != 0.0);
        assert_eq!(first_leak, None, "left input leaked into the right tail");
    }

    #[test]
    fn spring_echo_arrives_after_full_delay() {
        let mut chain = spring_chain();
        // Different levels per side, so an echo on the wrong channel shows.
        let impulse = |i: usize| if i == 0 { (1.0, 0.5) } else { (0.0, 0.0) };
        let (left, right) = space_wet(&mut chain, SPRING_DELAY * 2, impulse);
        for (side, wet, level) in [("left", &left, 1.0), ("right", &right, 0.5)] {
            let first = wet.iter().position(|&x| x != 0.0);
            assert_eq!(first, Some(SPRING_DELAY), "{side} echo timing");
            let echo = wet[SPRING_DELAY];
            assert!(
                (echo - level * SPRING_ECHO_GAIN).abs() < 1e-6,
                "{side} echo level {echo}, expected {}",
                level * SPRING_ECHO_GAIN
            );
        }
    }

    #[test]
    fn spring_mono_input_gives_identical_tails() {
        let mut chain = spring_chain();
        let burst = |i: usize| {
            let x = if i < 4_800 { noise(i) } else { 0.0 };
            (x, x)
        };
        let (left, right) = space_wet(&mut chain, SPRING_DELAY * 6, burst);
        assert!(left.iter().any(|&x| x != 0.0), "no tail");
        let first_diff = left.iter().zip(&right).position(|(l, r)| l != r);
        assert_eq!(first_diff, None, "mono input gave different tails");
    }

    #[test]
    fn spring_reset_clears_both_channels() {
        let mut chain = spring_chain();
        let (left, right) = space_wet(&mut chain, SPRING_DELAY * 3, |i| {
            (noise(i), noise(i + 7_919))
        });
        let tail = SPRING_DELAY * 2..;
        assert!(
            left[tail.clone()].iter().any(|&x| x != 0.0),
            "left not ringing"
        );
        assert!(right[tail].iter().any(|&x| x != 0.0), "right not ringing");

        chain.reset();
        let (left, right) = space_wet(&mut chain, SPRING_DELAY * 3, |_| (0.0, 0.0));
        assert!(
            left.iter().chain(&right).all(|&x| x == 0.0),
            "reset left spring state behind"
        );
    }
}

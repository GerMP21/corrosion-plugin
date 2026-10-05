//! Multistage envelope generator used by strike, friction, and sustain voices.
//!
//! The voice module drives this envelope at sample rate to shape excitation
//! and release behavior without allocating or consulting host state.

/// 6-Stage Multistage Envelope Generator (MSEG)
///
/// Stage order: Onset -> Attack -> Hold -> Decay -> Sustain -> Release
/// Supports looping between stages and curved transitions.
#[derive(Clone, Debug)]
/// Envelope state and controls for a multi-stage generator.
pub struct MSEG {
    // Stage timing (seconds)
    t_onset: f32,
    t_attack: f32,
    t_hold: f32,
    t_decay: f32,
    t_release: f32,

    // Stage levels (0.0 - 1.0)
    l_onset: f32,
    l_peak: f32,
    l_sustain: f32,
    l_end: f32,

    // Curve tensions (0.0 = linear, -1.0 = exponential/log, 1.0 = logarithmic/exp)
    c_onset: f32,
    c_attack: f32,
    c_decay: f32,
    c_release: f32,

    // Looping
    loop_mode: LoopMode,
    loop_start_stage: u8,
    loop_end_stage: u8,

    // Global modifiers
    global_time_scale: f32,
    velocity_to_level: f32,
    velocity_to_time: f32,

    // Runtime state
    gate: bool,
    stage: Stage,
    stage_time: f32,
    current_level: f32,
    sample_rate: f32,
    velocity_scale: f32,
    env_amount: f32,

    /// Most stage transitions resolved in a single sample since creation.
    #[cfg(test)]
    peak_transitions: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Individual envelope stages.
pub enum Stage {
    Idle,
    Onset,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Looping behavior for the sustain/decay region.
pub enum LoopMode {
    Off,
    Forward,
    PingPong,
}

impl MSEG {
    /// Create the default envelope configuration.
    pub fn new() -> Self {
        Self {
            t_onset: 0.01,
            t_attack: 0.05,
            t_hold: 0.02,
            t_decay: 0.3,
            t_release: 0.1,

            l_onset: 0.1,
            l_peak: 1.0,
            l_sustain: 0.5,
            l_end: 0.0,

            c_onset: 0.0,
            c_attack: -0.5,
            c_decay: 0.3,
            c_release: 0.0,

            loop_mode: LoopMode::Off,
            loop_start_stage: 3,
            loop_end_stage: 4,

            global_time_scale: 1.0,
            velocity_to_level: 1.0,
            velocity_to_time: 0.0,

            gate: false,
            stage: Stage::Idle,
            stage_time: 0.0,
            current_level: 0.0,
            sample_rate: 48000.0,
            velocity_scale: 1.0,
            env_amount: 1.0,
            #[cfg(test)]
            peak_transitions: 0,
        }
    }

    /// Configure as a simple AR envelope for hits.
    pub fn hit_envelope(attack_ms: f32, release_ms: f32) -> Self {
        let mut env = Self::new();
        env.t_onset = 0.0;
        env.l_onset = 0.0;
        env.t_attack = attack_ms / 1000.0;
        env.t_hold = 0.0;
        env.t_decay = 0.0;
        env.t_release = release_ms / 1000.0;
        env.l_sustain = 0.0;
        env.loop_mode = LoopMode::Off;
        env
    }

    /// Configure for continuous friction behavior.
    pub fn scrape_envelope() -> Self {
        let mut env = Self::new();
        env.t_onset = 0.005;
        env.t_attack = 0.1;
        env.t_hold = 0.05;
        env.t_decay = 0.2;
        env.loop_mode = LoopMode::Forward;
        env.loop_start_stage = 3;
        env.loop_end_stage = 4;
        env
    }

    /// Configure for tension-rise and snap behavior.
    pub fn tension_rise_envelope() -> Self {
        let mut env = Self::new();
        env.t_onset = 0.0;
        env.t_attack = 0.5;
        env.t_hold = 0.0;
        env.t_decay = 0.1;
        env.l_peak = 0.9;
        env.l_sustain = 0.0;
        env.c_attack = -0.8;
        env
    }

    /// Set the sample rate used for per-sample timing.
    pub fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
    }

    /// Set stage durations in seconds.
    pub fn set_stage_times(
        &mut self,
        onset: f32,
        attack: f32,
        hold: f32,
        decay: f32,
        release: f32,
    ) {
        self.t_onset = onset.max(0.0);
        self.t_attack = attack.max(0.0);
        self.t_hold = hold.max(0.0);
        self.t_decay = decay.max(0.0);
        self.t_release = release.max(0.0);
    }

    /// Set stage levels in the 0..1 range.
    pub fn set_stage_levels(&mut self, onset: f32, peak: f32, sustain: f32, end: f32) {
        self.l_onset = onset.clamp(0.0, 1.0);
        self.l_peak = peak.clamp(0.0, 1.0);
        self.l_sustain = sustain.clamp(0.0, 1.0);
        self.l_end = end.clamp(0.0, 1.0);
    }

    /// Set curve tensions for onset, attack, decay, and release.
    pub fn set_curves(&mut self, onset: f32, attack: f32, decay: f32, release: f32) {
        self.c_onset = onset.clamp(-1.0, 1.0);
        self.c_attack = attack.clamp(-1.0, 1.0);
        self.c_decay = decay.clamp(-1.0, 1.0);
        self.c_release = release.clamp(-1.0, 1.0);
    }

    /// Set looping behavior and loop bounds.
    /// Whether the loop region is currently engaged. Used by tests to verify
    /// Drone-mode wiring.
    pub fn is_loop_enabled(&self) -> bool {
        !matches!(self.loop_mode, LoopMode::Off)
    }

    pub fn set_loop(&mut self, mode: LoopMode, start_stage: u8, end_stage: u8) {
        self.loop_mode = mode;
        self.loop_start_stage = start_stage.clamp(1, 4);
        self.loop_end_stage = end_stage.clamp(1, 4);
    }

    /// Set how velocity affects level and time.
    pub fn set_velocity_response(&mut self, to_level: f32, to_time: f32) {
        self.velocity_to_level = to_level.clamp(0.0, 1.0);
        self.velocity_to_time = to_time.clamp(0.0, 1.0);
    }

    /// Set the global time scale.
    pub fn set_global_time_scale(&mut self, scale: f32) {
        self.global_time_scale = scale.clamp(0.1, 10.0);
    }

    /// Trigger the envelope with a normalized velocity.
    pub fn trigger(&mut self, velocity: f32) {
        self.gate = true;
        self.stage = Stage::Onset;
        self.stage_time = 0.0;
        self.velocity_scale = 0.5 + velocity * 0.5;
        self.current_level = 0.0;
    }

    /// Enter the release stage if the envelope is active.
    pub fn release(&mut self) {
        self.gate = false;
        if self.stage != Stage::Idle && self.stage != Stage::Release {
            self.stage = Stage::Release;
            self.stage_time = 0.0;
        }
    }

    /// Set the final output amount.
    pub fn set_env_amount(&mut self, amount: f32) {
        self.env_amount = amount.clamp(0.0, 1.0);
    }

    /// Upper bound on stage transitions resolved in one sample. Six stages
    /// plus a loop jump fit with room to spare; a loop region whose stages
    /// all have zero length stops here instead of spinning.
    const MAX_TRANSITIONS: usize = 8;

    /// Process one sample and return the envelope level.
    pub fn process_sample(&mut self) -> f32 {
        let dt = 1.0 / self.sample_rate;
        let time_scale =
            self.global_time_scale * (1.0 - self.velocity_to_time * (1.0 - self.velocity_scale));
        let step = dt * time_scale;

        self.stage_time += step;

        // Resolve finished stages iteratively (this used to recurse into
        // `process_sample`, which never terminated for some loop settings —
        // review C2). Each transition starts the new stage at `step` rather
        // than 0, matching the time the recursive version added on every
        // re-entry, so existing output stays bit-identical.
        let mut transitions = 0;
        while transitions < Self::MAX_TRANSITIONS && self.stage_finished() {
            if self.stage == Stage::Release {
                self.stage = Stage::Idle;
                self.current_level = self.l_end;
                return self.velocity_scaled(self.current_level);
            }
            self.advance_stage();
            self.stage_time = step;
            transitions += 1;
        }
        #[cfg(test)]
        {
            self.peak_transitions = self.peak_transitions.max(transitions);
        }

        let (target_level, stage_duration, curve) = match self.stage {
            Stage::Idle => {
                self.current_level = 0.0;
                return 0.0;
            }
            Stage::Onset => (self.l_onset, self.t_onset, self.c_onset),
            Stage::Attack => (self.l_peak, self.t_attack, self.c_attack),
            Stage::Hold => (self.l_peak, self.t_hold, 0.0),
            Stage::Decay => (self.l_sustain, self.t_decay, self.c_decay),
            Stage::Sustain => (self.l_sustain, 1.0, 0.0),
            Stage::Release => (self.l_end, self.t_release, self.c_release),
        };

        let progress = (self.stage_time / stage_duration).clamp(0.0, 1.0);
        let curved_progress = Self::apply_curve(progress, curve);

        let start_level = match self.stage {
            Stage::Onset => 0.0,
            Stage::Attack => self.l_onset,
            Stage::Hold => self.l_peak,
            Stage::Decay => self.l_peak,
            Stage::Sustain => self.l_sustain,
            Stage::Release => self.l_sustain,
            _ => 0.0,
        };

        self.current_level = start_level + (target_level - start_level) * curved_progress;

        self.velocity_scaled(self.current_level)
    }

    /// Apply velocity as a pure gain: `level · ((1 − vtl) + vtl · velocity_scale)`.
    ///
    /// A zero level stays silent at any velocity. The previous formula,
    /// `0.5 + (level − 0.5)·velocity_scale·vtl + level·(1 − vtl)`, left a
    /// floor under low-velocity notes and stepped to 0 when Release ended
    /// (review C4).
    fn velocity_scaled(&self, level: f32) -> f32 {
        let gain = (1.0 - self.velocity_to_level) + self.velocity_to_level * self.velocity_scale;
        (level * gain).clamp(0.0, 1.0) * self.env_amount
    }

    /// Whether the current stage has run its course and should hand over.
    fn stage_finished(&self) -> bool {
        match self.stage {
            Stage::Idle => false,
            Stage::Onset => self.t_onset <= 0.0 || self.stage_time >= self.t_onset,
            Stage::Attack => self.stage_time >= self.t_attack,
            Stage::Hold => self.stage_time >= self.t_hold,
            Stage::Decay => self.stage_time >= self.t_decay,
            Stage::Sustain => !self.gate,
            Stage::Release => self.stage_time >= self.t_release,
        }
    }

    fn advance_stage(&mut self) {
        self.stage_time = 0.0;
        let next = match self.stage {
            Stage::Onset => Stage::Attack,
            Stage::Attack => Stage::Hold,
            Stage::Hold => Stage::Decay,
            Stage::Decay | Stage::Sustain => {
                if self.gate {
                    Stage::Sustain
                } else {
                    Stage::Release
                }
            }
            Stage::Release | Stage::Idle => Stage::Idle,
        };
        // Reaching the loop end stage with the gate held restarts the loop.
        self.stage = match self.loop_region() {
            Some((start, end)) if self.gate && next == end => start,
            _ => next,
        };
    }

    /// The active loop as `(start, end)` stages, or `None` when looping is off
    /// or the region is empty (start at or after end). Stage indices follow
    /// the host params: 1 Attack, 2 Hold, 3 Decay, 4 Sustain; Onset (0) and
    /// Release (5) are never looped and clamp into that range.
    fn loop_region(&self) -> Option<(Stage, Stage)> {
        let (start, end) = match self.loop_mode {
            LoopMode::Off => return None,
            // PingPong is still partial: it always loops Decay → Sustain.
            LoopMode::PingPong => (3, 4),
            LoopMode::Forward => (
                self.loop_start_stage.clamp(1, 4),
                self.loop_end_stage.clamp(1, 4),
            ),
        };
        (start < end).then(|| (Self::loop_stage(start), Self::loop_stage(end)))
    }

    fn loop_stage(index: u8) -> Stage {
        match index {
            1 => Stage::Attack,
            2 => Stage::Hold,
            3 => Stage::Decay,
            _ => Stage::Sustain,
        }
    }

    /// Apply a curve tension to linear progress.
    fn apply_curve(t: f32, curve: f32) -> f32 {
        if curve.abs() < 0.001 {
            return t;
        }

        if curve < 0.0 {
            let exponent = 1.0 + curve.abs();
            t.powf(exponent)
        } else {
            let exponent = 1.0 + curve;
            1.0 - (1.0 - t).powf(exponent)
        }
    }

    /// Return whether the envelope is active.
    pub fn is_active(&self) -> bool {
        self.stage != Stage::Idle
    }

    /// Return the current stage.
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// Return the current level.
    pub fn current_level(&self) -> f32 {
        self.current_level
    }
}

impl Default for MSEG {
    /// Construct the default envelope.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a over the bit patterns of `held` seconds of gated output followed
    /// by `released` seconds after `release()`, at 48 kHz.
    fn output_hash(mut mseg: MSEG, velocity: f32, held: f32, released: f32) -> u64 {
        let sr = 48_000.0;
        mseg.set_sample_rate(sr);
        mseg.trigger(velocity);
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut feed = |x: f32| {
            for byte in x.to_bits().to_le_bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        };
        for _ in 0..(held * sr) as usize {
            feed(mseg.process_sample());
        }
        mseg.release();
        for _ in 0..(released * sr) as usize {
            feed(mseg.process_sample());
        }
        hash
    }

    fn configured(mode: LoopMode, start: u8, end: u8) -> MSEG {
        let mut mseg = MSEG::new();
        mseg.set_loop(mode, start, end);
        mseg
    }

    /// Golden output for loop Off, the default/Drone Forward 3 → 4 loop,
    /// `scrape_envelope()` and PingPong. First captured from the recursive
    /// implementation before the C2 fix (loop timing must not drift), then
    /// re-captured once for C4, when velocity became a pure gain: below
    /// full velocity the old floor is gone, and at full velocity the only
    /// change is f32 rounding (see `full_velocity_matches_previous_formula`).
    #[test]
    fn loop_output_matches_golden() {
        let cases = [
            ("off", configured(LoopMode::Off, 3, 4), 0.8),
            ("forward 3->4", configured(LoopMode::Forward, 3, 4), 0.8),
            (
                "forward 3->4 v=0.3",
                configured(LoopMode::Forward, 3, 4),
                0.3,
            ),
            ("scrape", MSEG::scrape_envelope(), 1.0),
            ("pingpong", configured(LoopMode::PingPong, 3, 4), 0.8),
        ];
        let golden = [
            0x6545_734c_fe98_dcb4_u64,
            0x589e_fab7_76f4_e923,
            0x97a1_1d94_3312_2db4,
            0x41c9_3e5d_5f8e_a43a,
            0x589e_fab7_76f4_e923,
        ];
        for ((name, mseg, velocity), expected) in cases.into_iter().zip(golden) {
            assert_eq!(
                output_hash(mseg, velocity, 3.0, 1.0),
                expected,
                "{name}: output changed"
            );
        }
    }

    /// Regression for review C2: loop start = Sustain used to recurse forever
    /// in `process_sample()`. Debug builds overflow the stack; optimized
    /// builds turn the tail call into an endless loop that hangs the audio
    /// thread. Run on a small stack with a deadline so either form fails.
    #[test]
    fn loop_start_at_sustain_does_not_recurse() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(move || {
                let mut mseg = configured(LoopMode::Forward, 4, 4);
                mseg.set_sample_rate(48_000.0);
                mseg.trigger(1.0);
                let finite = (0..96_000)
                    .map(|_| mseg.process_sample())
                    .all(|x| x.is_finite());
                let _ = tx.send(finite);
            })
            .expect("spawn test thread");
        let finite = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("envelope did not finish 2 s of audio within 10 s (hang or crash)");
        assert!(finite);
    }

    fn triggered(mode: LoopMode, start: u8, end: u8) -> MSEG {
        let mut mseg = configured(mode, start, end);
        mseg.set_sample_rate(48_000.0);
        mseg.trigger(1.0);
        mseg
    }

    /// Triggers `mseg` at 48 kHz and runs it past onset+attack+hold+decay
    /// (0.38 s at default times) into sustain.
    fn held_into_sustain(mut mseg: MSEG, velocity: f32) -> MSEG {
        mseg.set_sample_rate(48_000.0);
        mseg.trigger(velocity);
        for _ in 0..24_000 {
            mseg.process_sample();
        }
        assert_eq!(mseg.stage(), Stage::Sustain);
        mseg
    }

    /// At full velocity with `velocity_to_level = 1` (every factory preset),
    /// the pre-C4 formula `0.5 + (level − 0.5)·velocity_scale` equals `level`
    /// up to f32 rounding. Pin that the new gain keeps full-velocity output
    /// within 1e-7 of it (measured max difference: 1.5e-8).
    #[test]
    fn full_velocity_matches_previous_formula() {
        let mut mseg = MSEG::scrape_envelope();
        mseg.set_sample_rate(48_000.0);
        mseg.trigger(1.0);
        for i in 0..192_000 {
            if i == 144_000 {
                mseg.release();
            }
            let new = mseg.process_sample();
            if mseg.stage() == Stage::Idle {
                assert_eq!(new, 0.0);
                continue;
            }
            let level = mseg.current_level();
            let previous =
                (0.5 + (level - 0.5) * mseg.velocity_scale).clamp(0.0, 1.0) * mseg.env_amount;
            assert!(
                (new - previous).abs() <= 1e-7,
                "sample {i}: {new} vs previous formula {previous}"
            );
        }
    }

    #[test]
    fn zero_level_is_silence_at_low_velocity() {
        // Regression for review C4: the old velocity formula put a
        // 0.5·(1 − velocity_scale) floor under a zero stage level.
        let mut mseg = MSEG::new();
        mseg.set_stage_levels(0.1, 1.0, 0.0, 0.0);
        let mut mseg = held_into_sustain(mseg, 0.0);
        for _ in 0..4_800 {
            assert_eq!(mseg.process_sample(), 0.0);
        }
    }

    #[test]
    fn velocity_to_level_off_adds_no_offset() {
        for velocity in [0.0, 0.5, 1.0] {
            let mut mseg = MSEG::new();
            mseg.set_velocity_response(0.0, 0.0);
            let mut mseg = held_into_sustain(mseg, velocity);
            assert_eq!(mseg.process_sample(), 0.5, "velocity {velocity}");
        }
    }

    #[test]
    fn velocity_scales_level_multiplicatively() {
        // velocity_scale = 0.5 + v/2, so full level at velocity 0 is half.
        let mut mseg = MSEG::new();
        mseg.set_stage_levels(0.1, 1.0, 1.0, 0.0);
        let mut mseg = held_into_sustain(mseg, 0.0);
        assert_eq!(mseg.process_sample(), 0.5);
    }

    #[test]
    fn release_reaches_idle_without_a_step() {
        let mut mseg = held_into_sustain(MSEG::new(), 0.3);
        mseg.release();
        let mut previous: Option<f32> = None;
        let mut largest_release_step = 0.0_f32;
        let mut final_step = None;
        for _ in 0..48_000 {
            let x = mseg.process_sample();
            if let Some(p) = previous {
                let step = (x - p).abs();
                if mseg.is_active() {
                    largest_release_step = largest_release_step.max(step);
                } else {
                    final_step = Some(step);
                    break;
                }
            }
            previous = Some(x);
        }
        let final_step = final_step.expect("envelope never went idle");
        assert!(
            final_step <= largest_release_step,
            "release ended with a {final_step} step (largest in-release step {largest_release_step})"
        );
    }

    #[test]
    fn custom_loop_end_restarts_before_decay() {
        let mut mseg = triggered(LoopMode::Forward, 1, 3);
        for i in 0..96_000 {
            mseg.process_sample();
            assert!(
                !matches!(mseg.stage(), Stage::Decay | Stage::Sustain),
                "entered {:?} at sample {i} with a 1 -> 3 loop held",
                mseg.stage()
            );
        }
    }

    #[test]
    fn empty_loop_region_holds_sustain() {
        let mut mseg = triggered(LoopMode::Forward, 4, 4);
        // Onset + attack + hold + decay ≈ 0.38 s at default times.
        for _ in 0..24_000 {
            mseg.process_sample();
        }
        assert_eq!(mseg.stage(), Stage::Sustain);
        let level = mseg.process_sample();
        for _ in 0..48_000 {
            assert_eq!(mseg.process_sample(), level, "sustain level drifted");
            assert_eq!(mseg.stage(), Stage::Sustain);
        }
    }

    #[test]
    fn start_after_end_matches_loop_off() {
        assert_eq!(
            output_hash(configured(LoopMode::Forward, 3, 1), 0.8, 3.0, 1.0),
            output_hash(configured(LoopMode::Off, 3, 1), 0.8, 3.0, 1.0),
        );
    }

    #[test]
    fn release_exits_running_loop() {
        let mut mseg = triggered(LoopMode::Forward, 3, 4);
        for _ in 0..48_000 {
            mseg.process_sample();
        }
        mseg.release();
        // Default release is 0.1 s; allow one 512-sample buffer on top.
        for _ in 0..(4_800 + 512) {
            mseg.process_sample();
        }
        assert!(!mseg.is_active(), "still {:?} after release", mseg.stage());
    }

    #[test]
    fn every_loop_configuration_is_bounded() {
        let zero_times = |mut mseg: MSEG| {
            mseg.set_stage_times(0.0, 0.0, 0.0, 0.0, 0.0);
            mseg
        };
        for mode in [LoopMode::Off, LoopMode::Forward, LoopMode::PingPong] {
            for start in 0..=5 {
                for end in 0..=5 {
                    for zero in [false, true] {
                        let mut mseg = configured(mode, start, end);
                        if zero {
                            mseg = zero_times(mseg);
                        }
                        mseg.set_sample_rate(48_000.0);
                        mseg.trigger(1.0);
                        for i in 0..96_000 {
                            if i == 48_000 {
                                mseg.release();
                            }
                            let x = mseg.process_sample();
                            assert!(
                                (0.0..=1.0).contains(&x),
                                "{mode:?} {start}->{end} zero_times={zero}: sample {i} = {x}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn default_and_drone_loops_never_hit_transition_cap() {
        for mut mseg in [
            configured(LoopMode::Off, 3, 4),
            configured(LoopMode::Forward, 3, 4),
            MSEG::scrape_envelope(),
        ] {
            mseg.set_sample_rate(48_000.0);
            mseg.trigger(1.0);
            for i in 0..144_000 {
                if i == 96_000 {
                    mseg.release();
                }
                mseg.process_sample();
            }
            assert!(
                mseg.peak_transitions < MSEG::MAX_TRANSITIONS,
                "hit the transition cap ({} transitions)",
                mseg.peak_transitions
            );
        }
    }

    #[test]
    fn mseg_starts_idle() {
        let mseg = MSEG::new();
        assert!(!mseg.is_active());
        assert_eq!(mseg.stage(), Stage::Idle);
    }

    #[test]
    fn mseg_triggers_to_onset() {
        let mut mseg = MSEG::new();
        mseg.trigger(1.0);
        assert!(mseg.is_active());
        assert_eq!(mseg.stage(), Stage::Onset);
    }

    #[test]
    fn mseg_advances_through_stages() {
        let mut mseg = MSEG::new();
        mseg.set_sample_rate(1000.0);
        mseg.set_stage_times(0.01, 0.01, 0.01, 0.01, 0.01);
        mseg.trigger(1.0);

        let mut stages_seen = vec![];
        for _ in 0..100 {
            stages_seen.push(mseg.stage());
            mseg.process_sample();
        }

        assert!(
            stages_seen.contains(&Stage::Onset),
            "Should see Onset stage"
        );
        assert!(
            stages_seen.contains(&Stage::Attack),
            "Should see Attack stage"
        );
        assert!(stages_seen.contains(&Stage::Hold), "Should see Hold stage");
    }

    #[test]
    fn mseg_reaches_peak() {
        let mut mseg = MSEG::new();
        mseg.set_sample_rate(1000.0);
        mseg.set_stage_times(0.0, 0.01, 0.0, 0.0, 0.01);
        mseg.set_stage_levels(0.0, 1.0, 0.0, 0.0);
        mseg.trigger(1.0);

        let mut max_level: f32 = 0.0;
        for _ in 0..100 {
            let level = mseg.process_sample();
            max_level = max_level.max(level);
        }

        assert!(
            max_level > 0.5,
            "MSEG should reach significant level, got {}",
            max_level
        );
    }

    #[test]
    fn mseg_release_works() {
        let mut mseg = MSEG::new();
        mseg.set_sample_rate(1000.0);
        mseg.set_stage_times(0.0, 0.001, 0.001, 0.0, 0.01);
        mseg.trigger(1.0);

        for _ in 0..5 {
            mseg.process_sample();
        }

        mseg.release();
        assert_eq!(mseg.stage(), Stage::Release);

        for _ in 0..50 {
            mseg.process_sample();
        }
        assert!(!mseg.is_active());
    }

    #[test]
    fn curve_application() {
        assert!((MSEG::apply_curve(0.5, 0.0) - 0.5).abs() < 0.001);

        let convex = MSEG::apply_curve(0.5, -0.5);
        assert!(
            convex < 0.5,
            "Convex curve should be below linear at midpoint"
        );

        let concave = MSEG::apply_curve(0.5, 0.5);
        assert!(
            concave > 0.5,
            "Concave curve should be above linear at midpoint"
        );
    }

    #[test]
    fn velocity_affects_level() {
        let mut mseg_low = MSEG::new();
        let mut mseg_high = MSEG::new();

        mseg_low.set_sample_rate(1000.0);
        mseg_high.set_sample_rate(1000.0);
        mseg_low.set_stage_times(0.0, 0.01, 0.0, 0.0, 0.01);
        mseg_high.set_stage_times(0.0, 0.01, 0.0, 0.0, 0.01);
        mseg_low.set_velocity_response(1.0, 0.0);
        mseg_high.set_velocity_response(1.0, 0.0);

        mseg_low.trigger(0.3);
        mseg_high.trigger(1.0);

        let max_low: f32 = (0..100)
            .map(|_| mseg_low.process_sample())
            .fold(0.0, f32::max);
        let max_high: f32 = (0..100)
            .map(|_| mseg_high.process_sample())
            .fold(0.0, f32::max);

        // Velocity is a pure gain: velocity_scale = 0.5 + v/2, so the peak at
        // velocity 0.3 is 0.65 of the peak at 1.0 (the pre-C4 formula gave
        // 0.825 by adding a floor).
        let ratio = max_low / max_high;
        assert!(
            (ratio - 0.65).abs() < 1e-3,
            "peak ratio should be 0.65, got {ratio} (high={max_high}, low={max_low})"
        );
    }
}

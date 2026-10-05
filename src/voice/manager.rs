//! Fixed-size voice pool and scheduling logic for Corrosion.
//!
//! `VoiceManager` owns the polyphonic voice array, handles note-on
//! and note-off dispatch, and performs quietest-voice stealing when
//! all slots are occupied. `src/lib.rs` calls into this module from
//! the host audio callback.

use super::{Voice, VoiceControls};

/// Maximum number of simultaneously active voices.
pub const MAX_VOICES: usize = 8;

/// Fixed-size polyphonic voice pool with allocation-free scheduling.
pub struct VoiceManager {
    voices: [Voice; MAX_VOICES],
    frame_counter: u64,
    /// Channel-wide MIDI expression state. Stored here so a note-on after a
    /// pitch-bend / aftertouch / CC1 event still picks up the active channel
    /// values, not just notes that were already playing when the event arrived.
    pitch_bend_semitones: f32,
    channel_pressure: f32,
    mod_wheel: f32,
}

/// Map a voice peak-hold value to a steal priority score.
///
/// Non-finite values are treated as the quietest possible voices so a
/// corrupted or unstable voice is preferentially reclaimed instead of
/// panicking during comparison.
fn comparable_peak_hold(peak_hold: f32) -> f32 {
    if peak_hold.is_finite() {
        peak_hold
    } else {
        f32::NEG_INFINITY
    }
}

impl VoiceManager {
    /// Create a voice manager with all slots initialized.
    ///
    /// # Returns
    ///
    /// A manager ready to receive note events.
    pub fn new() -> Self {
        Self {
            voices: [
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
                Voice::new(),
            ],
            frame_counter: 0,
            pitch_bend_semitones: 0.0,
            channel_pressure: 0.0,
            mod_wheel: 0.0,
        }
    }

    /// Silence every voice and clear channel expression in place, leaving
    /// the manager as `new()` would for playback purposes but without
    /// allocating, so the host may call it from the audio thread.
    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.reset();
        }
        self.frame_counter = 0;
        self.pitch_bend_semitones = 0.0;
        self.channel_pressure = 0.0;
        self.mod_wheel = 0.0;
    }

    /// Trigger a note using default voice controls.
    ///
    /// This is the simple wrapper used when the caller does not need
    /// per-note control overrides.
    ///
    /// # Arguments
    ///
    /// * `note` - MIDI note number for the new voice.
    /// * `velocity` - MIDI velocity used to scale excitation energy.
    /// * `profile_id` - Resonator profile to load for the voice.
    /// * `size` - Material size parameter forwarded to the voice.
    /// * `rust` - Material rust parameter forwarded to the voice.
    /// * `damage` - Material damage parameter forwarded to the voice.
    /// * `exciter_type` - Integer selector for the exciter model.
    // Kept positional to match the voice API and avoid a call-site refactor.
    #[allow(clippy::too_many_arguments)]
    pub fn note_on(
        &mut self,
        note: u8,
        velocity: f32,
        profile_id: crate::dsp::ModalProfileId,
        size: f32,
        rust: f32,
        damage: f32,
        exciter_type: i32,
    ) {
        self.note_on_with_controls(
            note,
            velocity,
            profile_id,
            size,
            rust,
            damage,
            exciter_type,
            VoiceControls::default(),
        );
    }

    /// Trigger a note with explicit voice controls.
    ///
    /// # Arguments
    ///
    /// * `note` - MIDI note number for the new voice.
    /// * `velocity` - MIDI velocity used to scale excitation energy.
    /// * `profile_id` - Resonator profile to load for the voice.
    /// * `size` - Material size parameter forwarded to the voice.
    /// * `rust` - Material rust parameter forwarded to the voice.
    /// * `damage` - Material damage parameter forwarded to the voice.
    /// * `exciter_type` - Integer selector for the exciter model.
    /// * `controls` - Complete per-voice control snapshot.
    // Mirrors the per-note host control snapshot; changing shape is out of scope here.
    #[allow(clippy::too_many_arguments)]
    pub fn note_on_with_controls(
        &mut self,
        note: u8,
        velocity: f32,
        profile_id: crate::dsp::ModalProfileId,
        size: f32,
        rust: f32,
        damage: f32,
        exciter_type: i32,
        controls: VoiceControls,
    ) {
        // First try: find an inactive voice; this keeps the common case
        // deterministic and avoids unnecessary stealing.
        if let Some(idx) = self.voices.iter().position(|v| !v.is_rendering()) {
            self.apply_channel_expression_to(idx);
            self.voices[idx].note_on_with_controls(
                note,
                velocity,
                profile_id,
                self.frame_counter,
                size,
                rust,
                damage,
                exciter_type,
                controls,
            );
            return;
        }

        // Steal the quietest voice when all slots are occupied. Peak hold is
        // the primary score; older voices lose ties so long tails are preferred
        // over recently triggered notes.
        let steal_idx = self
            .voices
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let peak_cmp = comparable_peak_hold(a.peak_hold())
                    .partial_cmp(&comparable_peak_hold(b.peak_hold()))
                    .unwrap_or(std::cmp::Ordering::Equal);
                if peak_cmp != std::cmp::Ordering::Equal {
                    return peak_cmp;
                }
                let age_cmp = a.start_frame().cmp(&b.start_frame());
                if age_cmp != std::cmp::Ordering::Equal {
                    return age_cmp;
                }
                std::cmp::Ordering::Equal
            })
            .map(|(idx, _)| idx);

        if let Some(idx) = steal_idx {
            self.apply_channel_expression_to(idx);
            self.voices[idx].note_on_with_controls(
                note,
                velocity,
                profile_id,
                self.frame_counter,
                size,
                rust,
                damage,
                exciter_type,
                controls,
            );
        }
    }

    /// Seed a freshly-claimed slot with the current channel expression state
    /// so a pitch-bent / pressed / mod-wheeled context isn't lost when a new
    /// note starts. Channel pressure and mod wheel are persistent on the
    /// channel; the per-note poly pressure resets inside `Voice::note_on`.
    fn apply_channel_expression_to(&mut self, idx: usize) {
        let voice = &mut self.voices[idx];
        voice.set_channel_pressure(self.channel_pressure);
        voice.set_mod_wheel(self.mod_wheel);
        // The new resonator that note_on builds will pick up the bend below;
        // tracking the semitone value here lets the voice apply it during
        // note_on_with_controls.
        voice.set_pitch_bend_semitones(self.pitch_bend_semitones);
    }

    /// Apply a channel pitch bend (semitones) to every voice on this channel.
    ///
    /// Hosts deliver pitch bend as a normalized `0.0..=1.0` value; convert to
    /// semitones with `voice::PITCH_BEND_RANGE_SEMITONES` before calling.
    pub fn set_pitch_bend_semitones(&mut self, semitones: f32) {
        self.pitch_bend_semitones = semitones;
        for voice in &mut self.voices {
            if voice.is_rendering() {
                voice.set_pitch_bend_semitones(semitones);
            }
        }
    }

    /// Apply channel aftertouch (`MidiChannelPressure`) to every voice.
    pub fn set_channel_pressure(&mut self, pressure: f32) {
        self.channel_pressure = pressure.clamp(0.0, 1.0);
        for voice in &mut self.voices {
            if voice.is_rendering() {
                voice.set_channel_pressure(self.channel_pressure);
            }
        }
    }

    /// Apply CC1 mod wheel to every voice on the channel.
    pub fn set_mod_wheel(&mut self, value: f32) {
        self.mod_wheel = value.clamp(0.0, 1.0);
        for voice in &mut self.voices {
            if voice.is_rendering() {
                voice.set_mod_wheel(self.mod_wheel);
            }
        }
    }

    /// Test-only accessor for the underlying voice pool. Crate-internal so
    /// the lib-level integration tests can inspect voice state without
    /// widening the public surface.
    #[cfg(test)]
    pub(crate) fn voices_for_test(&self) -> &[Voice; MAX_VOICES] {
        &self.voices
    }

    /// Apply polyphonic aftertouch to the voice currently playing `note`.
    ///
    /// Silently no-op if no voice is playing that note — matches host
    /// semantics where poly pressure for a released note is meaningless.
    pub fn set_poly_pressure(&mut self, note: u8, pressure: f32) {
        for voice in &mut self.voices {
            if voice.is_rendering() && voice.note() == note {
                voice.set_poly_pressure(pressure);
                break;
            }
        }
    }

    /// Apply held-note automation to every active voice.
    ///
    /// Held-note (still-active) voices receive new damping/brightness/strike-
    /// position values once per buffer; tail voices keep their note-on snapshot
    /// so they decay with the timbre they were captured with.
    // Mirrors the per-buffer host parameter shape; collapsing into a struct
    // would be a broader refactor.
    #[allow(clippy::too_many_arguments)]
    pub fn update_live_controls(
        &mut self,
        damping: f32,
        brightness: f32,
        strike_position: f32,
        coupling_stiffness: f32,
        position_wander: f32,
        position_envelope: f32,
        fundamental_anchor: f32,
        sample_rate: u32,
    ) {
        for voice in &mut self.voices {
            if voice.is_active() {
                voice.update_live_controls(
                    damping,
                    brightness,
                    strike_position,
                    coupling_stiffness,
                    position_wander,
                    position_envelope,
                    fundamental_anchor,
                    sample_rate,
                );
            }
        }
    }

    pub fn note_off(&mut self, note: u8) {
        if let Some(idx) = self
            .voices
            .iter()
            .position(|v| v.is_active() && v.note() == note)
        {
            self.voices[idx].note_off();
        }
    }

    /// Render the current mono sample for the whole voice pool.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Current host sample rate in hertz.
    ///
    /// # Returns
    ///
    /// The mixed mono output sample.
    pub fn process_sample(&mut self, sample_rate: u32) -> f32 {
        // Advance the shared frame clock so note age can participate in
        // tie-breaking during voice stealing.
        self.frame_counter += 1;
        let mut sum = 0.0f32;
        for voice in &mut self.voices {
            if voice.is_rendering() {
                sum += voice.process_sample(sample_rate);
            }
        }
        sum
    }

    /// Render the current stereo sample pair for the whole voice pool.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Current host sample rate in hertz.
    /// * `width` - Stereo width forwarded to each voice.
    ///
    /// # Returns
    ///
    /// The mixed `(left, right)` output samples.
    pub fn process_sample_stereo(&mut self, sample_rate: u32, width: f32) -> (f32, f32) {
        // Advance the shared frame clock so note age can participate in
        // tie-breaking during voice stealing.
        self.frame_counter += 1;
        let mut left_sum = 0.0f32;
        let mut right_sum = 0.0f32;
        for voice in &mut self.voices {
            if voice.is_rendering() {
                let (left, right) = voice.process_sample_stereo(sample_rate, width);
                left_sum += left;
                right_sum += right;
            }
        }
        (left_sum, right_sum)
    }
}

impl Default for VoiceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{comparable_peak_hold, VoiceManager, MAX_VOICES};
    use crate::voice::Voice;

    #[test]
    fn single_voice_is_not_downmixed_by_empty_slots() {
        let mut manager = VoiceManager::new();
        let mut direct_voice = Voice::new();

        manager.note_on(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );
        direct_voice.note_on(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            0,
            1.0,
            0.0,
            0.0,
            2,
        );

        let sample_rate = 48_000u32;
        let mut manager_peak = 0.0f32;
        let mut direct_peak = 0.0f32;
        for _ in 0..4096 {
            manager_peak = manager_peak.max(manager.process_sample(sample_rate).abs());
            direct_peak = direct_peak.max(direct_voice.process_sample(sample_rate).abs());
        }

        assert!(manager_peak > direct_peak * 0.95, "single voice should not be divided by empty slots: manager={manager_peak} direct={direct_peak}");
    }

    #[test]
    fn eight_simultaneous_notes_audible() {
        let mut manager = VoiceManager::new();
        let notes = [48u8, 51, 55, 59, 62, 65, 69, 72];
        for &note in &notes {
            manager.note_on(
                note,
                100.0,
                crate::dsp::ModalProfileId::Pipe,
                1.0,
                0.0,
                0.0,
                1,
            );
        }

        let sample_rate = 48_000u32;
        let mut peak = 0.0f32;
        for _ in 0..48_000 {
            let sample = manager.process_sample(sample_rate);
            peak = peak.max(sample.abs());
            assert!(sample.is_finite(), "non-finite output");
        }
        assert!(
            peak > 0.01,
            "8 simultaneous notes should be audible, peak={peak}"
        );
    }

    #[test]
    fn ninth_note_does_not_crash() {
        let mut manager = VoiceManager::new();
        for note in 0..12u8 {
            manager.note_on(
                48 + note,
                100.0,
                crate::dsp::ModalProfileId::Pipe,
                1.0,
                0.0,
                0.0,
                2,
            );
        }
        manager.note_on(
            72,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );
        let sample_rate = 48_000u32;
        let sample = manager.process_sample(sample_rate);
        assert!(sample.is_finite());
    }

    #[test]
    fn note_off_deactivates_correct_voice() {
        let mut manager = VoiceManager::new();
        manager.note_on(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );
        manager.note_on(
            64,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );
        manager.note_off(60);

        let active_count = manager.voices.iter().filter(|v| v.is_active()).count();
        assert_eq!(active_count, 1, "only one voice should remain active");
    }

    #[test]
    fn note_off_tail_still_renders_after_slot_becomes_inactive() {
        let mut manager = VoiceManager::new();
        manager.note_on(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );
        let sample_rate = 48_000u32;

        for _ in 0..256 {
            manager.process_sample(sample_rate);
        }

        manager.note_off(60);
        assert_eq!(manager.voices.iter().filter(|v| v.is_active()).count(), 0);

        let tail_energy: f32 = (0..4096)
            .map(|_| manager.process_sample(sample_rate).abs())
            .sum();
        assert!(tail_energy > 0.01, "note-off tail should still render");
    }

    #[test]
    fn new_note_uses_unused_slot_before_released_tail() {
        let mut manager = VoiceManager::new();
        manager.note_on(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );

        let sample_rate = 48_000u32;
        for _ in 0..256 {
            manager.process_sample(sample_rate);
        }

        manager.note_off(60);
        manager.note_on(
            67,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );

        let rendering_notes: Vec<u8> = manager
            .voices
            .iter()
            .filter(|voice| voice.is_rendering())
            .map(|voice| voice.note())
            .collect();

        assert!(rendering_notes.contains(&60));
        assert!(rendering_notes.contains(&67));
    }

    #[test]
    fn steals_quietest_voice_when_all_active() {
        let mut manager = VoiceManager::new();
        // Fill all 8 slots
        for note in 0..MAX_VOICES as u8 {
            manager.note_on(
                48 + note,
                100.0,
                crate::dsp::ModalProfileId::Pipe,
                1.0,
                0.0,
                0.0,
                2,
            );
        }
        // Render enough frames for most voices to decay below their initial peak
        let sample_rate = 48_000u32;
        for _ in 0..48_000 {
            manager.process_sample(sample_rate);
        }
        // Record which notes were active before the 9th note
        let active_notes_before: Vec<u8> = manager
            .voices
            .iter()
            .filter(|v| v.is_active())
            .map(|v| v.note())
            .collect();

        // Send a 9th note — should steal the quietest (most decayed) voice
        manager.note_on(
            80,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );

        // The new note should be present in the active voices
        let has_new_note = manager
            .voices
            .iter()
            .any(|v| v.is_active() && v.note() == 80);
        assert!(has_new_note, "9th note should have stolen a voice slot");

        // Still only 8 active voices
        let active_count = manager.voices.iter().filter(|v| v.is_active()).count();
        assert_eq!(active_count, 8, "should still have exactly 8 active voices");

        // At least one original note should have been stolen
        let active_notes_after: Vec<u8> = manager
            .voices
            .iter()
            .filter(|v| v.is_active())
            .map(|v| v.note())
            .collect();
        let stolen: Vec<u8> = active_notes_before
            .into_iter()
            .filter(|n| !active_notes_after.contains(n))
            .collect();
        assert!(
            !stolen.is_empty(),
            "at least one original voice should have been stolen"
        );
    }

    #[test]
    fn non_finite_peak_hold_is_preferred_for_stealing() {
        let finite_peak = 0.25_f32;
        let non_finite_peak = f32::NAN;

        assert!(comparable_peak_hold(non_finite_peak) < comparable_peak_hold(finite_peak));
    }

    #[test]
    fn channel_pitch_bend_reaches_all_active_voices() {
        // Two notes are playing. A pitch bend event should retune both
        // voices' resonators, not just one.
        let mut manager = VoiceManager::new();
        manager.note_on(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );
        manager.note_on(
            64,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );

        let baseline_freqs: Vec<f32> = manager
            .voices
            .iter()
            .filter(|v| v.is_rendering())
            .map(|v| v.resonator.modes[0].spec.frequency_hz)
            .collect();

        manager.set_pitch_bend_semitones(12.0);

        let bent_freqs: Vec<f32> = manager
            .voices
            .iter()
            .filter(|v| v.is_rendering())
            .map(|v| v.resonator.modes[0].spec.frequency_hz)
            .collect();

        assert_eq!(baseline_freqs.len(), 2);
        assert_eq!(bent_freqs.len(), 2);
        for (b, post) in baseline_freqs.iter().zip(bent_freqs.iter()) {
            assert!(
                (post - b * 2.0).abs() < b * 0.01,
                "octave bend should reach every voice: base={b}, post={post}"
            );
        }
    }

    #[test]
    fn pitch_bend_persists_for_new_notes_on_channel() {
        // A pitch bend held on the channel must apply to notes triggered after
        // the bend event (mirrors how a DAW automates pitch wheel during a run).
        let mut manager = VoiceManager::new();
        manager.set_pitch_bend_semitones(7.0);
        manager.note_on(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );

        let voice = manager
            .voices
            .iter()
            .find(|v| v.is_rendering())
            .expect("a voice should be rendering");
        let unbent = crate::voice::midi_to_hz(60);
        let actual = voice.resonator.modes[0].spec.frequency_hz;
        // First mode is roughly the fundamental; bend factor ≈ 1.498 for +7st.
        assert!(
            actual > unbent * 1.4,
            "new notes should inherit the channel bend: base={unbent}, actual={actual}"
        );
    }

    #[test]
    fn poly_pressure_targets_only_matching_note() {
        let mut manager = VoiceManager::new();
        manager.note_on(
            60,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );
        manager.note_on(
            64,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );

        manager.set_poly_pressure(60, 1.0);

        let target = manager
            .voices
            .iter()
            .find(|v| v.is_rendering() && v.note() == 60)
            .expect("note 60 should be playing");
        let other = manager
            .voices
            .iter()
            .find(|v| v.is_rendering() && v.note() == 64)
            .expect("note 64 should be playing");

        assert!(
            target.expression_gain() > other.expression_gain() + 0.1,
            "poly pressure should only reach the matching note"
        );
    }
}

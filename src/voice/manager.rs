//! Fixed-size voice pool and scheduling logic for Corrosion.
//!
//! `VoiceManager` owns the polyphonic voice array and handles note-on and
//! note-off dispatch. When every slot is sounding it steals a released tail
//! first, then a held note at least 100 ms old, and only then the oldest
//! held note (see `steal_index`). `src/lib.rs` calls into this module from
//! the host audio callback.

use super::{Voice, VoiceControls};

/// Maximum number of simultaneously active voices.
pub const MAX_VOICES: usize = 16;

/// Fixed-size polyphonic voice pool with allocation-free scheduling.
pub struct VoiceManager {
    voices: [Voice; MAX_VOICES],
    frame_counter: u64,
    /// Sample rate of the most recent render call, used to express the
    /// steal-protection window in time rather than samples.
    sample_rate: u32,
    /// Channel-wide MIDI expression state. Stored here so a note-on after a
    /// pitch-bend / aftertouch / CC1 event still picks up the active channel
    /// values, not just notes that were already playing when the event arrived.
    pitch_bend_semitones: f32,
    channel_pressure: f32,
    mod_wheel: f32,
    /// Sustain pedal (CC64) down: key-ups mark voices pedal-held instead of
    /// releasing them (`m13-sustain-pedal-and-panic`).
    sustain_down: bool,
}

/// A held note younger than this is only stolen when every held note is.
/// Covers a slow (friction) attack, with margin.
const STEAL_PROTECTION_SECONDS: f32 = 0.1;

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

/// How the host addressed a note event (`m12-per-note-expression`): its
/// voice ID, when the host gives one (CLAP note IDs, VST3 note IDs), and its
/// MIDI channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoteAddress {
    pub voice_id: Option<i32>,
    pub channel: u8,
}

impl VoiceManager {
    /// Create a voice manager with all slots initialized.
    ///
    /// # Returns
    ///
    /// A manager ready to receive note events.
    pub fn new() -> Self {
        Self {
            voices: std::array::from_fn(|_| Voice::new()),
            frame_counter: 0,
            sample_rate: 48_000,
            pitch_bend_semitones: 0.0,
            channel_pressure: 0.0,
            mod_wheel: 0.0,
            sustain_down: false,
        }
    }

    /// MIDI notes of the held (not yet released) voices, in slot order.
    /// Allocation-free.
    pub fn active_notes(&self) -> impl Iterator<Item = u8> + '_ {
        self.voices
            .iter()
            .filter(|voice| voice.is_active())
            .map(|voice| voice.note())
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
        self.sustain_down = false;
    }

    /// Silence every voice in place, as `reset` does, but keep the channel
    /// state (pedal, mod wheel, bend, pressure): All Sound Off mutes sound,
    /// it doesn't move controllers (`m13-sustain-pedal-and-panic`).
    pub fn silence(&mut self) {
        for voice in &mut self.voices {
            voice.reset();
        }
        self.frame_counter = 0;
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
    /// Takes a silent slot when one exists; otherwise steals one (released
    /// tails first, then held notes past the protection window, then the
    /// oldest held note — see `steal_index`). Allocation-free.
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
        self.note_on_addressed(
            NoteAddress::default(),
            note,
            velocity,
            profile_id,
            size,
            rust,
            damage,
            exciter_type,
            controls,
        );
    }

    /// `note_on_with_controls` for a note the host addressed: the voice
    /// remembers `address`, so later note-offs, chokes, pressure and tuning
    /// can find it by voice ID (`m12-per-note-expression`).
    #[allow(clippy::too_many_arguments)]
    pub fn note_on_addressed(
        &mut self,
        address: NoteAddress,
        note: u8,
        velocity: f32,
        profile_id: crate::dsp::ModalProfileId,
        size: f32,
        rust: f32,
        damage: f32,
        exciter_type: i32,
        controls: VoiceControls,
    ) {
        self.start_voice(
            address,
            note,
            note,
            velocity,
            profile_id,
            size,
            rust,
            damage,
            exciter_type,
            controls,
        );
    }

    /// Start a kit pad on `key`, sounding at `pitch_note`, with output gain
    /// `gain` (`per-key-kits`). The voice answers note-offs and expression
    /// on `key` and ignores live control updates.
    #[allow(clippy::too_many_arguments)]
    pub fn note_on_pad(
        &mut self,
        address: NoteAddress,
        key: u8,
        pitch_note: u8,
        velocity: f32,
        profile_id: crate::dsp::ModalProfileId,
        size: f32,
        rust: f32,
        damage: f32,
        exciter_type: i32,
        controls: VoiceControls,
        gain: f32,
    ) {
        if let Some(idx) = self.start_voice(
            address,
            key,
            pitch_note,
            velocity,
            profile_id,
            size,
            rust,
            damage,
            exciter_type,
            controls,
        ) {
            self.voices[idx].mark_pad(key, gain);
        }
    }

    /// Start a voice for `key` sounding at `pitch_note`; returns its slot.
    #[allow(clippy::too_many_arguments)]
    fn start_voice(
        &mut self,
        address: NoteAddress,
        key: u8,
        pitch_note: u8,
        velocity: f32,
        profile_id: crate::dsp::ModalProfileId,
        size: f32,
        rust: f32,
        damage: f32,
        exciter_type: i32,
        controls: VoiceControls,
    ) -> Option<usize> {
        // Re-striking a note the pedal is holding releases the older
        // instance, so repeated notes under the pedal don't pile up.
        for voice in &mut self.voices {
            if voice.is_pedal_held() && voice.note() == key {
                voice.note_off();
            }
        }

        // First try: find an inactive voice; this keeps the common case
        // deterministic and avoids unnecessary stealing.
        // Otherwise every slot is sounding: steal one (see `steal_index`).
        let slot = self
            .voices
            .iter()
            .position(|v| !v.is_rendering())
            .or_else(|| self.steal_index());
        if let Some(idx) = slot {
            self.apply_channel_expression_to(idx);
            self.voices[idx].note_on_with_controls(
                pitch_note,
                velocity,
                profile_id,
                self.frame_counter,
                size,
                rust,
                damage,
                exciter_type,
                controls,
            );
            self.voices[idx].set_address(address.voice_id, address.channel);
        }
        slot
    }

    /// The voice the host's voice ID names, among voices that pass `filter`
    /// (`m12-per-note-expression`). `None` when the event has no ID or no
    /// such voice exists, so the caller falls back to key and channel.
    ///
    /// The plugin sends no voice-terminated event, so a host may reuse an ID
    /// while the old note's tail still sounds. Among voices with the same
    /// ID, a held note wins over a tail, and the newest wins after that.
    fn voice_with_id(
        &self,
        voice_id: Option<i32>,
        filter: impl Fn(&Voice) -> bool,
    ) -> Option<usize> {
        let id = voice_id?;
        self.voices
            .iter()
            .enumerate()
            .filter(|(_, v)| filter(v) && v.voice_id() == Some(id))
            .max_by_key(|(_, v)| (v.is_active(), v.start_frame()))
            .map(|(idx, _)| idx)
    }

    /// Choose the voice to steal when every slot is sounding.
    ///
    /// Voices fall into three tiers, lowest stolen first:
    /// 0. released tails;
    /// 1. notes the sustain pedal holds after their key-up;
    /// 2. held keys at least [`STEAL_PROTECTION_SECONDS`] old;
    /// 3. younger held keys.
    ///
    /// Tiers 0 to 2 give up the quietest voice (`peak_hold`, NaN counting
    /// as quietest), with the oldest losing ties. Tier 3 gives up the oldest:
    /// a note that just started hasn't reached its peak yet, so its level
    /// says nothing about how audible it will be.
    fn steal_index(&self) -> Option<usize> {
        let protect_frames = (STEAL_PROTECTION_SECONDS * self.sample_rate as f32) as u64;
        let tier = |voice: &Voice| {
            if !voice.is_active() {
                0
            } else if voice.is_pedal_held() {
                1
            } else if self.frame_counter.saturating_sub(voice.start_frame()) >= protect_frames {
                2
            } else {
                3
            }
        };
        self.voices
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let (tier_a, tier_b) = (tier(a), tier(b));
                tier_a.cmp(&tier_b).then_with(|| {
                    let by_level = if tier_a == 3 {
                        std::cmp::Ordering::Equal
                    } else {
                        comparable_peak_hold(a.peak_hold())
                            .partial_cmp(&comparable_peak_hold(b.peak_hold()))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    };
                    by_level.then_with(|| a.start_frame().cmp(&b.start_frame()))
                })
            })
            .map(|(idx, _)| idx)
    }

    /// MIDI notes of every sounding voice, held or tailing, in slot order.
    /// Allocation-free.
    pub fn sounding_notes(&self) -> impl Iterator<Item = u8> + '_ {
        self.voices
            .iter()
            .filter(|voice| voice.is_rendering())
            .map(|voice| voice.note())
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

    /// Test-only: frames since the last reset or silence (note seeds and
    /// steal protection derive from it).
    #[cfg(test)]
    pub(crate) fn frame_counter_for_test(&self) -> u64 {
        self.frame_counter
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

    /// Poly pressure as the host addressed it: the voice with its voice ID,
    /// or else the first sounding voice on `note` and the address's channel
    /// (`m12-per-note-expression`).
    pub fn set_poly_pressure_addressed(&mut self, address: NoteAddress, note: u8, pressure: f32) {
        let idx = self
            .voice_with_id(address.voice_id, Voice::is_rendering)
            .or_else(|| {
                self.voices.iter().position(|v| {
                    v.is_rendering() && v.note() == note && v.channel() == address.channel
                })
            });
        if let Some(idx) = idx {
            self.voices[idx].set_poly_pressure(pressure);
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
        position_wander_rate: f32,
        fundamental_anchor: f32,
        exciter_pressure: f32,
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
                    position_wander_rate,
                    fundamental_anchor,
                    exciter_pressure,
                    sample_rate,
                );
            }
        }
    }

    /// The host sample rate the voices were last rated for.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Rate every voice, idle ones included, for the host's sample rate, so
    /// a note-on builds its bank once at that rate and its exciter starts
    /// at it (`m03-m20-start-at-host-rate-and-report-latency`). Cheap when
    /// the rate is unchanged; allocation-free.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        for voice in &mut self.voices {
            voice.set_rate(sample_rate as f32);
        }
    }

    /// Release the oldest held voice playing `note`, so a pitch started
    /// twice releases first-in, first-out.
    pub fn note_off(&mut self, note: u8) {
        if let Some(idx) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_active() && !v.is_pedal_held() && v.note() == note)
            .min_by_key(|(_, v)| v.start_frame())
            .map(|(idx, _)| idx)
        {
            self.release_key(idx);
        }
    }

    /// A note-off as the host addressed it: the held voice with its voice
    /// ID, or else the oldest held voice on `note` and the address's channel
    /// (`m12-per-note-expression`).
    pub fn note_off_addressed(&mut self, address: NoteAddress, note: u8) {
        let held = |v: &Voice| v.is_active() && !v.is_pedal_held();
        let idx = self.voice_with_id(address.voice_id, held).or_else(|| {
            self.voices
                .iter()
                .enumerate()
                .filter(|(_, v)| held(v) && v.note() == note && v.channel() == address.channel)
                .min_by_key(|(_, v)| v.start_frame())
                .map(|(idx, _)| idx)
        });
        if let Some(idx) = idx {
            self.release_key(idx);
        }
    }

    /// Apply `action` to the sounding voice with the address's voice ID, or
    /// else to every sounding voice on `note` and the address's channel.
    fn for_addressed_voices(
        &mut self,
        address: NoteAddress,
        note: u8,
        action: impl Fn(&mut Voice),
    ) {
        if let Some(idx) = self.voice_with_id(address.voice_id, Voice::is_rendering) {
            action(&mut self.voices[idx]);
            return;
        }
        for voice in &mut self.voices {
            if voice.is_rendering() && voice.note() == note && voice.channel() == address.channel {
                action(voice);
            }
        }
    }

    /// Per-note tuning (`PolyTuning`), in semitones on top of the channel
    /// bend, for the addressed note.
    pub fn set_note_tuning(&mut self, address: NoteAddress, note: u8, semitones: f32) {
        self.for_addressed_voices(address, note, |voice| {
            voice.set_note_tuning_semitones(semitones)
        });
    }

    /// Host choke: the addressed note fades to silence in 2 ms and its voice
    /// is freed.
    pub fn choke(&mut self, address: NoteAddress, note: u8) {
        self.for_addressed_voices(address, note, Voice::choke);
    }

    /// A key-up on voice `idx`: released, or held by a down sustain pedal.
    fn release_key(&mut self, idx: usize) {
        if self.sustain_down {
            self.voices[idx].hold_by_pedal();
        } else {
            self.voices[idx].note_off();
        }
    }

    /// Sustain pedal (CC64) down or up. Lifting it releases every note the
    /// pedal was holding; notes whose keys are still down keep sounding.
    pub fn set_sustain(&mut self, down: bool) {
        self.sustain_down = down;
        if !down {
            for voice in &mut self.voices {
                if voice.is_pedal_held() {
                    voice.note_off();
                }
            }
        }
    }

    /// Whether the sustain pedal is down.
    pub fn sustain_down(&self) -> bool {
        self.sustain_down
    }

    /// All Notes Off (CC123): every held key is released as if lifted, so a
    /// down pedal still holds them (MIDI 1.0 behaviour).
    pub fn all_notes_off(&mut self) {
        for idx in 0..MAX_VOICES {
            let voice = &self.voices[idx];
            if voice.is_active() && !voice.is_pedal_held() {
                self.release_key(idx);
            }
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
        // Advance the shared frame clock so note age can drive stealing.
        self.frame_counter += 1;
        self.sample_rate = sample_rate;
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
        // Advance the shared frame clock so note age can drive stealing.
        self.frame_counter += 1;
        self.sample_rate = sample_rate;
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
    fn sixteen_simultaneous_notes_all_sound() {
        assert_eq!(MAX_VOICES, 16);
        let mut manager = VoiceManager::new();
        for i in 0..MAX_VOICES as u8 {
            manager.note_on(
                36 + 3 * i,
                100.0,
                crate::dsp::ModalProfileId::Pipe,
                1.0,
                0.0,
                0.0,
                1,
            );
        }
        let sample_rate = 48_000u32;
        let mut peaks = [0.0_f32; MAX_VOICES];
        for _ in 0..24_000 {
            let sample = manager.process_sample(sample_rate);
            assert!(sample.is_finite(), "non-finite output");
            for (peak, voice) in peaks.iter_mut().zip(&manager.voices) {
                *peak = peak.max(voice.peak_hold());
            }
        }
        assert_eq!(sounding(&manager).len(), MAX_VOICES);
        for (i, peak) in peaks.iter().enumerate() {
            assert!(*peak > 0.01, "voice {i} is silent (peak {peak})");
        }
    }

    #[test]
    fn seventeenth_note_steals_and_keeps_sixteen_sounding() {
        let mut manager = VoiceManager::new();
        for i in 0..MAX_VOICES as u8 {
            manager.note_on(
                40 + i,
                100.0,
                crate::dsp::ModalProfileId::Pipe,
                1.0,
                0.0,
                0.0,
                2,
            );
        }
        run_seconds(&mut manager, 48_000, 0.01);
        manager.note_on(
            100,
            100.0,
            crate::dsp::ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            2,
        );
        let sample = manager.process_sample(48_000);
        assert!(sample.is_finite());
        let notes = sounding(&manager);
        assert!(notes.contains(&100), "the seventeenth note should sound");
        assert_eq!(notes.len(), MAX_VOICES);
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
        // Fill every slot
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
        // Record which notes were active before the extra note, and which
        // is quietest (all are held and well past the protection window).
        let active_notes_before: Vec<u8> = manager
            .voices
            .iter()
            .filter(|v| v.is_active())
            .map(|v| v.note())
            .collect();
        let quietest = manager
            .voices
            .iter()
            .min_by(|a, b| a.peak_hold().total_cmp(&b.peak_hold()))
            .map(|v| v.note())
            .unwrap();

        // Send one note more than the pool holds — should steal the quietest (most decayed) voice
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
        assert!(
            has_new_note,
            "the extra note should have stolen a voice slot"
        );

        // Still a full pool of active voices
        let active_count = manager.voices.iter().filter(|v| v.is_active()).count();
        assert_eq!(active_count, MAX_VOICES, "the pool should stay full");

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
        assert_eq!(
            stolen,
            vec![quietest],
            "the quietest voice should be stolen"
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

    // --- Stealing order (OpenSpec change fix-voice-stealing-16-voices) ---

    use crate::dsp::ModalProfileId;
    use crate::voice::VoiceControls;

    const BOW: i32 = 1;
    const HAND_STRIKE: i32 = 2;

    /// Start a held note with an explicit exciter and MIDI velocity.
    fn start(manager: &mut VoiceManager, note: u8, velocity: f32, exciter: i32) {
        manager.note_on_with_controls(
            note,
            velocity,
            ModalProfileId::Pipe,
            1.0,
            0.0,
            0.0,
            exciter,
            VoiceControls::default(),
        );
    }

    fn run_seconds(manager: &mut VoiceManager, sample_rate: u32, seconds: f32) {
        for _ in 0..(sample_rate as f32 * seconds) as usize {
            manager.process_sample(sample_rate);
        }
    }

    /// Notes of every sounding (held or tailing) voice.
    fn sounding(manager: &VoiceManager) -> Vec<u8> {
        manager
            .voices
            .iter()
            .filter(|voice| voice.is_rendering())
            .map(|voice| voice.note())
            .collect()
    }

    fn peak_of(manager: &VoiceManager, note: u8) -> f32 {
        manager
            .voices
            .iter()
            .find(|voice| voice.is_rendering() && voice.note() == note)
            .map(|voice| voice.peak_hold())
            .expect("note not sounding")
    }

    #[test]
    fn released_tail_is_stolen_before_held_notes() {
        // Quiet held notes and one loud note that is released just before
        // the pool overflows: by level alone the held notes would go first.
        let mut manager = VoiceManager::new();
        for i in 0..MAX_VOICES as u8 - 1 {
            start(&mut manager, 40 + i, 20.0, HAND_STRIKE);
        }
        start(&mut manager, 100, 127.0, HAND_STRIKE);
        run_seconds(&mut manager, 48_000, 0.2);
        manager.note_off(100);
        run_seconds(&mut manager, 48_000, 0.001);
        let tail = peak_of(&manager, 100);
        let held_min = (0..MAX_VOICES as u8 - 1)
            .map(|i| peak_of(&manager, 40 + i))
            .fold(f32::INFINITY, f32::min);
        assert!(
            tail > held_min,
            "setup: tail {tail} should be louder than a held note {held_min}"
        );

        start(&mut manager, 110, 100.0, HAND_STRIKE);
        let notes = sounding(&manager);
        assert!(
            !notes.contains(&100),
            "the released tail should have been stolen"
        );
        for i in 0..MAX_VOICES as u8 - 1 {
            assert!(notes.contains(&(40 + i)), "held note {} was stolen", 40 + i);
        }
        assert!(notes.contains(&110));
    }

    #[test]
    fn rapid_notes_over_a_full_pool_do_not_steal_each_other() {
        // Review finding from C3: quiet, slow-attack new notes kept stealing
        // the previous new note while the old notes rang on.
        let mut manager = VoiceManager::new();
        for i in 0..MAX_VOICES as u8 {
            start(&mut manager, 30 + i, 100.0, HAND_STRIKE);
        }
        run_seconds(&mut manager, 48_000, 0.2);
        for i in 0..MAX_VOICES as u8 {
            start(&mut manager, 80 + i, 30.0, BOW);
            for _ in 0..64 {
                manager.process_sample(48_000);
            }
        }
        let mut notes = sounding(&manager);
        notes.sort_unstable();
        let expected: Vec<u8> = (80..80 + MAX_VOICES as u8).collect();
        assert_eq!(notes, expected, "every new note should still sound");
    }

    /// Fill the pool with loud bowed (sustaining) notes, aged `old_seconds`,
    /// plus one quiet young bowed note aged `young_seconds`; return the
    /// manager.
    fn full_pool_with_quiet_young_note(
        sample_rate: u32,
        old_seconds: f32,
        young_seconds: f32,
    ) -> VoiceManager {
        let mut manager = VoiceManager::new();
        for i in 0..MAX_VOICES as u8 - 1 {
            start(&mut manager, 40 + i, 127.0, BOW);
        }
        run_seconds(&mut manager, sample_rate, old_seconds);
        start(&mut manager, 100, 10.0, BOW);
        run_seconds(&mut manager, sample_rate, young_seconds);
        let young = peak_of(&manager, 100);
        for i in 0..MAX_VOICES as u8 - 1 {
            let old = peak_of(&manager, 40 + i);
            assert!(
                young < old,
                "setup: young note {young} should be quieter than {old}"
            );
        }
        manager
    }

    #[test]
    fn quiet_young_note_survives_stealing() {
        let mut manager = full_pool_with_quiet_young_note(48_000, 0.2, 0.02);
        start(&mut manager, 110, 100.0, HAND_STRIKE);
        let notes = sounding(&manager);
        assert!(notes.contains(&100), "the 20 ms old note was stolen");
        assert!(notes.contains(&110));
        assert_eq!(notes.len(), MAX_VOICES);
    }

    #[test]
    fn oldest_held_note_is_stolen_when_every_note_is_young() {
        let mut manager = VoiceManager::new();
        for i in 0..MAX_VOICES as u8 {
            start(&mut manager, 40 + i, 100.0, HAND_STRIKE);
            run_seconds(&mut manager, 48_000, 0.002);
        }
        start(&mut manager, 110, 100.0, HAND_STRIKE);
        let notes = sounding(&manager);
        assert!(
            !notes.contains(&40),
            "the oldest note should have been stolen"
        );
        assert!(notes.contains(&110));
        assert_eq!(notes.len(), MAX_VOICES);
    }

    #[test]
    fn protection_window_is_one_hundred_milliseconds_at_any_rate() {
        for sample_rate in [44_100, 96_000] {
            // 90 ms old: still protected, so a loud old note goes instead.
            let mut manager = full_pool_with_quiet_young_note(sample_rate, 0.2, 0.09);
            start(&mut manager, 110, 100.0, HAND_STRIKE);
            assert!(
                sounding(&manager).contains(&100),
                "{sample_rate} Hz: note stolen inside the 100 ms window"
            );

            // 110 ms old: no longer protected, and it is the quietest.
            let mut manager = full_pool_with_quiet_young_note(sample_rate, 0.2, 0.11);
            start(&mut manager, 110, 100.0, HAND_STRIKE);
            assert!(
                !sounding(&manager).contains(&100),
                "{sample_rate} Hz: quietest note kept after the 100 ms window"
            );
        }
    }

    #[test]
    fn free_slot_is_used_before_any_tail_in_a_nearly_full_pool() {
        let mut manager = VoiceManager::new();
        for i in 0..MAX_VOICES as u8 - 1 {
            start(&mut manager, 40 + i, 100.0, HAND_STRIKE);
        }
        run_seconds(&mut manager, 48_000, 0.05);
        for i in 0..MAX_VOICES as u8 - 1 {
            manager.note_off(40 + i);
        }
        run_seconds(&mut manager, 48_000, 0.01);
        assert_eq!(
            sounding(&manager).len(),
            MAX_VOICES - 1,
            "setup: tails ringing"
        );
        start(&mut manager, 110, 100.0, HAND_STRIKE);
        let notes = sounding(&manager);
        assert_eq!(notes.len(), MAX_VOICES, "the free slot should be used");
        for i in 0..MAX_VOICES as u8 - 1 {
            assert!(notes.contains(&(40 + i)), "tail {} was cut", 40 + i);
        }
    }

    #[test]
    fn quietest_tail_is_stolen_first() {
        let mut manager = VoiceManager::new();
        for i in 0..MAX_VOICES as u8 - 2 {
            start(&mut manager, 40 + i, 127.0, BOW);
        }
        start(&mut manager, 100, 127.0, HAND_STRIKE);
        start(&mut manager, 101, 15.0, HAND_STRIKE);
        run_seconds(&mut manager, 48_000, 0.2);
        manager.note_off(100);
        manager.note_off(101);
        run_seconds(&mut manager, 48_000, 0.001);
        let (loud, quiet) = (peak_of(&manager, 100), peak_of(&manager, 101));
        assert!(
            quiet < loud,
            "setup: tail 101 ({quiet}) should be quieter than 100 ({loud})"
        );

        start(&mut manager, 110, 100.0, HAND_STRIKE);
        let notes = sounding(&manager);
        assert!(!notes.contains(&101), "the quieter tail should be stolen");
        assert!(notes.contains(&100), "the louder tail should keep sounding");
        assert_eq!(notes.len(), MAX_VOICES);
    }

    #[test]
    fn note_off_releases_the_oldest_voice_on_a_repeated_pitch() {
        // Put the newer note in an earlier slot than the older one, so slot
        // order and age disagree: free slot 0 after the first note-on.
        let mut manager = VoiceManager::new();
        start(&mut manager, 50, 100.0, HAND_STRIKE);
        start(&mut manager, 60, 100.0, HAND_STRIKE);
        run_seconds(&mut manager, 48_000, 0.01);
        manager.voices[0].reset();
        start(&mut manager, 60, 100.0, HAND_STRIKE);
        assert_eq!(manager.voices[0].note(), 60, "setup: newer note in slot 0");
        run_seconds(&mut manager, 48_000, 0.01);
        manager.note_off(60);
        let mut voices: Vec<_> = manager
            .voices
            .iter()
            .filter(|v| v.is_rendering() && v.note() == 60)
            .collect();
        voices.sort_by_key(|v| v.start_frame());
        assert_eq!(voices.len(), 2);
        assert!(!voices[0].is_active(), "the older voice should be released");
        assert!(voices[1].is_active(), "the newer voice should stay held");
    }
}

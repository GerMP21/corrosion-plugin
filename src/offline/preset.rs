//! Offline preset rendering through the plugin's own output path (H13).
//!
//! A preset is loaded into a real [`Corrosion`] instance (every parameter,
//! macros, BODY, Space, limiter mode) and rendered with
//! `Corrosion::process_block`, the same per-block code `process()` runs for
//! the host. Tools (`render_presets`) and the preset level checks hear exactly
//! what the plugin outputs.

use std::sync::Arc;

use nih_plug::prelude::{NoteEvent, Plugin};

use crate::presets::Preset;
use crate::{Corrosion, CorrosionParams};

/// The block size the offline renderer runs `process_block` with: a typical
/// host buffer, so per-block parameter updates happen as they would in a host.
pub const OFFLINE_BLOCK_SIZE: usize = 512;

/// What a scheduled note does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OfflineNoteKind {
    /// Note on with a host velocity in `0.0..=1.0` (nih-plug's scale; the
    /// plugin maps it to the voices' MIDI scale).
    On { velocity: f32 },
    /// Note off.
    Off,
}

/// A note event at an absolute frame of an offline render.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OfflineNote {
    /// Frame from the start of the render at which the event happens.
    pub frame: usize,
    /// MIDI note number.
    pub note: u8,
    pub kind: OfflineNoteKind,
}

impl OfflineNote {
    /// A note on at `frame`.
    pub fn on(frame: usize, note: u8, velocity: f32) -> Self {
        Self {
            frame,
            note,
            kind: OfflineNoteKind::On { velocity },
        }
    }

    /// A note off at `frame`.
    pub fn off(frame: usize, note: u8) -> Self {
        Self {
            frame,
            note,
            kind: OfflineNoteKind::Off,
        }
    }

    /// The host event for this note, with `timing` relative to its block.
    fn to_event(self, timing: u32) -> NoteEvent<()> {
        match self.kind {
            OfflineNoteKind::On { velocity } => NoteEvent::NoteOn {
                timing,
                voice_id: None,
                channel: 0,
                note: self.note,
                velocity,
            },
            OfflineNoteKind::Off => NoteEvent::NoteOff {
                timing,
                voice_id: None,
                channel: 0,
                note: self.note,
                velocity: 0.0,
            },
        }
    }
}

/// The frames of one offline render.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderedFrames {
    /// The plugin's output.
    pub output: Vec<(f32, f32)>,
    /// The same frames before the output limiter (after output gain), for
    /// level measurements that count how often the limiter would act.
    pub pre_limiter: Vec<(f32, f32)>,
}

/// A plugin instance with a preset loaded, rendered offline through the
/// plugin's own output path (`Corrosion::process_block`). State carries over
/// between [`render`](Self::render) calls, as between host buffers.
pub struct PresetRenderer {
    plugin: Corrosion,
}

impl PresetRenderer {
    /// A fresh plugin instance with `preset` loaded, reset as a host resets
    /// it before playback (smoothers start at the preset's values).
    pub fn new(preset: &Preset) -> Self {
        Self::from_params(preset.clone().into_params())
    }

    /// A fresh plugin instance with `params`, reset as a host resets it.
    pub(crate) fn from_params(params: CorrosionParams) -> Self {
        let mut plugin = Corrosion {
            params: Arc::new(params),
            ..Corrosion::default()
        };
        plugin.reset();
        Self { plugin }
    }

    /// The plugin instance, for tests that act on it between renders.
    #[cfg(test)]
    pub(crate) fn plugin_mut(&mut self) -> &mut Corrosion {
        &mut self.plugin
    }

    /// Render the next `frame_count` frames in [`OFFLINE_BLOCK_SIZE`]
    /// blocks, delivering `notes` (sorted by frame, frames counted from the
    /// start of this call) at their frames.
    pub fn render(
        &mut self,
        notes: &[OfflineNote],
        sample_rate: u32,
        frame_count: usize,
    ) -> RenderedFrames {
        render_blocks(
            &mut self.plugin,
            notes,
            sample_rate,
            frame_count,
            OFFLINE_BLOCK_SIZE,
        )
    }
}

/// Render `frame_count` frames of `plugin` in host-sized blocks, delivering
/// `notes` (sorted by frame) at their frames.
fn render_blocks(
    plugin: &mut Corrosion,
    notes: &[OfflineNote],
    sample_rate: u32,
    frame_count: usize,
    block_size: usize,
) -> RenderedFrames {
    assert!(
        notes.windows(2).all(|w| w[0].frame <= w[1].frame),
        "offline notes must be sorted by frame"
    );
    let mut limited = vec![(0.0, 0.0); frame_count];
    let mut unlimited = vec![(0.0, 0.0); frame_count];
    let mut pending = notes.iter().peekable();
    let mut block_start = 0;
    while block_start < frame_count {
        let block_len = block_size.min(frame_count - block_start);
        let block_end = block_start + block_len;
        // The block's events, with timing relative to the block, as a host
        // hands them to `process()`.
        let mut block_events = std::iter::from_fn(|| {
            pending
                .next_if(|n| n.frame < block_end)
                .map(|n| n.to_event((n.frame.max(block_start) - block_start) as u32))
        });
        let mut next_event = block_events.next();
        plugin.process_block(
            block_len,
            sample_rate,
            None,
            &mut next_event,
            || block_events.next(),
            |i, out, pre| {
                limited[block_start + i] = out;
                unlimited[block_start + i] = pre;
            },
        );
        block_start = block_end;
    }
    RenderedFrames {
        output: limited,
        pre_limiter: unlimited,
    }
}

fn frame_count(sample_rate: u32, seconds: f32) -> usize {
    (sample_rate as f32 * seconds) as usize
}

/// Render `preset` offline as the plugin outputs it: voices, master drive,
/// post chain, output gain and the preset's limiter mode, in
/// [`OFFLINE_BLOCK_SIZE`] blocks with `notes` (sorted by frame) delivered at
/// their frames. Returns `seconds × sample_rate` stereo frames.
pub fn render_preset(
    preset: &Preset,
    notes: &[OfflineNote],
    sample_rate: u32,
    seconds: f32,
) -> Vec<(f32, f32)> {
    PresetRenderer::new(preset)
        .render(notes, sample_rate, frame_count(sample_rate, seconds))
        .output
}

/// [`render_preset`]'s frames before the output limiter (after output gain),
/// for level measurements that count how often the limiter would act.
pub fn render_preset_pre_limiter(
    preset: &Preset,
    notes: &[OfflineNote],
    sample_rate: u32,
    seconds: f32,
) -> Vec<(f32, f32)> {
    PresetRenderer::new(preset)
        .render(notes, sample_rate, frame_count(sample_rate, seconds))
        .pre_limiter
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::LOOKAHEAD_LIMITER_SAMPLES;

    const SAMPLE_RATE: u32 = 48_000;
    const SECONDS: f32 = 1.5;

    fn factory(stem: &str) -> Preset {
        let path = format!(
            "{}/presets/factory/{stem}.corrosion-preset",
            env!("CARGO_MANIFEST_DIR")
        );
        Preset::load(std::path::Path::new(&path)).expect("load preset")
    }

    /// Note on at frame 0 and off at 0.5 s; a second note starts mid-block
    /// so events inside a block are exercised too.
    fn notes() -> Vec<OfflineNote> {
        vec![
            OfflineNote::on(0, 60, 1.0),
            OfflineNote::on(10_000, 67, 0.6),
            OfflineNote::off(24_000, 60),
            OfflineNote::off(30_123, 67),
        ]
    }

    /// The plugin as a host drives it: an instance with the preset loaded and
    /// reset, then `process()`'s per-buffer path (`process_block`) for each
    /// 512-sample buffer, with each buffer's events handed over one at a time
    /// at block-relative timings. Written out independently of
    /// `render_blocks` so a change to how the offline renderer slices blocks
    /// or schedules notes shows up here.
    fn host_render(preset: &Preset, notes: &[OfflineNote]) -> Vec<(f32, f32)> {
        let mut plugin = Corrosion {
            params: Arc::new(preset.clone().into_params()),
            ..Corrosion::default()
        };
        plugin.reset();
        let total = (SAMPLE_RATE as f32 * SECONDS) as usize;
        let mut out = Vec::with_capacity(total);
        let mut start = 0;
        while start < total {
            let len = 512.min(total - start);
            let mut events: Vec<NoteEvent<()>> = notes
                .iter()
                .filter(|n| (start..start + len).contains(&n.frame))
                .map(|n| n.to_event((n.frame - start) as u32))
                .collect();
            events.reverse();
            let mut next = events.pop();
            let mut buffer = vec![(0.0_f32, 0.0_f32); len];
            plugin.process_block(
                len,
                SAMPLE_RATE,
                None,
                &mut next,
                || events.pop(),
                |i, frame, _| buffer[i] = frame,
            );
            out.extend(buffer);
            start += len;
        }
        out
    }

    fn assert_bit_identical(a: &[(f32, f32)], b: &[(f32, f32)], what: &str) {
        assert_eq!(a.len(), b.len(), "{what}: lengths differ");
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            assert_eq!(
                (x.0.to_bits(), x.1.to_bits()),
                (y.0.to_bits(), y.1.to_bits()),
                "{what}: frame {i} differs: {x:?} vs {y:?}"
            );
        }
    }

    fn peak(frames: &[(f32, f32)]) -> f32 {
        frames
            .iter()
            .fold(0.0_f32, |m, &(l, r)| m.max(l.abs()).max(r.abs()))
    }

    /// Spec "Post-chain preset": a factory preset with Space, master drive
    /// and BODY, plus the post chain's Lorenz drive, renders offline exactly
    /// as the plugin's processing path plays it.
    #[test]
    fn offline_render_equals_plugin_path_for_a_post_chain_preset() {
        let mut preset = factory("sheet_lightning");
        assert!(preset.extra.space_mode > 0 && preset.extra.space_amount > 0.0);
        assert!(preset.body > 0.0 && preset.drive > 0.0);
        preset.extra.drive_amount = 1.0;
        let offline = render_preset(&preset, &notes(), SAMPLE_RATE, SECONDS);
        assert_bit_identical(&offline, &host_render(&preset, &notes()), "sheet_lightning");

        // The post chain is really in the path: Space keeps ringing after
        // both notes are released, and the output isn't the voices alone.
        let tail = &offline[40_000..];
        assert!(peak(tail) > 1e-4, "no Space tail: peak {}", peak(tail));
        let mut dry = preset.clone();
        dry.extra.space_mode = 0;
        let dry_render = render_preset(&dry, &notes(), SAMPLE_RATE, SECONDS);
        assert!(
            offline.iter().zip(&dry_render).any(|(a, b)| a != b),
            "Space Off renders the same as Space on"
        );
    }

    /// Spec "Lookahead limiter": with Limiter Mode = Lookahead the offline
    /// render equals the plugin's output, and carries the limiter's delay.
    #[test]
    fn offline_render_equals_plugin_path_with_the_lookahead_limiter() {
        let mut hard = factory("beam_strike_industrial");
        hard.extra.limiter_mode = 0;
        let mut lookahead = hard.clone();
        lookahead.extra.limiter_mode = 1;

        let offline = render_preset(&lookahead, &notes(), SAMPLE_RATE, SECONDS);
        assert_bit_identical(
            &offline,
            &host_render(&lookahead, &notes()),
            "beam_strike_industrial (Lookahead)",
        );

        // Below the ceiling the Lookahead limiter only delays: its output is
        // the Hard output, LOOKAHEAD_LIMITER_SAMPLES later.
        let hard_render = render_preset(&hard, &notes(), SAMPLE_RATE, SECONDS);
        assert!(
            peak(&hard_render) < hard.extra.analog_ceiling,
            "test needs a preset under the ceiling"
        );
        let delay = LOOKAHEAD_LIMITER_SAMPLES;
        assert!(offline[..delay].iter().all(|&f| f == (0.0, 0.0)));
        assert_bit_identical(
            &offline[delay..],
            &hard_render[..hard_render.len() - delay],
            "Lookahead vs delayed Hard",
        );
    }

    /// The pre-limiter tap is the limited output before the Hard clamp.
    #[test]
    fn pre_limiter_frames_clamp_to_the_output() {
        let mut preset = factory("beam_strike_industrial");
        preset.output *= 4.0;
        let ceiling = preset.extra.analog_ceiling.clamp(0.1, 1.0);
        let out = render_preset(&preset, &notes(), SAMPLE_RATE, SECONDS);
        let pre = render_preset_pre_limiter(&preset, &notes(), SAMPLE_RATE, SECONDS);
        assert!(peak(&pre) > ceiling, "test needs samples over the ceiling");
        for (o, p) in out.iter().zip(&pre) {
            assert_eq!(
                *o,
                (p.0.clamp(-ceiling, ceiling), p.1.clamp(-ceiling, ceiling))
            );
        }
    }
}

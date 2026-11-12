//! The engine of the browser build (`add-web-app`).
//!
//! [`WebEngine`] plays the role a plugin host plays for [`Corrosion`]: it
//! prepares the plugin at the page's sample rate, applies parameter changes
//! (by index into `param_map()`, whose order the editor's own
//! `CorrosionParams` shares), queues MIDI, and renders blocks through
//! `Corrosion::process_block`, the code `process()` and the offline renderer
//! run. It is target-independent so it can be tested natively; the wasm
//! exports live in the `web/engine` crate.
//!
//! Setting a parameter needs `ParamPtr::set_normalized_value` and
//! `update_smoother`, which upstream NIH-plug keeps crate-private. The `web/`
//! workspace patches NIH-plug to make them public; that is why this module
//! is behind the `web-host` feature.

pub mod editor_side;
pub mod protocol;

use nih_plug::prelude::{NoteEvent, ParamPtr, Params, Plugin};

pub use editor_side::{EditorBridge, WebGuiContext};
pub use protocol::{Command, Report};

use crate::gui_notes::{EditorNoteEvent, NoteMask};
use crate::kit::KitState;
use crate::{Corrosion, Preset};

/// The most frames one [`WebEngine::process`] call renders. An AudioWorklet
/// asks for 128; the margin lets a host render larger blocks.
pub const MAX_BLOCK: usize = 1024;

/// The most MIDI events queued between two blocks. Events past it are
/// dropped (a block is 2.7 ms at 48 kHz: no player fills this).
pub const MAX_PENDING_EVENTS: usize = 512;

/// Corrosion driven by a web page instead of a plugin host.
pub struct WebEngine {
    plugin: Corrosion,
    sample_rate: f32,
    /// The plugin's parameters in `param_map()` order, with their IDs.
    params: Vec<ParamPtr>,
    ids: Vec<String>,
    /// MIDI received since the last block, played at its first sample.
    pending: Vec<NoteEvent<()>>,
    tempo: Option<f32>,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl WebEngine {
    /// A default patch prepared at `sample_rate`, as a host initializes and
    /// resets the plugin before playback.
    pub fn new(sample_rate: f32) -> Self {
        let mut plugin = Corrosion::default();
        let (ids, params): (Vec<String>, Vec<ParamPtr>) = plugin
            .params()
            .param_map()
            .into_iter()
            .map(|(id, ptr, _)| (id, ptr))
            .unzip();
        for ptr in &params {
            // SAFETY: the pointers point into `plugin.params`, an `Arc`
            // this engine keeps alive for as long as it holds them.
            unsafe { ptr.update_smoother(sample_rate, true) };
        }
        plugin.prepare(sample_rate);
        plugin.reset();
        Self {
            plugin,
            sample_rate,
            params,
            ids,
            pending: Vec::with_capacity(MAX_PENDING_EVENTS),
            tempo: None,
            left: vec![0.0; MAX_BLOCK],
            right: vec![0.0; MAX_BLOCK],
        }
    }

    /// How many parameters [`set_param`](Self::set_param) addresses.
    pub fn param_count(&self) -> usize {
        self.params.len()
    }

    /// The index of the parameter with this ID, as [`set_param`](Self::set_param)
    /// takes it.
    pub fn param_index(&self, id: &str) -> Option<usize> {
        self.ids.iter().position(|param_id| param_id == id)
    }

    /// Set parameter `index` (in `param_map()` order) to a normalized value,
    /// as host automation does: the value changes now and its smoother
    /// glides from the old one. Out-of-range indices are ignored.
    pub fn set_param(&mut self, index: usize, normalized: f32) {
        if let Some(ptr) = self.params.get(index) {
            // SAFETY: see `new`.
            unsafe {
                if ptr.set_normalized_value(normalized) {
                    ptr.update_smoother(self.sample_rate, false);
                }
            }
        }
    }

    /// Load a preset as the editor's preset browser does: every parameter
    /// moves to the preset's value (smoothed, like automation), and the kit
    /// and its pad table are replaced. Allocates; call it between blocks.
    pub fn load_preset(&mut self, preset: &Preset) {
        let loaded = preset.clone().into_params();
        for (index, (_, ptr, _)) in loaded.param_map().into_iter().enumerate() {
            // SAFETY: `ptr` points into `loaded`, alive in this scope.
            let normalized = unsafe { ptr.unmodulated_normalized_value() };
            self.set_param(index, normalized);
        }
        let kit = loaded.kit.read().unwrap_or_else(|e| e.into_inner()).clone();
        self.set_kit(kit);
    }

    /// The normalized value of parameter `index`, or `None` out of range.
    pub fn param(&self, index: usize) -> Option<f32> {
        // SAFETY: see `new`.
        self.params
            .get(index)
            .map(|ptr| unsafe { ptr.unmodulated_normalized_value() })
    }

    /// Queue a MIDI message (note, CC, pitch bend, pressure) for the next
    /// block. Unparsable messages and messages past the queue are dropped.
    pub fn midi(&mut self, status: u8, data1: u8, data2: u8) {
        if self.pending.len() < MAX_PENDING_EVENTS {
            if let Ok(event) = NoteEvent::from_midi(0, &[status, data1, data2]) {
                self.pending.push(event);
            }
        }
    }

    /// The tempo synced effects follow, as a host transport reports it.
    /// `None` (or a non-positive tempo) means no host tempo.
    pub fn set_tempo(&mut self, bpm: Option<f32>) {
        self.tempo = bpm.filter(|bpm| *bpm > 0.0);
    }

    /// Render `frames` frames (at most [`MAX_BLOCK`]) into
    /// [`left`](Self::left) and [`right`](Self::right), playing the queued
    /// MIDI at the first sample. Allocation-free.
    pub fn process(&mut self, frames: usize) {
        let frames = frames.min(MAX_BLOCK);
        let mut events = self.pending.drain(..);
        let mut next_event = events.next();
        let left = &mut self.left;
        let right = &mut self.right;
        self.plugin.process_block(
            frames,
            self.sample_rate as u32,
            self.tempo,
            &mut next_event,
            || events.next(),
            |i, (l, r), _| {
                left[i] = l;
                right[i] = r;
            },
        );
    }

    /// The left channel of the last [`process`](Self::process) call.
    pub fn left(&self) -> &[f32] {
        &self.left
    }

    /// The right channel of the last [`process`](Self::process) call.
    pub fn right(&self) -> &[f32] {
        &self.right
    }

    /// Pointers to the output buffers, for a wasm host that reads them from
    /// linear memory.
    pub fn output_ptrs(&mut self) -> (*mut f32, *mut f32) {
        (self.left.as_mut_ptr(), self.right.as_mut_ptr())
    }

    /// Apply a batch of editor [`Command`]s before the next block.
    /// Allocation-free except for a kit, which rebuilds the pad table.
    pub fn apply(&mut self, batch: &[u8]) {
        for command in protocol::decode(batch) {
            match command {
                Command::Param { index, normalized } => self.set_param(index as usize, normalized),
                Command::Midi([status, data1, data2]) => self.midi(status, data1, data2),
                Command::EditorNoteOn { note, velocity } => {
                    self.plugin
                        .editor_notes
                        .try_push(EditorNoteEvent::On { note, velocity });
                }
                Command::EditorNoteOff { note } => {
                    self.plugin
                        .editor_notes
                        .try_push(EditorNoteEvent::Off { note });
                }
                Command::ReleaseEditorNotes => self.plugin.editor_notes.request_release_all(),
                Command::Tempo(bpm) => self.set_tempo(Some(bpm)),
                Command::Kit(json) => {
                    if let Ok(kit) = serde_json::from_str::<KitState>(json) {
                        self.set_kit(kit);
                    }
                }
            }
        }
    }

    /// The [`Report`] for the editor; takes the diode meter's peak.
    pub fn report(&mut self) -> Report {
        let mut held = NoteMask::default();
        for note in self.plugin.voice_manager.active_notes() {
            held.set(note, true);
        }
        Report {
            diode_reduction_db: self.plugin.diode_meter.take(),
            voices: self.active_voices() as u32,
            held,
        }
    }

    /// Replace the kit and rebuild its pad table. Allocates.
    fn set_kit(&mut self, kit: KitState) {
        self.plugin.params.kit_table.load(&kit);
        *self
            .plugin
            .params
            .kit
            .write()
            .unwrap_or_else(|e| e.into_inner()) = kit;
    }

    /// How many voices are sounding.
    pub fn active_voices(&self) -> usize {
        self.plugin.voice_manager.active_notes().count()
    }
}

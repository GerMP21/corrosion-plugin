//! The web editor's side of the bridge (`add-web-app`): a NIH-plug
//! [`GuiContext`] that keeps the editor's own parameters (a mirror of the
//! engine's) and queues every change as a [`Command`] for the engine, plus
//! the kit, the keyboard's notes and the engine's [`Report`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nih_plug::prelude::{GuiContext, ParamPtr, ParamSetter, Params, PluginApi};
use nih_plug::wrapper::state::PluginState;

use super::protocol::{Command, Report};
use crate::dsp::DiodeMeter;
use crate::gui_notes::{EditorNoteEvent, EditorNotes};
use crate::{CorrosionParams, Preset};

/// The editor's host: sets the mirror and queues the change for the engine.
pub struct WebGuiContext {
    index: HashMap<ParamPtr, u32>,
    outbox: Mutex<Vec<u8>>,
}

// SAFETY: the `ParamPtr`s are map keys only here (never dereferenced), and
// the page runs the editor on one thread.
unsafe impl Send for WebGuiContext {}
unsafe impl Sync for WebGuiContext {}

impl WebGuiContext {
    /// A context for `params`, the editor's parameters.
    pub fn new(params: &CorrosionParams) -> Self {
        let index = params
            .param_map()
            .into_iter()
            .enumerate()
            .map(|(i, (_, ptr, _))| (ptr, i as u32))
            .collect();
        Self {
            index,
            outbox: Mutex::new(Vec::new()),
        }
    }

    /// Queue `command` for the engine.
    pub fn push(&self, command: Command) {
        command.encode(&mut self.outbox.lock().unwrap_or_else(|e| e.into_inner()));
    }

    fn take(&self) -> Vec<u8> {
        std::mem::take(&mut *self.outbox.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

impl GuiContext for WebGuiContext {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Standalone
    }

    fn request_resize(&self) -> bool {
        true
    }

    unsafe fn raw_begin_set_parameter(&self, _param: ParamPtr) {}

    unsafe fn raw_set_parameter_normalized(&self, param: ParamPtr, normalized: f32) {
        param.set_normalized_value(normalized);
        if let Some(&index) = self.index.get(&param) {
            // Send what the parameter took (stepped parameters snap).
            let normalized = param.unmodulated_normalized_value();
            self.push(Command::Param { index, normalized });
        }
    }

    unsafe fn raw_end_set_parameter(&self, _param: ParamPtr) {}

    fn get_state(&self) -> PluginState {
        PluginState {
            version: String::new(),
            params: Default::default(),
            fields: Default::default(),
        }
    }

    fn set_state(&self, _state: PluginState) {}
}

/// Everything the web editor shares with "its" plugin, bridged to the
/// engine: parameters, kit, keyboard notes, the diode meter.
pub struct EditorBridge {
    params: Arc<CorrosionParams>,
    notes: Arc<EditorNotes>,
    diode_meter: Arc<DiodeMeter>,
    context: Arc<WebGuiContext>,
    kit_generation: u32,
    voices: u32,
}

impl EditorBridge {
    pub fn new(
        params: Arc<CorrosionParams>,
        notes: Arc<EditorNotes>,
        diode_meter: Arc<DiodeMeter>,
    ) -> Self {
        let context = Arc::new(WebGuiContext::new(&params));
        let kit_generation = params.kit_table.generation();
        Self {
            params,
            notes,
            diode_meter,
            context,
            kit_generation,
            voices: 0,
        }
    }

    /// The setter the editor's widgets take.
    pub fn setter(&self) -> ParamSetter<'_> {
        ParamSetter::new(self.context.as_ref())
    }

    /// Load `preset` into the editor and, through the batch, the engine.
    pub fn load_preset(&self, preset: &Preset) {
        let loaded = preset.clone().into_params();
        let ours = self.params.param_map();
        for ((_, from, _), (_, to, _)) in loaded.param_map().into_iter().zip(ours) {
            // SAFETY: `from` points into `loaded` and `to` into
            // `self.params`, both alive here.
            unsafe {
                let normalized = from.unmodulated_normalized_value();
                self.context.raw_set_parameter_normalized(to, normalized);
            }
        }
        let kit = loaded.kit.read().unwrap_or_else(|e| e.into_inner()).clone();
        // The kit table's generation moves, so the next batch sends the kit.
        self.params.kit_table.load(&kit);
        *self.params.kit.write().unwrap_or_else(|e| e.into_inner()) = kit;
    }

    /// The commands queued since the last call: parameter changes, the
    /// keyboard's notes and, if it changed, the kit.
    pub fn take_batch(&mut self) -> Vec<u8> {
        let context = &self.context;
        self.notes.drain(|event| match event {
            EditorNoteEvent::On { note, velocity } => {
                context.push(Command::EditorNoteOn { note, velocity })
            }
            EditorNoteEvent::Off { note } => context.push(Command::EditorNoteOff { note }),
        });
        if self.notes.take_release_all() {
            context.push(Command::ReleaseEditorNotes);
        }
        let generation = self.params.kit_table.generation();
        if generation != self.kit_generation {
            self.kit_generation = generation;
            let kit = self.params.kit.read().unwrap_or_else(|e| e.into_inner());
            if let Ok(json) = serde_json::to_string(&*kit) {
                context.push(Command::Kit(&json));
            }
        }
        context.take()
    }

    /// Apply an engine [`Report`]; malformed reports are ignored.
    pub fn apply_report(&mut self, bytes: &[u8]) {
        if let Some(report) = Report::decode(bytes) {
            self.diode_meter.publish(report.diode_reduction_db);
            self.notes.publish_held(report.held.iter());
            self.voices = report.voices;
        }
    }

    /// Voices sounding, from the last report.
    pub fn voices(&self) -> u32 {
        self.voices
    }

    /// The editor's parameters.
    pub fn params(&self) -> &Arc<CorrosionParams> {
        &self.params
    }
}

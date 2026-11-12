//! Corrosion's editor in a web page (`add-web-app`): the plugin's egui
//! editor drawn by eframe into a `<canvas>`, with an [`EditorBridge`] as its
//! host. Each frame's changes go to the page as one command batch (the page
//! posts it to the engine's AudioWorklet); the engine's reports come back
//! through [`push_report`].

use std::cell::RefCell;
use std::sync::Arc;

use corrosion::dsp::DiodeMeter;
use corrosion::gui::WebEditor;
use corrosion::gui_notes::EditorNotes;
use corrosion::web::EditorBridge;
use corrosion::CorrosionParams;
use wasm_bindgen::prelude::*;

thread_local! {
    /// Engine reports received since the editor's last frame.
    static REPORTS: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    /// Set by `release_keys`, honoured at the next frame.
    static RELEASE_KEYS: RefCell<bool> = const { RefCell::new(false) };
}

struct CorrosionApp {
    bridge: EditorBridge,
    editor: WebEditor,
    post: js_sys::Function,
}

impl CorrosionApp {
    fn new(post: js_sys::Function, seed: u32) -> Self {
        let params = Arc::new(CorrosionParams::default());
        let notes = Arc::new(EditorNotes::new());
        let diode_meter = Arc::new(DiodeMeter::new());
        let bridge = EditorBridge::new(params.clone(), notes.clone(), diode_meter.clone());
        let editor = WebEditor::new(params, diode_meter, notes, seed);
        Self {
            bridge,
            editor,
            post,
        }
    }
}

impl eframe::App for CorrosionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for report in REPORTS.with(|reports| std::mem::take(&mut *reports.borrow_mut())) {
            self.bridge.apply_report(&report);
        }
        if RELEASE_KEYS.with(|flag| std::mem::take(&mut *flag.borrow_mut())) {
            self.editor.release_keys();
        }
        self.editor.frame(ctx, &self.bridge.setter());
        let batch = self.bridge.take_batch();
        if !batch.is_empty() {
            let bytes = js_sys::Uint8Array::from(batch.as_slice());
            let _ = self.post.call1(&JsValue::NULL, &bytes);
        }
        // The strip's highlights and the limiter dot follow the engine.
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

/// Start the editor in `canvas`. `post` receives each command batch (a
/// `Uint8Array`) for the engine; `seed` seeds the randomizer.
#[wasm_bindgen]
pub async fn start(
    canvas: web_sys::HtmlCanvasElement,
    post: js_sys::Function,
    seed: u32,
) -> Result<(), JsValue> {
    let options = eframe::WebOptions::default();
    eframe::WebRunner::new()
        .start(
            canvas,
            options,
            Box::new(move |_cc| Ok(Box::new(CorrosionApp::new(post, seed)))),
        )
        .await
}

/// An engine report (the bytes the worklet posted).
#[wasm_bindgen]
pub fn push_report(bytes: &[u8]) {
    REPORTS.with(|reports| reports.borrow_mut().push(bytes.to_vec()));
}

/// Release the keyboard's held notes (the page lost focus or was hidden).
#[wasm_bindgen]
pub fn release_keys() {
    RELEASE_KEYS.with(|flag| *flag.borrow_mut() = true);
}

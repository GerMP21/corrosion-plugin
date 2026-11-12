//! Corrosion's editor in a web page (`add-web-app`): the plugin's egui
//! editor drawn by eframe into a `<canvas>`, with an [`EditorBridge`] as its
//! host. Each frame's changes go to the page as one command batch (the page
//! posts it to the engine's AudioWorklet); the engine's reports come back
//! through [`push_report`]. The page also hands over its stored user
//! presets and the session, and persists what the editor saves.

use std::cell::RefCell;
use std::sync::Arc;

use corrosion::dsp::DiodeMeter;
use corrosion::gui::WebEditor;
use corrosion::gui_notes::EditorNotes;
use corrosion::presets::browser::{self, BrowserPreset};
use corrosion::web::EditorBridge;
use corrosion::{CorrosionParams, Preset};
use wasm_bindgen::prelude::*;

/// What the page's calls leave for the editor's next frame.
#[derive(Default)]
struct Inbox {
    reports: Vec<Vec<u8>>,
    release_keys: bool,
    refresh_presets: bool,
    load: Option<Preset>,
    message: Option<(String, bool)>,
}

thread_local! {
    static INBOX: RefCell<Inbox> = RefCell::new(Inbox::default());
    /// The editor's parameters, for `current_preset_json`.
    static PARAMS: RefCell<Option<Arc<CorrosionParams>>> = const { RefCell::new(None) };
    /// Whether the last frame had the pointer over a popup or menu, for
    /// [`wants_wheel`].
    static WANTS_WHEEL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn with_inbox<T>(f: impl FnOnce(&mut Inbox) -> T) -> T {
    INBOX.with(|inbox| f(&mut inbox.borrow_mut()))
}

struct CorrosionApp {
    bridge: EditorBridge,
    editor: WebEditor,
    post: js_sys::Function,
    /// The canvas, sized to the editor's height so the page scrolls rather
    /// than the editor.
    canvas: web_sys::HtmlCanvasElement,
    /// The canvas height last set, in CSS pixels.
    canvas_height: f32,
}

impl CorrosionApp {
    fn new(
        canvas: web_sys::HtmlCanvasElement,
        post: js_sys::Function,
        seed: u32,
        session: Option<Preset>,
    ) -> Self {
        let params = Arc::new(CorrosionParams::default());
        let notes = Arc::new(EditorNotes::new());
        let diode_meter = Arc::new(DiodeMeter::new());
        let bridge = EditorBridge::new(params.clone(), notes.clone(), diode_meter.clone());
        if let Some(session) = session {
            // Before the editor's first frame: its batch carries the session
            // to the engine.
            bridge.load_preset(&session);
        }
        PARAMS.with(|shared| *shared.borrow_mut() = Some(params.clone()));
        let editor = WebEditor::new(params, diode_meter, notes, seed);
        Self {
            bridge,
            editor,
            post,
            canvas,
            canvas_height: 0.0,
        }
    }
}

impl eframe::App for CorrosionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let inbox = with_inbox(std::mem::take);
        for report in &inbox.reports {
            self.bridge.apply_report(report);
        }
        if inbox.release_keys {
            self.editor.release_keys();
        }
        if let Some(preset) = &inbox.load {
            self.bridge.load_preset(preset);
        }
        if inbox.refresh_presets || inbox.load.is_some() {
            self.editor.refresh_user_presets();
        }
        if let Some((message, is_error)) = inbox.message {
            self.editor.set_message(message, is_error);
        }
        self.editor.frame(ctx, &self.bridge.setter());
        WANTS_WHEEL.with(|wants| wants.set(WebEditor::wants_wheel(ctx)));
        // Points are CSS pixels times the zoom factor.
        let height = (self.editor.content_height() * ctx.zoom_factor()).ceil();
        if height > 0.0 && height != self.canvas_height {
            self.canvas_height = height;
            let _ = self
                .canvas
                .style()
                .set_property("height", &format!("{height}px"));
        }
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
/// `Uint8Array`) for the engine; `seed` seeds the randomizer; `session` is
/// the last session's preset JSON, if any (an unreadable one is ignored).
#[wasm_bindgen]
pub async fn start(
    canvas: web_sys::HtmlCanvasElement,
    post: js_sys::Function,
    seed: u32,
    session: Option<String>,
) -> Result<(), JsValue> {
    let session = session.and_then(|json| Preset::from_json_str(&json).ok());
    let options = eframe::WebOptions::default();
    let app_canvas = canvas.clone();
    eframe::WebRunner::new()
        .start(
            canvas,
            options,
            Box::new(move |_cc| Ok(Box::new(CorrosionApp::new(app_canvas, post, seed, session)))),
        )
        .await
}

/// Whether a mouse-wheel event over the canvas belongs to the editor (the
/// pointer is over an open popup or menu). Otherwise the page keeps it and
/// scrolls.
#[wasm_bindgen]
pub fn wants_wheel() -> bool {
    WANTS_WHEEL.with(|wants| wants.get())
}

/// An engine report (the bytes the worklet posted).
#[wasm_bindgen]
pub fn push_report(bytes: &[u8]) {
    with_inbox(|inbox| inbox.reports.push(bytes.to_vec()));
}

/// Release the keyboard's held notes (the page lost focus or was hidden).
#[wasm_bindgen]
pub fn release_keys() {
    with_inbox(|inbox| inbox.release_keys = true);
}

/// Replace the user presets with the page's stored ones: a JSON array of
/// `{folder, file, json}`. Returns false if it doesn't parse.
#[wasm_bindgen]
pub fn set_user_presets(stored: &str) -> bool {
    match serde_json::from_str::<Vec<BrowserPreset>>(stored) {
        Ok(presets) => {
            browser::replace_all(presets);
            with_inbox(|inbox| inbox.refresh_presets = true);
            true
        }
        Err(_) => false,
    }
}

/// The presets the editor saved (or the page imported) since the last
/// call, as a JSON array of `{folder, file, json}`, for the page to store.
#[wasm_bindgen]
pub fn take_saved_presets() -> String {
    serde_json::to_string(&browser::take_saved()).unwrap_or_else(|_| "[]".into())
}

/// Open a `.corrosion-preset` file: load it (the engine follows) and keep
/// it with the user presets. Returns the preset's name, or the parse error.
#[wasm_bindgen]
pub fn open_preset_file(json: &str) -> Result<String, JsValue> {
    match browser::import(json) {
        Ok(preset) => {
            let name = preset.name.clone();
            with_inbox(|inbox| {
                inbox.message = Some((format!("Opened \"{name}\""), false));
                inbox.load = Some(preset);
            });
            Ok(name)
        }
        Err(error) => {
            let message = format!("Couldn't open the file: {error}");
            with_inbox(|inbox| inbox.message = Some((message.clone(), true)));
            Err(JsValue::from_str(&message))
        }
    }
}

/// The current sound as `.corrosion-preset` JSON named `name` (every
/// parameter and the kit): the session to keep, or a file to download.
#[wasm_bindgen]
pub fn current_preset_json(name: &str) -> Option<String> {
    PARAMS.with(|shared| {
        let params = shared.borrow();
        let params = params.as_ref()?;
        serde_json::to_string_pretty(&Preset::from_params(name, params)).ok()
    })
}

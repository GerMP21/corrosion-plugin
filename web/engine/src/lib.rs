//! WebAssembly exports of Corrosion's engine for the browser build
//! (`add-web-app`).
//!
//! The AudioWorklet instantiates this module directly (no JS glue: worklets
//! have no `TextDecoder` or `fetch`), so the interface is plain C-ABI
//! functions over a [`Handle`]. The worklet:
//! - copies each batch of editor commands into the inbox
//!   ([`engine_inbox`]) and calls [`engine_apply`];
//! - calls [`engine_process`] per quantum and reads the channels at
//!   [`engine_left`] and [`engine_right`];
//! - calls [`engine_report`] now and then and posts the bytes at the
//!   returned pointer back to the editor.

use corrosion::web::protocol::REPORT_LEN;
pub use corrosion::web::{WebEngine, MAX_BLOCK};

/// The engine and the buffers the worklet reads and writes.
pub struct Handle {
    engine: WebEngine,
    inbox: Vec<u8>,
    report: [u8; REPORT_LEN],
}

/// A new engine with the default patch at `sample_rate`. Never freed: a page
/// holds one engine for its lifetime.
#[no_mangle]
pub extern "C" fn engine_new(sample_rate: f32) -> *mut Handle {
    Box::into_raw(Box::new(Handle {
        engine: WebEngine::new(sample_rate),
        inbox: vec![0; 64 * 1024],
        report: [0; REPORT_LEN],
    }))
}

/// Memory for a command batch of `len` bytes. Grows (allocates) only when a
/// batch is larger than any before (a kit).
///
/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_inbox(handle: *mut Handle, len: u32) -> *mut u8 {
    let inbox = &mut (*handle).inbox;
    if inbox.len() < len as usize {
        inbox.resize(len as usize, 0);
    }
    inbox.as_mut_ptr()
}

/// Apply the first `len` bytes of the inbox as a command batch.
///
/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_apply(handle: *mut Handle, len: u32) {
    let handle = &mut *handle;
    let len = (len as usize).min(handle.inbox.len());
    handle.engine.apply(&handle.inbox[..len]);
}

/// Encode the engine's report; returns where its bytes are.
///
/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_report(handle: *mut Handle) -> *const u8 {
    let handle = &mut *handle;
    handle.report = handle.engine.report().encode();
    handle.report.as_ptr()
}

/// The report's length in bytes.
#[no_mangle]
pub extern "C" fn engine_report_len() -> u32 {
    REPORT_LEN as u32
}

/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_process(handle: *mut Handle, frames: u32) {
    (*handle).engine.process(frames as usize);
}

/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_left(handle: *mut Handle) -> *mut f32 {
    (*handle).engine.output_ptrs().0
}

/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_right(handle: *mut Handle) -> *mut f32 {
    (*handle).engine.output_ptrs().1
}

/// A MIDI message straight from Web MIDI.
///
/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_midi(handle: *mut Handle, status: u32, data1: u32, data2: u32) {
    (*handle)
        .engine
        .midi(status as u8, data1 as u8, data2 as u8);
}

/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_set_param(handle: *mut Handle, index: u32, normalized: f32) {
    (*handle).engine.set_param(index as usize, normalized);
}

/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_param_count(handle: *mut Handle) -> u32 {
    (*handle).engine.param_count() as u32
}

/// The index of the parameter whose UTF-8 ID is the first `len` inbox
/// bytes, or -1.
///
/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_param_index(handle: *mut Handle, len: u32) -> i32 {
    let handle = &*handle;
    let id = &handle.inbox[..(len as usize).min(handle.inbox.len())];
    std::str::from_utf8(id)
        .ok()
        .and_then(|id| handle.engine.param_index(id))
        .map_or(-1, |index| index as i32)
}

/// Load the `.corrosion-preset` JSON in the first `len` inbox bytes;
/// returns 1 on success, 0 if it doesn't parse. Allocates.
///
/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_load_preset(handle: *mut Handle, len: u32) -> u32 {
    let handle = &mut *handle;
    let json = &handle.inbox[..(len as usize).min(handle.inbox.len())];
    match std::str::from_utf8(json)
        .ok()
        .map(corrosion::Preset::from_json_str)
    {
        Some(Ok(preset)) => {
            handle.engine.load_preset(&preset);
            1
        }
        _ => 0,
    }
}

/// Tempo in BPM; 0 or less means no host tempo.
///
/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_set_tempo(handle: *mut Handle, bpm: f32) {
    (*handle).engine.set_tempo(Some(bpm));
}

/// # Safety
/// `handle` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_active_voices(handle: *mut Handle) -> u32 {
    (*handle).engine.active_voices() as u32
}

/// The most frames one [`engine_process`] call renders.
#[no_mangle]
pub extern "C" fn engine_max_block() -> u32 {
    MAX_BLOCK as u32
}

//! WebAssembly exports of Corrosion's engine for the browser build
//! (`add-web-app`).
//!
//! The AudioWorklet instantiates this module directly (no JS glue: worklets
//! have no `TextDecoder` or `fetch`), so the interface is plain C-ABI
//! functions over a handle. The worklet reads the output channels from linear
//! memory at the pointers [`engine_left`] and [`engine_right`] return.

pub use corrosion::web::{WebEngine, MAX_BLOCK};

/// A new engine with the default patch at `sample_rate`. Never freed: a page
/// holds one engine for its lifetime.
#[no_mangle]
pub extern "C" fn engine_new(sample_rate: f32) -> *mut WebEngine {
    Box::into_raw(Box::new(WebEngine::new(sample_rate)))
}

/// # Safety
/// `engine` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_process(engine: *mut WebEngine, frames: u32) {
    (*engine).process(frames as usize);
}

/// # Safety
/// `engine` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_left(engine: *mut WebEngine) -> *mut f32 {
    (*engine).output_ptrs().0
}

/// # Safety
/// `engine` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_right(engine: *mut WebEngine) -> *mut f32 {
    (*engine).output_ptrs().1
}

/// # Safety
/// `engine` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_midi(engine: *mut WebEngine, status: u32, data1: u32, data2: u32) {
    (*engine).midi(status as u8, data1 as u8, data2 as u8);
}

/// # Safety
/// `engine` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_set_param(engine: *mut WebEngine, index: u32, normalized: f32) {
    (*engine).set_param(index as usize, normalized);
}

/// # Safety
/// `engine` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_param_count(engine: *mut WebEngine) -> u32 {
    (*engine).param_count() as u32
}

/// Tempo in BPM; 0 or less means no host tempo.
///
/// # Safety
/// `engine` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_set_tempo(engine: *mut WebEngine, bpm: f32) {
    (*engine).set_tempo(Some(bpm));
}

/// # Safety
/// `engine` must come from [`engine_new`].
#[no_mangle]
pub unsafe extern "C" fn engine_active_voices(engine: *mut WebEngine) -> u32 {
    (*engine).active_voices() as u32
}

/// The most frames one [`engine_process`] call renders.
#[no_mangle]
pub extern "C" fn engine_max_block() -> u32 {
    MAX_BLOCK as u32
}

/// `len` bytes of wasm memory for the host to write into (a parameter ID).
/// Never freed: a page asks for a handful of short-lived buffers.
#[no_mangle]
pub extern "C" fn engine_alloc(len: u32) -> *mut u8 {
    Box::leak(vec![0u8; len as usize].into_boxed_slice()).as_mut_ptr()
}

/// The index of the parameter whose UTF-8 ID is at `id`, or -1.
///
/// # Safety
/// `engine` must come from [`engine_new`], and `id..id + len` must be
/// readable memory (from [`engine_alloc`]).
#[no_mangle]
pub unsafe extern "C" fn engine_param_index(
    engine: *mut WebEngine,
    id: *const u8,
    len: u32,
) -> i32 {
    let id = std::slice::from_raw_parts(id, len as usize);
    std::str::from_utf8(id)
        .ok()
        .and_then(|id| (*engine).param_index(id))
        .map_or(-1, |index| index as i32)
}

/// Load the `.corrosion-preset` JSON at `json`; returns 1 on success, 0 if
/// it doesn't parse.
///
/// # Safety
/// As [`engine_param_index`].
#[no_mangle]
pub unsafe extern "C" fn engine_load_preset(
    engine: *mut WebEngine,
    json: *const u8,
    len: u32,
) -> u32 {
    let json = std::slice::from_raw_parts(json, len as usize);
    match std::str::from_utf8(json)
        .ok()
        .map(corrosion::Preset::from_json_str)
    {
        Some(Ok(preset)) => {
            (*engine).load_preset(&preset);
            1
        }
        _ => 0,
    }
}

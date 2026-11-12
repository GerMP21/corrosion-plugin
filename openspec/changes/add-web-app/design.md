# Design

## Context

The engine already has a host-agnostic entry point, `Corrosion::process_block(block_len, sample_rate, transport, next_event, …)`, used by the offline renderer (`src/offline/preset.rs::render_blocks`). The editor is egui (`render_ui(ui, params, &ParamSetter, state, scale)`) and already runs headless in tests with a null `GuiContext`. It shares four things with the audio side: `Arc<CorrosionParams>` (atomics plus the kit table), `editor_state`, `Arc<DiodeMeter>` and `Arc<EditorNotes>`.

A spike (`cargo check --lib --no-default-features --target wasm32-unknown-unknown`) shows the DSP compiles for WebAssembly as is. The only errors are in NIH-plug: its VST3 wrapper (default `vst3` feature) and `event_loop.rs` (no `OsEventLoop` on wasm). `std::fs` and threads appear only in tests and the preset folders.

The cost on native: 16 voices at Render use about 36 % of one core. WebAssembly in an AudioWorklet is single-threaded, has no SIMD unless enabled, and gets 128-frame blocks (2.7 ms at 48 kHz) with no slack.

## Goals / Non-Goals

**Goals:**
- The real engine and the real editor, compiled from the same sources as the plugin.
- Audio in an AudioWorklet, never on the main thread.
- Every parameter, preset, kit and editor feature, plus Web MIDI, the computer keyboard, a tempo, session autosave and user presets.
- Free public hosting on Cloudflare Workers, deployed from `main`.
- No change to the native plugins' sound or binaries.

**Non-Goals:**
- Running as a plugin inside a web DAW (WAM, WebCLAP).
- Audio input, effect mode, recording or rendering to a file (possible later).
- Safari Web MIDI (Safari doesn't have it). Mobile layouts.
- Multithreaded wasm (`SharedArrayBuffer` needs cross-origin isolation headers and nightly Rust for wasm threads; out of scope).

## Decisions

### 1. Two wasm instances talking through messages (Architecture 2)

```
 Main thread                                   AudioWorklet thread
┌───────────────────────────────┐             ┌──────────────────────────────┐
│ editor.wasm (eframe, canvas)  │  postMessage│ engine.wasm                  │
│  render_ui + WebGuiContext ───┼────────────▶│  CorrosionParams (own copy)  │
│  CorrosionParams (mirror)     │ param, note,│  process_block(128)          │
│  host shell (JS): MIDI, BPM,  │ kit, state  │  → Float32 L/R outputs       │
│  storage, file up/download    │◀────────────┼─ diode meter, voice count,   │
└───────────────────────────────┘ meter, load │  CPU load (every ~30 ms)     │
                                              └──────────────────────────────┘
```

Each side builds its own `CorrosionParams`. The editor's `WebGuiContext` implements NIH-plug's public `GuiContext`: `set_parameter_normalized` writes the local mirror and posts `{param id, normalized}`. The engine applies it to its copy at the start of the next block, like host automation. Notes from the piano strip, computer keys and Web MIDI post `{note on/off, key, velocity, channel}` and become `NoteEvent`s at the next block's sample 0. State (presets, kit table, session restore) posts the same JSON the plugin saves.

**Alternatives:**
- One wasm instance with shared memory (Architecture 1): it needs `SharedArrayBuffer`, cross-origin isolation headers and wasm threads (nightly Rust). Rejected for now: more fragile hosting, unstable toolchain.
- A JavaScript editor: rewrites the whole UI and drifts from the plugin. Rejected.

Messages add up to one block (2.7 ms) of latency for controls and notes, below what a visitor notices.

### 2. NIH-plug on wasm

Build the web crate with `nih_plug` without default features (no `vst3`), and patch the one place that fails: the event loop. Options, in order:
1. An upstream-style `cfg(target_arch = "wasm32")` stub `OsEventLoop` in a vendored or patched NIH-plug (`[patch]`, the same mechanism as other forks), doing nothing (the web host has no background thread).
2. If the patch grows, feature-gate Corrosion's own use of NIH-plug types so the engine crate depends only on the parameter types.

The native plugin keeps the unpatched dependency: the patch applies only to the web crate's build (separate workspace member, or `cfg` on the target).

### 3. Engine bindings

A thin `wasm-bindgen` layer exposes `new(sample_rate)`, `set_param(id, normalized)`, `note(…)`, `load_state(json)`, `set_tempo(bpm)` and `process(out_l, out_r)`. The worklet processor (JS) passes its output buffers through preallocated wasm memory: no allocation per block, as the plugin's real-time rules require. The engine runs `initialize` with the AudioContext's sample rate and a 128-frame max block.

### 4. Editor on the main thread

`eframe` (egui 0.31, glow/WebGL2 backend) on a `<canvas>`, calling `render_ui` with the `WebGuiContext`. Changes needed in the editor, behind `cfg(target_arch = "wasm32")`:
- "Open folder" hidden.
- Preset browser: factory presets compiled in (`src/presets/factory/` is code already); user presets read from and written to IndexedDB through the host shell instead of the file system.
- Save offers a file download as well.

### 5. Host shell (JavaScript)

Plain JS, no framework: a start button (AudioContext needs a gesture), Web MIDI selection, a BPM field, a CPU meter, export/import of the session, upload/download of `.corrosion-preset` files, version, source link and license. The session autosaves to `localStorage` (debounced, about 1 s after the last change).

### 6. Performance defaults

The first spike task measures the engine's cost per 128-frame block in Chrome and Firefox, at each quality mode and 1, 4, 8 and 16 voices. Its numbers set the web defaults (quality, voice cap). If 16 voices at Normal don't fit, the web build lowers the voice cap; the plugin doesn't change. The page shows the worklet's load and warns above about 80 %.

### 7. Hosting: Cloudflare Workers with static assets

A Worker serves the built files (`wrangler.toml` with an `[assets]` directory), with long cache headers on hashed wasm and JS files and `application/wasm` content type. No server code beyond static serving.

**Build location (open question, decided in task 5.1):**
- **A.** Cloudflare's Git integration (Workers Builds) runs the build. It may lack `rustup` and the wasm target; installing them each build is slow.
- **B.** GitHub Actions builds the wasm on `main` and deploys with `wrangler deploy` (needs a Cloudflare API token as a repository secret). Preview deploys on PRs are possible.

B is the likely choice: the toolchain is already in CI, and the headless check runs before the deploy.

## Risks / Trade-offs

- **[CPU]** wasm may be 1.5–3× slower than native, and the worklet has 2.7 ms per block. → Measure first (task 1); lower web defaults; enable wasm SIMD (`+simd128`, supported in current Chrome, Firefox and Safari).
- **[NIH-plug patch]** Maintaining a patch on a dependency. → Keep it to the event-loop stub; document it; consider upstreaming.
- **[Two parameter copies]** The editor's mirror and the engine's copy can disagree after a lost message. → Messages are ordered and reliable within a page; state loads send the full state, not deltas.
- **[Browser support]** No Web MIDI in Safari; WebGL2 required. → Feature detection with clear messages; the keyboards always work.
- **[Bundle size]** Two wasm files with egui and the factory bank. → `opt-level = "s"` or `z` for the editor, `wasm-opt`, compression by Cloudflare. Measure and record.
- **[Sound mismatch]** gnu vs browser math libraries differ, so outputs won't be bit-identical. → Compare with a level and spectrum tolerance against the offline renderer.

## Open Questions

- Build in Cloudflare (A) or GitHub Actions (B)? Decided with the maintainer in task 5.1.
- Web voice cap and default quality: set from task 1's numbers.
- The public URL: a `*.workers.dev` subdomain or a custom domain?

## Measurements

<!-- Filled during implementation: wasm cost per block, bundle sizes, sound comparison. -->

### 1.1 The engine on wasm32 (As built)

`cargo check --lib --no-default-features --target wasm32-unknown-unknown` passes with three changes and no NIH-plug patch:
- `Cargo.toml`: NIH-plug keeps its default features (VST3) on every target except `wasm32`, which gets `default-features = false`; `assert_process_allocs` is left off wasm too. Resolver 2 keeps the features per target, so native builds are unchanged (`cargo tree -e features` shows `default` and `assert_process_allocs` on native, neither on wasm).
- `src/lib.rs`: `impl Vst3Plugin`, `nih_export_clap!` and the VST3 export module are `cfg(not(target_arch = "wasm32"))`.
- The event-loop stub of Decision 2 is not needed: without the `vst3` feature nothing uses `OsEventLoop`.

**Found here:** NIH-plug keeps parameter setters crate-private (`ParamMut`, `ParamPtr::set_normalized_value`, `update_smoother`, `wrapper::state`). Only its wrappers can change a parameter, so the web engine had no public way to apply the editor's changes.

**Decision (maintainer, 2026-11-12): patch NIH-plug for the web build only.** `web/` is a Cargo workspace of its own, with `[patch."https://github.com/robbert-vdh/nih-plug"]` pointing at `web/vendor/nih-plug`. `web/scripts/fetch-nih-plug.sh` clones NIH-plug at the revision `Cargo.lock` pins and applies `web/nih-plug-public-setters.patch` (two lines: `set_normalized_value` and `update_smoother` on `ParamPtr` become `pub`). The plugin's workspace never sees the patch. The code that calls the setters, `src/web.rs` (`WebEngine`), sits behind Corrosion's `web-host` feature, which only the web workspace enables.

### 1.2 Engine in an AudioWorklet (As built)

- `src/web.rs::WebEngine`: what a host does for the plugin. `Corrosion::prepare` (factored out of `initialize`, which now calls it: same steps, same order) at the page's rate, every smoother reset to its value, `reset()`. Parameters are set by index in `param_map()` order (the editor's `CorrosionParams` has the same order), MIDI goes through `NoteEvent::from_midi` into a 512-event queue played at the next block's first sample, and `process()` runs `Corrosion::process_block` into two preallocated 1024-frame buffers. `load_preset` moves every parameter to the preset's value (smoothed) and replaces the kit and its pad table.
- `web/engine`: C-ABI exports over a handle (`engine_new`, `engine_process`, `engine_left`/`engine_right`, `engine_midi`, `engine_set_param`, `engine_param_index`, `engine_load_preset`, `engine_set_tempo`, …). No wasm-bindgen: the worklet instantiates the module with no imports (the module has none).
- **Found here:** Chromium silently drops a `WebAssembly.Module` posted to an AudioWorklet port (the next message arrives, that one never does). The page posts the wasm bytes and the worklet compiles them.
- The worklet's message handlers only run once the `AudioContext` runs, so the page resumes the context before sending `init`.
- `web/scripts/build.sh` builds `web/dist` (wasm with `+simd128`, ~607 KB uncompressed, no `wasm-opt` yet). `web/scripts/smoke.cjs` checks it in headless Chromium: silence before the note (RMS 0), RMS 0.31 with one voice after it, no console errors.
- Tests (`web/engine/tests/engine.rs`, native): a note sounds and silence is silent; a parameter change reaches the engine (Output to 0 drops the level > 40 dB); the default patch matches the offline renderer within 0.5 dB RMS; a loaded factory preset matches its offline render within 1 dB. The engine API is new, so these weren't red first on old code: each fails if its step is removed (no `midi`, no `set_param`, no `load_preset`).

### 1.3 Cost per 128-frame block (budget 2667 µs at 48 kHz)

Measured in this container. V8 in Node 22 (`web/scripts/bench-engine.mjs`, `bench-presets.mjs`; V8 is Chrome's engine) against the native engine (`print_native_block_cost`, `print_preset_block_cost`, release). Firefox is not installed here: **not measured**. Single runs; this container's noise is ±10–15 %.

Default patch, held notes (µs per block):

| Quality | Voices | Native | wasm | wasm + SIMD |
|---|---|---|---|---|
| Eco | 16 | 87 | 184 | 129 |
| Normal | 1 / 4 / 8 / 16 | 34 / 53 / 79 / 133 | 46 / 69 / 107 / 186 | 46 / 120 / 195 / 188 |
| High | 16 | 171 | 280 | 235 |
| Render | 1 / 4 / 8 / 16 | 73 / 94 / 145 / 194 | 115 / 122 / 186 / 311 | 93 / 135 / 186 / 293 |

wasm costs 1.3–2.1× native. The default patch is light (12 % of the budget at Render with 16 voices).

The costliest factory presets (native, 16-note chord at each preset's own quality): `bow/sheet_metal` 1344 µs, `tension_snap/sheet_metal` 1189, `felt_mallet/sheet_metal` 1040, `particle_rain/sheet_metal` 1007, `metal_pipe/industrial_cog` 1006; median of the bank 406 µs. In wasm:

| Preset | 1 voice | 4 | 8 | 16 | 16, SIMD |
|---|---|---|---|---|---|
| `bow/sheet_metal` | 141 (5 %) | 516 (19 %) | 925 (35 %) | 1777 (67 %) | 1630 (61 %) |
| `tension_snap/sheet_metal` | 122 | 426 | 936 | 1714 (64 %) | 1628 (61 %) |
| `metal_pipe/industrial_cog` | 132 | 391 | 739 | 1543 (58 %) | 1412 (53 %) |
| `felt_mallet/chain` | 135 | 451 | 907 | 1661 (62 %) | 1605 (60 %) |

SIMD (`+simd128`, autovectorized only) saves 5–10 % on the heavy presets; the build uses it. The worst preset with all 16 voices takes 61 % of the block on this machine's core, so a laptop core about 1.6× slower than this container's would start dropping out there.

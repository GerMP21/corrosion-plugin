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

# Tasks

## 1. Spike: engine in an AudioWorklet

- [x] 1.1 Build the engine for `wasm32-unknown-unknown` without NIH-plug's `vst3` feature, with the event-loop stub (Decision 2), and record what had to change
- [x] 1.2 Minimal page: AudioWorklet running `process_block` on the default patch, one note on a button
- [x] 1.3 Measure the cost per 128-frame block in Chrome and Firefox (Eco, Normal, Render × 1, 4, 8, 16 voices), with and without `simd128`; record in design.md (V8 only: no Firefox in this container)
- [x] 1.4 Stop and ask the maintainer with the numbers: web voice cap and default quality

## 2. Bridge

- [x] 2.1 Test first: a param change, note on/off, kit table and state message each reach the engine and change its output (native test of the message decoder)
- [x] 2.2 Message protocol and engine bindings (Decision 3), no allocation per block (extend `no_alloc` to the decoder)
- [x] 2.3 `WebGuiContext` implementing `GuiContext`, posting changes and keeping the mirror
- [x] 2.4 Engine → editor: diode meter, voice count and load, every ~30 ms

## 3. Editor on a canvas

- [x] 3.1 eframe on a `<canvas>` calling `render_ui`; every panel, help badge, kit bar and piano strip shown
- [x] 3.2 `cfg(wasm32)`: hide "Open folder"; user presets through the host shell; Save also downloads
- [x] 3.3 Computer keyboard and piano strip play through the bridge (same key map as the plugin)

## 4. Host features

- [x] 4.1 Start-audio button and state (AudioContext gesture rule)
- [x] 4.2 Web MIDI: device selection, notes, velocity, bend, mod wheel, sustain; message when unavailable
- [x] 4.3 BPM field feeding the transport (Echo sync)
- [x] 4.4 Session autosave to `localStorage` and restore; export and import of the session
- [x] 4.5 User presets in IndexedDB; upload and download of `.corrosion-preset` files (plugin presets migrate)
- [x] 4.6 CPU meter and overload warning; version (commit), source link and GPLv3 notice

## 5. Build, CI and deploy

- [x] 5.1 Decide the build location with the maintainer (Cloudflare build or GitHub Actions + wrangler)
- [x] 5.2 Web build script (`wasm-pack`/`wasm-bindgen`, `wasm-opt`), static assets, `wrangler.toml`; record bundle sizes
- [x] 5.3 CI job: build the web app and run a headless Chromium check (page loads, start, note, RMS > 0, no console errors)
- [x] 5.4 Sound check: web engine vs offline renderer on a few presets within a level and spectrum tolerance
- [x] 5.5 Cloudflare Workers project connected to the repository, deployed from `main`
- [x] 5.6 Native plugins unchanged: render hashes of the 16 exciters and the bank tests match before and after

## 6. Close-out

- [x] 6.1 README (link, browser support, limits) and `docs/ARCHITECTURE.md` (web host, bridge, patch)
- [x] 6.2 fmt, clippy `-D warnings`, full test suite, GUI lane, no_alloc, cargo doc
- [x] 6.3 Update "Current state" in CLAUDE.md

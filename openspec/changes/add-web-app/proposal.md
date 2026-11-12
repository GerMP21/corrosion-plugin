# Proposal

## Why

Trying Corrosion today means downloading a zip, finding the plugin folder, clearing the macOS quarantine and opening a DAW. A free, public web page that runs the whole plugin in the browser, with the same engine and the same editor, lets anyone hear and test it in seconds, and lets the community report problems without installing anything.

## What Changes

- A web build of Corrosion: the unchanged engine compiled to WebAssembly and running in an AudioWorklet, and the unchanged editor drawn in a `<canvas>`.
- A small web host that plays the role of the DAW:
  - audio output at the browser's sample rate;
  - notes from Web MIDI, the editor's piano strip and the computer keyboard;
  - a tempo for Echo sync;
  - autosave of the session;
  - user presets stored in the browser, with download and upload of `.corrosion-preset` files.
- The editor and the engine run as two instances that talk through messages:
  - parameters, notes, the kit and the state go from the editor to the engine;
  - the diode meter goes from the engine to the editor.
- A free public site on Cloudflare Workers, connected to the GitHub repository and redeployed from `main`.
- A headless browser check in CI: the page loads, plays a note, and the output is not silent.
- The plugin builds (VST3, CLAP) and their behaviour don't change. Whatever the web build needs from NIH-plug is isolated behind a build target or feature, so native builds stay byte-for-byte as today.

## Capabilities

### New Capabilities
- `web-app`: Corrosion running fully in a web browser. It covers playing (audio, MIDI, keyboards), the full editor with every parameter, presets and kits, session persistence, the browser and performance limits, and public deployment.

### Modified Capabilities
<!-- None: the plugin's behaviour is unchanged. -->

## Impact

- **Code:**
  - a new web crate or target (the WebAssembly engine bindings and the web host);
  - a web `GuiContext`;
  - a message bridge;
  - small `cfg` gates where the plugin touches the OS: the event loop, preset folders, "Open folder", and the NIH-plug VST3 wrapper.
- **Build:**
  - the `wasm32-unknown-unknown` target and a wasm bundler (e.g. `wasm-pack`);
  - static assets (HTML, JS, the AudioWorklet processor);
  - Cloudflare Workers config (`wrangler`).
- **CI and deploy:** the web build in CI, a headless Chromium check, and the Cloudflare Workers project connected to the repository.
- **Docs:** README (link to the web app, its limits), `docs/ARCHITECTURE.md` (the web host and the bridge).
- **License:** the GPLv3 applies to the web build. The page links the source.

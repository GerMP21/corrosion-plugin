# Design

## Context

- **`Corrosion::process`** does per-buffer setup (smoothers, `apply_post_chain_params`, latency, live controls), then for each sample:
  - MIDI events;
  - `voice_manager.process_sample_stereo`;
  - `apply_drive` with the smoothed drive;
  - `post_chain.process`;
  - smoothed output gain;
  - the limiter (Lookahead or hard).
- **`render_presets`** builds its own `VoiceManager` and a standalone `BodyResonator`, and never touches `PostProcessingChain`.
- **The test renderer `render_frames`** (in `src/lib.rs`, test-only) rebuilds most of the path by hand: post-chain params, drive, post chain, output gain and the hard limiter.

## Goals / Non-Goals

**Goals:** one implementation of the output path, used by the host, the tools and the tests.

**Non-Goals:**
- A full host simulation (automation, transport).
- Changing any DSP.

## Decisions

1. **Extract `Corrosion::render_frame(&mut self, sample_rate, width, limiter_mode) -> (f32, f32)`,** covering voices → drive → post chain → gain → limiter.
   - `process()` keeps per-buffer setup and event handling, and calls it per sample.
   - The per-buffer setup is extracted as `Corrosion::begin_block(sample_rate, tempo)`, so offline code runs the identical setup.
   - *Alternative:* drive `process()` with a fake `ProcessContext` and `Buffer`. nih-plug's types are awkward to build outside a host, and it couples tools to the plugin API.
2. **`offline::render_preset(preset, notes, sample_rate, seconds) -> Vec<(f32, f32)>`.**
   - It builds a `Corrosion` with the preset's parameters and calls `begin_block` every 512 samples (a typical host block, so per-block parameter updates happen as in a host).
   - It sends note events at their frames, and collects `render_frame` output.
3. **Tools and tests use it.**
   - `render_presets` loses its bespoke path; its debug metrics run on the new output.
   - The lib test renderer becomes a thin wrapper, so the preset level tables measure the shipping path.
4. **Equality test:** a unit test runs the same preset and note through `begin_block`/`render_frame` as `process()` would sequence them, against `offline::render_preset`, and requires bit-identity.

## Risks / Trade-offs

- **[Refactoring `process()` could change real-time behaviour]** → The extraction is mechanical (no new state). `tests/no_alloc.rs` (debug) and the full suite gate it, and the C1 DC test exercises the post chain.
- **[Block size affects smoothing]** → The offline renderer uses a fixed 512-sample block. That is documented; output-gain and drive smoothers are per-sample anyway.

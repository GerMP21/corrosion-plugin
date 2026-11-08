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

## As built

- **Decision 1:** `process()` calls `Corrosion::process_block(block_len, sample_rate, host_tempo, next_event, fetch_next, write)`, which runs `begin_block` and then, per sample, the due MIDI events and one output frame. The frame is two inlined steps, `render_frame_unlimited` (voices → drive → post chain → output gain) and `limit_frame`, and `write` receives both the limited frame and the frame before the limiter. There is no single `render_frame`: the level checks need the pre-limiter frame to count limiter hits. `begin_block` returns a `BlockSetup` (sample rate, width, limiter mode and threshold), and `process()` reports latency from it after the block. The old `output_stage` is gone; its two tests use `limit_frame`.
- **Decision 2:** `offline::render_preset(preset, notes, sample_rate, seconds)` takes `OfflineNote`s (absolute frame, note, on with a 0–1 host velocity, or off), builds the plugin with `into_params()`, calls `reset()` as a host does before playback, and runs `process_block` in `OFFLINE_BLOCK_SIZE` (512) blocks. `render_preset_pre_limiter` returns the same render before the limiter.
- **Decision 4:** a test can't call `process()` itself, because `nih_plug::Transport` can only be built inside nih-plug (its constructor and several fields are `pub(crate)`). Sharing `process_block` makes the host and offline paths one function, so the equality tests compare `render_preset` against an independently written host-style driver (512-sample buffers, block-relative event timings handed over one at a time). A mutation check, delivering every event at its block's start, makes both tests fail.
- **Spec scenario "Post-chain preset":** no factory preset uses the post chain's drive (`drive_amount` is 0 in all 50; the ones with Space, BODY and some drive use the master Drive, a separate stage before the post chain). The test takes `sheet_lightning` (Space Factory, BODY 2.52, master Drive 1.5) and sets `drive_amount` to 1.0 before rendering, so it covers both drive stages. The factory presets are placeholders (see `CLAUDE.md`), so the scenario is about the stages, not about a preset file being unmodified.
- **Tools:** `render_presets` now writes **stereo** 16-bit WAVs (it wrote a mono mix) and releases the note at `--note-off` (default 1 s of the 2 s render, as the level checks do) so Space tails are heard. Velocity stays on the MIDI 0–127 scale on the command line and is passed as a 0–1 host velocity. The manifest's peak, RMS and checksum cover both channels; its behaviour metrics use the mono mix.
- **`PresetRenderer` (added after verification):** `offline::PresetRenderer` holds a plugin instance with a preset loaded (or, crate-internal, any `CorrosionParams`) and renders successive calls through `process_block`, keeping state between them. `render_preset` and `render_preset_pre_limiter` are thin wrappers over it. Four tests that hand-built the whole output path now use it: `plugin_reset_with_sounding_space_is_alloc_free_and_silent`, `hosted_normalized_note_velocity_reaches_output_chain`, the limiter tests' `render`, and `c5_default_body`. Their numbers didn't change (default note peak 0.451730; C5 default BODY 0.04 → peak 0.1180). Tests that feed a synthetic signal straight into the post chain (`chassis_volume_does_not_override_body_zero`, `post_chain_drive_rms`, the C6 echo impulse) probe that stage on purpose and keep doing so, as does `measure_preset`, a historical replica of the pre-C5 routing.
- **Test renderer:** `render_frames` takes BODY on the preset's 0–5 scale (not a 0–1 amount), so it sets `preset.body` without a round trip through ×5 and ÷5.

## Measurements

### Preset levels (tasks 1.1 and 2.2)

`preset_levels_are_sane` and `preset_levels_table` print identical text, for every factory preset, before the change, after task 1.1 and after task 2.2. Both pass, with no limiter hits.

### The proposal's "identical numbers" didn't hold at first

With `render_frames` switched to the offline renderer, the levels moved by up to −0.06 dB RMS and −1.8 % peak (Chain Whip Snap; Sheet Lightning −0.03 dB, Beam Strike Industrial −0.007 dB). Hit counts didn't change. Isolated one stage at a time on the old renderer: calling `reset()`, running `update_live_controls` every 512 samples and re-applying the post-chain parameters every 512 samples each left the output bit-identical. The whole difference came from the master-drive smoother. `reset()` started it at Drive **without** the Violence macro's drive bias, and `begin_block` then ramped it to Drive + bias (τ = 20 ms). The plugin did this after every host reset, for the 11 factory presets with Violence ≠ 0.5. Starting the smoother at the biased drive made the offline render bit-identical to the old renderer for those three presets.

The maintainer chose to fix `reset()`: it now starts the smoother at `clamp(Drive + bias, 0, 5)`, where `begin_block` aims it. This is a small DSP change, limited to roughly the first 100 ms after a reset with Violence ≠ 0.5. `reset_starts_the_drive_smoother_at_the_biased_drive` failed before the fix (first drive 1.4003, target 1.65) and passes after it.

### Audition set (task 3.1)

`cargo run --release --bin render_presets` re-rendered all 50 factory presets to `output/` (stereo, 16-bit, 48 kHz, 96 000 frames). Every preset has a WAV, and `scripts/check_wav.py` passes on all 50. The set is now about 19 MB, twice the mono set.

### `process()` cost before and after (added after verification)

A test can't call `process()` (see Decision 4), so the comparison used a temporary, uncommitted replica of `process()`'s body on each tree: the base commit before H13 (`79d4714`, in a clean worktree with its own target dir) and this branch. Each replica was copied verbatim, with the host context replaced by locals, and wrote into a real nih-plug `Buffer`. Scenario: `plate_resonance_bath` (Space Factory, BODY 2.65, Violence 0.5, so the `reset()` fix doesn't change it), 16 notes, 400 blocks of 512 samples, release build, 9 runs each. Rounds alternated before and after.

| Round | Before (median of 9) | After (median of 9) | After / before |
|---|---|---|---|
| 1 | 235.92 ms | 230.44 ms | 0.977 |
| 2 | 238.34 ms | 233.14 ms | 0.978 |
| 3 | 233.17 ms | 229.52 ms | 0.984 |

The output hash was identical on both trees (`63dc088664ee6f21`), so the extraction into `process_block` is bit-identical through the real per-buffer path. The time is about 2 % lower, inside this container's noise: no regression. `benches/performance.rs` gains `plugin_output_path_sixteen_voices` (`sheet_lightning`, 16 notes, 4 096 samples through `PresetRenderer`, plugin built outside the timed part), about 18.8 ms on the first run, as a baseline for future changes.

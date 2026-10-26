# Proposal

## Why

`src/bin/render_presets.rs` renders each preset as the voice manager, then master drive, then a standalone `BodyResonator`, then a hard clamp. That's not the plugin's output path. It leaves out the post chain (filter, Lorenz drive, spread, Space, clipper), the macros' biases, output-gain smoothing and the selected limiter mode. Its comment says it exists so presets are auditioned "as the plugin would sound", but the `output/*.wav` audition set doesn't sound like the plugin. That is why C1, the filter's DC rail, was never heard in renders (fable review 2026-07-07, finding **H13**).

## What Changes

- **One shared output path.** The plugin's per-sample output path becomes a single method on `Corrosion`:
  - voices;
  - master drive;
  - post chain;
  - output gain;
  - limiter.

  `process()` calls it, and so does a new offline renderer in `corrosion::offline`, so the two can't drift apart.
- **`render_presets` renders through that renderer,** with the preset loaded into a real `Corrosion` instance (macros, BODY, Space, limiter mode, everything). The duplicated drive/body/limiter code in the binary is removed.
- **The test renderer** used by the preset level checks (`render_frames` in `src/lib.rs`) uses the same offline renderer, replacing its own copy of the path.
- **Re-render the audition set** in `output/`.

## Capabilities

### New Capabilities
- `preset-rendering`: offline renders of presets match what the plugin outputs.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:**
  - `src/lib.rs`: extract the per-sample output path from `process()`, and switch the test renderer;
  - `src/offline/mod.rs`: a preset renderer;
  - `src/bin/render_presets.rs`: use it.
- **Tests:** the preset level checks now measure through the shared path, which should give identical numbers, since the test renderer already ran the same stages. They must still pass.
- **Artifacts:** `output/*.wav` re-rendered.
- **Docs:** the README's render instructions, and the review's H13 status.

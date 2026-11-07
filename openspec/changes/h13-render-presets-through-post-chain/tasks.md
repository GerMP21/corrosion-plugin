# Tasks

## 1. Shared output path

- [ ] 1.1 Extract `begin_block` and `render_frame` from `Corrosion::process` without changing behaviour. Verify the full test suite and `tests/no_alloc.rs` (debug) pass, and that `preset_levels_are_sane` reports the same numbers as before (record them).
- [ ] 1.2 Add `offline::render_preset` (512-sample blocks, note events, stereo frames), and a bit-identity test against the `process()` sequencing for a Space + drive + BODY preset and for a Lookahead preset. Verify both pass.

## 2. Tools and tests

- [ ] 2.1 Switch `src/bin/render_presets.rs` to `offline::render_preset` and remove its private drive/body/limiter path. Verify `cargo run --bin render_presets -- --limit 3` writes WAVs equal (after 16-bit quantization) to the renderer's output, with a small check in the binary's tests or a script.
- [ ] 2.2 Make the lib test renderer (`render_frames`) a wrapper over `offline::render_preset`. Verify `preset_levels_table` and `preset_levels_are_sane` give the same numbers as in 1.1.

## 3. Artifacts, docs and close-out

- [ ] 3.1 Re-render `output/*.wav` with `render_presets`, and update the README render instructions. Verify every preset has a WAV and that `scripts/check_wav.py` passes on a sample of them.
- [ ] 3.2 Mark H13 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy and the full suite, and verify all are green.

# Tasks

## 1. Tests first

- [x] 1.1 Add the `voice-mix-dc` tests (held bow, felt mallet strike against the bypassed path, low sines, a note without an offset, other rates) and a test-only switch to bypass the blocker. Verify the DC tests fail on the current code and the low-end tests pass; record today's values in `design.md` under Measurements.

## 2. The blocker

- [x] 2.1 Add the per-channel one-pole DC blocker on the voice mix before the master drive (decisions 1–4), cleared by `reset()` and the panic clear, rated from the host rate. Verify the `voice-mix-dc` tests pass at 44.1, 48 and 96 kHz, and `tests/no_alloc.rs` in debug. Stop and ask if 5 Hz can't meet the strike and low-end bounds together.
- [x] 2.2 Measure the output DC across the factory bank before and after (share of power, largest 20 ms mean), and the CPU of `plugin_output_path_sixteen_voices` against a clean worktree (median of 5). Record both.

## 3. Close-out

- [x] 3.1 Re-run `preset_levels_are_sane` and record the result (no level pass: the factory presets are placeholders, maintainer 2026-11-10; three presets fall under its floor, see design.md). Run the full suite on gnu and musl, fmt, clippy `-D warnings`, `cargo doc`.
- [x] 3.2 Document the blocker in `docs/ARCHITECTURE.md` (signal chain, voice output) and the README; update `CLAUDE.md`'s current state and execution order. Then resume `add-asymmetric-diode-stage` (re-measure its levels and its blocker's open question).

# Tasks

## 1. Failing stealing tests

- [ ] 1.1 Add the `voice-allocation` stealing scenarios as unit tests in `src/voice/manager.rs`, written against `MAX_VOICES` so they hold at any pool size. Cover:
  - tail stolen before held notes;
  - rapid notes over a full pool (one every 64 samples, after the pool has been held 100 ms);
  - the quiet young note surviving, with its level measured in the test, not assumed;
  - every held note young, so the oldest is stolen;
  - the 100 ms window at 44.1 and 96 kHz.

  Verify that the tail-first, rapid-notes and quiet-young-note tests FAIL on the current code (`cargo test --no-default-features --lib voice::manager`).

## 2. Stealing order

- [ ] 2.1 Track the sample rate passed to `process_sample` / `process_sample_stereo` (default 48 kHz) and derive the 100 ms protection window from it. Verify the 44.1 / 96 kHz window test passes.
- [ ] 2.2 Replace the steal selection with the three-tier key from design Decision 1:
  - tailing voices first, then held voices at least 100 ms old, then young held voices;
  - within the first two tiers, quietest first, oldest losing ties;
  - within young held voices, oldest first.

  Keep NaN peaks counting as quietest. Verify that every 1.1 test passes, along with `steals_quietest_voice_when_all_active`, `new_note_uses_unused_slot_before_released_tail` and `non_finite_peak_hold_is_preferred_for_stealing`.
- [ ] 2.3 Update the doc comments on `note_on_with_controls` and the module so they describe the new order. Verify that `cargo doc --no-deps --no-default-features` gives no new warnings.

## 3. Sixteen voices

- [ ] 3.1 Set `MAX_VOICES = 16`, build the pool with `std::array::from_fn`, and rename the tests that name a count (`eight_simultaneous_notes_audible`, `ninth_note_does_not_crash`) to match the spec's 16/17 scenarios. Verify that the manager tests and `tests/automation_stress.rs` pass.
- [ ] 3.2 Run `tests/no_alloc.rs` in a **debug** build (`cargo test --no-default-features --test no_alloc`). Verify that the stealing test now fills 16 voices and steals 16 more with no allocation, and fix any comment that still says 8.
- [ ] 3.3 Add a 16-voice benchmark next to the 8-voice one in `benches/performance.rs`. Record Normal- and Render-quality 16-voice costs (% of one core, real-time at 48 kHz) in `openspec/changes/fix-voice-stealing-16-voices/design.md` under "Implementation findings". Verify the numbers are recorded.
- [ ] 3.4 Update the docs:
  - the README polyphony line, including the CPU note, and the layout line;
  - `docs/ARCHITECTURE.md`: the `MAX_VOICES` diagram, the `manager.rs` line and the memory note;
  - the `docs/PRD.md` "implemented" polyphony row;
  - the 8-voice headroom comment in `src/voice/mod.rs`.

  Verify with `grep -rn "8-voice\|8 voices\|MAX_VOICES = 8" README.md docs/ARCHITECTURE.md src` that nothing stale remains. Historical requirement text in the PRD and the dated code review stays as is.

## 4. Integration

- [ ] 4.1 Run the full gate:
  - `cargo test --workspace --no-default-features` (debug);
  - `cargo test --no-default-features --release`;
  - `cargo fmt --check`;
  - `cargo clippy --workspace --all-targets --no-default-features -- -D warnings` on Rust 1.97 and 1.99;
  - the render CLI smoke test and `scripts/check_wav.py`;
  - `c5_verify_revoiced_presets` (single-note preset levels must be unchanged).

  Verify all green.

# Tasks

## 1. Drive range and mix

- [ ] 1.1 Add unit tests for the spec scenarios: wet shares 0.2/0.6/1.0 at amounts 1/3/5, Violence offset pushing past 1, and unity output for a small signal at amounts 0/2.5/5 (chaos and starvation 0). Verify the range and Violence tests fail on the current code.
- [ ] 1.2 Widen the clamp in `apply_post_chain_params` and in the test renderer's copy to `0.0..=5.0`. Verify the range and Violence tests pass.
- [ ] 1.3 Change `LorenzDrive`'s mix to `wet·w + input·(1 − w)`. Verify the unity-output test passes, along with the existing post-chain and drive tests (`cargo test --no-default-features --lib post`).
- [ ] 1.4 Update the `drive_amount` parameter docs in `src/params.rs` (0–5 maps to 0–100 % wet) and the `LorenzDrive` doc comment. Verify `cargo doc --no-deps --no-default-features` adds no warnings.

## 2. Presets and close-out

- [ ] 2.1 Run `preset_levels_are_sane` and `tests/preset_roundtrip.rs`. Verify both pass, and record the three drive presets' level change in the review.
- [ ] 2.2 Mark H1 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md` with the change name. Run `cargo fmt --check` and `cargo clippy --no-default-features --all-targets -- -D warnings`, and verify both are clean.

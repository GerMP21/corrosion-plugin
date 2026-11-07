# Tasks

## 1. Remove

- [ ] 1.1 Confirm there are no callers (`grep -rn "get_state\|load_state" src tests benches`). Delete both methods and any now-unused imports. Verify `cargo build --no-default-features` and the default-feature build succeed.
- [ ] 1.2 Remove mentions of them from doc comments, `docs/ARCHITECTURE.md` and the README, if any. Verify `grep -rn "get_state\|load_state" docs README.md src` is empty.

## 2. Close-out

- [ ] 2.1 Mark H8 FIXED in `FABLE-CODE-REVIEW-2026-07-07.md`. Run fmt, clippy and the full test suite, and verify all are green, including the host state round-trip tests (`tests/preset_roundtrip.rs`).

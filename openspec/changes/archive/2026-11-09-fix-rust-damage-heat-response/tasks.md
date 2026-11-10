# Tasks

## 1. Tests first

- [x] 1.1 Add the `material-transforms` tests: Rust sweep, Rust eats the highs first, Damage sweep, Damage splits modes, Heat darker, Heat warble (all nine objects where the scenario says so). Verify the Rust sweep, Damage sweep, Heat darker and Heat warble tests fail on the current code, and record the failing values in `design.md` under Measurements.

## 2. Rust and Damage

- [x] 2.1 Rewrite `corroded` as a damping increase (decision 1). Verify both Rust tests pass and `resonator-level`'s loudness tests still pass.
- [x] 2.2 Rewrite `damaged` with the saturating amount (decision 2). Verify both Damage tests pass, and "Damage on versus off" (within 2 dB) still passes.

## 3. Heat

- [x] 3.1 Replace Heat's uniform gain loss with the spectral tilt and high-mode decay loss (decision 3). Verify the Heat darker test passes.
- [x] 3.2 Add the thermal warble as a pitch dynamic (LFO state on the resonator, advanced in `prepare_sample`, included in `has_dynamics`). Verify the warble test passes and `tests/no_alloc.rs` passes in debug.

## 4. Fixture, docs and close-out

- [x] 4.1 Regenerate the golden mode-bank fixture with the reason in its doc comment, and verify it matches on gnu and musl. Record the final constants and the measured sweep tables in `design.md`.
- [x] 4.2 Update `docs/ARCHITECTURE.md`'s transformations table and the README if it describes these controls. Run fmt, clippy `-D warnings` and the full suite, and verify all are green.

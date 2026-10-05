# Tasks

## 1. Capture current behavior

- [x] 1.1 Add a golden-output unit test in `src/dsp/envelopes/mod.rs` that records 3 s held + 1 s released at 48 kHz for loop Off (start 3/end 4) and Forward 3 → 4 with default stage times, using values captured from the current code and stored in the test (hash or decimated samples); verify it passes on the unmodified code
- [x] 1.2 Add a test that runs Forward with start 4 / end 4 for one sample on a dedicated thread with a small stack and asserts the thread completes; verify it FAILS (stack overflow) on the current code *(release builds hang instead of overflowing; the test uses a 10 s deadline so it fails on either)*

## 2. Remove recursion

- [x] 2.1 Rewrite `process_sample()` as a single time advance followed by a capped iterative transition loop (`MAX_TRANSITIONS = 8`), with no call to `process_sample()` from inside itself; verify 1.2 now passes and `grep -n "self.process_sample()" src/dsp/envelopes/mod.rs` only matches outside the method
- [x] 2.2 Resolve the extra-`dt`-per-transition question from design Decision 2 by running 1.1; keep or drop the behavior so 1.1 stays bit-identical, and document the choice in a code comment; verify 1.1 passes

## 3. Loop semantics

- [x] 3.1 Normalize bounds in `set_loop()` (start and end clamped to 1..=4, `loop_active = mode != Off && start < end`) and trigger the loop on entering the end stage while the gate is held; verify 1.1 still passes *(implemented as `loop_region()` evaluated per transition rather than a cached flag — see design Decision 3)*
- [x] 3.2 Add unit tests: Forward 1 → 3 never enters Decay or Sustain while held; start 4/end 4 holds sustain; start 3/end 1 equals loop Off output; gate release during a 3 → 4 loop reaches Idle within release time + one buffer; verify all pass
- [x] 3.3 Add the exhaustive sweep test (mode 0–2 × start 0–5 × end 0–5, 1 s held + 1 s released at 48 kHz) asserting every sample is finite and in [0, 1], plus a check that the transition cap is never hit for default and Drone settings; verify it passes
- [x] 3.4 Update the `loop_start_stage` / `loop_end_stage` / `loop_mode` doc comments in `src/params.rs` to describe the clamping, the start < end rule, and PingPong's current behavior; verify `cargo doc --no-deps --no-default-features` builds

## 4. Voice-level guard and close out

- [x] 4.1 Add a test in `tests/` that plays a friction-family note (e.g. Bow) through `VoiceManager` with loop Forward, start 4, end 4 for 2 s and asserts finite output; verify it passes
- [x] 4.2 Run `cargo test --no-default-features --release`, `cargo fmt --check`, and `cargo clippy --no-default-features --all-targets`; verify all green with no new warnings
- [x] 4.3 Update `FABLE-CODE-REVIEW-2026-07-07.md` C2 status from IN PROGRESS to FIXED with the commit reference (noting PingPong remains partial); verify the heading and priority list reflect the new status

## 5. Verification follow-up

- [x] 5.1 Run `every_loop_configuration_is_bounded` on a thread with a 60 s deadline (a release-build regression hangs rather than overflows) and assert every configuration reaches Idle after release
- [x] 5.2 Add a positive check to `custom_loop_end_restarts_before_decay` (at least two Hold → Attack restarts in 2 s) and assert `release_exits_running_loop` actually enters Release from a looping stage
- [x] 5.3 Make `is_loop_enabled()` report whether a non-empty loop region is set (empty regions such as Forward 4 → 4 never loop), move the stray `set_loop` doc line, and state the real longest transition chain in the `MAX_TRANSITIONS` comment
- [x] 5.4 Note in the code review that bit-identity held until C4 (commit `3540048`), and mark the replaced `handle_loop()` in this change's proposal and design

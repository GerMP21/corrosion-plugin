# Design

## Context

`SpringReverb` (`src/dsp/post_processing/space.rs`) is a mono model: one `[f32; 8192]` delay line, a `delay_index`, a `delay_length` (80 ms × (0.5 + tank size), clamped 1000–8000 samples), and a one-pole `dispersion_state` in the feedback path. `PostProcessingChain` owns one instance and, in Spring mode, calls `process()` twice per frame (left, then right). `FactoryEcho` already keeps separate left/right buffers, which is the pattern to match. `SpringReverb::reset()` clears the buffer and dispersion state but not `delay_index`.

## Goals / Non-Goals

**Goals:**
- Per-channel spring state; correct delay timing; no cross-talk.
- Mono in → identical tails, so mono sources don't suddenly gain a stereo image.

**Non-Goals:**
- Stereo decorrelation (e.g. slightly different spring lengths per side). This is a possible sound-design follow-up, but it would change mono sources and isn't needed to fix the bug.
- Changing the spring model, its parameters or ranges.
- Factory reverb, which already processes a mono sum by design, and the echo, which is already stereo.

## Decisions

1. **Two `SpringReverb` instances in `PostProcessingChain` (`spring_left`, `spring_right`).** `set_spring_params`, `set_sample_rate` and `reset` fan out to both; Spring mode calls each with its own channel. It is the smallest correct change and reuses the existing tested mono model unchanged. Alternative: rewrite `SpringReverb` with stereo buffers like `FactoryEcho` — rejected because it duplicates logic for no behavioural gain. The two-instance approach is the same thing structurally.

2. **`reset()` also zeroes `delay_index`.** Then a reset spring starts from a known position, and the "reset clears both channels" scenario is deterministic. Output after a reset was already silent (the buffer is zeroed), so this changes no audible behaviour.

3. **Extract the Space stage into `PostProcessingChain::process_space(left, right) -> (wet_l, wet_r)` and test that.** The bug is in how the chain calls the spring, so the tests must go through the chain's own Space code, not `SpringReverb` alone. But the stages before it (the mono body remix today, HRTF spread when width > 0) cross-mix channels on their own, so a whole-chain left-only test would fail for reasons unrelated to C6. Testing the extracted stage isolates the spring routing and keeps this change independent of C5. `process()` calls `process_space` and adds the wet × space amount as before. For the echo-timing scenario, set tank size so `delay_length` is a known value and assert the first wet sample index.

## Implementation findings

- **The mono-input test failed on the old code too.** Identical L/R inputs did not give identical tails: the shared dispersion state is updated twice per frame, so the interleaved channels diverge from the first echo on. Symmetry didn't hide the bug.
- **Echo timing before the fix was exactly half a delay:** the first echo came at 1,920 samples for a 3,840-sample spring, on the left channel.
- **Preset audition (`audition.md`):** both Spring presets now echo at 80 ms instead of 40 ms. Level moved by under 0.2 dB and neither hits the limiter. Both feed the spring a nearly mono signal (spread width 0.5), so the cross-talk itself was barely audible on them.
- **Spring Reverb Tap was re-voiced (BODY 1.60 → 2.26)** to keep C5's per-preset level guarantee. The fix moved it to −0.63 dB from its pre-C5 RMS, past the 0.5 dB that `c5_verify_revoiced_presets` allows. C5's own matching procedure gave the new value. The entry in `C5_REVOICE` (`src/bin/seed_presets.rs`) carries a comment, and only that preset file was regenerated.
- **The C5 preset harness gained `render_preset_frames`**, which returns the stereo frames. `render_preset` now computes its stats from that, with unchanged results. The new ignored test `c6_spring_preset_measurements` uses it.

## Risks / Trade-offs

- [The two spring presets sound different (longer echo, no cross-talk)] → Intended. Mention them by name in the summary so they can be auditioned.
- [Extra 32 KB per plugin instance] → Negligible; it is allocated at construction, not on the audio thread (no new allocation path; the `tests/no_alloc.rs` reset test covers it).
- [Interaction with C5's routing change] → Independent. C5 changes what feeds the spread stage; Spring still receives the spread output per channel. Whichever lands second re-runs the spring tests.

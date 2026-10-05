# Design

## Context

Note-on call chain (all inside `Plugin::process()`):

```
handle_note_event (lib.rs)
 → VoiceManager::note_on → Voice::note_on_with_controls
   → self.resonator = ModalResonator::with_algorithm_controls_and_note(...)
       generate_algorithm_modes   → per-object generate_modes() -> Vec<ModalModeSpec>
       transform_algorithm_modes  → new Vec; ModalModeSpec::damaged() -> vec![..] per mode (1 or 2 entries)
       from_mode_specs            → .collect::<Vec<SecondOrderMode>>()
                                    BidirectionalInteractionBus::initialize → Vec::resize
   (old resonator dropped → its Vecs are freed)
```

Bank size = base mode count of the object profile (largest is IndustrialCog, ≈12) × quality scale (clamped 0.25–4.0, 2.0 at Render) × up to 2 (damage companion). `Plugin::reset()` replaces `VoiceManager` and `PostProcessingChain` wholesale.

The existing allocation check (`assert_no_alloc`) is only available on non-Windows targets; Windows remains unchecked as documented in `docs/BACKLOG.md` P4.

## Goals / Non-Goals

**Goals:**
- Zero allocations/deallocations on every audio-thread path in the spec.
- Bit-for-bit (within 1e-6) identical mode banks.
- Tests that exercise the real note-on path inside the no-alloc zone.

**Non-Goals:**
- The per-sample coefficient rebuild for dynamic objects / pitch bend (review H12) — performance, not allocation.
- Modes above Nyquist (H11).
- Enabling the allocation check on Windows.
- Lock-free background bank construction (see Decision 1 alternatives).

## Decisions

1. **Preallocated per-voice storage, rebuilt in place.** Each `Voice` owns its `ModalResonator`, whose `modes` vec, interaction-bus coefficients, and two scratch vecs (`raw`, `transformed`) are created with capacity `MAX_MODES` at `Voice::new()`. Note-on calls a new `ModalResonator::rebuild_in_place(&mut self, …)` that `clear()`s and refills those buffers. `clear()` + `push()` within capacity never allocates, and nothing is dropped. Alternatives:
   - *Fixed-size arrays (`[SecondOrderMode; MAX_MODES]` + len)*: also allocation-free, but touches every resonator loop and grows the `Voice` struct on the stack; vec-with-capacity is a smaller diff with the same guarantee.
   - *Build banks off-thread and swap lock-free*: adds latency between note-on and sound plus a concurrency mechanism; overkill since generation is cheap.

2. **Generators write into a caller-provided buffer.** Change the generator trait from `fn generate_modes(...) -> Vec<ModalModeSpec>` to `fn generate_modes_into(..., out: &mut Vec<ModalModeSpec>)`. Keep a thin `generate_modes()` wrapper (allocating) for offline/tests so call sites outside the audio path are unchanged.

3. **`damaged()` returns a fixed-size result.** Return a small stack value (e.g. `([ModalModeSpec; 2], usize)` or an iterator over at most two modes) instead of `Vec<Self>`. The transform pass pushes into the preallocated `transformed` buffer.

4. **`MAX_MODES` is derived, then enforced.** Define `MAX_MODES = MAX_BASE_MODE_COUNT × 4 (scale clamp max) × 2 (damage)`, where `MAX_BASE_MODE_COUNT` is computed from the profile tables. A unit test asserts every object × quality × extreme character value produces ≤ `MAX_MODES` modes. At runtime, `rebuild_in_place` truncates at capacity rather than pushing past it, so an out-of-bound generator degrades (drops highest modes) instead of allocating.

5. **`reset()` resets in place.** Add `VoiceManager::reset()` and `PostProcessingChain::reset()` (the latter likely composes existing per-stage `reset()` methods) and call them from `Plugin::reset()` instead of constructing new instances. Sample-rate-dependent buffer sizing stays in `initialize()`, which hosts call off the audio thread.

6. **Test the real path.** Rewrite `tests/no_alloc.rs` so note-on/off, stealing, pitch bend, and reset run inside `assert_no_alloc`. Add a plugin-level test that sends note events through `process()` if the existing test harness (`tests/daw`) supports it; otherwise cover through `VoiceManager`, which is the same code path. *(Verification follow-up: `plugin_reset_with_sounding_space_is_alloc_free_and_silent` in `src/lib.rs` resets the whole plugin under `assert_no_alloc` with notes ringing through Echo and Spring, then checks the full chain is silent.)*

7. **Golden mode-bank test before refactoring.** Capture mode banks from the current implementation (object × quality × damage 0/0.5/1) into a test fixture first; the refactor must reproduce them.

## Implementation findings

- **The allocation check was a no-op in release builds.** `assert_no_alloc` defaults to `disable_release`, and nih-plug only installs its checking allocator under `debug_assertions`. A probe that allocated inside `assert_no_alloc` passed in `--release`, and aborted in debug. `tests/no_alloc.rs` is only meaningful in debug builds, which CI uses (`cargo test --workspace --no-default-features`); its header and the README/ARCHITECTURE notes now say so. A violation aborts the process, so failing cases had to be found one test at a time.
- **Exciters allocated too.** The review only listed the modal bank. Three exciters grew `Vec`s on the audio thread. (The per-exciter note-on test first iterated selectors 0..=15, so it never ran ParticleRain, selector 16; the verification follow-up corrected it to 1..=16, and all three now run under the check.) MetalChain grows `links` on trigger, and WireBrush reserved 100 impulses against a clamp of 5000. ParticleRain grows `particles` while processing. Each now preallocates to its own clamp (20 links, 5000 impulses). Particle rain is capped at 64 live particles, above its physical ceiling of 60 (≤ 2 spawns per sample × ≤ 30-sample lifetime), so the sound is unchanged.
- **`MAX_MODES` = 96**, computed at compile time from the profile tables (`ModalProfile::from_id` / `mode_count` are now `const fn`). The largest bank actually produced is 40 (Render quality, damage on).
- **Reset leaves voices in their natural idle state** instead of matching `Voice::new()` field by field. It clears rendering flags, resonator filter history, envelope and excitation state, and expression. That is the same state a voice reaches when its tail ends, and the next note-on rebuilds the bank and re-triggers the exciter. `PostProcessingChain::reset()` already existed and resets in place. Post-chain parameters are re-applied every buffer, so nothing is lost versus constructing a new chain.
- `with_algorithm_controls_and_note` now builds an empty preallocated resonator and calls `rebuild_in_place`, so the golden fixture exercises the in-place path. A second test rebuilds one reused resonator across all 108 cases (growing and shrinking the bank) and requires each to match a fresh build.

## Risks / Trade-offs

- [`MAX_MODES` too small for a future object] → Decision 4's test fails on any new object that exceeds it; runtime truncation keeps audio-thread safety. *(Verification found the first version of that test checked the already-truncated bank and could not fail. It now checks the generator's raw count against `MAX_RAW_MODES` and the untruncated damage expansion against `MAX_MODES`; `rebuild_in_place` debug-asserts the raw bound, since a raw overflow would reallocate the scratch buffer before truncation.)*
- [Generator refactor subtly reorders modes] → Golden fixture (Decision 7) compares order and values.
- [Memory per voice grows] → ~96 modes × (spec + second-order state) × 8 voices is a few tens of KB total; acceptable.
- [Allocation hidden in other note-on helpers not listed in the review] → The rewritten no-alloc test is the authority; any remaining allocation fails it and gets fixed within this change.

## Verification follow-up findings

- **Voice stealing reuses the newest voice.** The strengthened steal test showed that eight rapid note-ons over a full pool all stole the same slot: quietest-peak stealing ranks a voice that is only 64 samples old as the quietest, so each new note replaced the previous new note while the eight old notes kept sounding. This is allocation-free and outside C3's scope; it is recorded here as a stealing-policy question. The test asserts that each note-on steals a slot and sounds, not which slot it takes.

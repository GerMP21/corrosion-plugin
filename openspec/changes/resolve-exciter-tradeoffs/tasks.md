# Tasks

## 1. Baseline and failing tests

- [x] 1.1 Confirm the four completed changes are archived and their specs are in `openspec/specs/` (`openspec list --specs` shows `resonator-level` and `sample-rate-independence`). Record the 16-voice benchmark baseline (Normal and Render, 4 096 samples, gnu release) in `design.md` under "Implementation findings". Verify the numbers are recorded.
- [x] 1.2 Write the spec scenarios as tests, all failing on the current code. Remove the per-exciter tolerances, the 20 ms timing windows and the 0.01 → 1e-3 threshold change, so the existing tests state the spec again.
  - `src/voice/mod.rs`:
    - the median-level sweep;
    - the all-16-exciter overdrive sweep;
    - the excess-drive scenario;
    - the stability sweep with overdrive protection off (behind a test-only switch);
    - the strikes-come-to-rest scenario.
  - `src/dsp/resonators/core.rs`: the 15 kHz mode level across rates, and the 16→22 kHz sweep click check.

  Verify each new or tightened test fails, and record the measured numbers in the findings.

## 2. Noise and resonator foundations

- [x] 2.1 Add `dsp::utils::noise::Noise` (xorshift32, seeded per voice and note, uniform in [−1, 1)), with unit tests for determinism and a flat spectrum (equal power in four octave bands, within 1 dB). Inventory every `sin(...)·43758` generator (`grep -rn 43758 src`) and replace them all; per-sample noise forces go through `rate::white_noise`. Verify the grep is empty and the noise tests pass.
- [x] 2.2 Mode amplitude `b0/ω` (gain × `sin ω / ω`) and the 18–20 kHz raised-cosine output fade, from each mode's current frequency, replacing `MODE_CEILING_HZ`. Update the note-on estimate to match and re-measure `STRIKE_LEVEL_REF` for the default patch. Verify:
  - the 15 kHz and sweep tests pass;
  - `loudness_does_not_depend_on_pitch`, `objects_are_comparably_loud` and `quality_mode_and_damage_do_not_change_loudness` still pass;
  - the sustained-level test passes up to 192 kHz.

  Re-capture `tests/fixtures/mode_bank_hashes.txt` with the reason in its comment, and confirm it matches on musl and gnu.
- [x] 2.3 Cache the interaction bus's strike-position weights. Recompute only when the effective position changes, and precompute the per-mode blend. Verify `rebuilding_a_reused_resonator_matches_a_fresh_one` and the interaction tests pass, `tests/no_alloc.rs` passes in debug, and the 16-voice benchmark is recorded in the findings.
- [x] 2.4 Extend `stabilize-mallet-and-grinding-contact`'s `contact_prediction()` to include the high-band fade weights. Verify its prediction test still passes.

## 3. Exciters

- [ ] 3.1 Reuse `stabilize-mallet-and-grinding-contact`'s `dsp::utils::contact` solver. HardMallet and HeavyGrinding are already done there. Pass the compliance through `process_sample` for the position- or velocity-dependent exciters (design Decision 2; confirm the list by reading each exciter). Convert:
  - HandStrike, FeltMallet, Drumstick, MetalPipe and MetalChain;
  - ParticleRain, StiffPoint, CorrugatedDrag, TensionRise and TensionSnap;
  - Bow and PneumaticJet where they read velocity.

  Verify the stability sweep passes.
- [ ] 3.2 Strikes come to rest (HardMallet already leaves after its strike, from `stabilize-mallet-and-grinding-contact`): Drumstick goes inactive at the bounce limit or below the rest velocity. Verify the rest scenario passes and Drumstick and HardMallet are both timed on the whole-note peak.
- [ ] 3.3 Sub-sample events (design Decision 5):
  - ParticleRain spawns;
  - WireBrush impulses;
  - TensionSnap and TensionRise spikes;
  - MetalChain impacts;
  - HandStrike's snap sample;
  - the Hit envelope's attack sampled at sample centres.

  Verify each with a unit test that one event's area and centre of mass match at 44.1, 48 and 96 kHz.
- [ ] 3.4 MetalChain rattle: a one-pole high-pass with kept state and coefficient `exp(−2π·fc/fs)`, with a unit test of its −3 dB point. Remove the ARCHITECTURE §12 note about the bug. Verify the test passes.

## 4. Drive levels and governor

- [ ] 4.1 Add the per-voice drive governor (design Decision 1): follower 1 ms attack and 200 ms release through `rate::`, threshold 0.7, and a test-only off switch. Verify `tests/no_alloc.rs` passes in debug and the excess-drive scenario passes.
- [ ] 4.2 Replace `EXCITER_DRIVE_TRIM` with `EXCITER_DRIVE_LEVEL`, set so each exciter's median pre-clamp peak over the sweep is about 0.35, with `print_exciter_overdrive` reporting the medians. This includes HeavyGrinding, whose level `stabilize-mallet-and-grinding-contact` deferred to the governor: its median sits about 19 dB under the struck exciters because IndustrialCog C2 responds about 40× more than its median note. Verify:
  - the median-level, overdrive and excess-drive scenarios pass;
  - `every_exciter_has_the_same_level_at_any_sample_rate` and `struck_attack_timing_does_not_depend_on_sample_rate` pass with no per-exciter exceptions;
  - `sixteen_simultaneous_notes_all_sound` passes at 0.01;
  - `hosted_normalized_note_velocity_reaches_output_chain` passes.

  Record the per-exciter medians, how many sweep notes engage the governor and by how much, and the sample-rate numbers at 44.1/96 (and 192 kHz, informational). If an exciter misses 1.5 dB because of chaos rather than a fault, stop and bring the numbers to the user (design Risks).
- [ ] 4.3 Update `docs/ARCHITECTURE.md`:
  - the exciter drive levels and governor;
  - implicit contact;
  - the noise generator;
  - sub-sample events;
  - the mode amplitude and fade, replacing the hard-cut text;
  - the sample-rate rule's exception list, which should now be empty.

  Update the README loudness paragraph. Verify `cargo doc --no-deps --no-default-features` gives no new warnings.

## 5. Presets

- [ ] 5.1 Re-run `preset_levels_table` and apply its table to `PRESET_LEVELS` (same rules: trim limiter hits, raise anything more than 12 dB below the median). Regenerate with `cargo run --bin seed_presets`, and write `openspec/changes/resolve-exciter-tradeoffs/levels.md` with before and after per preset. Verify `preset_levels_are_sane` and `tests/preset_roundtrip.rs` pass.

## 6. Integration

- [ ] 6.1 Run the full gate:
  - debug and release tests, and the gnu lib tests;
  - `tests/no_alloc.rs` in debug;
  - fmt;
  - clippy on 1.99, and on 1.97 with `-A unknown_lints`;
  - the render smoke test with `scripts/check_wav.py`;
  - `preset_levels_are_sane`;
  - the 16-voice benchmarks at or below the pre-normalization commit (`748d832`) measured on the same machine in the same session. In task 1.1 that was Normal 8.57 ms and Render 21.10 ms per 4 096 samples, with runs varying about ±5 %. If they aren't, apply the struct-of-arrays fallback and re-measure.

  Record everything in the findings. Verify all green.

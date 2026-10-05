# Design

## Context

`MSEG::process_sample()` (`src/dsp/envelopes/mod.rs`) computes `current_level` by interpolating between stage levels, then applies velocity:

```
velocity_adjusted = 0.5 + (level − 0.5)·velocity_scale·vtl + level·(1 − vtl)
output = clamp(velocity_adjusted, 0, 1) · env_amount
```

When Release finishes, the stage loop sets `current_level = l_end` and returns `l_end · env_amount` directly, skipping the velocity formula. At full velocity (`velocity_scale = 1`) the formula reduces to `level + 0.5·(1 − vtl)`. With the presets' `vtl = 1` that is exactly `level`, and the Release → Idle return (`l_end = 0`) matches, so the bug only shows below full velocity or with `vtl < 1`.

Since C2, the stage loop is iterative and a golden-hash test pins the output for loop Off / Forward 3 → 4 / scrape / PingPong at velocities 0.8 and 0.3. Those hashes include the floor.

## Goals / Non-Goals

**Goals:**
- Zero level gives zero output; velocity is a pure gain.
- No step at Release → Idle.
- Keep C2's loop-timing protection after re-capturing the golden output.

**Non-Goals:**
- Velocity → time (`velocity_to_time`) behaviour, curve shapes, or stage timing.
- Hit (one-shot AR) and Specialty (ADSR) envelopes in `voice/mod.rs`; they don't use this formula.
- Re-voicing presets for the quieter sub-maximal-velocity friction notes.

## Decisions

1. **Multiplicative velocity gain:** `gain = (1 − vtl) + vtl · velocity_scale`; `output = (level · gain).clamp(0, 1) · env_amount`. This is the formula the review proposes. It is identical to today at full velocity, and it preserves zeros. Alternative: a velocity *curve* (e.g. squared): rejected as scope creep; it changes feel at full velocity too.

2. **Apply the same gain on the Release → Idle sample.** Return `l_end · gain · env_amount` (= 0 for the default `l_end = 0`). Once velocity no longer adds a floor, the last Release samples already approach `l_end`, so this makes the hand-off continuous. Alternative: a short fade on entering Idle, rejected because it isn't needed once the floor is gone.

3. **Re-capture the C2 golden hashes in a separate, explained step.** First confirm that the C2 timing tests (Sustain-start deadline test, sweep, cap test, loop-end semantics) pass unchanged. Then regenerate only the hash constants, and record in the test comment that they changed for C4 on purpose. Run the full-velocity cases before and after: with `vtl = 1` the new formula is mathematically identical there, so their hashes (e.g. `scrape_envelope()` at velocity 1.0) must not change. Only the sub-maximal-velocity hashes may move. That is a check that the math change is the only change.

## Implementation findings

- **Full velocity is equal up to rounding, not bit-identical.** `0.5 + (level − 0.5)` and `level` round differently in f32, so even the full-velocity golden hash (`scrape_envelope()` at 1.0) moved. A side-by-side check over 4 s found a maximum difference of 1.5e-8 (≈ −156 dB) in 795 of 192,000 samples. The hash check in Decision 3 was replaced with a permanent test, `full_velocity_matches_previous_formula`, that requires full-velocity output to match the old formula within 1e-7 per sample. All five golden hashes were then re-captured, and the golden test was renamed `loop_output_matches_golden`.
- **The release-end click measured 0.175 at velocity 0.3**, against a largest in-release step of 0.00007. After the fix the final step is within the release's own slope.

- **Release starts from the current level (added after verification).** Verification noted that Release always started from `l_sustain`, so releasing during Attack or Decay jumped. Release now ramps from `release_level`, captured when the gate closes. Releases from Sustain are unchanged (loop Off kept its golden hash); the looping golden cases, which release mid-Decay, were re-captured. `release_starts_from_the_current_level` fails on the old start level (0.25 → 0.05 jump).
- **Non-zero `l_end` is documented, not changed.** Release reports `l_end` on its hand-off sample and the envelope outputs 0 from then on, so a non-zero end level ends with a step. Voices always use 0; `set_stage_levels` documents it.

## Risks / Trade-offs

- [Sub-maximal-velocity friction notes become noticeably quieter in sustain] → Intended (it was a floor). Note it in the summary. Presets keep `vtl = 1`, so full-velocity playing is unchanged up to f32 rounding (≤ 1.5e-8).
- [Golden test re-capture could hide an unrelated timing regression] → Decision 3's ordering. The C2 behavioural tests must pass *before* the hashes are touched, and `full_velocity_matches_previous_formula` (1e-7) stands in for the full-velocity hash check, which f32 rounding defeated (see findings).

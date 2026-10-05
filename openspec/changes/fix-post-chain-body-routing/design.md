# Design

## Context

`PostProcessingChain::process(left, right)` today:

```
mono = (L + R)/2 → filter → drive ──┬─ [Eco] → L·0.3 + driven·0.7 / R·0.3 + driven·0.7 → clipper
                                    └─ body (no dry term) → L·0.3 + bodied·0.7 / R·0.3 + bodied·0.7
                                       → HRTF spread → space (Off | Factory | Spring | Echo) → clipper
```

- `FemBodyResonator::process` returns only the 8-mode bank sum, scaled by `(0.5 + 0.5·volume)`. `volume` also shifts every mode frequency by `1 / (0.5 + volume)`.
- `lib.rs` calls `set_body_params(chassis_material, chassis_volume.max(body / 5))`. All 50 presets have `chassis_volume = 0.5` and `body` 0.4–1.5, so `body` never wins. The BODY fader in the GUI is dead, and `body` defaults to 0.
- Since C1, the filter has unity passband gain, so the `driven` signal is roughly at voice level.
- An existing unit test in `post_chain.rs` asserts that Eco output *differs* because body/spread/space are bypassed. It encodes the behaviour being removed.

User decisions (2026-10-05): BODY becomes a parallel wet amount and factory presets are re-voiced to keep today's colour; Eco runs the full chain.

## Goals / Non-Goals

**Goals:**
- Dry filtered/driven stereo signal at the output; body added in parallel at the BODY amount.
- Stereo preserved through filter/drive.
- BODY and Chassis Volume each control one thing.
- Eco = same chain, cheaper.
- Presets keep roughly their current body colour.

**Non-Goals:**
- HRTF spread's own mono-heavy remix (it rebuilds both channels mostly from the mono sum when width > 0). That is a separate finding, and this change only makes sure the input to spread is real stereo.
- Body resonator physics (mode table, decay model, level-vs-frequency response).
- Reverb/echo internals (C6 handles the spring reverb's stereo state).
- GUI layout changes beyond the BODY label/tooltip meaning.

## Decisions

1. **Dual-mono filter and drive.** Hold `filter_l/filter_r` and `drive_l/drive_r`. Parameter setters, `set_sample_rate` and `reset` fan out to both. The Lorenz drive's chaotic state starts identical in both instances, so a mono input stays mono (L = R). Alternative: mid/side processing — rejected because it doesn't map onto the existing mono-designed filter/drive parameters any better than dual-mono, and it adds a matrix.

2. **Parallel body on the mono sum.** `body_in = (driven_l + driven_r)/2`, `body_wet = body.process(body_in)`, `out_ch = driven_ch + body_wet · body_amount`. The body is a resonant colouring of the instrument's chassis, so feeding it the mono sum keeps one body (and its CPU cost) rather than two. Alternative: a stereo body — rejected as unnecessary for C5; it can follow later if wanted.

3. **BODY → `body_amount = body / 5` (0–1), applied in the chain. `chassis_volume` → body tuning only.** Remove the `(0.5 + 0.5·volume)` gain from `FemBodyResonator::process` and the `max()` in `lib.rs`. Add `PostProcessingChain::set_body_amount`, smoothed per sample with the same one-pole pattern as the filter (≈20 ms) so BODY automation doesn't zipper.

4. **Eco runs every stage.** Delete the Eco early return. Eco keeps its 1× clipper oversampling; voice-level modal density is already set separately. CPU: body (8 modes) + spread + space are cheap compared with the voices, and Eco still saves the expensive oversampling.

5. **Re-voice presets by measurement.** The old output was `0.3·dry_unfiltered + 0.7·body(...)·(0.5 + 0.5·0.5)`; the new one is `filtered + body·amount`. For a representative input (each preset's own render), pick a single `body` value that matches the old body-to-dry energy ratio. Apply it to every preset in `src/bin/seed_presets.rs`, regenerate `presets/factory/*` with `cargo run --bin seed_presets`, and set the `body` parameter default to the same value so a fresh instance matches. Expected to land near BODY ≈ 3.5 (70 % wet); the task records the measured value. The old per-preset `body` values were inaudible, so nothing intentional is lost by replacing them. A single value is used because there's no evidence the old differences were tuned by ear.

6. **Level guard.** Measure the default note's peak and the per-preset RMS before and after. Accept if the default note peak stays within ±6 dB of the post-C1 value (0.118), and no preset clips the limiter more than it did before. If it fails, the fix is the body's internal gain constant, not BODY's mapping.

## Risks / Trade-offs

- [Every preset changes sound] → Accepted by the user. Decision 5 keeps the body colour near today's; the summary lists what changed. `render_presets` output can be re-rendered for audition. It currently bypasses the post chain (H13), so the audition should use the plugin or a post-chain render.
- [Eco gets more expensive] → Bounded: body (8 two-pole modes), spread and one space stage per plugin, not per voice. Note the measured `benches/performance.rs` delta in the summary.
- [Presets saved by users with the old BODY values become drier] → Their BODY values (≤ ~1.5) now mean ≤ 30 % wet, where before they were inaudible. Mention it in release notes; no migration, since we can't recover intent.
- [The existing Eco-differs test encodes the removed behaviour] → Replace it with the spec's "Eco keeps reverb/body" scenarios, which test the opposite on purpose.

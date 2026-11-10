> **Superseded.** This review (2026-05-27) is kept for history only and no longer matches the code. The current review is [`FABLE-CODE-REVIEW-2026-07-07.md`](../../FABLE-CODE-REVIEW-2026-07-07.md), whose findings are tracked as OpenSpec changes.

# Corrosion — Critical Code Review & Doc Conformance

**Scope:** full `src/` tree except `src/presets/` (excluded by request).
**Date:** 2026-05-27.
**Method:** read of every module in the audio path plus build/test infra, cross-checked
against `docs/detailed-specs/`, `docs/corrosion_plugin_prd_and_specs.md`, and
`docs/master-prompt.md`.

**Headline:** The *core* synthesis engine (modal resonator, interaction bus,
transformations, exciters, voice management, envelopes) is real, well-structured,
and faithful to the detailed specs. The *post-processing/space/output* layer is a
set of lightweight approximations whose names and the detailed-spec descriptions
oversell them, and it contains one genuine functional defect (oversampling no-op)
and one correctness bug (factory-reverb delay mutation). Real-time-safety
discipline is good.

---

## 1. Verification of the four claims in the request

| Claim | Verdict | Detail |
|---|---|---|
| Oversampling configurable 1×/4×/8×/16× mapped to Eco/Normal/High/Render, already implemented | **Partially true** | The *mapping* exists and is wired end-to-end. But the clipper does not actually oversample — see Finding 1. So the modes do not differ at the clipper. |
| Idle rendering profiled and optimized | **True** | `Voice::rendering` + `is_rendering()` gate; `VoiceManager` skips non-rendering voices in both mono and stereo paths. |
| Aliasing being measured | **True (as a proxy)** | `offline::analyze_post_chain_aliasing` builds a sine, runs it through the Render-quality post chain, and reports harmonic vs non-harmonic DFT energy (`alias_ratio_db`). Exposed via `render --suite aliasing` and `--suite all`; covered by a unit test. It is a residual-energy proxy, not an oversampled-reference comparison. |
| Output ceiling −0.3 dBFS | **True** | `LIMITER_THRESHOLD = 0.9661` (= 10^(−0.3/20)); `apply_output_limiter` hard-clamps to ±0.9661; `analog_ceiling` default 0.9661, clamped to [0.5, 1.0]. |

---

## 2. Defects & bugs (ranked)

### Finding 1 — `OversampledClipper` does not oversample (functional defect)
`src/dsp/post_processing/oversampled_clipper.rs`, `process()`:

```rust
let mut os_samples = [0.0f32; 16];
os_samples.fill(input);                  // zero-order hold of ONE input value
let mut sum = 0.0f32;
for sample in os_samples.iter().take(os_factor) {
    sum += self.diode_clip(*sample);     // diode_clip(input), identical each iter
}
sum / os_factor as f32                    // == diode_clip(input) for every factor
```

Because all `os_factor` "oversamples" are the same value, the average equals a
single `diode_clip(input)` regardless of factor. There is **no interpolation, no
polyphase filtering, and no anti-aliasing decimation**; `upsample_state` /
`downsample_state` are never read. Consequences:

- Eco (1×), Normal (4×), High (8×), Render (16×) produce **identical** clipper output.
- The advertised "16× oversampled true-peak clipper with linear-phase AA filter"
  (`detailed-specs/post-processing.md`, `signal-chain.md`) does not exist.
- The aliasing harness therefore measures *unmitigated* nonlinear aliasing.
- The test `quality_mode_changes_oversample_factor` passes for the **wrong reason**:
  Eco differs from Render only because of the chain's Eco *stage bypass*
  (body/spread/space), not because of oversampling. The test does not actually
  exercise the clipper factor.

**Fix direction:** implement real upsampling (polyphase or at minimum linear
interpolation across the previous/next input), clip at the high rate, then
low-pass decimate; or downgrade the docs to "soft diode clip, no oversampling"
and remove the dead factor/state. Either way, code and docs must agree.

### Finding 2 — `FactoryReverb::update_delays` mutates comb delays cumulatively
`src/dsp/post_processing/space.rs`:

```rust
fn update_delays(&mut self) {
    let size_scale = 0.5 + self.size * 1.5;
    for i in 0..4 {
        self.comb_delays[i] = (self.comb_delays[i] as f32 * size_scale) as usize; // reads its own prior value
        self.comb_delays[i] = self.comb_delays[i].clamp(100, 2000);
    }
}
```

`update_delays` re-scales the **already-scaled** `comb_delays` every call, and
`lib.rs` calls `set_factory_params` (→ `set_parameters` → `update_delays`) **once
per sample, unconditionally** (even when Space is off). With `size = 0.5`,
`size_scale = 1.25 > 1`, so the delays rail to the 2000 clamp within a few
samples; with small sizes they collapse to 100. The `factory_size` control thus
does not behave as a room-size control. **Fix:** derive `comb_delays` from
immutable base constants each update (as `SpringReverb::update_delay` already does
correctly).

### Finding 3 — Post-chain parameters are set per sample (CPU + root cause of #2)
`src/lib.rs::process` calls ~10 `post_chain.set_*` setters inside the per-sample
loop. Several recompute coefficients (`WdfLadderFilter::update_coefficients` →
`tan()`; `FemBodyResonator::update_mode_frequencies`; reverb delay recompute).
Host parameters change at most once per block, so this is wasted audio-rate work
and directly causes Finding 2. The `master-prompt.md` performance section
explicitly calls for control-rate updates. **Fix:** apply setters once per buffer
(or only on change), keeping coefficient recompute out of the sample loop.

### Finding 4 — Doc/string says "17 exciter types"; there are 16
`src/lib.rs` module docstring line 5 ("17 different exciter types") and the stale
root `CODE_REVIEW.md` both say 17. The `ExciterType` enum and `exciter_model_items`
define exactly 16 (IDs 1–16), and `README.md` correctly says 16. Fix the docstring.

### Finding 5 — Dead / unread code
- `Voice::process_sample_stereo` branches on `self.exciter_type == 0` for a 2.0
  boost, but `exciter_type` is always 1–16 (`from_int(0)` → HandStrike = 2). Dead branch.
- `OversampledClipper::{upsample_state, downsample_state}` — allocated, reset,
  never read.
- `ModalResonator::{last_output, current_output}` — written each sample, never read.
- `InteractionState::{update_resonator_state, distribute_force_to_modes}`,
  `BidirectionalInteractionBus::{distribute_force, per_mode_forces}`,
  `mode_coefficient_2d`, `mode_coefficient_circular` — only used by tests, not the
  audio path (the resonator computes its own per-mode coefficients inline).

These are not bugs but they mislead readers and inflate the structs; either wire
them in or remove them.

### Finding 6 — Minor
- **`render_presets` not declared in `Cargo.toml`.** Only `[[bin]] render` is
  explicit; `render_presets` works via cargo auto-discovery of `src/bin/`. Harmless
  but inconsistent — declare it or document the reliance on auto-discovery.
- **`midi_to_hz` has no note bounds** (cosmetic; MIDI is 0–127 by construction).
- **WDF "ladder" integration** (`s = y[i] − g·y[i]`) is a cascade of one-pole-ish
  stages, not a true TPT/WDF ladder. Stable and fine sonically; the name oversells.

---

## 3. Strengths (keep)

- **Modal core is correct.** `SecondOrderMode` implements `y[n]=b0·x−a1·y1−a2·y2`
  with `a1=−2r·cosω`, `a2=r²`, `r=exp(−1/(decay·sr))` — exactly the spec, with
  sample-rate-cached coefficient rebuilds and `decay.max(EPSILON)` guards.
- **Interaction bus is faithful** to `exciter-resonator-interaction.md`: per-mode
  `sin((n+1)πP)` coefficients, fundamental lock/minimum, coupling blend, bounded
  position wander, displacement/velocity feedback to the exciter.
- **Transformations are faithful** to `transformation-algorithms.md` (size, rust,
  thickness, heat, sludge, damage-splitting), all on bounded newtypes that sanitize
  non-finite input.
- **Real-time safety is disciplined:** `tests/no_alloc.rs`, `assert_process_allocs`
  on non-Windows, denormal flush, `is_finite()` guards, NaN-safe voice stealing,
  fixed voice pool, integer-`match` exciter dispatch.
- **Honest in-code docstrings.** The post stages say "approximation",
  "FEM-inspired", "HRTF-inspired", "FDTD approximation" — the code is more honest
  than the detailed specs.
- **Good test and QA surface:** unit tests across DSP, integration tests
  (automation stress, preset roundtrip, drive/body/stereo/velocity/limiter),
  Criterion benches, deterministic offline renderer with manifests, CI lane.

---

## 4. Documentation conformance matrix

Legend: ✅ matches code · ⚠️ partial/aspirational · ❌ contradicted by code.

| Spec area (`detailed-specs/`) | Conformance | Notes |
|---|---|---|
| `exciter-algorithms.md` (16 exciters, models, params) | ✅ | All 16 implemented with the documented per-exciter parameters and DSP families. Category labels differ cosmetically (spec "Hit/Scrape/Other" vs code "Hit/Friction/Specialty"; Bow is Friction). |
| `exciter-resonator-interaction.md` (interaction bus, MSEG) | ✅ | Spatial coefficients, fundamental anchor, coupling, wander, and the 6-stage MSEG (with looping) all present. |
| `resonator-algorithms.md` (9 modal objects) | ✅ | Both the profile-table and algorithmic paths exist for all 9 objects. **Approved change (2026-05):** consolidate to algorithmic-only, remove `complex_algo`, expose curated per-object params, fix Chain/Tank pitch + wire Cable/Sheet dynamic hooks — see `docs/backlog.md`. |
| `transformation-algorithms.md` (size/rust/damage/thickness/heat/sludge/velocity) | ✅ | Formulas match closely (e.g., heat `·(1−heat·0.05)`, sludge `√(1/(1+s))`). |
| `signal-chain.md` — synth core ordering | ✅ | MIDI → exciter ↔ interaction ↔ resonator → post matches. |
| `signal-chain.md` — "post can be block processed" | ⚠️ | Post is per-sample today (incl. param setters). |
| `signal-chain.md` — limiter "−0.3 dBFS" | ✅ | `LIMITER_THRESHOLD = 0.9661`. |
| `signal-chain.md` — "Upsample/Downsample 16×" | ❌ | Oversampling is a no-op (Finding 1). |
| `post-processing.md` — WDF Newton-Raphson circuit | ⚠️ | 4-pole ladder approximation. |
| `post-processing.md` — Lorenz-coupled tube/fuzz | ⚠️ | Real Lorenz state, but used as a mild gain modulator only. |
| `post-processing.md` — HRTF with HRIR convolution | ⚠️ | Delay + 1-pole LP/HP + crossfeed; no HRIR, no θ/φ. |
| `post-processing.md` — FEM body, thousands of nodes | ⚠️ | 8 fixed modal resonators. |
| `post-processing.md` — 3D FDTD factory reverb | ⚠️ | 4 comb + 2 allpass (Schroeder); also buggy delays (Finding 2). |
| `post-processing.md` — FDTD helical spring PDE | ⚠️ | Single delay + 1-pole dispersion. |
| `post-processing.md` — Doppler multipath echo | ✅(ish) | Modulated stereo delay; reasonable match. |
| `post-processing.md` — 16× oversampled clipper, ≤0 dBFS | ❌ / ✅ | Ceiling honored; oversampling absent (Finding 1). |
| PRD MVP (3 objects, 6 params, "Hit/Scrape/Motor", tanh drive, 20 presets) | ❌ | Superseded: 9 objects, 127 params, 16 exciters (no "Motor"), Lorenz drive. PRD needs a current-state reconciliation (added). |

---

## 5. Recommended actions

**Critical**
1. Fix `OversampledClipper` to truly oversample, or rewrite the docs and delete the
   dead factor/state. Re-base the `quality_mode_changes_oversample_factor` test on
   the actual factor.
2. Fix `FactoryReverb::update_delays` to recompute from immutable base delays.

**High**
3. Move post-chain parameter application out of the per-sample loop (control-rate).
4. Correct the `lib.rs` "17 exciter types" docstring to 16.

**Medium**
5. Remove or wire up the dead fields/functions in Finding 5.
6. Align the `detailed-specs/post-processing.md` and `signal-chain.md` claims with
   the shipped approximations (done in this pass — see status banners there).
7. Update `params.rs` docstring "70+" → 127.

**Low**
8. Declare `render_presets` in `Cargo.toml`; bound `midi_to_hz`; rename/reword the
   "WDF" filter to reflect the approximation.

---

*This review supersedes the previous root `CODE_REVIEW.md`, which referenced files
that have since been renamed/removed (`src/dsp/resonator.rs`, `src/dsp/mseg.rs`,
`src/dsp/exciters/scrape.rs`) and a since-fixed `unwrap()` in `manager.rs`.*

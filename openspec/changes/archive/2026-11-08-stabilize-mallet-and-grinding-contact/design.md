# Design

## Context

See proposal.md, "Why", for the two faults.

- **The loop today.** Each sample, the voice calls the exciter with the resonator's previous-sample contact displacement `D` and velocity `V`, with `V = ΔD·fs/48k`, in 48 kHz-sample units. The returned force is multiplied by the force envelope `e` and the drive trim `t`, then fed to the resonator. Every mode then gets force `F·e·t·b_i`, where `b_i = (1 − c) + w_i·c` is the strike-position blend.
- **The modes** are coupled-form: `z_i ← p_i·z_i + g_i·x_i`, with output `y_i = Im z_i` and `Im g_i = b0_i`. The contact displacement is `D = Σ w_i·y_i`.
- **Immediate response.** This sample's displacement is therefore affine in this sample's force: `D = D_free + C·F`, where:
  - `D_free = Σ w_i·Im(p_i·z_i)` is where the resonator goes on its own;
  - `C = e·t·Σ w_i·b_i·b0_i` is the instantaneous compliance;
  - the velocity follows as `V = (D − D_prev)·fs/48k`.

## Goals / Non-Goals

**Goals:**
- No divergence for these two exciters at any drive. Tested at 16× above the new default.
- A median default note near the other struck exciters, and a worst case of at most 0.9 (see specs).
- The four "done when" tests pass, and `tests/no_alloc.rs` passes in debug.

**Non-Goals:**
- **Other exciters.** `resolve-exciter-tradeoffs` adopts the compliance and helper for the rest.
- **The hash noise** in HeavyGrinding (`sin(phase)·43758`), which is replaced in `resolve-exciter-tradeoffs`.
- **A per-voice drive governor.** The trims stay fixed here.
- **Bit-identical output** for these two exciters at 48 kHz. Every other exciter's output stays bit-identical.

## Decisions

1. **The resonator reports its immediate response on request.**
   - `ModalResonator::contact_prediction()` returns `(D_free, C₀, D_prev)`, where `C₀ = Σ w_i·b_i·b0_i` and `D_free` uses the current strike-position weights.
   - It costs one extra pass over the modes: two multiplies and an add per mode. It runs only when the active exciter needs it, so voices on other exciters pay nothing.
   - The voice scales `C₀` by the force envelope and the drive trim before passing it on, so the exciter sees its own force's real effect.
   - *Alternative:* compute `D_free` every sample inside `process_sample`. That costs every voice.

2. **A shared monotone contact solver** (`dsp::utils::contact`).
   - Both contact laws are monotone in `F`: more force means less penetration (mallet) or less slip (grinder). So `F = h(F)` has exactly one root in `[0, h(0)]`, or `[h(0), 0]` for a negative force.
   - The solver runs safeguarded Newton, kept inside the bracket and falling back to bisection, with at most 4 iterations. It is allocation-free, and it has a unit test against a brute-force solve.
   - This is the zero-delay feedback pattern already used for the filter (C1).
   - *Alternatives:*
     - Bounding the explicit terms, which only moves the threshold.
     - Oversampling the contact, which costs more and also only moves the threshold.

3. **HardMallet: dissipative Hertz, solved implicitly, and the mallet leaves.**
   - The force is `F = m·k·d^1.5·(1 + λ·v_rel)` for penetration `d > 0`, else 0 (Hunt–Crossley), with:
     - `d = x_m − (D_free + C·F)`;
     - `v_rel = v_m − V(F)`;
     - `λ = impact_damping` mapped to the old range.
   - The mallet's velocity is updated with the solved force.
   - "Gravity" applies only before the first contact, as the throw. After contact the mallet rebounds and goes inactive once it moves away and is separated by more than a small margin, with no re-pull into the surface.
   - The old energy-transfer and impact-count damping are replaced by λ.
   - The parameters keep their names and ranges: mass, stiffness, damping.

4. **HeavyGrinding: smooth friction, solved implicitly, and bounded tearing.**
   - Friction: `F_f = P·μ·tanh(v_rel / v_s)`, with `v_rel = v_g − V(F)` and `v_s = 0.05·v_g` (minimum 1e-4). This replaces the sign function, which flipped every sample on a high mode.
   - Solved with the same solver: more friction lowers the slip.
   - Tearing becomes `noise·min(|v_rel|, 2·v_g)·grit·2`, using the solved `v_rel`. It is then bounded by the grinder's own speed and can't grow with the resonator's motion.
   - The output keeps its `·v_g·3` scale. The trim absorbs the level.

5. **Trims re-tuned by measurement.** Run `print_exciter_overdrive` for exciters 4 and 10. Set each trim so its median over the 27-note sweep is the mean of Drumstick's and WireBrush's medians (about 0.176), then lower it if the worst case exceeds 0.9. Record the median, the worst case and where it falls.

## Risks / Trade-offs

- **[The worst/median spread over objects is more than 5× (14 dB), so the 0.9 worst case pins the median under 6 dB of the target]** → Measure first. If so, report it and stop: the per-voice governor in `resolve-exciter-tradeoffs` is the fix for spread. Don't widen the spec here.
- **[HeavyGrinding's hash noise breaks `every_exciter_has_the_same_level_at_any_sample_rate` once the exciter is louder]** → Its tolerance is the default 1.5 dB today. If it now fails because of the noise, which fails for the jet the same way, record the numbers and give it a documented tolerance like the jet's (3 dB), pending `resolve-exciter-tradeoffs`. A failure from the contact itself must be fixed.
- **[The mallet's bounce-off shortens its sustained excitation]** → Intended. The struck tail and attack-timing tests guard the behaviour. Once the mallet leaves, its whole-note peak should be the strike, so try dropping HardMallet from the 20 ms timing window, and keep the window only if the whole-note peak still fails.
- **[`contact_prediction` uses the strike-position weights from the start of the sample]** → The bus updates them before the modes run. Call the prediction after that update, so they are this sample's.

## Implementation findings

### Before the change (task 1.1)

The worst and median values below are each exciter's largest |resonator output| over 9 objects × C2/C4/C6, at velocity 127 and 48 kHz, as the drive multiplier rises above today's trim.

| Drive × | HardMallet median / worst | HeavyGrinding median / worst |
|---|---|---|
| 1 | 3.4e-3 / 3.7e-2 (Cog C2) | 5.1e-3 / 0.21 (Cog C2) |
| 2 | 6.5e-3 / 0.15 | 1.4e-2 / 0.43 |
| 2.5 | 8.0e-3 / **2.0** | 1.8e-2 / **2.2** (SheetMetal C6) |
| 4 | 1.3e-2 / 3.1 | 4.0e-2 / 6.4 |
| 16 | 7.2e-2 / 11.7 | 3.4 / **2.4e5** (Plate C6) |
| 64 | 1.1 / 46 | 109 / 6e26 |
| 256 | 4.4 / 173 | 798 / ∞ |

- **HardMallet doesn't diverge.**
  - Its median grows with the drive.
  - Its worst note (IndustrialCog C2) jumps 13.5× between drive 2 and 2.5: a switch into sustained pressing against the bank, where the ungated damping keeps feeding back.
  - Even at drive 1, the worst note is 11× the median (21 dB), which the strike-only model should close.
- **HeavyGrinding runs away.** Its median grows faster than the drive (86× from 4× to 16×), and Plate C6 reaches infinity.
- **New tests, failing as expected:**
  - `mallet_and_grinder_play_at_a_struck_exciter_level`: HardMallet's median is 0.0036, −33.8 dB from the struck target 0.176.
  - `mallet_and_grinder_contact_is_stable_far_above_default_drive`: HeavyGrinding on Plate C6 reaches 2.4e5.
  - `hard_mallet_leaves_after_its_strike`: force is still applied at 0.55 s.

### Contact prediction and solver (tasks 2.1–2.2)

- **Prediction accuracy.** `contact_prediction` matches the next `process_sample` within f32 rounding: about 1e-5 relative at worst, because the two paths sum the same terms in a different order. The test checks 1e-4 relative, on Pipe and SheetMetal (dynamic) at three strike-position and coupling settings.
- **Bit-identical output.** Calling the prediction doesn't change the output, which is bit-identical with and without it (Pipe, TautCable, SheetMetal, with wander on). A flag makes the sample's preparation (bus update and dynamic retune) run once.
- **Solver.** Plain Illinois regula falsi didn't converge in 16 iterations on a very stiff Hertzian contact (k = 1000, C = 2). The kink where penetration reaches 0 makes it creep. A bisection step whenever an iteration fails to halve the bracket, with at most 40 iterations, fixes it. The solve runs only for the two exciters here.

### HardMallet and HeavyGrinding (tasks 3.1–3.2)

- **HardMallet** is implemented as designed:
  - a Hunt–Crossley force only in contact (`λ = 0.1·damping / MALLET_STEP`), with the relative velocity in 48 kHz-sample units;
  - the throw (gravity and air drag) only before the first contact;
  - it leaves once it moves away and is 0.02 clear, with a 0.25 s strike cap.

  The held-note test passes: no force after 0.5 s. With its strike only, its whole-note peak is the strike, so **the 20 ms timing window for HardMallet is removed**. `struck_attack_timing_does_not_depend_on_sample_rate` passes on the whole note.
- **HeavyGrinding: one deviation from Decision 4.** Bounding the tearing by `min(|v_rel|, 2·v_g)` stopped the divergence, but it left a slow build-up. On Plate C6 the resonator climbed over about 0.75 s into a limit cycle at |y| ≈ 4–5, nearly independent of drive. The friction can't be the source: solved implicitly, it only removes energy. The source was the tearing amplitude following the resonator's own velocity, so the noise force was modulated by the motion it drives. Tearing now scales with the grinder's own speed only, which equals the slip whenever the resonator is slow, the usual case. The grinder's response is then linear in drive: Plate C6 gives 0.002 → 0.009 for 1× → 4×.
- **Stability.** Both pass the 16× drive sweep (resonator < 100 on every object and note).

### Trims (task 4.1)

| Exciter | Trim (was) | Median | Worst (where) | vs struck target 0.176 |
|---|---|---|---|---|
| HardMallet | 1.285 (1.58e-3) | 0.129 | 0.884 (IndustrialCog C2) | −2.7 dB ✓ |
| HeavyGrinding | 7.21e-3 (1.80e-3) | 0.020 | 0.850 (IndustrialCog C2) | −18.9 dB ✗ |

**HeavyGrinding misses the level target.** This is the design-Risks case, and the user chose to defer it.
- Its worst note (IndustrialCog C2) is about 41× its median.
- Friction alone and tearing alone show the same spread, about 29×. So the cause is how a steady or slowly varying force moves a low bank, which responds roughly as 1/f² and much more than a strike, not the grinder's mix.
- Bank normalization equalizes strikes only.
- The spec's level scenario is narrowed to HardMallet. Both exciters keep "no default note above 0.9", and both keep the stability scenario.
- HeavyGrinding is 4× louder than before (median 0.005 → 0.020) and stable. Its level is handed to the per-voice drive governor in `resolve-exciter-tradeoffs`.

### Gate (task 5.1)

- Debug and release test suites pass (musl), and the gnu lib tests pass: 221 passed.
- `tests/no_alloc.rs` passes in debug.
- `cargo fmt --check` is clean. Clippy with `-D warnings` is clean on 1.99, and on 1.97 with `-A unknown_lints`.
- `cargo doc` shows no new warnings.
- The family render smoke test passes `scripts/check_wav.py`.
- **Bit-identity:** 48 kHz render hashes against `main` match for every exciter except 4 and 10, and for the FEM body. The other 14 exciters and the body are bit-identical.
- **Presets:** the louder HardMallet and HeavyGrinding presets raised the bank's median preset level by about 1 dB, which left six presets 13.1 dB under it. `preset_levels_table` was re-run and those six raised to the new −12 dB floor: Cable Bow, Cable Scrape, Falling Chain Cluster, Sheet Bend, Thin Sheet Roll and Workshop Plate Scrape. `preset_levels_are_sane` and `tests/preset_roundtrip.rs` pass.

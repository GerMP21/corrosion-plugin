# Design

## Context

After `fix-resonator-sample-rate-gain`, mode i has input gain `b0_i = gain_i · (48k/fs)²`. A short force pulse of duration T, much shorter than a period, then sets mode i ringing at roughly:

```
A_i ≈ gain_i · 48k² · T / (2π · f_i)      (independent of fs, but ∝ 1/f)
```

The voice sums the modes' displacements, applies a velocity-scaled high-pass boost, then `clamp(±1)`.

Measured raw resonator levels:
- 1 ms pulse at C4: Tank about 125, Pipe about 25, Plate about 2.
- Bowed: Tank about 280, Pipe about 40, Plate about 2.2.
- Across the keyboard: ×4 from C4 to C2.

The generators (`basic.rs` and siblings) set mode gains with arbitrary per-object constants. For example, the Tank cavity uses `0.05 · cavity_mix` at `f_base · 0.5 / (1 + volume)`. Mode frequencies within a bank scale with the note, so a bank's shape is roughly pitch-invariant.

Factory presets were last levelled by C5, per preset, to their pre-C5 RMS, and that RMS included heavy clipping. They are placeholders; real presets come later.

## Goals / Non-Goals

**Goals:**
- One strike level for every object, pitch, quality mode and damage setting.
- No default-control note overdrives the voice clamp.
- Each object's timbre is unchanged: only the bank's overall scale moves.

**Non-Goals:**
- **Re-voicing presets carefully.** Only the light pass described in Decision 4.
- **Exciter-specific loudness.** For example, HandStrike is much quieter than Bow on the same object. Exciters keep their relative levels.
- **Replacing the hard clamp with a soft saturator.** Once nothing overdrives at default settings, the clamp only catches extreme settings.
- **Changing what `decay_seconds` means.** It stays a 1/e time constant (T60 ≈ 6.9·τ). That gets documented, not changed.
- **Sample-rate-dependent per-sample constants in the voice and exciters.** Covered by `fix-sample-rate-time-constants`.

## Decisions

1. **Normalize each bank at note-on by its pulse-response bound.**
   - Compute `L = Σ_i gain_i / f_i` over the bank's modes, after the material transforms (rust, damage, heat and so on) and before live controls.
   - Scale every mode's gain by `s = L_REF / L`.
   - Because `A_i ∝ gain_i / f_i`, the bound `Σ A_i` becomes the same constant for every bank. A lower note has lower `f_i`, so a larger `L` and a smaller `s`. Extra modes from quality or damage raise `L` and get scaled down.
   - It is O(modes) at note-on, uses no allocation, and lives in the shared rebuild path (`rebuild_in_place` and the constructors).

   *Alternatives:*
   - A per-object constant table measured at C4. It doesn't fix pitch, quality or damage, and goes stale whenever a generator changes.
   - Normalizing each mode by `f_i`, i.e. outputting velocity. That changes every object's timbre by +6 dB/octave, which is a sound-design change, not a level fix.
   - Measuring the actual peak at note-on. Too expensive and not deterministic enough.

2. **`L_REF` is chosen so the default patch keeps its loudness.** That is Pipe at C4 with default controls, through the C5 post chain, peaking at about 0.118. Set `L_REF` to the default patch's current `L` (measured once, then a named constant). The default note and `hosted_normalized_note_velocity_reaches_output_chain` (±6 dB of 0.118) then stay where they are, and the other objects move to the Pipe's level.

3. **Live brightness and damping act after normalization.** `set_damping_brightness` re-derives gains from `base_gain`, which becomes the normalized gain. A brightness sweep on a held note still tilts the level a little, as today. Dynamic pitch hooks (cable drop, sheet warp) don't renormalize, because the scale is fixed at note-on.

4. **A light pass for the placeholder presets.**
   - Render every factory preset with the existing harness, now through the normalized engine.
   - Keep `output` unless a preset hits the limiter (trim until it doesn't) or its RMS sits more than 12 dB below the bank's median (raise it to 12 dB below).
   - Record the result in `levels.md` in this change.
   - Retire `c5_verify_revoiced_presets` and `c5_revoice_table`, whose strict 0.5 dB match pinned clipped levels. Replace them with an ignored `preset_levels_are_sane` check that encodes these two rules. `tests/fixtures/c5_preset_baseline.md` goes with them.

5. **The overdrive requirement is tested inside the crate.** The pre-clamp voice signal isn't public, so the "no overdrive" scenario is a unit test in `src/voice/mod.rs`. It sweeps objects × {HandStrike, Bow, PneumaticJet} × {C2, C4, C6} and checks the value fed to the clamp.

6. **Each exciter gets a fixed drive trim** (decided with the user during implementation). Bank normalization keeps the default Pipe C4 bank where it was, and with it the exciters' own drive levels. So Bow (9.6× full scale on Pipe C4) and PneumaticJet (about 5 000×) still overdrove the clamp, and some contact exciters ran away on the brighter normalized banks.
   - `EXCITER_DRIVE_TRIM` in `src/voice/mod.rs` scales each exciter's force.
   - It was found iteratively over every object at C2/C4/C6, velocity 127, default controls (`print_exciter_overdrive`). Exciters with a runaway threshold were bisected, then set to half the highest stable trim, for about 6 dB of margin.
   - Exciter loudness stops being a non-goal. Exciters keep their character, but the continuous ones get much quieter (see findings).
   - *Alternatives:*
     - Narrowing the requirement to struck exciters.
     - A soft saturator in place of the clamp.
     - A sustained-level bank target for friction and specialty notes, `sqrt(Σ A_i²·min(τ_i, 2 s))`. That was tried and did not reduce the spread, because bow and jet levels come from nonlinear coupling, not from the bank's linear response.

## Risks / Trade-offs

- [The bowed level also depends on decay (τ), and `L` ignores τ, so a long-ringing bowed bank could still overdrive] → The overdrive test catches it. If it fails, extend the bound to `Σ gain_i · min(τ_i, τ_cap) / f_i` for friction-driven notes, or cap the friction family's makeup. Record the decision in Implementation findings rather than guessing now.
- [The peak sits below the bound by a bank-dependent amount, depending on how mode phases align] → The spec allows 6 dB across objects and 3 dB across pitch. If an object misses the band, the fix is a small per-object trim on top of `L` (a constant), recorded in the findings.
- [Every sound changes level, and Tank, Cog and Pipe lose their clipped grit] → Accepted (proposal). The presets are placeholders.
- [Exciters that couple to resonator displacement or velocity (bow, hand strike, mallets) see a different impedance once the bank is scaled] → Intended side effect. Exciter-family loudness stays a non-goal, and the overdrive test bounds the result.

## Implementation findings

### Before the change (task 1.1)

These are strike peaks at 48 kHz, with default controls and the default strike position, measured after `fix-sample-rate-time-constants`.

| Check | Before |
|---|---|
| Object spread at C4 (1 ms pulse) | 39 dB: CoilSpring 2.8, Plate 1.9, Pipe 27.8, Tank 133, IndustrialCog 174, TautCable 128. SheetMetal diverged to ∞. |
| Pitch, C2 and C6 vs C4 (1 ms pulse) | +11 to +18 dB at C2, and −12 to −33 dB at C6 |
| Render vs Eco | up to +5.8 dB (Chain) |
| Damage 1 vs 0 | up to +14.6 dB (CoilSpring) |
| Voice pre-clamp peak (velocity 127, C2/C4/C6) | HandStrike stays under 1.0. Bow reaches 2–6 700× full scale, PneumaticJet 1.6–180 000×, and SheetMetal with Bow or PneumaticJet diverges (10²⁰ and more). |

### Deviations from the design, and why

1. **The modes run in coupled form** (complex one-pole, `z = r·e^{iω}·z + b0·(cot ω + i)·x`, output `Im z`), not the direct-form biquad. The impulse response is the same (`b0·rⁿ·sin(ω(n+1)) / sin ω`), and the existing difference-equation tests still pass.
   - **SheetMetal diverged.** Its warp re-tunes every mode at audio rate, by up to 4×. In direct form, changing the coefficients under the stored `y1`/`y2` pumps energy into the mode. In coupled form a frequency change rotates the state and keeps its magnitude. TautCable's tension drop was pumping energy too: its C4 strike went from 128 to 25 before normalization.
   - **This is also the precision fix that `fix-resonator-sample-rate-gain` handed to this change.** The frequency comes from `r·sin ω`, not from `−2r·cos ω ≈ −2`, and the coefficients are computed in f64. The sustained-level test now includes 192 kHz and passes within 0.5 dB. In direct form, Tank lost 1.44 dB there.
2. **The normalization estimate is `‖w_i · b0_i / sin ω_i‖_1.3`, not `Σ gain_i / f_i`.**
   - `A_i = |w_i · b0_i / sin ω_i|` is mode i's true ring amplitude. `w_i` is the strike-position force weight that `process_sample` applies: `sin(nπ·0.5)` at the default centre strike, so ±1 or 0. Leaving `w_i` out, as `Σ gain/f` does, missed by 15 dB on CoilSpring, whose level sits in the modes a centre strike doesn't excite.
   - The peak of the summed modes lies between the incoherent `‖A‖₂` and the coherent `‖A‖₁`, which is reached once beating modes come into phase.
   - The measured true peak (over 1 s), against `‖A‖_p` across 9 objects × {C2, C4, C6, Eco, Render, damage 1}, spreads over 5.5 dB for p = 1, 4.5 dB for p = 1.15, 4.3 dB for p = 1.3 and 5.1 dB for p = 1.5. p = 1.3 was chosen.
   - Two alternatives were tried and dropped:
     - Simulating the impulse response at note-on for one period of the lowest mode. It missed peaks that come later from beating: Chain +12 dB, CoilSpring +15 dB.
     - Simulating long enough to catch those peaks, which costs too much at note-on.
   - The estimate is O(modes), allocation-free and deterministic. The scale is applied to each mode's gain at note-on, after the material transforms and the bus reset, and before the live damping and brightness controls.
   - `STRIKE_LEVEL_REF` = 0.5945 is the default patch's estimate (Pipe, MIDI C4, thickness 0.5, other controls at their defaults), so that patch is unchanged.
   - It lives in `rebuild_in_place`, the only path a note takes. The legacy profile-table constructors (`with_profile*`, `Default`) only feed the offline renderer and the pre-first-note resonator, and are left as they were.
3. **The spec's level scenarios use a one-sample impulse, not a 1 ms pulse.** A 1 ms pulse is a filter itself. Its response is `sin(πfT)/(πf)`, which is null at 1 kHz, about the C6 fundamental. So C6 read 10–17 dB low even for perfectly normalized banks.

### After normalization (strike peaks, one-sample impulse, 48 kHz)

- **Pitch:** C2 and C6 are within 3 dB of C4 for every object.
- **Objects:** every object at C4 is within 6 dB of every other.
- **Quality and damage:** Eco vs Render, and damage 0 vs 1, are within 2 dB for every object.
- **Golden fixture:** `tests/fixtures/mode_bank_hashes.txt` was re-captured, because every gain is scaled. It is identical on the musl and gnu targets, and `rebuilding_a_reused_resonator_matches_a_fresh_one` holds.
- `tests/no_alloc.rs` passes in a debug build.
- All other lib tests pass, including `hosted_normalized_note_velocity_reaches_output_chain` and the sample-rate tests from the earlier two changes.

### Further findings: modes above 20 kHz, drive trims (tasks 2.2–2.4)

**Mode ceiling.**
- Modes at or above `min(20 kHz, 0.45·fs)` get no input (`MODE_CEILING_HZ`).
- A mode's gain grows as `1 / sin ω`. SheetMetal's `n²` partials (up to 145 kHz at C4), its warp, and CoilSpring at C6 put modes next to or above Nyquist, and contact exciters then ran away (10²⁰ or more).
- Those modes only alias in any case. The note-on estimate skips them too.

**Drive trims (Decision 6).** Values are the worst pre-clamp peak over 9 objects × C2/C4/C6 at velocity 127, before and after the trims.

| Exciter | Trim | Worst before | Worst after | Median after |
|---|---|---|---|---|
| 1 Bow | 3.19e-3 | 282 | 0.90 | 0.047 |
| 2 HandStrike | 1 | 0.017 | 0.017 | 0.009 |
| 3 FeltMallet | 7.51e-2 | runaway (SheetMetal) | 0.99 | 0.048 |
| 4 HardMallet | 1.58e-3 † | 383 | 0.037 | 0.004 |
| 5 Drumstick | 0.138 | 30 | 0.62 | 0.153 |
| 6 WireBrush | 0.305 † | 1.9 | 0.35 | 0.199 |
| 7 MetalPipe | 0.107 | 523 | 0.61 | 0.070 |
| 8 MetalChain | 1 | 0.001 | 0.001 | 0.000 |
| 9 StiffPoint | 1.18e-3 | 9 705 | 0.90 | 0.076 |
| 10 HeavyGrinding | 1.80e-3 † | runaway (CoilSpring) | 0.21 | 0.005 |
| 11 CorrugatedDrag | 1.95e-3 | 461 | 0.90 | 0.027 |
| 12 TensionRise | 4.28e-2 | 17.5 | 0.90 | 0.105 |
| 13 PneumaticJet | 9.92e-4 | 9 870 | 0.90 | 0.085 |
| 14 ElectromagneticHum | 7.89e-4 | 1 141 | 0.90 | 0.080 |
| 15 TensionSnap | 0.240 | 1.6 | 0.99 | 0.165 |
| 16 ParticleRain | 4.74e-3 † | runaway (CoilSpring, Plate) | 0.087 | 0.020 |

† A runaway threshold: just above it, a contact or feedback loop diverges. The trim is set at half the highest stable value.
- HardMallet and HeavyGrinding are left at about −47 dBFS median, so they are effectively silent. Before this change they were running away into the clamp.
- Making them usable needs their contact models fixed. That is left as a follow-up; it is not a level question.
- IndustrialCog C2 sets the worst case for most exciters.

**Re-based tests (task 2.3).** Each of these was masked by clipping before, and each failure was checked against the new levels:
- `sixteen_simultaneous_notes_all_sound` (Bow on Pipe): bowed voices now peak at about 0.008, so the "silent" threshold moves from 0.01 to 1e-3. The tail threshold is 1e-4.
- `every_exciter_has_the_same_level_at_any_sample_rate`:
  - Bow: +1.56 dB at 44.1 kHz. The stick-slip limit cycle used to clip, so the tolerance goes to 2 dB.
  - PneumaticJet: +2.87 dB at 96 kHz. Its force RMS matches at every rate, but its hash noise (`sin(phase)·43758`, with an f32 phase growing 1.618 per sample) coarsens faster at higher rates. The tolerance goes to 3 dB.
- `struck_attack_timing_does_not_depend_on_sample_rate`: HardMallet's loudest peak now lands in its rebounds (53 ms at 96 kHz against 108 ms at 48 kHz), so it is timed on its first 20 ms, like the drumstick.
- `hosted_normalized_note_velocity_reaches_output_chain` passes unchanged.

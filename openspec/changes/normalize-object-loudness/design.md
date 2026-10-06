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

## Risks / Trade-offs

- [The bowed level also depends on decay (τ), and `L` ignores τ, so a long-ringing bowed bank could still overdrive] → The overdrive test catches it. If it fails, extend the bound to `Σ gain_i · min(τ_i, τ_cap) / f_i` for friction-driven notes, or cap the friction family's makeup. Record the decision in Implementation findings rather than guessing now.
- [The peak sits below the bound by a bank-dependent amount, depending on how mode phases align] → The spec allows 6 dB across objects and 3 dB across pitch. If an object misses the band, the fix is a small per-object trim on top of `L` (a constant), recorded in the findings.
- [Every sound changes level, and Tank, Cog and Pipe lose their clipped grit] → Accepted (proposal). The presets are placeholders.
- [Exciters that couple to resonator displacement or velocity (bow, hand strike, mallets) see a different impedance once the bank is scaled] → Intended side effect. Exciter-family loudness stays a non-goal, and the overdrive test bounds the result.

# Design

## Context

`ModalModeSpec::corroded`, `heated`, `sludge_loaded` and `damaged` (`src/dsp/profile.rs`) run per mode at note-on, in `transform_algorithm_modes_into`. After them, H11 prunes modes that can't sound (decay under 0.1 ms or above 80 kHz), and the bank is normalized to `STRIKE_LEVEL_REF` from mode gains, not decays. A uniform gain change is therefore cancelled, and a decay collapse isn't compensated. Pitch dynamics (TautCable, SheetMetal) go through `target_pitch_factor` and H12's incremental retune whenever `has_dynamics()` is true.

## Goals / Non-Goals

**Goals:** each control audible across its range, with no silence and no lost fundamental, and Heat doing what its name says.

**Non-Goals:**
- Changing ranges, IDs or the order of the transforms.
- Sludge and Thickness (not reported; Sludge's uniform gain loss is also cancelled by normalization, noted for later).
- Softening the exciter for Heat (the specification's `Exciter_Stiffness · (1 − 0.3·heat)`). A filter between the exciter and the resonator would bypass the implicit contact solver that keeps mallets and grinders stable (`stabilize-mallet-and-grinding-contact`). Dropped by the maintainer (2026-11-09): Heat stays spectral darkening plus warble.

## Decisions

1. **Rust as a damping increase.** `decay /= 1 + rust·(0.06 + 0.54·w²)` and `gain /= 1 + rust·0.6·w^1.5`, where `w` is the mode's position from fundamental (0) to top (1). At Rust 5 the fundamental's decay is divided by 1.3 and the top mode's by 4. The specification's frequency-power law, expressed by mode position so it doesn't depend on the absolute pitch.
2. **Damage saturates.** `s = 3·(1 − e^(−d/3))` (equal to `d` near 0, 2.85 at 10) drives the detunes in place of `d`, keeping today's character below about 1. The primary decay is divided by `1 + 0.12·s`. The companion keeps `(0.28 + 0.18·low)` of the decay (always positive). Its gain is `gain·(0.16 + 0.20·w)·s`, capped at the primary's gain. Damage 1 stays within 2 dB of Damage 0 in strike level (existing requirement).
3. **Heat:**
   - Pitch drop: unchanged (`1 − 0.05·heat`).
   - Brightness: a tilt, `gain /= 1 + heat·2.0·w^1.5`, and `decay /= 1 + heat·0.5·w`.
   - Warble: a pitch factor `1 + heat·0.0046·(0.6·sin 2π·0.37t + 0.4·sin 2π·0.83t)`, about ±8 cents, two incommensurate rates so it doesn't sound periodic. It is multiplied into `target_pitch_factor`, and the phase is advanced per sample in `prepare_sample` while heat > 0. `has_dynamics()` is true while heat > 0.
4. **Bounds measured, not assumed.** The 6 dB bound and the fundamental checks run on all nine objects at C4 with HandStrike at velocity 100 over 2 s. If a constant above doesn't meet them, it is tuned, and the final values are recorded under Measurements.

## Risks / Trade-offs

- **[Heated notes take the dynamic-retune path]** → H12's incremental retune bounds that cost (dynamics ≤ 1.6× static). The warble is slow, so most samples skip.
- **[The golden fixture changes]** → It is regenerated with the reason, on gnu and musl.

## As built

- **Rust:** the fundamental's term is 0.2, not the 0.06 of decision 1. With 0.06, Rust barely shortened the low modes (decay ÷1.05 at Rust 0.85), and the existing renders `rust_render_is_deterministic_and_reduces_brightness_and_decay` and `high_rust_render_is_darker_and_shorter_than_low_rust_render` failed, because removing the highs made the note relatively longer. 0.15, 0.2 and 0.25 all pass them. 0.2 halves the fundamental's decay at Rust 5 and keeps it audible.
- **Damage:** the primary decay divisor is `1 + s·(0.04 + 0.08·low)`, a little milder on high modes than the `1 + 0.12·s` of decision 2. The companion's gain is capped at the primary's.
- **Heat:** `heated()` takes the mode index and count, as `corroded()` and `damaged()` do. The warble's LFOs advance in `prepare_sample` (out of line) only while heat > 0. `has_dynamics()` includes the warble, so heated notes take H12's dynamic-retune path.

## Open Questions

- None. Exciter softening for Heat was dropped by the maintainer (2026-11-09).

## Measurements

### Before (task 1.1), Pipe C4 shown; the tests cover all nine objects

| Control | Value | RMS vs control at 0 | Lowest mode | Modes |
|---|---|---|---|---|
| Rust | 3 | −9.5 dB | 268 Hz | 1 |
| Rust | 4, 5 | silent | — | 0 |
| Damage | 4 | −14.5 dB | 279 Hz | 7 |
| Damage | 6 | −5.4 dB | **1 746 Hz** | 2 |
| Damage | 10 | silent | — | 0 |
| Heat | 1 | −0.04 dB | 255 Hz (−0.9 st) | 6 |

All six `material_transform_tests` failed. Rust and Damage hit silence or lost the fundamental on every object (SheetMetal was silent from Damage 6). Damage 5 left fewer modes than none (Pipe 6 → 3). Heat didn't change the spectral tilt (0.0 dB) or the pitch over time (0 cents).

### After (tasks 2.1–3.2), all nine objects, C4 HandStrike velocity 100, 2 s RMS relative to the control at 0

| Control | Values | Worst level change | Largest fundamental shift |
|---|---|---|---|
| Rust | 1–5 | −2.84 dB (Plate, Rust 5) | 0 |
| Damage | 1–10 | −3.23 dB (IndustrialCog, Damage 10) | 0.55 semitones |
| Heat | 0.5, 1 | −0.07 dB (SheetMetal, Heat 1) | 0.89 semitones (the intended 5 % drop) |

Per object, Rust at 5: Pipe −2.7, Plate −2.8, Tank −1.7, Chain −2.5, IBeam −2.6, TautCable −2.4, CoilSpring −2.6, SheetMetal −2.5, IndustrialCog −2.7 dB. Damage at 10: Pipe −1.9, Plate −2.5, Tank −0.8, Chain −2.7, IBeam −1.8, TautCable −1.6, CoilSpring −0.6, SheetMetal −1.3, IndustrialCog −3.2 dB.

- **Heat warble:** the Pipe fundamental wanders 14.4 cents peak to peak over 3 s at Heat 1 (0 at Heat 0). At Heat 1 every object's top mode is at least 6 dB further below its fundamental than at Heat 0.
- **Golden fixture:** all 108 cases build with Rust 0.4 and Heat 0.2, so all changed; the mode counts didn't. It matches on gnu and musl.
- **Cost of the warble:** heated notes take the dynamic-retune path. 16 Render Plate voices for 2 s, Heat 1 against Heat 0 in the same run (15 twin pairs per run, release build): medians 1.184, 1.202 and 1.188 over three runs (re-measured after restoring `has_dynamics`'s `#[inline]`, which an edit had moved onto the new function; with it misplaced the medians were 1.22–1.23). This is inside the 1.6× the maintainer accepted for pitch dynamics in H12, and part of it is the heated note sounding different.

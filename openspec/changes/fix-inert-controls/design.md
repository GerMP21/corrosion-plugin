# Design

## Context

`print_slot_effects` (`src/guide/clip_plan/tests.rs`, ignored) rendered one note, C3 at velocity 100, held 2 s, with each control at its minimum and maximum on its guide clip patch (the control's exciter or object on Plate for slots, Hand Strike on Plate otherwise, Render quality). Recorded 2026-11-14 in `write-user-guide-content`'s design:

| Control | Level min → max (dB) | Ring (s) | Bands (dB) |
|---|---|---|---|
| Slip Curve | −19.2 → −19.2 | 3.98 → 3.98 | 0.00 |
| Chatter Pitch | −17.5 → −17.5 | 4.00 → 4.00 | 0.00 |
| Chatter Damping | −17.5 → −17.5 | 4.00 → 4.00 | 0.00 |
| Chain Mass | −16.5 → −16.6 | 2.44 → 2.44 | 0.00 |
| Flesh Stiffness | −17.1 → −17.1 | 2.31 → 2.30 | 0.04 |
| Core Hardness | −26.4 → −26.4 | 0.74 → 0.74 | 0.03 |
| Grind Pressure | −12.5 → −12.4 | 4.00 → 4.00 | 0.06 |
| Link Count | −16.2 → −16.3 | 2.50 → 2.48 | 0.06 |
| Restitution Bounciness | −16.4 → −16.4 | 2.37 → 2.37 | 0.07 |
| Micro Bounce Limit | −16.4 → −16.4 | 2.37 → 2.37 | 0.16 |
| Compression Curve | −26.2 → −26.4 | 0.74 → 0.74 | 0.22 |
| Tip Stiffness | −16.3 → −16.5 | 2.39 → 2.37 | 0.25 |
| Strike Position, Position Envelope, Fundamental Anchor, Spring Dispersion, Position Wander Rate | no change | | 0.00 |

The object and envelope rows were measured on one patch only; some of them may act only in a context the clip patch lacks (task 1.1 decides).

Help texts against the measurement: Mute Decay 0.85 → 0.999 gives −8.1 dB and ring 2.23 → 1.46 s (help: lower mutes sooner); Flesh Damping +4.3 dB and brighter (help: softens the attack); Rattle Color −3.9 dB, darker (help: brighter; it raises a high-pass on the rattle noise); Stick Mass and Pipe Mass brighter (help: darker); Voltage Sag adds the 3rd and 5th harmonics (help: wavers and sags).

## Goals / Non-Goals

- Goal: every sound-shaping control audible in its context, in the direction its help says, guarded by a test.
- Non-goal: redesigning an exciter's model beyond what makes its controls act; new controls.

## Decisions

1. **Measure in context first** (task 1). The floor is 1 dB level, 10 % ring or 1 dB of band change beyond level, per the spec. Each failing control gets a cause before any fix.
2. **Fix per cause**, smallest change that makes the control act. Candidates seen so far: Stiff Point Scrape slips on every sample (deflection grows by `v_rel·0.001` per sample against a 0.05–0.35 threshold), so its spring never shapes the force; force-scaling controls (Chain Mass, Grind Pressure) appear normalised by the per-exciter drive level and the governor.
3. **Each fix is a decision for the maintainer when it changes a default sound**: the change stops and asks with the before and after numbers, as the project does.
4. **The guard** reuses the measurement in CI for the listed controls, at a lower floor than the spec's so render noise across libm can't trip it.

## Risks / Trade-offs

- [Presets move] → Reseed and re-level the bank; the bank tests and the guide clips follow.
- [A fix makes a model unstable] → `contact_is_stable_without_the_governor` and the overdrive tests run on every fix.

## Open Questions

- Which inert object controls are real (task 1.1).

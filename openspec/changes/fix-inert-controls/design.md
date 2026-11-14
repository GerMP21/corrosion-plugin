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

## Measurements

### Task 1.1: every control in its context (2026-11-14)

`print_control_responses` (`src/guide/control_response/tests.rs`, ignored, run in release) plays each sound-shaping control at its minimum, middle and maximum on its clip patch plus the context it needs (`control_response::context`), one note C3 at velocity 100 (two notes, velocity 0.25 and 1, for velocity controls), held 2 s (3 s for wander) with 2 s of tail. It reports the largest difference between any two points in level, ring length, octave-band shape (beyond the level change), stereo width (side over mid) and the 10 ms envelope (beyond the level change). The floor is 1 dB, 10 %, 1 dB, 1 dB and 1 dB.

Three points, not two: Strike Position's ends are mirror images (`sin(nπP)` has the same magnitude at P and 1 − P), so its minimum and maximum render alike. The envelope measure was added for controls that only move the sound in time. Contexts that were wrong at first and fixed: Space Mode needs Space Amount; the diode needs the chain driven into a low ceiling; Wander Rate needs a bow (a strike touches the object only at the start).

147 controls measured (UI Scale, Quality Mode, Note Mode and Limiter Mode excluded with reasons). **20 don't reach the floor:**

| Control | Level dB | Ring | Bands dB | Width dB | Env dB | Probable cause |
|---|---|---|---|---|---|---|
| Release (Pneumatic Jet; Hand Strike gave the same) | 0.00 | 0 % | 0.00 | 0.00 | 0.00 | The exciter's `release()` stops it at note-off, so the ADSR release never plays; on a strike the contact is over before the AD release matters |
| MSEG Release (Bow) | 0.00 | 0 % | 0.00 | 0.00 | 0.00 | Same: `bow_exciter.release()` drops the contact at note-off |
| Slip Curve | 0.00 | 0 % | 0.00 | 0.00 | 0.00 | Only widens the Stribeck transition (`stribeck_speed`), which the bow's relative velocity never sits in at these speeds |
| Chatter Pitch | 0.00 | 0 % | 0.00 | 0.00 | 0.00 | Stiff Point Scrape slips every sample: its deflection grows past the threshold each sample, so the spring force it shapes is never returned |
| Chatter Damping | 0.00 | 0 % | 0.00 | 0.00 | 0.00 | Same |
| Chain Mass | 0.00 | 0 % | 0.00 | 0.01 | 0.01 | Force scaling, normalised by the drive level and the governor |
| Restitution Bounciness | 0.00 | 0 % | 0.07 | 0.03 | 0.02 | Drumstick bounces aren't reached, or don't add audible force |
| Core Hardness | 0.01 | 0 % | 0.05 | 0.05 | 0.02 | Felt core term not reached at these velocities |
| Micro Bounce Limit | 0.01 | 0 % | 0.16 | 0.05 | 0.04 | As Restitution |
| Component Tolerance | 0.00 | 1 % | 0.01 | 0.00 | 0.07 | Filter tolerance noise too small to hear, even at resonance 0.6 |
| Flesh Stiffness | 0.05 | 0 % | 0.04 | 0.15 | 0.11 | Force scaling, normalised |
| Compression Curve | 0.12 | 3 % | 0.26 | 0.69 | 0.22 | Velocity-dependent term, too small |
| Impact Damping | 0.02 | 2 % | 0.56 | 0.06 | 0.39 | Hard mallet damping, too small |
| Grind Pressure | 0.10 | 0 % | 0.06 | 0.40 | 0.52 | Force scaling, normalised |
| Link Count | 0.31 | 2 % | 0.34 | 0.35 | 0.73 | Links land too close together to change the envelope |
| Chaos Depth (Drive Amt 3) | 0.06 | 2 % | 0.09 | 0.14 | 0.36 | Lorenz modulation of the drive is mild |
| Wire Stiffness (Spring) | 0.13 | 0 % | 0.05 | 0.10 | 0.20 | Spring reverb's dispersion barely changes |
| High Frequency Damping (Echo) | 0.44 | 2 % | 0.18 | 0.16 | 0.41 | Echo repeats decay too fast to hear the darkening |
| Cable Braid | 0.01 | 0 % | 0.69 | 0.55 | 0.15 | Some spectral change, under the floor |
| Diode Softness (driven) | 0.78 | 0 % | 0.38 | 0.96 | 0.47 | Close to the floor; acts only when clipping |

The causes are first readings from the code; task 2 confirms each before changing it. Earlier single-note findings that now pass in context: the gesture modifiers and Speed and Roughness (on Bow), Position Envelope, Fundamental Anchor (near a node), Strike Position (with its middle), Spring Dispersion, Bias Starvation, Diode Asymmetry, Space Mode, Wander Rate.

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

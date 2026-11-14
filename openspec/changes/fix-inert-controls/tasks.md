# Tasks

## 1. Measure

- [x] 1.1 Extend the measurement to put each control in its context (effect on, drive engaged, release, two velocities); list every control under the floor with its cause in design.md
- [x] 1.2 Write the control-response test from the list; run it red

## 2. Fix

- [x] 2.1 Stiff Point Scrape: Chatter Pitch and Chatter Damping shape the force
- [x] 2.2 Force controls normalised away (Chain Mass ✓ with Metal Chain's audible links; Grind Pressure ✓, Flesh Stiffness ✓, Core Hardness ✓, Impact Damping ✓; Grind Pressure, Flesh Stiffness, Core Hardness, Tip Stiffness and others from 1.1)
- [x] 2.3 Bounce and count controls (Restitution Bounciness ✓, Micro Bounce Limit ✓ by Drumstick re-strikes, Link Count ✓ by 2.2, Compression Curve ✓, Slip Curve ✓ by the sliding branch)
- [x] 2.4 Object and envelope controls found inert in 1.1 (Release ✓, MSEG Release ✓, Chaos Depth ✓, Braid Stiffness ✓, Wire Stiffness ✓, Component Tolerance ✓; High Frequency Damping and Diode Softness accepted under the floor by the maintainer, `UNDER_FLOOR`)
- [x] 2.5 Help texts: Mute Decay, Flesh Damping, Rattle Color, Stick Mass, Pipe Mass, Voltage Sag, and any other 1.1 finds (EN and ES)

## 3. Close-out

- [ ] 3.1 Bank tests (no reseed: the maintainer keeps the factory presets as they are, 2026-11-14); guide data and clips regenerated; drop `internal_rattle`'s QUIET declaration and the backlog's "Metal Chain at Internal Rattle 0" once the clip check confirms it is audible
- [ ] 3.2 Docs: ARCHITECTURE, design.md measurements; fmt, clippy, full lanes; commit and push to `dev`; CLAUDE.md Current state

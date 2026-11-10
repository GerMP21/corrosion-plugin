## 1. Tests first

- [x] 1.1 `harder_playing_never_silences_the_jet`: every object × Air Pressure {0.1, 1.1, 2.1} × Turbulence Chaos {0, 0.6, 2}, velocity 64 vs 127; fails before the fix (27 of 81 silent)
- [x] 1.2 `print_jet_default_notes` (ignored): hash and RMS of the default notes at velocities 100 and 127, recorded before the fix

## 2. Fix

- [x] 2.1 `PneumaticJet`: saturation knee `2/(3·chaos)` (infinite at chaos 0) computed in `set_parameters`; the relative velocity is held at the knee before the cubic
- [x] 2.2 Default notes at velocity 100 bit-identical; velocity 127 RMS unchanged to 0.01 dB (design, Measurements)

## 3. Bank and sweep

- [x] 3.1 Reseed the factory bank; the three silenced jets go back to try 0
- [x] 3.2 `print_velocity_sweep` (ignored): every parameter at both ends of its range, all exciters, three objects; findings in design

## 4. Close out

- [x] 4.1 `docs/ARCHITECTURE.md` (exciter notes), `docs/BACKLOG.md` (Env Amount question)
- [x] 4.2 fmt, clippy `-D warnings`, the test lanes, commit and push

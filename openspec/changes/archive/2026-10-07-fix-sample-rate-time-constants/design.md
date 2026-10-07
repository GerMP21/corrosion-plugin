# Design

## Context

The per-sample constants found by inventory, with `r = 48 000 / fs`:

| Where | Per-sample code | Kind → conversion |
|---|---|---|
| `voice/mod.rs:1256, 1335` | `highpass = 0.8·hp + 0.2·x` | one-pole pole → `0.8^r` |
| `voice/mod.rs:464` | `PEAK_DECAY = 0.999` | decay → `c^r` |
| `voice/mod.rs:463` | `TAIL_DEACTIVATE_FRAMES = 4800` | frame count → `/r` |
| `resonators/core.rs:916, 995` | `velocity = disp − prev_disp` | per-sample difference → `× fs/48k` (reported in 48 kHz-sample units, which the exciters assume) |
| `resonators/core.rs:857–858` | dynamics trackers `0.99 / 0.01`, `0.95 / 0.05` | one-pole → `c^r` |
| `exciters/bow.rs:88–89` | `v += (t − v)·0.001`; `pos += v` | smoother; step `× r` |
| `exciters/hand_strike.rs` | `v *= 0.99`; `pos += v·0.01`; `decay_state *= mute_decay` | decay; step; decay |
| `exciters/felt_mallet.rs` | `pos += v·0.001`; `v −= compression·0.5`; `v += 0.01`; `v *= 0.995` | step; per-sample acceleration `× r`; acceleration; decay |
| `exciters/hard_mallet.rs` | `pos += v·0.0005`; `v −= transfer/m`; `v *= 0.5` while `impact_count > 5`; `v += 0.02`; `v *= 0.98` | step; acceleration; decay plus count `/r`; acceleration; decay |
| `exciters/drumstick.rs` | `pos += v·0.0003`; `v −= decel·0.1`; `v += 0.05`; `v *= 0.999` | step; acceleration; acceleration; decay |
| `exciters/metal_pipe.rs` | `pos += v·0.0005`; `v += out·0.01`; `v −= F·0.05/m`; `v += 0.02`; `v *= 0.995` | step; acceleration ×3; decay |
| `exciters/metal_chain.rs` | `link.pos += v·0.0005`; `v += 0.03` (time already in ms) | step; acceleration |
| `exciters/wire_brush.rs` | single-sample impulses `amplitude` (timing already in ms) | impulse area → amplitude `/ r` |
| `exciters/stiff_point.rs` | `deflection += v_rel·0.001` | step |
| `exciters/tension_rise.rs` | `tension += v_rel·0.001`; `impulse_samples = 50` | step; count `/r` |
| `exciters/corrugated_drag.rs` | `position += v_rel·0.01` | step |
| `exciters/heavy_grinding.rs` | `noise *= 0.99`; `filter += (cutoff/10000)·(x − f)` | decay; smoother |
| `exciters/other_specialty.rs` PneumaticJet | `filter += (center/10000)·(raw − f)` | smoother |
| `exciters/other_specialty.rs` ElectromagneticHum | `dt = 1/self.sample_rate`, but **the voice never sets it** (always 48 kHz) | **bug:** wrong hum pitch at other rates; set the rate |
| `exciters/other_specialty.rs` TensionSnap | `pull_position += 0.001` | step |
| `exciters/other_specialty.rs` ParticleRain | `spawn += flow·0.1`; `p.v += 0.05`; `p.age += 0.001`; `decay = 0.99` | rate; acceleration; step; decay |
| `post_processing/fem_body.rs:65–72` | `decay` per sample (`r² = 1 − d`); `force = 0.05·x` | pole → `1 − (1−d)^r`; sustained-drive gain `× r²` (as in the resonator) |
| `dsp/interaction.rs` wander | already `rate_hz / sample_rate` | no change |

Deterministic `GOLDEN_STEP` and `rng_phase` noise sequences stay per sample (a non-goal).

## Goals / Non-Goals

**Goals:**
- Sample-rate-independent exciter, voice-tail, stealing-level and body behaviour.
- Bit-identical at 48 kHz.

**Non-Goals:**
- **Stochastic `GOLDEN_STEP` phase noise.** It stays per sample; the result is still broadband noise.
- **Changing any 48 kHz sound.**
- **The resonator gain** (covered by `fix-resonator-sample-rate-gain`).
- **Post-chain stages that already derive their coefficients from the sample rate** (filter, smoothers, reverbs, echo, spread delay).

## Decisions

1. **Three conversion forms, each exactly the identity at 48 kHz**, through a shared helper with `ratio = 48 000 / fs`:
   - **Decay or one-pole factors:** `c^ratio`. Rust's `powf(c, 1.0)` returns `c` exactly.
   - **Integration steps and phase rates:** `step · ratio`.
   - **Smoothers of the form `x += (t − x)·k`:** `1 − (1 − k)^ratio`.

   Coefficients are cached and recomputed only when the sample rate changes, so there's no per-sample `powf`.

   *Alternative:* re-express every constant as seconds (τ) and compute `exp(−1/(τ·fs))`. That's cleaner to read, but not bit-identical at 48 kHz, so every golden test would need re-capturing.

2. **Frame counts scale with `fs / 48k`.** `TAIL_DEACTIVATE_FRAMES` becomes `0.1 s · fs`, which is exactly 4 800 at 48 kHz.

3. **Exciters get the sample rate the same way the resonator does.** Each exciter that needs it gains a cached `rate_ratio` set by a `set_sample_rate`, which the voice calls at note-on and whenever the rate changes. This avoids widening every `process_sample` signature. There's no allocation, and nothing changes in `tests/no_alloc.rs`.

4. **The FEM body is normalized like the resonator.** Its per-sample `decay` becomes a factor-from-time. Its force term is scaled so the body's level and ring time hold across rates, matching the "same level and ring time" scenario.

5. **Tests are at voice level with spec tolerances.** New tests go in `src/voice/mod.rs` (crate-internal, so they can see `is_rendering` and timing), plus a body test in `fem_body.rs`. Bit-identity at 48 kHz rests on two things:
   - the conversion helpers are exactly the identity at `r = 1`, checked by a unit test;
   - an ignored test prints per-exciter 48 kHz render hashes, run before and after the change on the development machine, with the comparison recorded in the findings.

   Raw render hashes can't be asserted in CI, because libm differs between the musl and glibc targets (found in `fix-resonator-sample-rate-gain`). The existing 48 kHz golden suites (mode-bank fixture, envelope golden, preset level check) also have to pass unchanged.

## Risks / Trade-offs

- [Stateful contact models (mallets, drumstick) are nonlinear, so a time-step change isn't just a rescale, and some exciters may still miss the 1.5 dB / 10 % band at 44.1 kHz because of discretization] → Measure per exciter. If one misses, record it in Implementation findings with its numbers. Tolerances may only widen for that exciter, with a reason. A stochastic exciter is checked by RMS only.
- [The bit-identity claim depends on the helper returning exact identities at a ratio of 1.0] → A unit test on the helper (ratio 1.0 → exact), plus the 48 kHz golden hashes.
- [Many files are touched] → Each exciter is its own task with its own check, so a regression points to one file.

## Implementation findings

### Before the change (task 1.2)

On the new tests, with the resonator gain fix already in place, these exciters missed the 1.5 dB band (Pipe C4, velocity 100, 1 s held + 1 s released, RMS vs 48 kHz). The rest were already within it.

| Exciter | 44.1 kHz | 96 kHz |
|---|---|---|
| 2 HandStrike | +1.53 dB | −12.27 dB |
| 5 Drumstick | −2.26 dB | −3.66 dB |
| 6 WireBrush | +9.04 dB | — |
| 8 MetalChain | −2.17 dB | −3.61 dB |
| 12 TensionRise | +9.96 dB | −3.73 dB |
| 15 TensionSnap | — | −9.63 dB |

The FEM body rang for 0.025 s at 44.1 kHz against 0.023 s at 48 kHz.

### Bit-identity at 48 kHz

The ignored `print_48k_exciter_render_hashes` and `print_48k_body_render_hash` tests were run before the first edit and after the last one on the development machine (musl target). All 17 hashes match: 16 exciters plus the body.

### Additions to the inventory

- **One-sample force spikes keep their area: amplitude `/ r`.** This covers the tension-rise break, the tension-snap release and the metal-chain impacts, via `rate::impulse`. Before this, an impulse lost half its effect at 96 kHz, because one sample is half as long.
- **Metal-chain impacts:** the first contact sample's penetration grows with the step, so it is also divided by `r` before the `^1.5` contact law.
- **Metal-chain rattle noise** is per-sample white noise. It is scaled by `1 / √r` (`rate::white_noise`) so its density in the audible band holds.
- **Metal-chain rattle filter:** its "high-pass" (`apply_hpf`) never keeps its state, because `noise_state` is written back before the filter runs. So `1 − alpha` is a plain gain of about `(1000/fs)/cutoff`. That gain is now computed at the reference rate.

  The state bug itself is unchanged, because fixing it would change the 48 kHz sound. It is why the chain is nearly silent on the Pipe: about −80 dBFS RMS at the defaults.
- **Wire brush:** its filter coefficient uses the reference step, and its output is scaled by `fs / 48k` so each one-sample impulse keeps its area.

### After the change

The level differences below are the spec scenario: velocity 100, vs 48 kHz. The "avg" column is the power average over velocities 70, 85, 100, 115 and 127. It shows where the velocity-100 number is one unlucky point of a chaotic response.

| Exciter | 44.1 kHz | 96 kHz | avg 44.1 / 96 | Tolerance |
|---|---|---|---|---|
| 1 Bow | +0.02 | +0.00 | +0.30 / +0.29 | 1.5 |
| 2 HandStrike | +1.31 | −2.47 | +1.10 / −1.53 | **3.0** |
| 3 FeltMallet | −0.00 | −0.01 | −0.00 / −0.01 | 1.5 |
| 4 HardMallet | −0.01 | −0.13 | +0.03 / −0.27 | 1.5 |
| 5 Drumstick | +1.26 | −2.27 | +0.05 / −2.25 | **3.0** |
| 6 WireBrush | +1.98 | −0.15 | −0.11 / −0.03 | **2.5** |
| 7 MetalPipe | +0.14 | +0.33 | +0.13 / +0.37 | 1.5 |
| 8 MetalChain | −2.10 | −1.87 | −0.85 / −2.73 | **3.0** |
| 9 StiffPoint | +0.00 | +0.00 | −0.00 / +0.00 | 1.5 |
| 10 HeavyGrinding | +0.00 | +0.00 | +0.00 / +0.00 | 1.5 |
| 11 CorrugatedDrag | +0.00 | −0.00 | +0.00 / +0.01 | 1.5 |
| 12 TensionRise | +2.30 | +1.38 | −0.76 / −0.97 | **3.0** |
| 13 PneumaticJet | −0.01 | −0.03 | −0.01 / −0.03 | 1.5 |
| 14 ElectromagneticHum | −0.00 | −0.00 | −0.00 / −0.00 | 1.5 |
| 15 TensionSnap | −0.03 | +0.06 | −0.02 / +0.07 | 1.5 |
| 16 ParticleRain | −7.21 | −6.02 | −5.23 / −2.82 | **8.0** |

The struck tail test and the FEM body test pass. The body's peak and its 40 dB ring time are within 1 dB and 10 % at 44.1 and 96 kHz. `tests/no_alloc.rs` passes in a debug build.

### Widened tolerances and why

Every widened case below is sample-grid rounding or chaos, not an unconverted constant: the exciter's continuous-time behaviour matches across rates. Fixing the remaining differences would need sub-sample timing, which changes 48 kHz output, so it is out of scope here.

- **2 HandStrike (3 dB).**
  - The strike is about 5 reference samples (≈0.1 ms) of contact, and it sits on the start of the Hit envelope's attack ramp.
  - The discrete ramp starts at `dt/attack`, not 0, and the contact length rounds up to whole samples.
  - The pulse area is then 15 at 48 kHz against 11.25 at 96 kHz (in reference-sample units). That predicts −2.5 dB; measured −2.47 dB.
  - The same short pulse on the ramp is why HandStrike is so quiet on the Pipe (peak about 0.01).
- **5 Drumstick (3 dB; timing measured on the strike).**
  - The strike and first six bounces land at the same times at every rate: 7.50 ms, 8.5 ms and 9.2 ms.
  - After the bounce limit, the stick is pinned to the resonator and pulled back into it, so it keeps chattering (about 90 contacts in 0.5 s) while the note is held. That chatter is chaotic.
  - The note's loudest peak lands in it: 82.6 ms at 96 kHz against 160.4 ms at 48 kHz. So the timing scenario checks the drumstick's peak within its first 20 ms, which is its strike.
- **6 WireBrush (2.5 dB).** Its impulse times round to the sample grid. Averaged over velocities it is within 0.2 dB.
- **8 MetalChain (3 dB).**
  - The output is about −80 dBFS (see the filter-state bug above).
  - What remains is the hash-noise generator (an f32 phase passed through `sin`), plus a handful of impacts whose contact phase against the grid is random.
- **12 TensionRise (3 dB).** Clipped stick-slip, which is chaotic. Averaged over velocities it is within 1 dB.
- **16 ParticleRain (8 dB — effectively not met).**
  - On its own, the exciter's force is rate-independent: its mean, and its spectrum at 262 Hz–2 kHz, match within 1 dB at 44.1, 48 and 96 kHz.
  - Particles spawn on the sample grid at a fixed interval (about 7.35 reference samples, a 6.5 kHz pulse train). The grid rounding aliases that train to rate-dependent frequencies, which drive different Pipe modes.
  - The level is chaotic even at 48 kHz: a 0.1 % flow-rate change moves it by up to 2.4 dB, and +3 % moves it by −5.7 dB. 48 kHz happens to be the loudest rate.
  - Fix (follow-up): sub-sample spawn timing.
- **Outside the spec's rates:** MetalPipe is +6.9 dB at 192 kHz and Drumstick −8.7 dB. Both are within 0.4 dB and 2.3 dB at 44.1 and 96 kHz. They are not tested at 192 kHz, which isn't a spec rate.

### Gate (task 4.2)

- Tests pass in debug and release (musl), and the lib tests pass on gnu.
- `cargo fmt --check` passes, and clippy with `-D warnings` is clean on 1.99 (stable, which CI uses).
- On 1.97, clippy stops at `unknown lint: semicolon_in_expressions_from_non_local_macros`. That lint is named by an `#[allow]` in `src/lib.rs` that is already on `main` (it was added for 1.99), so it isn't from this change. With `-A unknown_lints`, 1.97 is clean.
- The family render smoke test passes `scripts/check_wav.py`, with no NaNs.
- `c5_verify_revoiced_presets` passes.

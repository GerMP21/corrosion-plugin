> **Implementation status (2026-05).** This file originally described an
> "Unbound Fidelity Tier" of research-grade DSP. The **shipped** post chain is a
> set of lightweight, real-time-safe *approximations* of those targets. Each
> section below states what is **Implemented** today and keeps the original
> **Design target** for reference. Verified against
> `src/dsp/post_processing/` and `src/lib.rs`. See `docs/ARCHITECTURE.md` §9 and
> `docs/code-review.md` for the conformance matrix and known defects.
>
> Chain order (`PostProcessingChain::process`, updated 2026-10 for review
> findings C1/C5/C6): per-channel WDF filter → per-channel Lorenz drive → FEM
> body on the mono sum, added in parallel to both channels at BODY / 5 → HRTF
> spread → Space → oversampled clipper. Every quality mode runs every stage. The
> whole chain runs **per sample** today, and `lib.rs` currently sets every
> stage's parameters once per sample (a known inefficiency — see code review
> Finding 3).

---

### `4. Post / Space / Output Processing`

### 1. Multimode Ladder Filter
**Implemented (`wdf_filter.rs`):** A 4-pole ladder approximation: four unity-DC-gain
TPT one-pole stages (`g = tan(π·fc/fs)`, cutoff clamped to 0.49·fs) with a soft
"transistor" clip between stages; zero-delay resonance feedback (`k = 4·res/0.9`,
solved against the linear cascade each sample) with a `1/(1+res·3.5)`
compensation gain. Tails always decay below 90 % resonance; from 90 % up the
filter self-oscillates at any cutoff, bounded by the clips. `component_tolerance`
injects a tiny hash-noise jitter while there is input (silence stays silent). It
is **not** a Newton-Raphson WDF circuit solver. Parameters: `filter_cutoff` (20 Hz–20 kHz, clamped), `filter_resonance`
(0–1), `component_tolerance` (0–1).

**Design target:** Wave Digital Filter / RK4 transistor-ladder circuit simulation
with parasitic capacitance, thermal noise, and true electrical self-oscillation.

### 2. Drive (Soft Saturation to Fuzz Chaos)
**Implemented (`lorenz_drive.rs`):** A genuine Lorenz attractor is integrated
(σ=10, ρ=28, β=8/3, clamped state) but is used only as a **mild multiplicative
gain modulator** (`1 + chaos·tanh(x·0.02)·0.5`) on top of a soft tube curve.
`bias_starvation` gates the signal via a crude threshold on the attractor's z
state. Output is dry/wet mixed and clamped. Parameters: `drive_amount` (0–5),
`bias_starvation` (0–1), `chaos_depth` (0–1).

**Design target:** Component-level tube/diode cascade coupled into a bifurcating
Lorenz system that drives the audio into true analog chaos.

### 3. Stereo Spread
**Implemented (`hrtf_spread.rs`):** Up to ~1 ms interaural delay + a one-pole
low-pass/high-pass split shaped by `listener_proximity`, plus width-based
crossfeed. There is **no HRIR convolution and no azimuth/elevation model**.
Parameters: `spread_width` (0–1), `listener_proximity` (0–1).

**Design target:** Per-mode 3D radiation pattern convolved with measured HRIR
filters (`h_{L,n}(t,θ,φ)`).

### 4. Body Resonator
**Implemented (`fem_body.rs`):** A bank of **8 fixed second-order modes**
(220–1400 Hz base) whose frequencies shift with `chassis_material` and
`chassis_volume`. It is a small modal coloration stage, **not** a finite-element
mesh. Parameters: `chassis_material` (0–1), `chassis_volume` (0–1; tuning only,
larger = lower). It runs in parallel on the mono sum of the driven channels and
is added to both at `body / 5` (the `Body` control, 0–5). Because lower modes
ring with more gain, a larger chassis also sounds somewhat louder at the same
BODY.

**Design target:** Pre-computed 3D FEM mesh of a wooden/metallic body with
thousands of interconnected nodes.

---

### `Space Modes`

> **Timing is sample-rate independent** (`h10-scale-space-effects-with-sample-rate`).
> Every Space delay is tuned as a sample count at 48 kHz and scaled by
> `sample_rate / 48 000`, so it is exactly the tuned value at 48 kHz and the same
> time at 44.1, 88.2, 96 and 192 kHz. Buffers are heap slices sized in `new()` for
> the longest delay at 192 kHz (`space::MAX_SAMPLE_RATE`), so no control position
> clamps and the audio thread never allocates:
>
> | Stage | Delay | Buffer at 192 kHz |
> |---|---|---|
> | Factory combs | 1553/1657/1789/1913 samples at 48 kHz × `0.5 + 1.5·size` | 15 312 each |
> | Factory allpasses | 200 / 300 samples at 48 kHz | 1 208 each |
> | Spring | 80 ms × `0.5 + tank` | 23 048 per channel |
> | Echo | `0.02 + 0.78·delay_time` s, ±10 % modulation, rounded to a sample | 168 968 per channel |
> | HRTF spread (before Space) | up to 1 ms | 256 per channel |
>
> About 1.8 MB per post chain. The four combs keep their mutually-prime
> spacing, so they stay distinct at every size.

### 5. Factory Reverb
**Implemented (`space.rs::FactoryReverb`):** A Schroeder-style reverb — 4 comb
filters + 2 allpass diffusers, with `wall_impedance` setting feedback and
`machinery_clutter` modulating it. It is **not** an FDTD wave solver.
> ✅ The old cumulative `update_delays` re-scaling (code review 2026-05,
> Finding 2) is fixed and has a regression test.

**Design target:** 3D Finite-Difference Time-Domain acoustic wave solver with
diffraction around injected boundary nodes.

### 6. Spring Reverb
**Implemented (`space.rs::SpringReverb`):** One delay line per channel (two
instances in the post chain, so each channel's tail comes only from its own
input) with a one-pole dispersion filter and tension-controlled feedback. **Not**
a helical PDE.
Parameters: `spring_tension`, `wire_stiffness`, `spring_tank_size`.

**Design target:** FDTD stiff-string PDE capturing longitudinal/transverse/
torsional waves.

### 7. Factory Echo
**Implemented (`space.rs::FactoryEcho`):** A modulated stereo delay;
`machinery_movement` modulates the read position for a gentle Doppler/detune,
`high_frequency_damping` rolls off the wet path. This is a reasonable match to the
design intent. Parameters: `delay_time`, `machinery_movement`, `high_frequency_damping`.

**Design target:** Doppler-shifted spatial multi-path delay off moving objects.

### 8. Output Block (Clipper + Limiter)
**Implemented (`oversampled_clipper.rs` + `lib.rs`):** A soft diode-style clip
with adjustable `analog_ceiling` (default 0.9661 ≈ −0.3 dBFS) and `diode_softness`.
The final output limiter (`apply_output_limiter`) hard-clamps to ±0.9661.
> ✅ The old zero-order-hold "oversampling" no-op (code review 2026-05,
> Finding 1) is fixed: the clipper interpolates between samples, so the
> 1×/4×/8×/16× factors give different, progressively less aliased output.

**Design target:** 16× polyphase upsample → analog diode clip → linear-phase
anti-aliasing decimation, guaranteeing a true-peak ceiling.

---

### Quality modes

`QualityMode` (`Eco`/`Normal`/`High`/`Render`) maps to `PostQualityMode`:

| Mode | Clipper oversample | Chain |
|---|---|---|
| Eco | 1× | full |
| Normal (default) | 4× | full |
| High | 8× | full |
| Render | 16× | full |

Since 2026-10 (review finding C5) Eco no longer bypasses the body, spread or
Space; quality modes trade fidelity and CPU (oversampling and, at the voice
layer, modal density), not effects.

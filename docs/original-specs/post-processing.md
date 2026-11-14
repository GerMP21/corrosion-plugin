> **Implementation status (checked 2026-11-14).** This file originally described an
> "Unbound Fidelity Tier" of research-grade DSP. The **shipped** post chain is a
> set of lightweight, real-time-safe *approximations* of those targets. Each
> section below states what is **Implemented** today and keeps the original
> **Design target** for reference. Verified against
> `src/dsp/post_processing/` and `src/lib.rs`. See `docs/ARCHITECTURE.md` §9 for
> the as-built reference, and `docs/archive/FABLE-CODE-REVIEW-2026-07-07.md`
> for the review findings and their fixes.
>
> Chain order (`PostProcessingChain::process`, updated 2026-10 for review
> findings C1/C5/C6): per-channel WDF filter → per-channel Lorenz drive → FEM
> body on the mono sum, added in parallel to both channels at BODY / 5 → HRTF
> spread → Space → oversampled clipper (the diode stage), then output gain and
> the Hard or Lookahead limiter. Every quality mode runs every stage. The chain
> runs **per sample**; `lib.rs` sets its parameters once per buffer
> (`apply_post_chain_params`), and the stages' smoothers glide between them.

---

### `4. Post / Space / Output Processing`

### 1. Multimode Ladder Filter
**Implemented (`wdf_filter.rs`):** A 4-pole ladder approximation: four unity-DC-gain
TPT one-pole stages (`g = tan(π·fc/fs)`, cutoff clamped to 0.49·fs) with a soft
"transistor" clip between stages; zero-delay resonance feedback (`k = 4·res/0.9`,
solved against the linear cascade each sample) with a `1/(1+res·3.5)`
compensation gain. Tails always decay below 90 % resonance; from 90 % up the
filter self-oscillates at any cutoff, bounded by the clips. `component_tolerance`
drifts the cutoff by a slow random walk (0.3 s, up to 30 % deep at 1,
`fix-inert-controls`) and adds a tiny noise jitter while there is input (silence
stays silent). It
is **not** a Newton-Raphson WDF circuit solver. Parameters: `filter_cutoff` (20 Hz–20 kHz, clamped), `filter_resonance`
(0–1), `component_tolerance` (0–1).

**Design target:** Wave Digital Filter / RK4 transistor-ladder circuit simulation
with parasitic capacitance, thermal noise, and true electrical self-oscillation.

### 2. Drive (Soft Saturation to Fuzz Chaos)
**Implemented (`lorenz_drive.rs`):** A genuine Lorenz attractor is integrated
(σ=10, ρ=28, β=8/3, clamped state) but is used only as a **multiplicative gain
modulator** (`1 + chaos·tanh(0.1·x)·0.9`, so Chaos Depth swings the level,
`fix-inert-controls`) on top of a soft tube curve. `bias_starvation` starves the
input gain to 0.3 while the attractor's `z` is above `38 − 22·starvation`,
through a 5 ms one-pole follower, so it sags instead of clicking; at 0 the stage
is bit-identical to no starvation (`m06-m09-smooth-starvation-and-oversample-clipper`).
Output is dry/wet mixed and clamped. Parameters: `drive_amount` (0–5),
`bias_starvation` (0–1), `chaos_depth` (0–1).

**Design target:** Component-level tube/diode cascade coupled into a bifurcating
Lorenz system that drives the audio into true analog chaos.

### 3. Stereo Spread
**Implemented (`hrtf_spread.rs`):** The mid (L+R) is delayed up to ~1 ms with
Width, the same for both channels (not an interaural delay), through a one-pole
low-pass/high-pass split shaped by `listener_proximity`; the undelayed side is
added back, then width-based crossfeed. One delay buffer
(`m07-m08-interpolate-echo-and-tidy-spread`). There is **no HRIR convolution and
no azimuth/elevation model**.
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
`machinery_movement` modulates the read position for a gentle Doppler/detune, its
depth smoothed (τ 20 ms). Unmodulated reads are whole samples; modulated reads use
4-point Hermite interpolation (`m07-m08-interpolate-echo-and-tidy-spread`).
`high_frequency_damping` is a one-pole low-pass in the feedback loop (20 kHz,
bypassed, at 0 → 1.5 kHz at 1), so each repeat is darker. Wet level fixed at 0.7.
This is a reasonable match to the design intent. Parameters: `delay_time`, `machinery_movement`, `high_frequency_damping`.

**Design target:** Doppler-shifted spatial multi-path delay off moving objects.

### 8. Output Block (Clipper + Limiter)
**Implemented (`oversampled_clipper.rs`, `half_band.rs`, `lib.rs`):** the diode
stage (`add-asymmetric-diode-stage`). Per polarity, a curve linear below
`K = L·(1 − w)` and `K + (L − K)·tanh((x − K)/(L − K))` above, where the limit is
`L = analog_ceiling / 1.3` in every quality mode, the knee width is
`w = 0.8·diode_softness`, and `diode_asymmetry` lowers the negative limit to
`L·2^(−a)`. It runs at 4/8/16× through cascaded 2× polyphase half-band IIR stages
(Eco runs it at the host rate), followed by a 2 Hz DC blocker faded in by the
asymmetry. After output gain the selected limiter stops at `analog_ceiling`
(default 0.9661 ≈ −0.3 dBFS): **Hard** clamps, **Lookahead** limits with a delay
reported to the host as latency. Alias at a 5 kHz tone twice the ceiling:
−19.7 dB Eco, −67.0 dB Normal, −94.2 dB High, −105.0 dB Render (defaults).
> ✅ The old zero-order-hold "oversampling" no-op (code review 2026-05,
> Finding 1) is fixed (`m06-m09-smooth-starvation-and-oversample-clipper`).

**Design target:** 16× polyphase upsample → analog diode clip → linear-phase
anti-aliasing decimation, guaranteeing a true-peak ceiling.

---

### Quality modes

`QualityMode` (`Eco`/`Normal`/`High`/`Render`) maps to `PostQualityMode`:

| Mode | Clipper oversample | Modal density (at note-on) | Chain |
|---|---|---|---|
| Eco | 1× | 0.5× | full |
| Normal (default) | 4× (2 half-band stages) | 1× | full |
| High | 8× (3 stages) | 1.5× | full |
| Render | 16× (4 stages) | 2× | full |

Since 2026-10 (review finding C5) Eco no longer bypasses the body, spread or
Space; quality modes trade fidelity and CPU (oversampling and, at the voice
layer, modal density), not effects.

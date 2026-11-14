> **Implementation status (checked 2026-11-14): IMPLEMENTED — algorithmic-only.** All 9
> modal objects below ship through a single construction path: the per-object
> algorithmic generators in `src/dsp/resonators/`, configured by the object's
> curated character params. They feed a bank of second-order modes in
> `src/dsp/resonators/core.rs`, run in coupled (complex one-pole) form with the
> direct-form biquad's poles, ring amplitude `b0/ω` and a `(48 000/fs)²` input
> scale (`docs/ARCHITECTURE.md` §5). Base mode counts come from the profile tables
> (`src/dsp/profiles/`, ~6–12 modes), which are **metadata only**
> (`mode_count`/budget/tests) and don't drive sound. The quality mode scales the
> count at note-on (Eco 0.5×, Normal 1×, High 1.5×, Render 2×); damage adds
> companions (at most 96 modes per voice, `MAX_MODES`). At note-on, modes that can
> never sound are pruned (`h11-prune-unsoundable-modes`) and the bank is
> normalized to one strike level (`STRIKE_LEVEL_REF`).
>
> **Consolidation (implemented 2026-05-27):** the algorithmic path is the **only**
> path, `complex_algo` was removed, and a **curated subset** of each object's
> parameters below is exposed to the user (host IDs in `src/params.rs`): Pipe `tube_diameter`; Plate
> `aspect_ratio` + `metal_stiffness` (UI id `plate_stiffness`); Tank `tank_volume` +
> `cavity_mix`; Chain `link_mass` + `instability`; IBeam `shear_density`; TautCable
> `braid_stiffness` + `tension_drop`; CoilSpring `dispersion_chirp` + `spring_slosh`;
> SheetMetal `metal_thinness`; IndustrialCog `tooth_dissonance`. Size/decay/
> brightness-like fields stay covered by the global controls. As part of the same
> change, **Chain and Tank's cavity are now pitch-tracking** (they previously
> ignored the MIDI note), and the **dynamic hooks** — TautCable's tension-drop
> `update_dynamic_frequencies` and SheetMetal's `apply_warping` — are wired into
> the per-sample loop. See `docs/ARCHITECTURE.md` §5.

# Resonator algorithms
Same as the exciter algorithms but for resonators

---

### `Resonator` / Modal Objects Expansion
Baseline Modal Bank Equation (Applies to all objects):
$$y_n''(t) + 2 d_n y_n'(t) + \omega_n^2 y_n(t) = F_{in}(t)$$
$$\text{Output}(t) = \sum_{n=1}^{N} g_n y_n(t)$$
*(Where $N$ is the number of modes, $d_n$ is damping, $\omega_n = 2\pi f_n$, and $g_n$ is the pickup/gain for that mode).*

---

### 1. Pipe
**DSP Model:** 1D Hollow Cylindrical Waveguide (Euler-Bernoulli Transverse Modes).
**Vibecode Logic:** A hollow metal tube creates a much clearer, semi-harmonic pitch compared to a flat plate. The modes are primarily transverse (bending) with a defined mathematical relationship, making it ring cleanly, but with enough stiffness dispersion to keep it sounding metallic rather than like a pure string.
**Mathematical State (Frequency Distribution):**
The transverse modes of a free-free pipe scale roughly by the square of a mode index coefficient ($\beta_n$):
$$f_n = \beta_n^2 \cdot \sqrt{\frac{E \cdot I}{\rho \cdot A}}$$
*(For a vibecode modal bank, this is often simplified to a harmonic base with a stiff inharmonic multiplier:)*
$$f_n = n \cdot f_1 \cdot \sqrt{1 + B \cdot n^2}$$

**Exposed Synth Parameters:**
* `pipe_length` ($f_1$): Sets the core fundamental pitch.
* `tube_diameter` ($B$): Controls the stiffness/inharmonicity. A wider pipe has higher $B$, creating a more bell-like, clanky ring; a narrow pipe sounds more like a chime.
* `sustain_time`: Scales the global damping ($d_n$), allowing for moderate, musical ringing.

---

### 2. Plate
**DSP Model:** 2D Rectangular Kirchhoff-Love Plate.
**Vibecode Logic:** A flat, thick piece of metal. Because waves travel in two dimensions simultaneously, the frequencies do not form a clean harmonic series. They create a dense, inharmonic cluster that sounds like a crash or a clang rather than a distinct note.
**Mathematical State:**
Modes are calculated using a 2D grid index ($m$ and $n$):
$$f_{m,n} = K_{plate} \cdot \left( \left(\frac{m}{L_x}\right)^2 + \left(\frac{n}{L_y}\right)^2 \right)$$
*(Where $K_{plate}$ represents the material thickness and stiffness, and $L_x, L_y$ are the physical dimensions).*

**Exposed Synth Parameters:**
* `plate_size`: Shifts the entire cluster of frequencies up or down.
* `aspect_ratio` ($L_x / L_y$): The most critical parameter for tuning the "flavor" of the inharmonicity. A perfect square creates harsh, beating dissonances. A long rectangle spreads the modes out.
* `metal_stiffness` ($K_{plate}$): High values push the frequencies further apart, creating a brighter, harsher clang.

---

### 3. Tank
**DSP Model:** 3D Cylindrical Shell (Coupled Metal + Acoustic Cavity).
**Vibecode Logic:** A tank or oil barrel is a thin metal shell wrapped into a cylinder, enclosing a volume of air. It produces a massive, low-frequency "boom." The modal bank must include both the structural vibrations of the curved shell (which has circumferential, axial, and radial modes) AND the Helmholtz resonance of the trapped air.
**Mathematical State:**
The structural modes ($f_{shell}$) are highly dense in the low-mid frequencies. The cavity mode ($f_{air}$) acts as a massive low-frequency boost:
$$f_{air} = \frac{v_{sound}}{2 \pi} \cdot \sqrt{\frac{A_{opening}}{V_{tank} \cdot L_{neck}}}$$
The output relies heavily on low-damping coefficients ($d_n$) for the bottom frequencies to simulate the trapped energy.

**Exposed Synth Parameters:**
* `tank_volume` ($V_{tank}$): Dictates the deepest sub-bass boom of the cavity resonance.
* `wall_thickness`: Controls the decay time. Thinner walls sustain longer and wobble; thicker walls produce a shorter, duller thud.
* `cavity_mix`: A macro that balances the amplitude of the deep $f_{air}$ mode against the metallic $f_{shell}$ modes.

---

### 4. Chain
**DSP Model:** Chaotic Weakly-Coupled Oscillator Array (Random Matrix Theory).
**Vibecode Logic:** You explicitly stated this *must* be a true modal profile, not just noise and reverb. To model a chain as a modal object, you don't calculate one shape. You generate a massive array of high-frequency modes (representing the small, individual links) and apply **dynamic, chaotic coupling** between them. The pitch is unstable because the active modes are constantly trading energy as the links grind against each other.
**Mathematical State:**
Instead of a clean geometric formula, the frequencies ($f_n$) are populated using a Gaussian Orthogonal Ensemble (GOE) to guarantee chaotic, repelling frequencies (no two modes are perfectly harmonic).
The damping ($d_n$) is extremely high for individual modes.
The defining characteristic is the dynamic gain/coupling. The amplitude of mode $n$ modulates the input energy of mode $n+1$:
$$y_{n}''(t) + 2 d_n y_{n}'(t) + \omega_n^2 y_n(t) = F_{in}(t) + \left( C_{chaos} \cdot y_{n-1}(t) \right)$$

**Exposed Synth Parameters:**
* `link_mass`: The base frequency range of the chaotic modal cluster. Heavier links = lower, chunkier transients.
* `chain_length`: Determines the total number of modes ($N$) generated in the bank. More links = a denser, rougher transient block.
* `instability` ($C_{chaos}$): The coHere is the complete "vibecode" blueprint for the expanded Resonator/Object category. 

---

### 5. I-Beam (Girder)
**DSP Model:** Timoshenko Thick Beam Theory.
**Vibecode Logic:** Standard bar models (Euler-Bernoulli) assume the material is infinitely thin, stretching high frequencies exponentially. A massive I-Beam suffers from shear deformation and rotational inertia, compressing the high-frequency modes into a dense, metallic cluster that doesn't "ping" like a xylophone but "thuds" like a bridge support.
**Mathematical State (Frequency Distribution):**
$$f_n = f_1 \cdot \frac{n^2}{\sqrt{1 + C \cdot n^2}}$$
*(Where $C$ represents the shear coefficient. As $n$ gets very large, the frequencies become linear rather than exponential).*

**Exposed Synth Parameters:**
* `beam_mass` ($f_1$): Shifts the fundamental frequency deep into the sub-bass.
* `shear_density` ($C$): Controls the compression of the high-frequency modes. High values = duller, stiffer, more concrete-like thud.
* `rigidity_damping`: Scales the $d_n$ curve so that high frequencies decay almost instantly, leaving only the low-mid "gong" wash.

---

### 6. Taut Cable (Elevator / Mooring Wire)
**DSP Model:** Stiff String with Tension-Modulated Pitch.
**Vibecode Logic:** A thick wire under massive tension. It has a harmonic base but suffers from stiffness, pushing partials sharp. Crucially, a hard strike temporarily stretches the wire, raising the pitch, which then falls back to resting state as the amplitude decays (the "boing" effect).
**Mathematical State (Frequency & Pitch Drop):**
$$f_n = n \cdot f_0 \sqrt{1 + B \cdot n^2}$$
*(Where $B$ is the inharmonicity coefficient).*
To model the tension drop dynamically per sample:
$$f_0(t) = f_{rest} + \left(k_{stretch} \cdot \sum |y_n(t)|\right)$$

**Exposed Synth Parameters:**
* `cable_tension` ($f_{rest}$): Base tuning of the cable.
* `braid_stiffness` ($B$): How sharp the upper harmonics are pushed. Low = piano string; High = solid iron rod.
* `tension_drop` ($k_{stretch}$): How violently the pitch envelopes downward after a hard strike.

---

### 7. Heavy Coil Spring (Suspension Spring)
**DSP Model:** Highly Dispersive Helical Waveguide.
**Vibecode Logic:** Springs are defined by extreme phase dispersion. High-frequency transient energy travels through the coil faster than low-frequency energy. In a modal bank, this is represented by an incredibly dense clustering of modes at the low end that space out at the top, combined with randomized phase offsets ($g_n$) to simulate the "sloshing" spiral energy.
**Mathematical State:**
$$f_n = f_1 \cdot (n^\alpha + \text{Jitter}_n)$$
*(Where $\alpha$ is typically between $0.5$ and $1.0$ for transverse helical modes, creating extreme low-end density).*
Pickup Gain Array ($g_n$) includes a comb-filter like structure:
$$g_n = \text{Envelope}_{pickup}(n) \cdot \cos(n \cdot \phi_{dispersion})$$

**Exposed Synth Parameters:**
* `coil_length` ($f_1$): Sets the lowest resonant thud of the spring.
* `dispersion_chirp` ($\phi_{dispersion}$): Controls the severity of the "pew" or laser-like transient sound by altering the phase relationship of the pickups.
* `spring_slosh` ($\text{Jitter}_n$): Detunes the perfect mathematical array to create chaotic, beating, metallic reverberation.

---

### 8. Sheet Metal (Thunder Sheet)
**DSP Model:** 2D Plate with Dynamic Buckling (Global Non-linear Tension).
**Vibecode Logic:** A very large, extremely thin surface. It doesn't just vibrate; it buckles. The massive displacement of the low frequencies physically warps the metal, acting as a global chaotic modulator for all the higher frequencies.
**Mathematical State:**
Instead of a static $\omega_n$, the instantaneous frequency of every mode wobbles based on the total low-frequency displacement:
$$\text{Warp}_{factor}(t) = 1 + \beta \cdot \left( \sum_{k=1}^{3} y_k(t) \right)^2$$
$$\omega_n(t) = \omega_{n,0} \cdot \text{Warp}_{factor}(t)$$

**Exposed Synth Parameters:**
* `sheet_size`: Scales the overall frequency footprint of the modal bank.
* `metal_thinness` ($\beta$): The buckling coefficient. Higher values mean the sheet roars, crashes, and wobbles chaotically when hit hard.
* `edge_damping`: Controls how freely the edges vibrate. Low damping = infinite trashy wash; High damping = choked, dark crash.

---

### 9. Industrial Cog (Circular Sawblade)
**DSP Model:** Circular Free-Boundary Plate with Mode Splitting.
**Vibecode Logic:** Rectangular plates use sine/cosine distributions, but circular objects use Bessel functions, grouping the frequencies into distinct "bell-like" rings. Because an industrial cog is never perfectly symmetrical, every mode is duplicated and slightly detuned, causing harsh, metallic beating (dissonance).
**Mathematical State:**
Base frequency distribution derived from Bessel roots ($\lambda_{m,n}$):
$$f_{base} \propto \lambda_{m,n}^2 \cdot \sqrt{\frac{\text{Stiffness}}{\text{Mass}}}$$
Mode Splitting (populating the bank with pairs):
$$f_{n}^{(1)} = f_{base, n} \cdot (1 - \epsilon)$$
$$f_{n}^{(2)} = f_{base, n} \cdot (1 + \epsilon)$$

**Exposed Synth Parameters:**
* `blade_radius`: Overall pitch/tuning of the fundamental.
* `tooth_dissonance` ($\epsilon$): The imperfection variable. At $0$, it sounds like a pure, angelic crotales bell. As you increase it, the modes split, causing rapid, eerie beating and harsh dissonance.
* `blade_thickness`: Shifts the energy concentration. Thick blades ring forever at high frequencies; thin blades emphasize the midrange "clank."upling coefficient. When high, energy bleeds erratically between the modes, completely destabilizing the pitch and creating that distinct, rough "crunch."
* `friction_decay`: Controls the $d_n$ multiplier. Ensures the individual decays remain short and choppy, preventing the chain from ringing out like a single bell.

---



> **Implementation status (checked 2026-11-14): IMPLEMENTED, with three
> deviations.** All six transforms (Size, Rust, Damage, Thickness, Heat, Sludge)
> plus the velocity-expressiveness pass ship in `src/dsp/profile.rs`
> (`scaled_for_size`, `corroded`, `thickened`, `heated`, `sludge_loaded`,
> `damaged`) operating on bounded newtypes in `src/dsp/transforms.rs`. Size,
> Thickness and Sludge match the formulas below (e.g. sludge mass loading
> `·√(1/(1+sludge))`). `fix-rust-damage-heat-response` changed the other three so
> they never silence a note or lose its fundamental:
> - **Rust:** `decay /= 1 + rust·(0.2 + 0.54·w²)`, `gain /= 1 + rust·0.6·w^1.5`
>   (`w` = mode position, 0 at the fundamental, 1 at the top): highs die first.
> - **Damage:** the splitting is driven by a saturating amount `3·(1 − e^(−d/3))`,
>   so the fundamental keeps its note and no decay reaches zero. The
>   amplitude-gated rattle injection happens in the voice.
> - **Heat:** besides the pitch drop `·(1 − heat·0.05)`, it darkens the spectrum
>   (`gain /= 1 + heat·2·w^1.5`, `decay /= 1 + heat·0.5·w`) and adds a thermal
>   warble (up to about ±8 cents at Heat 1, LFOs at 0.37 and 0.83 Hz).
>
> Transforms are applied at note-on and fixed for the note, except Heat's warble,
> which moves during it. See `docs/ARCHITECTURE.md` §8.

Transformations are where a physical modeling synth truly comes alive. Unlike a standard EQ or pitch-shifter, physical transformations alter the intrinsic physics of the object. 

To achieve the "Damage character pass" and "Velocity expressiveness pass" you outlined, we need to move beyond static values and introduce non-linear, amplitude-dependent behaviors. I have mathematically defined your existing transformations and added three highly industrial new ones: **Thickness (Gauge)**, **Heat (Thermal Expansion)**, and **Sludge (Fluid Viscosity)**.

Here is the expanded Vibecode blueprint for the `Transformations` block.

---

### 1. Size (Macro-Geometry)
**DSP Model:** Global Frequency and Damping Inverse Scaling.
**Vibecode Logic:** A larger object means waves have to travel further, lowering the pitch. Furthermore, larger objects have more mass to sustain the energy, meaning lower frequencies ring out significantly longer.
**Mathematical State:**
$$f_n = \frac{f_{n, base}}{\text{Size}}$$
$$d_n = d_{n, base} \cdot \left(\frac{1}{\sqrt{\text{Size}}}\right)$$

### 2. Rust (Surface Oxidation)
**DSP Model:** Frequency-Dependent Damping Multiplier.
**Vibecode Logic:** Rust doesn't just make an object "shorter"; it specifically eats high frequencies because the oxidized surface can no longer support rapid micro-vibrations. The low-end "thud" remains, but the "ping" dies instantly.
**Mathematical State:**
$$d_n = d_{n, base} \cdot (1 + \text{Rust\_Amount} \cdot f_n^\alpha)$$
*(Where $\alpha$ is a steepness curve, usually around $1.5$ to $2.0$, forcing high frequencies to decay exponentially faster).*

### 3. Damage (Structural Compromise & Rattle)
**DSP Model:** Mode Splitting + Amplitude-Dependent Non-linear Rattle.
**Vibecode Logic:** Simple detuning sounds like a chorus pedal. True damage means the object is cracked or loose. A crack breaks the symmetry of the object, splitting each frequency into two dissonant, beating frequencies. The "rattle" only occurs when the object vibrates hard enough to cause the broken pieces to buzz against each other.
**Mathematical State:**
*Mode Splitting (Detune):*
$$f_n^{(A)} = f_n \cdot (1 - \text{Damage\_Amount} \cdot \text{Random}_n)$$
$$f_n^{(B)} = f_n \cdot (1 + \text{Damage\_Amount} \cdot \text{Random}_n)$$
*Threshold Rattle Injection (Industrial Chatter):*
$$F_{chatter}(t) = \text{Noise}_{HPF}() \cdot \max(0, |v_m(t)| - \text{Rattle\_Threshold}) \cdot \text{Damage\_Amount}$$
*(This chatter force is injected directly back into the resonator during high-velocity swings).*

---

### `New Additions`

### 4. Thickness (Material Gauge)
**DSP Model:** Inharmonicity / Stiffness Modifier.
**Vibecode Logic:** Size changes the *length* of the pipe/plate, but Thickness changes the *gauge*. A massive, paper-thin sheet of metal sounds completely different from a small, 3-inch-thick vault door. Thickness drastically alters the spacing of the overtones without changing the fundamental note.
**Mathematical State:**
Instead of a simple harmonic multiplier, Thickness increases the stiffness coefficient ($B$):
$$f_n = n \cdot f_1 \cdot \sqrt{1 + (\text{Thickness} \cdot B_{base}) \cdot n^2}$$
**Sonic Result:** Low thickness = a trashy, splashy, cymbal-like wash. High thickness = a focused, bell-like, rigid clank.

### 5. Heat (Thermal Expansion & Instability)
**DSP Model:** Low-Frequency Wander & Rigidity Loss.
**Vibecode Logic:** Extreme heat (like a glowing red pipe in a foundry) causes the metal to expand and lose its structural rigidity. The pitch subtly drops and warbles due to thermal convection, and the overall brightness decreases.
**Mathematical State:**
$$f_n(t) = f_{n, base} \cdot (1 - \text{Heat\_Amount} \cdot 0.05) \cdot (1 + \text{Heat\_Amount} \cdot \text{LFO}_{wander}(t))$$
$$\text{Exciter\_Stiffness} = K_{base} \cdot (1 - \text{Heat\_Amount} \cdot 0.3)$$
**Sonic Result:** Adds an eerie, unstable, detuned warble to the object, while softening the attack of any strikes or scrapes.

### 6. Sludge (Fluid Coating / Viscosity)
**DSP Model:** Mass-Loading without Stiffness.
**Vibecode Logic:** Imagine hitting an oil barrel that is clean vs. one covered in 2 inches of thick, wet sludge. The sludge adds mass (dropping the pitch) but adds zero stiffness, while acting as an extreme broadband dampener. 
**Mathematical State:**
$$f_n = f_{n, base} \cdot \sqrt{\frac{\text{Mass}_{base}}{\text{Mass}_{base} + \text{Sludge\_Amount}}}$$
$$d_n = d_{n, base} + (\text{Sludge\_Amount} \cdot C_{viscous})$$
**Sonic Result:** A dark, muffled, incredibly heavy "thwack." Turns a ringing pipe into a deadened, concrete-like pillar.

---

### `Velocity Expressiveness Pass`

To tie these transformations to MIDI Velocity (so velocity affects more than just brightness), you build a **Velocity Macro Matrix**.
Instead of velocity hard-mapping to the exciter, velocity produces a normalized scalar ($V_{norm}$ from **0.0** to **1.0**) that non-linearly scales these physical properties:

1.  **Force:** $F_{out} = F_{base} \cdot V_{norm}^{1.5}$ (Harder hits scale exponentially louder).
2.  **Brightness:** Exciter stiffness $K = K_{base} \cdot (0.5 + 0.5 \cdot V_{norm})$.
3.  **Damage/Rattle:** The `Rattle_Threshold` in the Damage equation is overcome much faster at high velocities, meaning soft hits ring (mostly) clean, but slamming the key causes the object to violently buzz and crackle.
4.  **Excitation Decay (for Strikes):** The hammer mass leaves the object faster. $D_{exciter} = D_{base} \cdot (1 - V_{norm} \cdot 0.5)$. Hard hits have a sharper, faster transient; soft hits "linger" on the metal slightly longer, dulling the sound.

***

With these physical transformations mapped out mathematically, are you considering implementing them as global macros that affect the Exciter and Resonator simultaneously (e.g., turning up "Sludge" slows down a Scrape while also damping the Pipe), or will they be strictly isolated to the Resonator block?

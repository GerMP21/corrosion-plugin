> **Implementation status (checked 2026-11-14): IMPLEMENTED, with noted
> deviations.** All 16 exciters below ship in `src/dsp/exciters/` with the
> documented DSP models and tunable parameters, dispatched per-voice by integer in
> `src/voice/mod.rs`. Where the code differs from this file:
> - **Bow** never sticks. It uses a friction curve (Slip Curve sets how far the
>   grip carries into sliding), not true stick-slip; real stick-slip broke the
>   bowed chords and is in `docs/BACKLOG.md` (`fix-inert-controls`).
> - **Contact exciters** (Hand Strike, Felt Mallet, Hard Mallet, Drumstick, Metal
>   Pipe, Metal Chain, Heavy Grinding, Particle Rain, Tension Snap) solve their
>   contact implicitly against this sample's resonator response.
> - **Drumstick** re-strikes Micro Bounce Limit − 1 times after its first contact
>   and its micro-bounces.
> - **Pneumatic Jet** holds its force at its peak above `2/(3·chaos)` instead of
>   stalling (`fix-pneumatic-jet-saturation`).
> - **Global controls** Pressure, Speed and Roughness scale every exciter's
>   force, motion speeds and grit on top of the parameters below.
> - Every exciter's force passes a drive level and a per-voice drive governor.
>
> `docs/ARCHITECTURE.md` §6 has the details and the controls each change made audible.
>
> Naming note:
> the code groups exciters by *envelope family* — **Hit** (one-shot AR), **Friction**
> (6-stage MSEG), **Specialty** (ADSR) — so the spec's "Hit / Scrape / Other"
> categories map to Hit / Friction / Specialty, and **Bow** lives in the Friction
> family. There is no "Motor" exciter (an old PRD name); the rotating-machine role
> is covered by **Electromagnetic Hum**. See `docs/ARCHITECTURE.md` §6.

# Exciter algorithms and tweaking paramete0
Each exciter has a name which should be displayed in the selection option in the synth, along with a tooltip description, and the parameters that can be tweaked inside the synth.
In the exciter selection option they are divided by categories.

---

## Hit Category

---

### 1. Hand Strike
**DSP Model:** Linear Spring-Damper (Kelvin-Voigt). 
**Vibecode Logic:** Fleshy impact. Acts as a linear spring with massive damping. If the gate remains open, the hand stays at $x = 0$ (resting on the object), acting as a severe dampener to the modal resonator.
**Mathematical State:**
$$F(t) = \max(0, K \cdot (x_h - x_m) + D \cdot (v_h - v_m))$$

**Exposed Synth Parameters:**
* `hand_mass`: Overall force multiplier.
* `flesh_stiffness` ($K$): Very low. Controls initial attack transient.
* `flesh_damping` ($D$): Very high. Controls high-frequency absorption.
* `mute_decay`: How quickly the exciter state returns to 0 after striking (controls whether it behaves like a "slap" or a "palm mute").

---

### 2. Felt Mallet
**DSP Model:** Polynomial Non-linear Contact.
**Vibecode Logic:** Soft spring that hits a hard wall. At low velocities, the linear term dominates. At high velocities, the compression depth triggers a steep exponential stiffness.
**Mathematical State:**
$$\Delta x = \max(0, x_h - x_m)$$
$$F(t) = K_{soft} \cdot \Delta x + K_{hard} \cdot (\Delta x)^p$$

**Exposed Synth Parameters:**
* `mallet_mass`: Overall momentum.
* `felt_softness` ($K_{soft}$): The low-velocity stiffness (boom/thud).
* `core_hardness` ($K_{hard}$): The high-velocity stiffness multiplier.
* `compression_curve` ($p$): Usually $3.0$ to $5.0$. Controls how suddenly the felt bottoms out into the hard core.

---

### 3. Hard Mallet (e.g., Marimba Mallet / Rubber / Hard Plastic)
**DSP Model:** Standard Hertzian Contact.
**Vibecode Logic:** A heavy, rigid body. It causes a single, massive transfer of energy with a defined non-linear curve. The high mass and moderate damping prevent it from bouncing wildly.
**Mathematical State:**
$$F(t) = K \cdot \max(0, x_h - x_m)^{1.5} + D \cdot v_{rel}$$

**Exposed Synth Parameters:**
* `mallet_mass`: High default. Drives the low-end of the modal object.
* `material_stiffness` ($K$): High default. Controls the brightness of the strike.
* `impact_damping` ($D$): Prevents micro-bounces. Ensures a clean, single strike.

---

### 4. Drumstick (e.g., Hickory / Rigid Wood)
**DSP Model:** Hertzian Contact with State-Tracking / Micro-Bouncing.
**Vibecode Logic:** Unlike the Hard Mallet, a drumstick is light and highly rigid. The DSP must calculate the interaction force, let the modal object's vibration throw the stick backward ($x_h < x_m$), and allow it to fall back for 2 to 4 rapid micro-collisions within milliseconds.
**Mathematical State:**
$$F(t) = K \cdot \max(0, x_h - x_m)^{1.5}$$
*Requires a dedicated physics loop updating stick velocity $v_h(t+1) = v_h(t) - (F(t) / m_h)$ to allow for bouncing.*

**Exposed Synth Parameters:**
* `stick_mass`: Low default. 
* `tip_stiffness` ($K$): Very high. Controls the "ping" or "bite" of the tip.
* `restitution_bounciness`: Controls how much energy the stick retains after being thrown back by the modal object.
* `micro_bounce_limit`: Caps the number of times the state machine allows the stick to re-strike before forcing $x_h$ to rest.

---

### 5. Wire Brush
**DSP Model:** Stochastic Poisson Process (Impulse Cluster).
**Vibecode Logic:** Abandons mass/spring mechanics. Generates a cluster of microscopic Dirac impulses over a specific time window.
**Mathematical State:**
$$F(t) = \sum_{i=1}^{N} a_i \cdot \delta(t - t_i)$$
*(Where $t_i$ follows a Poisson distribution bounded by a time envelope).*

**Exposed Synth Parameters:**
* `wire_density` ($N$): Number of individual impulses generated.
* `spread_duration`: Time window (in ms) over which impulses occur. Short = slap; Long = sweep.
* `wire_stiffness`: A high-pass filter cutoff applied to the resulting impulse cluster before feeding it to the modal object.
* `amplitude_randomization`: Controls the variance of $a_i$ (how uniform the wires sound).

---

### 6. Metal Pipe (Metal-on-Metal)
**DSP Model:** Coupled Bi-directional Resonator.
**Vibecode Logic:** The exciter is an oscillator. Calculate a stiff Hertzian contact, but apply the resulting force both to the main synth and *back* into a mini-modal bank representing the pipe. The pipe's ringing modulates the collision.
**Mathematical State:**
$$F_{contact}(t) = K \cdot \max(0, x_pipe - x_{target})^{1.5}$$
$$Target_{in} = F_{contact}(t)$$
$$Pipe_{in} = -F_{contact}(t)$$ *(The pipe feeds its resulting high-frequency oscillation back into $x_pipe$)*

**Exposed Synth Parameters:**
* `pipe_mass`: Overall force multiplier.
* `metal_stiffness` ($K$): Extreme high default.
* `pipe_pitch`: Shifts the frequencies of the 2-3 resonant modes inside the exciter pipe.
* `pipe_ring_decay`: How long the exciter pipe continues to ring against the target object.

---

### 7. Metal Chain
**DSP Model:** Cascading Multi-Mass Stochastic Hertzian.
**Vibecode Logic:** A hybrid of the Brush and Hard Mallet. Calculates $N$ independent heavy impacts spread over time, while mixing in a burst of high-frequency noise at each impact to simulate links grinding.
**Mathematical State:**
For each link $i$ from $1$ to $N$:
$$F_i(t) = \max(0, K \cdot (x_{h,i} - x_m)^{1.5}) + (Noise_{HPF} \cdot \text{Gate}_i)$$
$$F_{total}(t) = \sum F_i(t)$$

**Exposed Synth Parameters:**
* `link_count` ($N$): Number of distinct heavy hits.
* `chain_mass`: Weight of each individual link.
* `drop_envelope_spread`: Time (in ms) between the first link hitting and the last link hitting.
* `internal_rattle`: Gain of the high-pass filtered noise injected at each link impact.
* `rattle_color`: The cutoff frequency of the high-pass filter for the rattle.

---

## Scrape Category

---

### 1. The Bow (Smooth Stick-Slip)
**DSP Model:** Elasto-plastic Friction (Stribeck Curve).
**Vibecode Logic:** A continuous friction model that grabs and releases the modal object, causing stable Helmholtz motion. It relies entirely on the velocity difference between the bow and the object.
**Mathematical State:**
$$v_{rel} = v_{bow} - v_m$$
$$\mu(v_{rel}) = \mu_{dynamic} + (\mu_{static} - \mu_{dynamic}) \cdot e^{-c |v_{rel}|}$$
$$F_{out}(t) = F_{pressure} \cdot \mu(v_{rel}) \cdot \text{sgn}(v_{rel})$$

**Exposed Synth Parameters:**
* `bow_pressure` ($F_{pressure}$): Scales the overall excitation force. Too low = it barely whispers. Too high = it chokes the resonator.
* `bow_speed` ($v_{bow}$): The primary driving variable.
* `rosin_grip` ($\mu_{static}$): The static friction coefficient. Higher values create a raspy, aggressive bite.
* `slip_curve` ($c$): Determines how smoothly the grip transitions into a slip.

---

### 2. Stiff Point Scrape (Nail / Awl / Pick)
**DSP Model:** Stiff Spring Chatter.
**Vibecode Logic:** A rigid point doesn't just slide; it acts as a very stiff, undamped spring. As it drags, the tip gets caught, bends back, and snaps forward, generating its own high-frequency micro-oscillation (chatter) that modulates the friction.
**Mathematical State:**
Requires a tiny, 1-D oscillator for the scraper tip:
$$v_{tip\_rel} = v_{scrape} - v_m$$
$$F_{chatter}(t) = K_{point} \cdot \int v_{tip\_rel} \, dt - D_{point} \cdot v_{tip\_rel}$$
*If the friction threshold is broken, the tip resets its position.*
$$F_{out}(t) = F_{chatter}(t) \cdot \mu_{dynamic}$$

**Exposed Synth Parameters:**
* `scrape_speed` ($v_{scrape}$): Driving speed.
* `point_pressure`: Increases the threshold needed for the tip to snap forward.
* `chatter_pitch` ($K_{point}$): The stiffness of the point, which translates directly to the frequency of the agonizing "squeak."
* `chatter_damping` ($D_{point}$): Controls how quickly the micro-snaps decay.

---

### 3. Heavy Grinding (Concrete / Sandpaper)
**DSP Model:** Coulomb Friction + Asperity Tearing (Velocity-Scaled Noise).
**Vibecode Logic:** When heavy, rough objects slide against the resonator, microscopic peaks collide and tear. This skips the clean mathematical curve of a bow and uses a baseline friction heavily modulated by chaotic noise.
**Mathematical State:**
$$v_{rel} = v_{grind} - v_m$$
$$F_{base}(t) = F_{pressure} \cdot \mu_{dynamic} \cdot \text{sgn}(v_{rel})$$
$$F_{tearing}(t) = \text{Noise}_{brownian}() \cdot |v_{rel}| \cdot \text{Grit\_Amount}$$
$$F_{out}(t) = F_{base}(t) + F_{tearing}(t)$$

**Exposed Synth Parameters:**
* `grind_speed` ($v_{grind}$): Scales the tearing noise amplitude.
* `grind_pressure` ($F_{pressure}$): The baseline dragging force.
* `surface_grit` ($\text{Grit\_Amount}$): Scales the ratio of pure friction to chaotic tearing noise.
* `grit_color`: The cutoff frequency of the noise generator (low-pass for heavy concrete, high-pass for fine sandpaper).

---

### 4. Corrugated Drag (Stick on a Grate / Ribbed Surface)
**DSP Model:** Spatial Wave Modulator.
**Vibecode Logic:** This simulates a scraper moving across macroscopic bumps. The normal force (pressure) isn't constant; it is rhythmically interrupted by the surface topology. The frequency of the impacts is strictly tied to the drag speed.
**Mathematical State:**
$$\text{Position}(t) = \int v_{drag} \, dt$$
$$F_{bump}(t) = \max\left(0, \sin\left(2\pi \cdot \frac{\text{Position}(t)}{\text{Spacing}}\right)\right)$$
$$F_{out}(t) = (F_{pressure} + (F_{bump}(t) \cdot \text{Depth})) \cdot \mu_{dynamic}$$

**Exposed Synth Parameters:**
* `drag_speed` ($v_{drag}$): Directly controls the resulting LFO rate of the bumps.
* `ridge_spacing` ($\text{Spacing}$): The physical distance between the bumps.
* `ridge_depth` ($\text{Depth}$): How deep the scraper falls between bumps.
* `exciter_mass`: Modulates the impact spike of hitting each ridge (heavier mass = punchier low-end clicks).

---

### 5. Tension Rise (Avalanche Slip / Creak)
**DSP Model:** Integrate-and-Fire Threshold Mechanics.
**Vibecode Logic:** Simulates a massive, slow build-up of force where the exciter sticks perfectly to the object for long periods, releasing energy in discrete, violent bursts (creaks or groans) only when the tension exceeds a breaking point.
**Mathematical State:**
$$\text{Tension} = \text{Tension} + (v_{pull} - v_m)$$
$$\text{If } \text{Tension} > \text{Threshold}:$$
$$\quad F_{out}(t) = \text{Tension} \cdot \text{Impulse\_Shape}(t)$$
$$\quad \text{Tension} = 0$$
$$\text{Else}:$$
$$\quad F_{out}(t) = 0$$

**Exposed Synth Parameters:**
* `pull_speed` ($v_{pull}$): How fast the tension accumulates.
* `break_threshold` ($\text{Threshold}$): How much force is required to cause a slip. High values create slow, massive groans; low values create fast, continuous creaking.
* `slip_stochasticity`: Adds random jitter to the threshold so the creaking isn't perfectly rhythmic.
* `creak_sharpness`: Controls the low-pass filter on the `Impulse_Shape`, determining if the slip is a dull thud or a sharp crack.

---

## Other Category

---

Grouping these into an `Other` or `Specialty` category is a smart way to keep your core UI clean while offering advanced sound design tools. 

Here is the "vibecode" blueprint for the "Other" category, formatted to match your existing documentation style so you can hand it straight to your AI assistant.

### `Other` (Specialty Exciters)
Planned as an expansion category for non-impact, non-frictional industrial forces.

Core model targets:
- fluid dynamics (air/steam)
- continuous energy injection (electricity)
- mechanical tension release
- stochastic continuous impacts

Internal flavor layer planned:

#### `OtherFlavor`
- PneumaticJet
- ElectromagneticHum
- TensionSnap
- ParticleRain

---

### 1. Pneumatic Jet (Steam / Air Valve)
**DSP Model:** Non-linear Bandlimited Turbulence.
**Vibecode Logic:** Simulates a pressurized gas jet hitting a rigid edge. Generates wideband noise that is non-linearly shaped by the relative velocity between the air stream and the modal object's vibration.
**Mathematical State:**
$$v_{rel} = v_{air} - v_m$$
$$F_{jet}(t) = \text{Noise}_{white}() \cdot (v_{rel}^2 - \text{Saturation} \cdot v_{rel}^3)$$
$$F_{out}(t) = \text{Bandpass}(F_{jet}(t), \text{Cutoff}_{freq}, Q)$$

**Exposed Synth Parameters:**
* `air_pressure` ($v_{air}$): The speed of the jet, driving the overall intensity.
* `nozzle_width` ($Q$): Narrow nozzles (high $Q$) = whistling tones; wide nozzles (low $Q$) = broad, roaring steam.
* `turbulence_chaos` ($\text{Saturation}$): Non-linear clipping that creates the "choking" sound of an overloaded valve.

---

### 2. Electromagnetic Hum (Transformers / Motors)
**DSP Model:** Lorentz Force Continuous Drive.
**Vibecode Logic:** An active, continuous tonal exciter. Instead of hitting or scraping, it injects an audio-rate, phase-locked electromagnetic wave directly into the resonator.
**Mathematical State:**
$$F_{ac}(t) = \text{Drive\_Gain} \cdot \left(\sin(2\pi \cdot f_{mains} \cdot t) + \text{Harmonics}\right)$$
$$F_{out}(t) = F_{ac}(t) \cdot \text{Proximity\_Envelope}(t)$$

**Exposed Synth Parameters:**
* `mains_frequency` ($f_{mains}$): Baseline hum (e.g., **50Hz** or **60Hz**).
* `coil_proximity`: How close the magnetic field is to the object, acting as the main volume/drive envelope.
* `voltage_sag` ($\text{Harmonics}$): Introduces odd-harmonic distortion to simulate a failing transformer.

---

### 3. Tension Snap (Wire Break / Gear Catch)
**DSP Model:** Linear Hook with Instantaneous Release.
**Vibecode Logic:** A catch-and-release mechanism. The exciter locks onto the object ($x_m$) and pulls it. Force builds linearly until a mechanical breaking point is reached, instantly dropping the force to zero and letting the object violently snap back.
**Mathematical State:**
$$\text{If } \text{Hooked}:$$
$$\quad \Delta x = \text{Position}_{pull}(t) - x_m$$
$$\quad F_{out}(t) = K_{hook} \cdot \Delta x$$
$$\quad \text{If } F_{out}(t) > \text{Snap\_Threshold}: \text{Hooked} = \text{False}$$
$$\text{If Not Hooked}:$$
$$\quad F_{out}(t) = 0$$

**Exposed Synth Parameters:**
* `pull_distance` ($\text{Position}_{pull}$): The excursion of the drag before the snap.
* `hook_stiffness` ($K_{hook}$): Stiff hooks create a massive build-up; soft hooks yield more before snapping.
* `snap_force` ($\text{Snap\_Threshold}$): The tension required to break the connection.

---

### 4. Particle Rain (Debris / Sand / Gravel)
**DSP Model:** Asynchronous Granular Emission (Continuous Poisson Cloud).
**Vibecode Logic:** A continuous stochastic generator. It spawns an infinite stream of microscopic, independent mass-spring impacts, bypassing the need to calculate one massive multi-link chain.
**Mathematical State:**
$$P_{spawn}(t) = \text{Flow\_Rate}$$
*For each spawned particle $i$ at time $t_i$:*
$$F_i(t) = m_{particle} \cdot \max\left(0, 1 - \frac{t - t_i}{\text{Decay}}\right)$$
$$F_{out}(t) = \sum_{i=1}^{\text{Active\_Particles}} F_i(t)$$

**Exposed Synth Parameters:**
* `flow_rate` ($\text{Flow\_Rate}$): The density of the debris stream (sparse trickle vs. heavy pour).
* `particle_mass` ($m_{particle}$): Low mass = light sand/glass; high mass = heavy gravel.
* `mass_variance`: Randomization applied to $m_{particle}$ per hit to ensure the debris sounds organic and chaotic.

---


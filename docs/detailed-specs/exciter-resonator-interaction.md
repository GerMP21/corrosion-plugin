> **Implementation status (checked 2026-11-14): IMPLEMENTED, faithful.** The interaction bus
> (`src/dsp/interaction.rs`) implements per-mode strike coefficients
> `c_n(P) = sin((n+1)·π·P)`, the `fundamental_anchor` minimum-coefficient clamp,
> the `coupling_stiffness` feed-forward↔weighted blend, and displacement/velocity
> feedback into the exciter. The effective strike position is
> `clamp(strike_position + position_envelope·envelope·0.5 + wander, 0, 1)`:
> Position Envelope sweeps it with the force envelope
> (`h09-route-envelope-to-strike-position`), and `position_wander` adds a bounded
> sine drift at Wander Rate (0.1–5 Hz). Contact exciters go further than this
> file: they solve their contact implicitly against this sample's resonator
> response (`ModalResonator::contact_prediction`, `dsp::utils::contact`). The
> 6-stage MSEG and the AR/ADSR/MSEG family split described later in this file are
> implemented in `src/dsp/envelopes/mod.rs` and `src/voice/mod.rs`. See
> `docs/ARCHITECTURE.md` §§6–7.

The interaction between the exciter and the resonator is the absolute core of physical modeling. It is the "glue" that turns isolated mathematical equations into a playable instrument. 

In modal synthesis, you do not just pipe audio from an exciter into a resonator like an effect plugin. Instead, they interact mechanically by exchanging three variables every single sample: **Force ($F$)**, **Displacement ($x$)**, and **Velocity ($v$)**.

Here is how you bridge the algorithms we have built so far, structured for your vibecode documentation.



### 1. The Interaction Point (Spatial Phasing)
When you hit a plate in the exact center, you excite the fundamental frequency, but you cancel out certain even-numbered harmonics. When you hit it near the edge, you excite all the high-frequency modes. 

To model this, the exciter cannot just feed a global force to the whole modal bank. It must multiply its force by an **Excitation Coefficient ($c_n$)** for each specific mode ($n$). This coefficient represents the amplitude of that specific mode at the physical strike position.

### 2. Bidirectional Coupling (Feedback)
For simple, cheap DSP, you can run a **Feed-forward** system: the exciter generates force, feeds it to the resonator, and ignores it. 

However, for realistic industrial sounds (like the *micro-bouncing of a drumstick*, the *stick-slip tearing of brake squeal*, or the *clanking of a pipe*), you must use a **Bidirectional** system. The exciter calculates force based on its own movement *minus* the movement of the resonator. 

---

### `Interaction_Bus` (The Vibecode Blueprint)

**DSP Model:** Bidirectional Modal Coupling Matrix.
**Vibecode Logic:** At every sample, the system calculates the current displacement and velocity of the resonator at the exact point of contact. It feeds those values backward into the exciter. The exciter computes the resulting collision/friction force, and feeds it forward into the modal bank, scaled by the strike position.

**Mathematical State:**

**Step 1: Resonator reports its state to the Exciter**
The resonator calculates its total displacement ($x_m$) and velocity ($v_m$) at the specific strike position ($P_{strike}$):
$$x_m(t) = \sum_{n=1}^{N} c_n(P_{strike}) \cdot y_n(t)$$
$$v_m(t) = \sum_{n=1}^{N} c_n(P_{strike}) \cdot y_n'(t)$$

**Step 2: Exciter calculates Force**
The exciter uses $x_m$ and $v_m$ in its algorithm (e.g., the Hertzian Hit or Stribeck Scrape) to figure out how much force is generated this sample:
$$F_{out}(t) = \text{Exciter\_Algorithm}(x_h, v_h, x_m, v_m)$$

**Step 3: Exciter injects Force into the Resonator**
The raw force $F_{out}(t)$ is distributed to every mode in the resonator, but scaled by that mode's coefficient at the strike position:
$$\text{For each mode } n:$$
$$F_{in, n}(t) = F_{out}(t) \cdot c_n(P_{strike})$$

**Step 4: Resonator updates its modes**
The modal bank processes the incoming force for that sample:
$$y_n''(t) + 2 d_n y_n'(t) + \omega_n^2 y_n(t) = F_{in, n}(t)$$

---

**Exposed Synth Parameters for the Interaction Bus:**
* `strike_position` ($P_{strike}$): Sweeps the excitation coefficients ($c_n$). Moving this from $0.0$ (edge) to $1.0$ (center) completely changes the harmonic makeup of the impact or scrape.
* `coupling_stiffness`: An optional multiplier on the feedback loop. At $100\%$, the exciter and resonator act as a unified physical object. At $0\%$, it degrades into a cheap feed-forward synth (useful for saving CPU on dense polyphonic pads).

***


The beauty of modal synthesis is that **pitch and strike position are inherently decoupled.** The resonant frequencies ($f_n$) dictate the pitch, while the spatial coefficients ($c_n$) dictate the timbre. Therefore, to move the strike position dynamically while locking the main frequency to the MIDI key, you simply hold $f_n$ static while updating $c_n$ continuously at control-rate or audio-rate. 

As the position moves, it sweeps through the nodes and antinodes of the object's geometry, creating a natural, organic comb-filtering/flanger effect as different harmonics phase in and out, all while the fundamental note stays perfectly anchored.

Here is the "vibecode" logic for implementing a Dynamic Strike Position.

---

### `Dynamic_Interaction_Bus` (Moving Excitation)

**DSP Model:** Time-Variant Spatial Coupling.
**Vibecode Logic:** The base frequencies ($\omega_n$) are calculated once upon the `noteOn` event based on the MIDI key. During the sustain phase, an LFO or Envelope sweeps the physical location of the exciter ($P_{strike}$). The DSP recalculates the excitation coefficient ($c_n$) for every mode dynamically. To ensure the "main frequency" never disappears (even if the scraper hits a dead-spot/node for that frequency), we can implement a `Fundamental_Lock` bypass.

**Mathematical State:**

**Step 1: Pitch Lock (Calculated once at `noteOn`)**
$$f_1 = \text{MIDI\_to\_Frequency}(\text{Key})$$
$$\text{Calculate array of } f_n \text{ based on Object type (Pipe, Plate, etc.)}$$

**Step 2: Dynamic Position Update (Calculated every sample/block)**
$$P_{strike}(t) = P_{base} + \text{Modulator}(t)$$
*(Where $P_{strike}$ is normalized between $0.0$ and $1.0$.)*

**Step 3: Recalculate Mode Amplitudes (Spatial Shape)**
For a 1D object (like a pipe or cable), the mode shape is typically a sine distribution:
$$c_n(t) = \sin(n \cdot \pi \cdot P_{strike}(t))$$
*(For a 2D Plate or Tank, this would use the respective 2D grid or Bessel shape).*

**Step 4: Fundamental Lock Constraint**
To guarantee the main note of the key never drops out (even if $P_{strike}$ crosses a physical node where $c_1$ would naturally equal $0$):
$$c_1(t) = \max(c_1(t), \text{Fundamental\_Minimum})$$

**Step 5: Apply Modulated Force**
$$F_{in, n}(t) = F_{out}(t) \cdot c_n(t)$$

---

**Exposed Synth Parameters for Dynamic Position:**
* `strike_position` ($P_{base}$): The starting point of the exciter on the object.
* `position_wander`: The depth of a slow, random LFO applied to $P_{strike}$, simulating a human hand unsteadily dragging a bow or scraper.
* `position_envelope`: Allows an envelope generator to sweep the position (e.g., the scrape starts at the edge and aggressively slides to the center during the attack).
* `fundamental_anchor` ($\text{Fundamental\_Minimum}$): A mix knob. At $0\%$, the fundamental frequency can phase out completely if the scraper hits a node. At $100\%$, the fundamental frequency always receives maximum excitation energy, ensuring the MIDI note is powerfully defined regardless of where the scrape is happening.

***

Now that the objects, exciters, and their dynamic physical interactions are mapped out, how do you plan to handle the final output—do you want to run this raw physical signal straight to the master, or are you planning an internal effects block (like a resonating body or spatializer) to place these industrial objects in a virtual acoustic space?


For a physical modeling synth focused on industrial textures, standard ADSR is not going to cut it. You absolutely need a **Multistage Envelope Generator (MSEG)**, at least for your continuous exciters.

Designing envelopes for physical modeling is fundamentally different from subtractive synthesis. In a traditional synth, an envelope controls an amplifier (volume) or a filter (EQ). In physical modeling, the envelope controls **physical force, pressure, and speed**. 

If you apply a linear ADSR to a "Brake Squeal" scrape, the pressure will just linearly rise, hold, and linearly fall. It will sound like a synthesizer. Real mechanical friction stutters, grabs, eases off, and bites again.

Here is the breakdown of how your envelope architecture should look for the different interaction types:

### 1. For Hits & Strikes: Exponential AR (Attack-Release)
Hits are impulsive. You do not "sustain" a hammer strike; once the mallet hits the metal, the physics are over, and the resonator takes over. 
* **Behavior:** A rapid attack (which controls the velocity/acceleration of the hammer) and an exponential decay (which brings the exciter mass back to a resting state).
* **Why ADSR fails here:** Having a "Sustain" parameter on a drumstick makes no physical sense. If the gate stays open, the stick should just bounce and rest, not continuously apply static force.

### 2. For Scrapes & Friction: Multistage Envelope (MSEG)

For bowing, dragging, and tearing, the user needs to draw the macro-gesture of the scrape. 
* **Behavior:** Users should be able to add multiple nodes to draw complex pressure curves. For example, a heavy "Tension Creak" might require a slow build up, a sudden dip in pressure as it slips, a secondary spike, and a slow drag to a halt.
* **Looping:** The MSEG should ideally have a loopable sustain segment so that if the user holds the key down, the scrape can rhythmically "chug" or cycle through a sequence of friction points.

### 3. For Modulation (Dynamic Strike Position): Complex LFO / MSEG Hybrid
When you modulate the strike position across the modal object, an ADSR will just move the scraper from the edge to the center and stop. 
* **Behavior:** You want the ability to create chaotic, wandering movements or rhythmic, machine-like sweeping. An MSEG that can be set to loop continuously (acting as an arbitrary-waveform LFO) is perfect for dragging the interaction point back and forth across the metal plate.

### Summary Blueprint
If you want to keep the UI clean but physically accurate, split the envelope duties:
1.  **Exciter Force Envelope:** Hardcoded AR for impulsive hits, and a user-drawable MSEG for scrapes/continuous exciters.
2.  **Resonator Damping Envelope:** This is where your traditional "Release" tail lives. It controls how quickly the modal object stops ringing after the key is lifted (simulating a hand grabbing the ringing pipe to mute it).

Do you plan to build the MSEG as a free-drawing UI where users can click to add infinite nodes, or would you prefer a fixed-stage complex envelope (like a DAHDSR with adjustable curve tensions) to keep the coding and UI footprint lighter?


To capture the raw, unpredictable physics of industrial friction (like a screeching brake pad or a groaning cable), your Multistage Envelope Generator (MSEG) should be designed around **Gesture** rather than just volume. 

When a human or a machine drags a heavy object across metal, there is an initial impact, a struggle to overcome static friction, a chaotic sliding phase, and a release. 

Here is the ideal 6-stage MSEG blueprint, structured for your "vibecode" documentation.

---

### `Friction_MSEG` (6-Stage Continuous Envelope)

**DSP Model:** 6-Node Tension-Curve Envelope with Sustain-Looping.
**Vibecode Logic:** This envelope drives the `bow_pressure`, `bow_speed`, or `scrape_speed` parameters. It requires independent control over the time ($t$), target level ($l$), and the curve tension/shape ($c$) for each segment. 



#### Stage 1: Onset (Contact)
The micro-moment the scraper physically touches the modal object before it starts dragging.
* **Physics:** A very brief spike in pressure, but low speed.
* **Parameters:** `t_onset` (Time), `l_onset` (Target Level), `c_onset` (Curve).
* **Sonic Result:** The initial "clack" or "thud" of the tool hitting the metal pipe before the scrape begins.

#### Stage 2: Attack (Acceleration / Bite)
The build-up of kinetic energy. The scraper digs in and accelerates.
* **Physics:** Force and speed ramp up rapidly to break the object's static friction.
* **Parameters:** `t_attack`, `l_peak`, `c_attack`.
* **Sonic Result:** The aggressive swell of the screech. A convex curve (fast rise, slow finish) sounds like an aggressive machine; a concave curve (slow rise, fast finish) sounds like a human hesitantly dragging a bow.

#### Stage 3: Hold (Tension Peak)
The moment of maximum pressure before the friction equalizes.
* **Physics:** The scraper is moving at top speed and maximum downward force, pushing the modal object to its absolute limit.
* **Parameters:** `t_hold` (Time to hold at `l_peak`).
* **Sonic Result:** The agonizing, deafening peak of a brake squeal or the loudest point of a bowed cymbal. 

#### Stage 4: Decay (Slip / Equalization)
The transition from chaotic static-breaking into a steady dynamic drag.
* **Physics:** The tool settles into a groove. Pressure eases slightly as momentum takes over.
* **Parameters:** `t_decay`, `l_sustain`, `c_decay`.
* **Sonic Result:** The screech stabilizes into a steady, continuous tone or grind.

#### Stage 5: Sustain (The Drag) & The Loop Matrix
This is the most critical stage for an industrial synth. A scrape shouldn't be perfectly static if the user holds the key down forever.
* **Physics:** The continuous dragging motion.
* **Parameters:** `l_sustain` (Level).
* **The Loop Logic:** You must include a `loop_enable` toggle. When active, while the gate is held, the envelope loops continuously between the `Decay` start point and the `Sustain` end point. 
* **Sonic Result:** If looped, it simulates the rhythmic "chug-chug-chug" of a rusty wheel turning, or a human frantically sawing back and forth.

#### Stage 6: Release (Lift / Deceleration)
The key is released. The tool is pulled off the metal.
* **Physics:** Speed drops to zero, pressure drops to zero.
* **Parameters:** `t_release`, `l_end` (Usually $0.0$), `c_release`.
* **Sonic Result:** The scrape grinds to a halt. (Note: the *resonator* will still ring out according to its own damping coefficients, but the *exciter* stops injecting energy).

---

### Expanded Vibecode Parameter List for the UI

To make this user-friendly while keeping the math intact, your UI will need these global envelope controls:
* `env_amount`: Macro scalar that dictates how much this envelope actually influences the target parameter.
* `velocity_to_peak`: Scales the `l_peak` value based on MIDI velocity (hit the key harder = the scrape reaches a higher maximum pressure).
* `loop_mode`: [Off, Forward, Ping-Pong]. Ping-pong is phenomenal for bowing, as it simulates the bow reversing direction (push/pull) smoothly.
* `curve_tension`: A universal slider that morphs all $c$ values from logarithmic (snappy) to linear to exponential (sluggish).

***

Making every single envelope parameter globally available shifts your synth from a hardcoded physical simulator into a true modular sound design environment. This is the right call for an industrial synth—it allows users to map an LFO to the "Attack Curve" or map MIDI velocity to the "Decay Time," resulting in highly organic, non-repeating friction textures.

By decoupling the MSEG from the exciter, the envelope becomes a pure mathematical control source. Here is the AI-friendly "vibecode" blueprint defining all the exposed parameters for your standalone Multistage Envelope Generator.

---

### `MSEG_Modulation_Source` (Fully Exposed Envelope)

**DSP Model:** Decoupled 6-Stage Parameterized Control Envelope.
**Vibecode Logic:** Generates a control signal that can be routed to any exciter or resonator parameter. Every time ($t$), level ($l$), and curve ($c$) is exposed as a floating-point variable that can itself be modulated.

#### 1. Node Parameters (The Shape)
These define the absolute geometry of the envelope.
* `t_onset`: Time from key-down to initial contact (0ms to 500ms).
* `l_onset`: Level at the moment of contact (typically an initial physical impact).
* `c_onset`: Curve tension into the onset node (logarithmic to exponential).
* `t_attack`: Time to reach maximum energy/pressure.
* `l_peak`: The maximum level output of the envelope.
* `c_attack`: Curve tension of the attack swell.
* `t_hold`: Duration the envelope stays locked at `l_peak` before decaying.
* `t_decay`: Time to fall from `l_peak` to the sustain resting state.
* `l_sustain`: The resting level while the gate is held open.
* `c_decay`: Curve tension of the slip/fall.
* `t_release`: Time to fall from current level to `l_end` after the gate closes.
* `l_end`: The final resting level (usually 0.0, but exposing it allows for drone capabilities).
* `c_release`: Curve tension of the final lift-off.

#### 2. Looping & Transport Parameters
These govern how the envelope behaves while the MIDI gate remains high.
* `loop_mode`: An integer/enum state `[0: Off, 1: Forward, 2: Ping-Pong]`. 
* `loop_start_stage`: Selects which node the loop resets to (e.g., `2` for Attack, `4` for Decay).
* `loop_end_stage`: Selects the boundary node that triggers the loop reset.
* `sync_rate`: If enabled, overrides absolute time ($t$) values with host tempo divisions (e.g., 1/16th notes) for rhythmic scraping.

#### 3. Global Macro Modifiers
These parameters scale the entire MSEG dynamically without having to recalculate every individual node.
* `global_time_scale`: A multiplier ($0.1$ to $10.0$) applied to all $t$ parameters. *Crucial for mapping to keytrack so lower notes scrape slower and higher notes scrape faster.*
* `velocity_to_level`: Scales all $l$ parameters globally based on incoming MIDI velocity.
* `velocity_to_time`: Inversely scales $t_attack$ and $t_decay$ based on velocity (hitting the key harder creates a faster, punchier envelope).

---

Since every parameter is now fully exposed as a standalone control source, will your modulation matrix route these signals strictly as unipolar values ($0.0$ to $1.0$) to drive the physics parameters, or will you allow bipolar routing ($-1.0$ to $1.0$) so the envelope can inversely subtract from an exciter's baseline speed or pressure?

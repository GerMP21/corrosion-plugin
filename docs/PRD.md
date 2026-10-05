# Corrosion — Industrial Physical Modeling Synthesizer

## Product Requirements Document and Specification-Driven Development Plan

**Working title:** Corrosion  
**Product type:** VST3 / CLAP software instrument  
**Framework target:** Rust + NIH-plug  
**Primary use case:** Industrial physical modeling synthesis for dark techno, industrial, EBM, dark ambient, cinematic sound design, noise, and experimental electronic music.

---

# 0. Current Implementation Status (2026-05)

> **Read this first.** This PRD captures the original product vision and an
> MVP-first plan. The shipped instrument has moved **well beyond the MVP** and a
> number of MVP-era specifics below are now superseded. For an accurate picture of
> what exists today, see `docs/ARCHITECTURE.md` and `docs/code-review.md`. This
> section reconciles the two.

**Where the product is now (verified in `src/`):**

| Area | PRD MVP text | Shipped today |
|---|---|---|
| Objects | Pipe, Plate, Tank (3) | **9**: Pipe, Plate, Tank, Chain, IBeam, TautCable, CoilSpring, SheetMetal, IndustrialCog |
| Exciters | enum `Hit, Scrape, Motor`; "Hit" MVP | **16** named models across Hit / Friction / Specialty families. **No "Motor"** — the rotating-machine role is "Electromagnetic Hum" |
| Parameters | ~6 MVP params | **127** host parameters with stable IDs (the §11 tables are an MVP subset) |
| Drive | `tanh(asymmetric·drive)` | master `apply_drive` (asymmetric waveshaper) **plus** a Lorenz-modulated post drive |
| Output safety | `clamp(out, −1.0, 1.0)` | hard limiter at **±0.9661 (≈ −0.3 dBFS)**; soft diode clipper before it |
| Quality modes | listed as future | **implemented**: Eco/Normal/High/Render (clipper oversampling + per-voice modal density; every mode runs the full post chain) |
| Polyphony | 8 voices | 8 voices, quietest-peak + oldest stealing, idle voices skip rendering |
| Post/space | "MVP may omit" | full chain: WDF-style filter, Lorenz drive, FEM-style body, HRTF-style spread, Factory/Spring/Echo space, oversampled clipper (all **approximations** — see code review) |
| Body/space DSP | algorithmic body resonator | shipped, but lightweight approximations of the high-fidelity `detailed-specs/post-processing.md` targets |
| Presets | "20 factory presets" | preset system exists (out of scope for the current review) |

**Known defects to track (from `docs/code-review.md`):**
- The oversampled clipper does **not** actually oversample (factor is a no-op), so
  the quality modes are sonically identical at the clipper.
- `FactoryReverb` comb delays are mutated cumulatively per sample, so `factory_size`
  does not behave correctly.
- Post-chain parameters are applied per-sample rather than at control rate.

**Planned engine change (approved 2026-05, not yet implemented):** the resonator
will move to a **single algorithmic engine** — the `complex_algo` toggle is being
removed, the per-object physics generators become the only path, and a curated set
of per-object "character" parameters is exposed (profile tables demoted to
mode-count/budget metadata). Chain/Tank pitch-tracking gaps get fixed and the
TautCable/SheetMetal dynamic behaviors get wired in. This will change the default
object timbres. Full breakdown and parameter table: `docs/backlog.md` →
"Algorithmic resonator engine"; design in `docs/ARCHITECTURE.md` §5.

**Sections of this PRD that remain valid:** product vision (§§2–6), target users,
non-goals, MIDI/velocity philosophy, real-time-safety requirements (§19, met),
testing strategy (§22, largely met), and the strategic identity (§32). Treat the
specific MVP parameter/object/exciter enumerations (§§10–11, §23.1, §30) as
historical — the current surface is larger and is documented in `ARCHITECTURE.md`.

---

# 1. Executive Summary

Corrosion is a software synthesizer plugin focused on physically modeled industrial materials and objects. Instead of using traditional oscillators as the primary sound source, Corrosion models the behavior of struck, scraped, bowed, rattled, and vibrated objects such as pipes, plates, tanks, chains, springs, vents, beams, and damaged machinery.

The instrument is designed around a core signal model:

```text
Exciter → Object Model → Modal Resonator Bank → Damage/Corrosion → Body/Space → Output
```

The plugin should feel like a playable factory of resonant material. It should not merely imitate realistic percussion; it should produce believable, physically suggestive industrial sounds that can be pushed into aggressive, unstable, musical, and cinematic territory.

The first production target is a VST3 / CLAP instrument built in Rust using NIH-plug. The MVP should prioritize sound quality, stable real-time performance, simple MIDI playability, and a focused parameter set before adding advanced sequencing, sample import, complex GUI, or effect-mode processing.

---

# 2. Product Vision

## 2.1 Vision Statement

Corrosion is an industrial material synthesizer: a plugin that lets musicians play, strike, scrape, deform, corrode, and overload virtual physical objects as musical instruments.

## 2.2 Product Positioning

Corrosion is not a general-purpose subtractive synth, wavetable synth, FM synth, or drum sampler. It is a specialized physical-modeling instrument for generating:

- metallic impacts
- pipe basses
- container booms
- rusted resonances
- chain rattles
- bowed steel drones
- industrial percussion
- machine-room atmospheres
- scraped metal tension beds
- unstable damaged-object timbres

## 2.3 One-Sentence Product Description

A physical modeling synth for struck, scraped, corroded, and resonant industrial objects.

## 2.4 Expanded Product Description

Corrosion generates sound from simulated interactions between exciters and resonant objects. A user selects an object, chooses how it is excited, shapes its material properties, adds damage and corrosion, places it inside a resonant body or space, and then plays it via MIDI or triggers it from a DAW sequence.

Where a traditional synthesizer asks the user to choose oscillators, filters, and envelopes, Corrosion asks:

- What object is vibrating?
- What material is it made of?
- How large or tense is it?
- How is it being excited?
- How damaged, rusted, bent, or unstable is it?
- What body or space is resonating around it?

---

# 3. Target Users

## 3.1 Primary Users

### Industrial Techno Producers

Need metallic percussion, distorted physical basses, machine-like loops, and aggressive transitional sounds.

### Dark Ambient / Drone Artists

Need evolving tension beds, resonant drones, scraped textures, and atmospheric mechanical soundscapes.

### Sound Designers

Need custom impacts, mechanical gestures, rusted physical objects, and cinematic industrial textures.

### Experimental Electronic Musicians

Need unusual sound generation models beyond subtractive, FM, and wavetable synthesis.

## 3.2 Secondary Users

### Film / Game Composers

Need tension, threat, factory ambience, impacts, and mechanical cues.

### Noise Musicians

Need unstable feedback, harsh resonances, metallic chaos, and controllable aggression.

### Modular Synth Users

Need a software equivalent of physical object excitation, resonators, and damaged materials.

---

# 4. Product Goals

## 4.1 Functional Goals

1. Generate physically plausible industrial object sounds in real time.
2. Respond to MIDI note input with expressive velocity-sensitive behavior.
3. Expose musically meaningful physical parameters.
4. Support VST3 and CLAP plugin formats.
5. Provide stable low-latency operation in modern DAWs.
6. Provide a focused preset library for immediate usability.
7. Support automation of all major sound parameters.
8. Allow future expansion into sequencer, effect mode, sample exciters, and deeper object modeling.

## 4.2 Sound Design Goals

The plugin must be able to produce:

- tuned pipe hits
- rusted metallic basses
- plate clangs
- oil tank booms
- warehouse impacts
- chain rattles
- industrial hi-hat-like textures
- bowed metal drones
- motor-driven resonances
- scraping metal transitions
- damaged resonant percussion
- dark cinematic metal atmospheres

## 4.3 UX Goals

1. The plugin should be understandable without needing to know DSP terminology.
2. The UI should emphasize physical metaphors: object, material, impact, damage, space.
3. The plugin should expose advanced behavior through macros before exposing technical parameters.
4. Users should be able to generate useful sounds quickly from presets and randomization.
5. The instrument should feel different from a standard subtractive synthesizer.

---

# 5. Non-Goals

The initial product is not intended to be:

1. A fully realistic acoustic simulation engine.
2. A general-purpose polyphonic synthesizer.
3. A sampler or rompler.
4. A full modular environment.
5. A replacement for a complete drum machine.
6. A high-fidelity orchestral physical-modeling instrument.
7. A neural synthesis engine.
8. A plugin host.
9. A DAW.
10. A sample library browser.

The core identity must remain focused: industrial physical modeling.

---

# 6. Competitive / Conceptual Differentiation

## 6.1 Traditional Physical Modeling Synths

Most physical modeling instruments focus on strings, brass, winds, mallets, or acoustic drum behavior. Corrosion focuses on industrial objects and damaged material states.

## 6.2 Traditional Drum Synths

Most drum synths use subtractive synthesis, FM, samples, or noise. Corrosion uses exciter-resonator physical behavior to generate physically suggestive impacts and resonances.

## 6.3 Wavetable / FM Synths

Wavetable and FM synths can produce metallic timbres, but they usually require abstract parameter programming. Corrosion uses object-oriented physical metaphors: pipe, plate, tank, chain, rust, impact, damping, damage.

## 6.4 Sample Libraries

Sample libraries are realistic but static. Corrosion is generative, responsive, tunable, and automatable. Every hit can vary.

---

# 7. High-Level System Architecture

## 7.1 Plugin Architecture

```text
DAW Host
  ↓
NIH-plug Plugin Wrapper
  ↓
Plugin Processor
  ├── Parameter Store
  ├── MIDI Event Handler
  ├── Voice Manager
  ├── Voice Instances
  │     ├── Exciter
  │     ├── Object Model
  │     ├── Modal Resonator Bank
  │     ├── Damage Processor
  │     ├── Body/Space Processor
  │     └── Voice Envelope / Tail Manager
  ├── Global FX
  ├── Output Limiter
  └── Plugin Editor / UI
```

## 7.2 Signal Flow

```text
MIDI Note / Trigger
  ↓
Exciter Generator
  ↓
Object Model Parameter Mapping
  ↓
Modal Resonator Bank
  ↓
Damage / Corrosion / Nonlinear Response
  ↓
Body Resonance / Space
  ↓
Global Drive / Limiter / Output Gain
```

## 7.3 DSP Philosophy

The engine should prioritize convincing musical behavior over perfect physics. The initial model should use an exciter-resonator approach with curated modal profiles for different industrial objects.

Full finite-element simulation, computational fluid modeling, and deep physical simulation are out of scope for the MVP.

---

# 8. Plugin Formats and Platform Support

## 8.1 Required Formats

- VST3 instrument
- CLAP instrument

## 8.2 Optional Future Formats

- Standalone application
- VST3 effect version
- CLAP effect version
- LV2
- AU, only if a macOS-specific distribution strategy is later chosen

## 8.3 Target Operating Systems

MVP:

- Windows 10/11 x64
- Linux x64

Future:

- macOS Apple Silicon
- macOS Intel

## 8.4 Recommended Development Host

- REAPER for quick plugin testing
- Bitwig for CLAP testing
- pluginval for validation

---

# 9. Modes of Operation

## 9.1 Instrument Mode

The plugin receives MIDI note input and generates audio. This is the MVP mode.

### Requirements

- MIDI note-on triggers an object excitation.
- MIDI note-off allows natural tail decay.
- Velocity affects strike force and brightness.
- Pitch is derived from MIDI note.
- All major parameters are automatable.

## 9.2 Percussion Kit Mode

Future mode where each MIDI note maps to a different object preset.

### Example

```text
C1  → Oil Tank Boom
D1  → Bent Plate Snare
F#1 → Chain Hat
G1  → Pipe Clang
A1  → Scrape Burst
```

## 9.3 Drone Mode

Future mode where an exciter can sustain continuously instead of being triggered by isolated note events.

Useful for:

- bowed metal
- motor resonance
- vent shaft drones
- factory hum
- dark ambient beds

## 9.4 Effect Mode

Future separate plugin variant. Incoming audio excites the object resonator.

Signal flow:

```text
Audio input → transient/envelope follower → object resonator → damage → space → output
```

---

# 10. Core DSP Components

---

# 10.1 Exciter System

## 10.1.1 Purpose

The exciter provides the initial energy that causes the modeled object to vibrate.

## 10.1.2 MVP Exciter: Hit

The Hit exciter simulates a short impact made of:

- impulse component
- short filtered noise burst
- velocity-dependent force
- hardness-dependent brightness

### Parameters

| Parameter | Range | Description |
|---|---:|---|
| Force | 0.0–1.0 | Strength of the strike |
| Hardness | 0.0–1.0 | Soft mallet to hard metal strike |
| Contact Time | 0.1–30 ms | How long the exciter remains in contact |
| Noise Burst | 0.0–1.0 | Amount of noisy transient |

### Behavior

- Higher force increases amplitude and resonator excitation.
- Higher hardness increases high-frequency transient content.
- Higher contact time softens the attack.
- Higher velocity maps to force and hardness by default.

## 10.1.3 Future Exciter: Scrape

Simulates continuous friction excitation.

### Parameters

| Parameter | Range | Description |
|---|---:|---|
| Pressure | 0.0–1.0 | Contact pressure |
| Speed | 0.0–1.0 | Scrape speed |
| Roughness | 0.0–1.0 | Surface irregularity |
| Stick-Slip | 0.0–1.0 | Intermittent friction behavior |
| Direction | -1.0–1.0 | Scrape motion direction |

### Sound

- rail scrapes
- bowed steel
- brake squeals
- tension risers
- metallic drones

## 10.1.4 Future Exciter: Motor

Simulates periodic mechanical excitation.

### Parameters

| Parameter | Range | Description |
|---|---:|---|
| RPM | 10–3000 | Rotation speed |
| Load | 0.0–1.0 | Mechanical resistance |
| Bearing Damage | 0.0–1.0 | Wobble/noise from damaged bearing |
| Pulse Sharpness | 0.0–1.0 | Smooth hum to pulsed knocking |
| Electrical Hum | 0.0–1.0 | Low hum component |

### Sound

- turbines
- failing motors
- machine drones
- rotating metal pulses

## 10.1.5 Future Exciter: Chain Collision

Simulates multiple small collisions between metal links.

### Parameters

| Parameter | Range | Description |
|---|---:|---|
| Link Count | 1–128 | Number of simulated links |
| Collision Density | 0.0–1.0 | Frequency of collisions |
| Tension | 0.0–1.0 | Loose to taut chain |
| Gravity | 0.0–1.0 | Weighted collision behavior |
| Randomness | 0.0–1.0 | Irregularity of impacts |

---

# 10.2 Object Model System

## 10.2.1 Purpose

The object model defines the resonant material being excited.

Objects are represented using curated modal profiles plus parameter transformations for size, material, damping, rust, damage, and stress.

## 10.2.2 MVP Object: Pipe

### Timbre

- hollow
- ringing
- pitched
- tubular
- good for basses and tuned percussion

### Parameters

| Parameter | Effect |
|---|---|
| Size | Larger pipe lowers pitch and extends decay |
| Diameter | Changes hollowness and mode balance |
| Thickness | More thickness increases stiffness and brightness |
| Damping | Shortens decay |
| Strike Position | Changes mode excitation balance |

### Typical Sounds

- rusted pipe bass
- hollow clang
- tuned steel percussion
- dark industrial bell

## 10.2.3 MVP Object: Plate

### Timbre

- broad
- inharmonic
- metallic
- unstable
- crash-like

### Parameters

| Parameter | Effect |
|---|---|
| Size | Larger plate lowers resonances |
| Thickness | Thicker plate increases stiffness |
| Bend | Detunes and destabilizes modes |
| Edge Looseness | Adds rattle and secondary resonances |
| Damping | Controls decay length |

### Typical Sounds

- bent sheet snare
- warehouse door clang
- thunder sheet hit
- metallic crash

## 10.2.4 MVP Object: Tank

### Timbre

- booming
- hollow
- low-body resonance
- metallic upper tail

### Parameters

| Parameter | Effect |
|---|---|
| Size | Larger tank produces deeper boom |
| Air Cavity | Controls hollow resonance |
| Wall Thickness | Controls metallic sharpness |
| Lid Looseness | Adds rattle |
| Internal Pressure | Changes pitch and resonance tension |

### Typical Sounds

- oil drum hit
- container boom
- metallic kick layer
- cinematic impact

## 10.2.5 Future Object: Chain

### Timbre

- rattling
- irregular
- high transient density
- little stable pitch

### Typical Sounds

- industrial hi-hats
- metallic shakers
- dragged chains
- loose machine parts

## 10.2.6 Future Object: Spring / Wire

### Timbre

- boingy
- tense
- pitch-sliding
- unstable

### Typical Sounds

- spring reverb hits
- wire snaps
- horror stingers
- resonant tension effects

## 10.2.7 Future Object: Vent Shaft

### Timbre

- hollow
- noisy
- air-coupled
- tunnel-like

### Typical Sounds

- duct drones
- pressure bursts
- ventilation ambience
- industrial room tones

---

# 10.3 Modal Resonator Bank

## 10.3.1 Purpose

The modal resonator bank converts excitation energy into object-specific resonances.

Each object profile contains modes:

```text
mode = frequency ratio + gain + decay
```

The note frequency determines the base frequency. Object properties transform mode frequency, gain, decay, and stability.

## 10.3.2 Mode Data Structure

```rust
struct ModeSpec {
    ratio: f32,
    gain: f32,
    decay: f32,
}
```

## 10.3.3 Runtime Mode State

```rust
struct ResonantMode {
    b0: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
    pan_l: f32,
    pan_r: f32,
}
```

## 10.3.4 Resonator Formula

Use a second-order resonator:

```text
y[n] = b0*x[n] - a1*y[n-1] - a2*y[n-2]
```

Where:

```text
omega = 2πf / sample_rate
r = exp(-1 / (decay_seconds * sample_rate))
a1 = -2r cos(omega)
a2 = r²
```

## 10.3.5 Parameter Transformations

### Size

```text
larger size → lower modal frequencies
```

Implementation:

```text
frequency = base_frequency * ratio / size
```

### Rust

```text
more rust → shorter decay + less high-frequency gain + more noise
```

Implementation:

```text
gain *= max(0, 1 - rust * ratio * high_loss_factor)
decay *= max(min_decay_factor, 1 - rust * decay_loss_factor)
```

### Damage

```text
more damage → detuning + instability + nonlinear buzz
```

Implementation:

```text
ratio *= 1 + random_offset * damage
```

### Material

Material should alter:

- modal distribution
- damping curve
- brightness
- inharmonicity
- stiffness

Material is optional for MVP and required for Version 0.2.

---

# 10.4 Damage / Corrosion Processor

## 10.4.1 Purpose

The damage processor transforms a clean modeled object into a stressed, corroded, unstable industrial object.

## 10.4.2 MVP Damage Behavior

MVP damage includes:

- asymmetric saturation
- mode detuning
- gain irregularity
- transient roughness

## 10.4.3 Damage Parameters

| Parameter | Range | Description |
|---|---:|---|
| Rust | 0.0–1.0 | Dulls, shortens, and roughens the sound |
| Damage | 0.0–1.0 | Adds detuning, buzz, and instability |
| Drive | 1.0–12.0 | Saturation strength |

## 10.4.4 Future Damage Parameters

| Parameter | Range | Description |
|---|---:|---|
| Loose Parts | 0.0–1.0 | Adds rattle and secondary impacts |
| Bend | 0.0–1.0 | Detunes modes and changes inharmonicity |
| Crack | 0.0–1.0 | Adds split transients and buzz |
| Stress | 0.0–1.0 | Increases pitch instability under high force |
| Overload | 0.0–1.0 | Adds nonlinear resonator overload |

## 10.4.5 Saturation Formula

Simple MVP formula:

```text
asymmetric = x + damage * asymmetry_amount * x²
output = tanh(asymmetric * drive)
```

Future formula should support selectable nonlinear modes:

- soft saturation
- hard clipping
- wavefolding
- diode-like asymmetry
- transformer-like saturation
- resonator overload

---

# 10.5 Body / Space Processor

## 10.5.1 Purpose

Body and space processing gives the sound physical context.

## 10.5.2 MVP

MVP may omit full space processing or use a simple body resonator.

## 10.5.3 Future Body Types

| Body Type | Sound |
|---|---|
| Small Metal Box | Tight, resonant, claustrophobic |
| Oil Tank | Booming, enclosed, metallic |
| Vent Shaft | Hollow, filtered, tunnel-like |
| Concrete Room | Hard reflections, dark low-mids |
| Warehouse | Large, wide, long decay |
| Underpass | Slapback and low-mid resonance |

## 10.5.4 Implementation Options

- comb filters
- allpass diffuser
- modal body resonator
- convolution impulse responses
- feedback delay network

For Version 0.2, use a lightweight algorithmic body resonator before implementing convolution.

---

# 11. Parameters

---

# 11.1 MVP Parameters

| ID | Name | Type | Range / Values | Default | Automatable |
|---|---|---|---|---:|---|
| object | Object | enum | Pipe, Plate, Tank | Pipe | Yes |
| size | Size | float | 0.25–4.0 | 1.0 | Yes |
| rust | Rust | float | 0.0–1.0 | 0.35 | Yes |
| damage | Damage | float | 0.0–1.0 | 0.25 | Yes |
| drive | Drive | float | 1.0–12.0 | 2.0 | Yes |
| output | Output | float | 0.0–1.0 | 0.5 | Yes |

---

# 11.2 Version 0.2 Parameters

| ID | Name | Type | Range / Values | Default | Automatable |
|---|---|---|---|---:|---|
| exciter | Exciter | enum | Hit, Scrape, Motor | Hit | Yes |
| material | Material | enum | Iron, Steel, Aluminum, Glass, Concrete | Steel | Yes |
| damping | Damping | float | 0.0–1.0 | 0.4 | Yes |
| strike_pos | Strike Position | float | 0.0–1.0 | 0.5 | Yes |
| hardness | Hardness | float | 0.0–1.0 | 0.8 | Yes |
| loose_parts | Loose Parts | float | 0.0–1.0 | 0.0 | Yes |
| width | Width | float | 0.0–1.0 | 0.5 | Yes |

---

# 11.3 Version 1.0 Parameters

| Category | Parameters |
|---|---|
| Exciter | Force, Hardness, Contact Time, Scrape Pressure, Scrape Speed, Motor RPM, Bearing Damage |
| Object | Object Type, Size, Thickness, Tension, Damping, Material, Strike Position |
| Damage | Rust, Damage, Bend, Crack, Loose Parts, Stress, Overload |
| Body | Body Type, Body Amount, Space Size, Distance, Width |
| Sequencer | Step On, Pitch, Velocity, Object Lock, Exciter Lock, Rust Lock, Damage Lock, Probability, Microtiming |
| Output | Drive, Tone, Limiter, Output Gain |

---

# 12. Macro Controls

Macro controls should offer musical access to multiple internal parameters.

## 12.1 MVP Macros

| Macro | Internal Behavior |
|---|---|
| Mass | Changes size, lower resonance, decay length |
| Corrosion | Increases rust, high-frequency loss, noise, instability |
| Violence | Increases force, hardness, drive, transient brightness |
| Damage | Increases mode detuning, asymmetry, buzz, rattle |

## 12.2 Future Macros

| Macro | Internal Behavior |
|---|---|
| Tension | Increases stiffness, pitch, stress, instability |
| Ritual | Darkens tone, lengthens decay, adds low resonance and drone component |
| Machinery | Adds motor pulse, bearing noise, rotational modulation |
| Space | Increases body amount, room size, width, tail |

---

# 13. MIDI Behavior

## 13.1 Note-On

On MIDI note-on:

1. Find inactive voice.
2. If no inactive voice, steal quietest or oldest voice.
3. Convert MIDI note to base frequency.
4. Configure object modal bank.
5. Trigger exciter.
6. Start tail energy tracking.

## 13.2 Note-Off

For percussive hit mode:

- Note-off does not immediately stop audio.
- Resonator tail decays naturally.

For future sustained modes:

- Note-off releases continuous excitation.
- Resonator continues until tail falls below threshold.

## 13.3 Velocity Mapping

Velocity should map to:

| Destination | Mapping |
|---|---|
| Force | Strong |
| Transient brightness | Medium |
| Exciter noise burst | Medium |
| Damage response | Optional weak mapping |
| Output amplitude | Moderate, not exclusive |

Velocity should not merely control loudness.

## 13.4 Aftertouch Mapping

Future default:

| Aftertouch | Destination |
|---|---|
| Channel pressure | Exciter pressure or damage stress |
| Poly pressure | Per-note scrape/bow pressure |

## 13.5 Mod Wheel Mapping

Default:

```text
Mod Wheel → Corrosion Macro
```

Future user-configurable mapping should be supported.

---

# 14. Voice Management

## 14.1 MVP Polyphony

- 8 voices

## 14.2 Future Polyphony

- User-selectable: 1, 4, 8, 16 voices

## 14.3 Voice Stealing

Priority:

1. Inactive voice
2. Quietest active voice
3. Oldest active voice

## 14.4 Tail Detection

Each voice tracks tail energy.

A voice becomes inactive when:

```text
exciter is inactive AND tail_energy < threshold
```

Suggested threshold:

```text
0.00001
```

---

# 15. Preset System

## 15.1 Preset Requirements

Presets must store:

- parameter values
- object selection
- exciter selection
- macro values
- optional sequencer state in future versions

## 15.2 Preset Categories

| Category | Example Presets |
|---|---|
| Pipe | Rusted Pipe Bass, Hollow Mainline, Broken Steam Tube |
| Plate | Bent Sheet Snare, Warehouse Door, Thunder Panel |
| Tank | Oil Drum Boom, Empty Container, Submerged Boiler |
| Chain | Hanging Chain, Dragged Links, Loose Machinery |
| Motor | Basement Generator, Failing Turbine, Bearing Collapse |
| Scrape | Rail Pressure, Brake Scream, Bowed Steel |
| Drone | Furnace Room, Air Duct Choir, Ritual Machine |
| Percussion Kits | Factory Kit, Pipe Kit, Rust Kit |

## 15.3 MVP Factory Presets

Minimum 20 presets:

1. Rusted Pipe Bass
2. Hollow Pipe Clang
3. Iron Bell
4. Broken Beam Hit
5. Bent Plate Snare
6. Warehouse Door
7. Metallic Crash
8. Oil Tank Boom
9. Empty Container
10. Submerged Boiler
11. Factory Knock
12. Hard Pipe Tick
13. Corroded Bass Hit
14. Deep Tank Kick Layer
15. Sharp Plate Rim
16. Rusted Alarm Tone
17. Distant Steel Hit
18. Heavy Industrial Tom
19. Short Metal Knock
20. Long Pipe Tail

---

# 16. User Interface Specification

---

# 16.1 UI Philosophy

The UI should communicate the physical model visually and semantically.

Avoid presenting the synth as:

```text
Oscillator → Filter → Amp
```

Instead present it as:

```text
Exciter → Object → Damage → Space
```

## 16.2 MVP UI

The MVP may use NIH-plug's generic editor or a minimal custom editor.

Required controls:

- Object
- Size
- Rust
- Damage
- Drive
- Output

## 16.3 Version 0.2 UI Layout

```text
+--------------------------------------------------+
| Corrosion                                        |
| Industrial Physical Modeling Synthesizer         |
+--------------------------------------------------+
| [EXCITER] → [OBJECT] → [DAMAGE] → [SPACE]        |
+--------------------------------------------------+
| Macro Controls: Mass | Impact | Corrosion | Damage |
+--------------------------------------------------+
| Resonator Visual / Object Display                |
+--------------------------------------------------+
| Presets | Randomize | Output                    |
+--------------------------------------------------+
```

## 16.4 Object Panel

Controls:

- Object Type
- Size
- Material
- Damping
- Strike Position

## 16.5 Exciter Panel

Controls:

- Exciter Type
- Force
- Hardness
- Pressure
- Speed / RPM depending on exciter

## 16.6 Damage Panel

Controls:

- Rust
- Damage
- Loose Parts
- Bend
- Overload

## 16.7 Space Panel

Controls:

- Body Type
- Body Amount
- Width
- Distance

## 16.8 Visual Feedback

Future visual elements:

- object icon or silhouette
- resonant mode bars
- strike position indicator
- damage overlay
- tail energy meter
- output meter

---

# 17. Randomization

## 17.1 Purpose

Randomization should help users discover industrial timbres quickly while preserving musical usefulness.

## 17.2 Randomize Modes

| Mode | Behavior |
|---|---|
| Safe Random | Small variations around current patch |
| Object Random | Changes object parameters only |
| Damage Random | Changes rust, damage, loose parts, drive |
| Full Random | Randomizes all sound parameters within musical ranges |
| Mutate | Generates related version of current patch |

## 17.3 Constraints

Randomization must avoid:

- dangerous output levels
- inaudible patches
- extreme DC offset
- fully silent combinations
- uncontrolled resonator blowup

---

# 18. Sequencer Specification — Future Version

## 18.1 Purpose

The sequencer turns Corrosion into an industrial pattern generator.

## 18.2 Basic Features

- 16 or 32 steps
- per-step note
- per-step velocity
- per-step probability
- per-step microtiming
- per-step object lock
- per-step exciter lock
- per-step rust/damage/drive lock
- step retrigger

## 18.3 Step Data

```rust
struct Step {
    enabled: bool,
    note: u8,
    velocity: f32,
    probability: f32,
    microtiming: f32,
    object_lock: Option<ObjectType>,
    exciter_lock: Option<ExciterType>,
    rust_lock: Option<f32>,
    damage_lock: Option<f32>,
    drive_lock: Option<f32>,
}
```

## 18.4 DAW Sync

The sequencer must sync to host tempo and transport.

Required host-derived data:

- BPM
- sample position
- playing state
- loop state if available

---

# 19. Real-Time Safety Requirements

## 19.1 Audio Thread Restrictions

The audio thread must not perform:

- heap allocation
- file I/O
- logging
- mutex locking
- JSON parsing
- preset loading
- string formatting
- dynamic vector resizing
- unbounded loops
- blocking operations

## 19.2 Audio Thread Allowed Operations

- fixed-size array processing
- preallocated voice rendering
- atomic parameter reads
- simple math
- coefficient interpolation
- smoothed parameter updates
- deterministic pseudo-random generation with local state

## 19.3 Memory Allocation

All primary DSP structures must be allocated during:

- plugin construction
- initialization
- prepare-to-play
- preset load outside audio thread

---

# 20. Performance Requirements

## 20.1 MVP Performance Targets

At 48 kHz, 128-sample buffer:

| Scenario | Target CPU |
|---|---:|
| 1 active voice | < 1% on modern desktop CPU |
| 8 active voices | < 8% on modern desktop CPU |
| Idle plugin | Near 0% |

## 20.2 Stability Targets

- No denormal slowdowns.
- No NaN or infinite output.
- No uncontrolled feedback blowups.
- No clipping above hard safety limit.
- Plugin must recover safely from extreme automation.

## 20.3 Safety Limiter

The output stage must include a hard safety clip or limiter.

MVP:

```text
output = clamp(output, -1.0, 1.0)
```

Future:

- soft clipper
- lookahead optional limiter
- output meter warning

---

# 21. Code Architecture

## 21.1 Recommended File Structure

```text
corrosion/
  Cargo.toml
  src/
    lib.rs
    params.rs
    voice.rs
    voice_manager.rs
    dsp/
      mod.rs
      exciter.rs
      resonator.rs
      objects.rs
      damage.rs
      body.rs
      limiter.rs
    ui/
      mod.rs
      editor.rs
    presets/
      mod.rs
  xtask/
    Cargo.toml
    src/
      main.rs
```

## 21.2 Plugin Shell

Responsible for:

- NIH-plug integration
- format export
- buffer processing
- parameter exposure
- MIDI event handling
- voice manager dispatch

## 21.3 Params Module

Responsible for:

- parameter definitions
- default values
- enum parameters
- smoothing styles
- user-visible names
- parameter IDs

## 21.4 Voice Module

Responsible for:

- note state
- exciter state
- resonator state
- tail detection
- rendering samples

## 21.5 DSP Modules

DSP modules should be independent from plugin UI and host code.

### Desired Property

The DSP should be testable without loading the plugin.

---

# 22. Testing Strategy

## 22.1 Unit Tests

Test:

- MIDI note to frequency conversion
- resonator coefficient generation
- resonator stability
- saturation output range
- object profile selection
- tail deactivation logic

## 22.2 DSP Tests

Render deterministic test buffers and verify:

- no NaN
- no infinity
- output is not silent when excited
- output decays after excitation
- rust changes spectral energy
- damage changes waveform shape

## 22.3 Plugin Tests

Use pluginval to verify:

- plugin loads
- parameters automate
- process callback is stable
- no crashes during random automation
- no crashes during rapid note events

## 22.4 Manual DAW Tests

Test in:

- REAPER
- Bitwig
- Ardour

Scenarios:

- MIDI notes
- fast sequences
- long decays
- automation
- preset changes
- buffer size changes
- sample rate changes

---

# 23. Acceptance Criteria

## 23.1 MVP Acceptance Criteria

Corrosion 0.1 is acceptable when:

1. The plugin builds as VST3 and CLAP.
2. The plugin loads in REAPER.
3. MIDI note-on triggers audible sound.
4. Pipe, Plate, and Tank object modes sound clearly different.
5. Size changes pitch/resonance behavior.
6. Rust audibly darkens/shortens/roughens sound.
7. Damage audibly destabilizes or saturates sound.
8. Drive audibly increases nonlinear aggression.
9. Output gain works safely.
10. No crash occurs under rapid MIDI note triggering.
11. No NaN or infinite samples occur.
12. 8 voices can play simultaneously.
13. At least 20 factory presets are included.
14. Plugin does not allocate memory in the sample render path.
15. Plugin passes basic plugin validation.

## 23.2 Version 0.2 Acceptance Criteria

1. Scrape exciter implemented.
2. Chain object implemented.
3. Stereo modal spread implemented.
4. Body resonance implemented.
5. Basic custom GUI implemented.
6. Randomizer implemented.
7. Velocity mapping improved.
8. 40+ factory presets included.

## 23.3 Version 1.0 Acceptance Criteria

1. Sequencer implemented.
2. Per-step locks implemented.
3. Preset browser implemented.
4. Multiple exciter types implemented.
5. Multiple body/space types implemented.
6. User-configurable modulation mappings implemented.
7. 100+ presets included.
8. Documentation included.
9. Installer or clear installation package provided.

---

# 24. Development Roadmap

## Milestone 0 — Research Prototype

Goal: Validate sound in Python or Rust command-line renderer.

Deliverables:

- modal resonator proof of concept
- pipe/plate/tank profiles
- WAV renders
- chosen parameter ranges

## Milestone 1 — Minimal Plugin

Goal: First VST3/CLAP instrument that produces sound.

Deliverables:

- NIH-plug project
- MIDI note handling
- one voice
- pipe object
- hit exciter
- output audio

## Milestone 2 — MVP Synth

Goal: Usable early instrument.

Deliverables:

- 8 voices
- Pipe, Plate, Tank
- Size, Rust, Damage, Drive, Output
- stable rendering
- generic editor
- 20 presets

## Milestone 3 — Industrial Character

Goal: Make it sound unique.

Deliverables:

- scrape exciter
- chain object
- stereo spread
- rattle generator
- body resonator
- better saturation

## Milestone 4 — Product UX

Goal: Make it usable and inspiring.

Deliverables:

- custom GUI
- visual object model
- preset browser
- randomizer
- macro controls

## Milestone 5 — Sequenced Instrument

Goal: Industrial techno workflow.

Deliverables:

- 16/32-step sequencer
- per-step locks
- host sync
- probability
- microtiming
- kit mode

## Milestone 6 — Version 1.0 Release

Goal: Public-ready release.

Deliverables:

- VST3 and CLAP bundles
- user manual
- factory preset library
- demo sounds
- validation reports
- installation instructions

---

# 25. Sound Design Specification

## 25.1 Core Sound Identity

The plugin should sound:

- metallic
- physical
- corroded
- heavy
- unstable
- resonant
- industrial
- dark
- percussive
- cinematic

It should avoid sounding:

- clean by default
- EDM-polished by default
- generic analog
- sample-library static
- purely academic
- overly realistic and sterile

## 25.2 Patch Families

### Bass

- pipe bass
- tank sub hit
- damaged metallic low tone
- resonant industrial tom

### Percussion

- plate snare
- chain hat
- pipe rim
- tank kick layer
- beam knock

### Drones

- bowed plate
- motorized pipe
- vent resonance
- warehouse body hum

### Transitions

- scrape rise
- pressure release
- collapsing metal tail
- reverse resonator swell

### Cinematic Impacts

- container slam
- boiler hit
- iron door shock
- underpass impact

---

# 26. Documentation Requirements

## 26.1 User Manual

Must explain:

- what Corrosion is
- object/exciter concept
- basic controls
- MIDI behavior
- preset system
- automation
- installation
- troubleshooting

## 26.2 Developer Documentation

Must explain:

- architecture
- DSP modules
- parameter mappings
- real-time safety assumptions
- build instructions
- testing instructions
- release process

## 26.3 Sound Design Guide

Must include recipes:

- create a rusted pipe bass
- create a bent plate snare
- create an oil tank boom
- create a chain hi-hat
- create a bowed metal drone
- create an industrial loop

---

# 27. Release Packaging

## 27.1 Required Release Artifacts

For each release:

- VST3 bundle
- CLAP bundle
- README
- changelog
- preset folder if external
- installation instructions

## 27.2 Versioning

Use semantic versioning:

```text
0.1.0 — first MVP (Gate 2)
0.2.0 — scrape/chain/body update (Gate 3)
0.3.0 — GUI/randomizer update (Gate 4)
0.4.0+ — sequencer (Gate 5)
0.9.0+ — core feature complete (Gate 6)
1.0.0 — public release (requires extensive polish/testing post-Gate 6)
```

**Note**: The 6 gates are milestone checkpoints. Gate 6 marks "core feature complete," but significant additional work (polish, optimization, platform support, expanded testing, etc.) will be required before a v1.0 public release.

---

# 28. Risks and Mitigations

## 28.1 Risk: DSP Sounds Weak

Mitigation:

- prototype modal profiles offline
- compare object families frequently
- prioritize strong saturation/body behavior
- use sound-design presets early

## 28.2 Risk: CPU Too High

Mitigation:

- fixed maximum mode count
- block-rate parameter updates
- avoid per-sample coefficient recalculation
- reduce voice count initially
- optimize resonator loops

## 28.3 Risk: UI Scope Creep

Mitigation:

- ship generic editor first
- custom GUI only after sound engine works
- keep MVP controls minimal

## 28.4 Risk: Plugin Format Complexity

Mitigation:

- rely on NIH-plug bundling
- test early in REAPER and Bitwig
- keep plugin shell simple

## 28.5 Risk: Physical Modeling Becomes Too Academic

Mitigation:

- judge by musical usefulness
- design presets around real production use
- keep macros expressive and intuitive

---

# 29. Implementation Priorities

## Priority 1 — Sound

The instrument must sound compelling before UI complexity is added.

## Priority 2 — Stability

The plugin must never produce dangerous output, crash under normal DAW use, or allocate in the audio path.

## Priority 3 — Simplicity

The first version must remain focused.

## Priority 4 — Expressiveness

Velocity and automation should change physical behavior, not just volume.

## Priority 5 — Expandability

The architecture should allow new objects, exciters, damage models, and sequencing without rewriting the plugin.

---

# 30. Final MVP Definition

The first complete version of Corrosion should be:

```text
A VST3/CLAP industrial physical modeling instrument with:

- 8-voice polyphony
- MIDI note input
- Hit exciter
- Pipe, Plate, and Tank object models
- Modal resonator bank
- Rust and Damage controls
- Drive and Output controls
- Velocity-sensitive excitation
- Safe output limiter
- Generic or minimal GUI
- 20 factory presets
```

This version is small enough to build but complete enough to validate the product concept.

---

# 31. Future Product Expansion

After the MVP, Corrosion can evolve into a larger ecosystem:

## Corrosion Instrument

Main VST3/CLAP synth.

## Corrosion FX

Audio effect that processes incoming sounds through industrial resonators.

## Corrosion Kits

Expansion preset packs for industrial techno, dark ambient, cinematic impacts, and experimental percussion.

## Corrosion Sequencer

Pattern-based industrial groove engine.

## Corrosion Lab

Standalone sound-design workstation for rendering one-shots, loops, and drones.

---

# 32. Strategic Product Principle

Corrosion should not compete by having the most features.

It should compete by having the clearest sound identity:

```text
physically modeled industrial objects under stress
```

Every feature should reinforce that identity.


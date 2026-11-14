# Corrosion

Corrosion is an industrial physical-modeling instrument built as a VST3 and CLAP plugin. MIDI notes strike, bow, scrape or blow modal models of metal objects (pipes, plates, tanks, chains, beams, cables, springs, sheets, cogs), and a post chain shapes them. It is tuned for damaged metal, friction, weight and mechanical tension, not for synth-style oscillators.

**Try it in your browser, no install: <https://corrosion-vst.org/play/>** (see [Try it in the browser](#try-it-in-the-browser)). The project's site is <https://corrosion-vst.org>, and the Corrosion Wiki, the user guide in English and Spanish, is at <https://corrosion-vst.org/wiki/>: every control explained, with audio examples, recipes and troubleshooting.

Package version: `0.2.0`. Preset schema version: `4`; files from schemas 1–3 are migrated on load (`src/presets/mod.rs::migrate_preset_json`).

## How to Use

### Try it in the browser

The whole plugin, the same engine and editor, also runs in a web page: no download, no DAW. The web app is built from this repository and redeployed from `main` on Cloudflare, at **<https://corrosion-vst.org/play/>** (the old `preview.corrosion-vst.org` address redirects there; sessions and presets saved in the browser at the old address stay with it). Click **Start audio**, then play the piano strip, the computer keys or a MIDI keyboard.

- **Browsers:** tested in Chromium. It needs AudioWorklet, WebAssembly SIMD and WebGL 2, which current Chrome, Edge, Firefox and Safari have, but only Chromium has been checked so far. Web MIDI is in Chrome and Edge, and in Firefox after a permission prompt; Safari has none. Without it the on-screen and computer keyboards still play.
- **Your sound and presets stay in your browser:** the session is restored on your next visit, Save keeps presets in the browser, **Download preset** gives a `.corrosion-preset` file the plugin opens, and **Open preset…** loads one made with the plugin.
- **Limits:** the web engine runs on one core in the browser's audio thread. The heaviest presets with all 16 voices can be too much for a slower computer: the CPU figure in the header turns red when it gets close. A BPM field stands in for the host tempo.

### Install

1. Download the zip for your platform and format from the [`nightly` pre-release](../../releases/tag/nightly). It is rebuilt by every green CI run on `main`: tested on its platform and checked with clap-validator and pluginval.
2. Unzip it and move the `Corrosion.vst3` or `Corrosion.clap` bundle into your plugin folder (the zip also holds the license):

   | Platform | VST3 | CLAP |
   |---|---|---|
   | Windows (x86_64) | `C:\Program Files\Common Files\VST3` | `C:\Program Files\Common Files\CLAP` |
   | macOS (Apple Silicon and Intel) | `~/Library/Audio/Plug-Ins/VST3` | `~/Library/Audio/Plug-Ins/CLAP` |
   | Linux (x86_64) | `~/.vst3` | `~/.clap` |

3. **macOS only:** the bundles are ad-hoc signed, not notarized, so macOS quarantines them after a download. Before the first use run `xattr -dr com.apple.quarantine ~/Library/Audio/Plug-Ins/VST3/Corrosion.vst3` (and the same for `CLAP/Corrosion.clap`).
4. Rescan plugins in your host and load **Corrosion** (vendor "Synaptech Solutions") on an instrument track.

To build it yourself instead, see [Build And Run](#build-and-run).

### First sounds

1. **Pick a preset.** Click the preset name in the header to open the browser: Bass, Lead, Chords, Drones, Percussion, Drums and Experimental, then your own presets under User. The **‹ ›** arrows step through the current folder.
2. **Play it** from your MIDI keyboard or the host, or from the piano strip at the bottom of the editor: click the keys, or use the computer keys `A W S E D F T G Y H U J K` (C4 to C5) while the pointer is over the editor.
3. **Change the object and how it is played.** **Exciter** chooses what plays the object (a bow, a mallet, a chain, a jet of air…), **Object** what is played. **Size** moves the pitch and weight (pitch is roughly the key divided by Size). **Rust**, **Damage** and **Heat** age the metal.
4. **Shape the result** with the four **Macros** (Mass, Corrosion, Violence, Brightness), then the post chain: filter, **Drive**, **BODY**, spread and **Space** (Factory, Spring or Echo).
5. **Hover the ⓘ badge** next to any control for a one- or two-sentence explanation.
6. **Save** your sound: type a name next to the browser, pick a user folder and press Save. **Randomize** draws a variation within safe limits.

### Playing a drum kit

1. Load a kit from **Drums**; Play switches to **Kit**. Each key from C2 plays its own pad on the General MIDI layout (36 kick, 38 snare, 42 closed hat, …).
2. To build your own: dial a sound in Tonal mode and play it, switch Play to Kit, click a key in the kit zone (C2–B3) and press **Assign**. The pad keeps the pitch you heard; change it with **Tune**. **Clear** removes a pad. See [Kit mode](#kit-mode).

### Playing it expressively

- Velocity, pitch bend, channel and polyphonic pressure, and the mod wheel act on held notes; CLAP and VST3 hosts can also address single notes (per-note tuning and choke).
- The sustain pedal (CC64) holds notes; CC120 / CC123 stop everything.
- **Drone** note mode keeps bowed, scraped and dragged notes sounding until you let go.
- **Quality** trades CPU for density and smoother aliasing: Eco, Normal (default), High, Render (for bouncing).

## What It Ships

- VST3 and CLAP plugins via NIH-plug, for Windows (x86_64), macOS (universal: Apple Silicon and Intel) and Linux (x86_64).
- Stereo instrument layout (no audio input; an audio-in "Effect mode" is on the roadmap).
- **MIDI expression**: notes with velocity, pitch bend (default ±2 semitones, ±24 safety clamp), channel and polyphonic pressure, and CC1 mod wheel, all routed live to active voices.
- **Per-note expression** (CLAP and VST3 hosts): notes the host addresses by voice ID are released, pressed, retuned and choked individually, so two overlapping notes on one key stay separate. Per-note tuning (`PolyTuning`) shifts one note in semitones on top of the channel pitch bend; a host choke fades the note to silence in 2 ms without a click. Hosts that send plain MIDI match notes by key and channel; the oldest note on a key is released first. Channel MPE (per-channel bend and pressure zones) isn't supported: channel messages stay global.
- **Sustain pedal and panic**: CC64 holds released notes until the pedal lifts (re-striking a held note replaces it; pedal-held notes are stolen before held keys). CC123 All Notes Off releases every held key (a down pedal still holds them), CC120 All Sound Off fades everything, Space tails included, to silence in 5 ms, and CC121 Reset All Controllers lifts the pedal and returns mod wheel, bend and pressure to rest. Other CCs are ignored (full CC mapping is on the roadmap).
- 16-voice polyphony. When the pool is full, released tails are stolen first, then held notes at least 100 ms old (quietest first, oldest on ties); a note that just started is only stolen if every held note is that young. Idle voices cost nothing; a full pool at Render quality measured about 36 % of one core on the development machine.
- 130+ host parameters covering exciters, objects, transformations, post-processing, four macros and three note modes.
- An egui editor with UI scaling (50 to 150 % in 5 % steps), controls for every audible parameter, inline help on every control, a piano strip, a preset browser with folders, user presets, a randomizer, and a kit editor.
- **Kit mode**: an AFX-style drum kit, one pad (a whole sound with its own tune and level) per key, saved with presets and projects.
- A factory bank of 214 presets in folders: 60 designed by hand (bass, lead, chords, drones, percussion), 10 drum kits and 144 experimental ones, one per exciter × resonator, all at a common loudness (`src/presets/factory/`, written by `src/bin/seed_presets.rs`).
- Offline renderers for comparison suites, preset batch renders, and aliasing analysis with asserted budgets.

## Current Sound Surface

### Exciters (16)

`Bow`, `HandStrike`, `FeltMallet`, `HardMallet`, `Drumstick`, `WireBrush`, `MetalPipe`, `MetalChain`, `StiffPoint`, `HeavyGrinding`, `CorrugatedDrag`, `TensionRise`, `PneumaticJet`, `ElectromagneticHum`, `TensionSnap`, `ParticleRain`.

Grouped into three envelope families:

| Family | Members | Envelope |
|---|---|---|
| Hit | HandStrike, FeltMallet, HardMallet, Drumstick, WireBrush, MetalPipe, MetalChain | one-shot AR |
| Friction | Bow, StiffPoint, HeavyGrinding, CorrugatedDrag, TensionRise | 6-stage MSEG (loopable) |
| Specialty | PneumaticJet, ElectromagneticHum, TensionSnap, ParticleRain | ADSR |

Each exciter has its own panel of physical controls (mass, stiffness, rosin, flow rate, …) that changes with the selection. The exciter and the object are coupled both ways: the object's displacement and velocity feed back into the exciter, so a bow sticks and slips on the surface it excites.

### Objects (9)

`Pipe`, `Plate`, `Tank`, `Chain`, `IBeam`, `TautCable`, `CoilSpring`, `SheetMetal`, `IndustrialCog`.

Each object is an algorithmic modal resonator generator with curated **character controls** (Pipe Diameter, Plate Aspect and Stiffness, Tank Volume and Cavity Mix, Chain Link Mass and Instability, Beam Shear, Cable Braid and Tension Drop, Spring Dispersion and Slosh, Sheet Thinness, Cog Dissonance: 14 in total). `TautCable` and `SheetMetal` also run per-sample dynamics (an amplitude-driven pitch drop, a low-frequency warp). Struck Pipe, I-Beam and Taut Cable ring for seconds; Rust and Sludge shorten them.

### Main Control Groups

- **Sound**: `Exciter`, `Object`, `Size`, `Rust`, `Damage`, `Drive`, `Output`, `Width`, `Body`. Master **Drive** (0–5) fades from clean to fully shaped over 0–1; above 1 it pushes the shaper harder (more saturation, not more level). **Output** is shown and typed in dB (−∞ to +40 dB); its knob puts 0 dB a little past the middle and spreads −20 to +6 dB over about a fifth of its travel. Presets and saved projects keep their level, but **host automation of Output recorded before this taper plays back at different levels** (BREAKING).
- **Friction envelope (MSEG)**: six stages; **Onset Time** sets the onset stage's length and **Onset Level** (0–1, default 0.01) the level it rises to. Loop modes Off, Forward and PingPong (PingPong currently always loops Decay → Sustain). **Env Amount** scales the whole envelope; at 0 the friction exciters are silent. Older presets and projects load with Onset Level equal to their old Onset value (clamped to 0–1).
- **Host parameter names** are unique, so automation lists can tell them apart: `Resonator Brightness` and `Macro Brightness`, `Brush Wire Stiffness` and `Spring Wire Stiffness`, `Onset Time` and `Onset Level`. Parameter IDs are unchanged.
- **Global exciter controls** (under the exciter selector, each neutral at its default): **Pressure** trims every exciter's force by −12 to +12 dB and follows automation on held notes; **Speed** scales motion speeds ×0.5 to ×2 (bow, scrape, grind, drag, pull, particle flow); **Roughness** scales grit and randomness ×0 to ×3.33 (rosin grip, surface grit, slip, turbulence, brush and rattle randomness, particle mass variance). Speed and Roughness apply at note-on. Exciters with no motion or grit parameter ignore them.
- **Rust, Damage, Heat**: Rust (0–5) dulls and shortens the high partials first while the low thud keeps ringing; Damage (0–10) cracks the object into beating, detuned mode pairs without losing the note; Heat (0–1) lowers the pitch slightly, darkens, and adds a slow thermal warble. None of them silences a note: across each range the level stays within about 3 dB.
- **Macros** (neutral at 0.5): `Mass`, `Corrosion`, `Violence`, `Brightness`. Each biases a cluster of destinations: Mass the exciter's mass controls and the object's size; Corrosion damping and rust; Violence drive, chaos and damage; Brightness the post-chain filter and resonator brightness.
- **Note Mode** (the header's NOTE MODE selector): `Tonal` (default: the key sets the pitch; friction notes follow the envelope and fade even while held), `Kit` (each key plays its own pad, see [Kit mode](#kit-mode); keys without a pad are silent), `Drone` (the friction envelope loops Decay and Sustain, so friction notes ring until release; struck notes still decay).
- **Quality**: `Eco` / `Normal` / `High` / `Render`. Controls the final clipper's oversampling (1×/4×/8×/16×, half-band IIR filters) **and** per-voice modal density (0.5×/1.0×/1.5×/2.0× the object's base mode count). Every mode runs the full post chain and the same diode curve, so modes trade fidelity and CPU, not effects. **The two parts change at different times:** the clipper's oversampling switches at once, but a note's mode count is fixed when it starts, so held notes and their tails keep their density until the next note-on.
- **Limiter**: `Hard` (default, zero-latency hard clamp at ≈−0.3 dBFS) or `Lookahead` (48-sample window, instant attack, 50 ms release; reports its 1 ms latency to the host).
- **Diode stage** (the final clipper, in the Brickwall Limiter panel): a soft-knee diode curve that sounds good driven hot.
  - **Diode Softness** (0–1, default 0.5) sets the knee: 0 is a hard clip, 1 starts bending about 14 dB under the limit.
  - **Diode Asymmetry** (0–1, default 0.25) clips the negative half earlier (up to 6 dB at 1, like one diode against two), adding even harmonics. A slow DC blocker keeps the result centred; at 0 the curve is symmetric and unfiltered.
  - The panel's red dot lights with the stage's gain reduction, fully at 6 dB, and fades over about a second.
  - Presets and projects saved before this stage load with Softness 0 and Asymmetry 0, close to their old hard clip. **Host automation of Diode Softness recorded before it plays back with a different knee** (BREAKING).
- **Inline help**: every control has a small ⓘ badge after its label. Hover the badge (not the control) for a one- or two-sentence explanation; exciter and object controls explain themselves for the selected model. Selectors explain the selected option (a trial feature).
- **No DC at the output**: bowed and slow-contact notes push the resonators to one side while they play. A 10 Hz high-pass on the voices' mix, before the master drive, removes that offset (30 Hz loses 0.05 dB), so the drives and the clipper bite both halves of the wave evenly and the host gets no DC.
- **Post chain**: per-channel 4-pole ladder filter → Lorenz drive (dual-mono, so the stereo image survives), plus the FEM body resonance added in parallel on the mono sum → HRTF spread → Space (Factory / Spring / Echo) → oversampled diode clipper → limiter. **BODY** (0–5) sets the body's mix level; **Chassis Volume** only sets its size (larger = lower resonances, which also ring a little louder at the same BODY). Cutoff, resonance and master drive are smoothed at audio rate (~20 ms) with a first-call snap, so static settings are unchanged.
- **Echo**: Machinery Movement sweeps the delay smoothly (interpolated reads, no zipper). High Frequency Damping darkens the repeats, more with each one, without changing their level: 0 leaves them full-range, 1 rolls them off above about 1.5 kHz.
- **Tempo sync**: off by default. `sync_rate < 0.05` (shown as "Off") keeps the Echo free-running on Delay Time. Above it, six musical divisions (1/16 → 2/1, shown by name) drive the delay from the host tempo when the host reports one. A division longer than the echo's 0.8 s is halved until it fits, so it stays on the beat (1/1 at 120 BPM plays as 1/4).
- **Per-voice modulation**: damping, brightness, strike position, coupling, position wander and its rate, position envelope, and fundamental anchor automate through to held notes; released tails keep their note-on values so decays stay consistent.
- **Strike position movement**: **Position Envelope** moves the strike point with the note's force envelope (up to half the bar at full envelope), so a strike or a bow swell sweeps the timbre. **Wander** adds a slow drift at its own **Wander Rate** (0.1–5 Hz). Older presets and saved DAW projects that used Position Envelope as the wander's rate load as the equivalent Wander Rate.
- **Release snap**: releasing a bowed, scraped or dragged note snaps the object free of the friction and can ring out louder than the held note (up to about +30 dB on the default patch). It is kept as a feature; a control to tame it is on the roadmap.

**Loudness.**
- Every object's mode bank is normalized at note-on to one strike level, so objects, registers (C2–C6), quality modes and damage settings land within a few dB of each other.
- Each exciter has a drive level that puts its typical note at one common level, so struck, bowed, scraped and blown exciters play about equally loud.
- A per-voice drive governor turns the exciter's force down when a note would near full scale, so no note overdrives the voice and none is clipped, whatever the drive.

### Factory Presets

The factory bank lives under `presets/factory/`, in folders. The preset browser shows them as submenus:

- **Bass**, **Lead**, **Chords**, **Drones**, **Percussion**: 12 presets each, designed by hand and measured so each folder keeps its promise:
  - basses keep most of their energy under 500 Hz;
  - leads and chords play the key's pitch;
  - chord patches are harmonic and sustain, so they can be played as chords;
  - drones hold their level while the key is down;
  - percussion dies away on its own.
- **Drums**: ten Kit-mode drum kits on the General MIDI layout (36 kick, 37 side stick, 38 snare, 39 clap, 41 / 45 / 48 toms, 42 / 44 / 46 closed, pedal and open hats, 49 crash, 51 ride), each from one family of objects: Foundry, Scrapyard, Pipe, Rust Belt, Boiler Room, Rail Yard, Wire, Sheet Storm, Clockwork and Junk Percussion. Pads are tuned for their role (kicks an octave down, hats and cymbals three to four octaves up) and checked for drum character: a heavy, short kick, a snare brighter than the kick, bright hats with the open hat ringing longer than the closed one, and rising toms.
- **Experimental**: one preset for every exciter on every resonator (16 × 9), grouped by exciter. Each draws every parameter from a fixed seed within playable ranges, so they're varied, sometimes strange, and always audible.

Every factory preset is levelled so a held middle C sits at the same loudness (−17.7 dBFS RMS), with headroom so even a full-velocity note barely touches the limiter. Drum pads are levelled the same way, each held alone on its own key. Regenerate the bank with `cargo run --release --bin seed_presets`.

> **Note: the factory presets are experimental.** Levels and the folder checks are measured, but the sound design isn't finished: some presets sound very similar to each other, and some don't sound like their name says. Help is welcome, both fixing the existing presets and adding new ones. Presets are built in code (`src/presets/factory/curated.rs` for the hand-made folders, `experimental.rs` and `drums.rs` for the others) and written to disk by the seeder; see [Factory presets in CLAUDE.md](CLAUDE.md#factory-presets) for the workflow, or open an issue describing a preset that misses its mark.

### User Presets and Randomizer

The editor's **Save** writes the current sound as a named preset (every parameter except UI Scale, and the kit if there is one) into your user preset folder, or one of its subfolders, picked in the selector next to the name. You can't save into the factory bank. Saving over an existing name asks first. The browser's **User** submenu mirrors the folder's subfolders (up to four levels deep), and picks up folders you add while the plugin is open. The **‹ ›** arrows step through the presets of the current folder. **Open folder** shows the user folder in your file manager (it is created if missing), so you can add, rename or delete folders and presets there. The folder:

- Windows: `%APPDATA%\Corrosion\Presets`
- macOS: `~/Library/Application Support/Corrosion/Presets`
- Linux and others: `$XDG_DATA_HOME/corrosion/presets`, or `~/.local/share/corrosion/presets`

**Randomize** draws a new Size, Rust, Damage, Drive, Width, BODY and Output within its mode's ranges and safety limits (patches stay audible, and extreme damage never meets extreme drive):

- **Safe:** gentle variations.
- **Object:** size only.
- **Damage:** rust, damage and a little drive.
- **Full:** everything, over wide ranges.

Every change is sent to the host as a normal parameter edit, so host undo applies.

The editor opens at 75 % (810 × 630). The **SCALE** selector (50–150 %) resizes the plugin window in hosts that allow it; the controls always fit the window's width, and taller exciter panels scroll.

### Playing from the editor

A piano strip at the bottom of the editor (C2–C6, MIDI 36–84) stays in view while the panels scroll:

- **Mouse:** click a key to play it, drag across keys for a glissando, release to stop. Pressing lower on a key plays louder.
- **Computer keyboard:** `A W S E D F T G Y H U J K` play C4 to C5 (white keys on the home row, black keys above), at a fixed velocity; in Kit mode they play C2 to C3 (36–48), where the drum pads are. Keys match by letter, so on AZERTY the row is the same letters, not the same positions.
- Computer keys play only while the editor has keyboard focus **and the mouse pointer is over it**, and not while you type a preset name. Moving the pointer off the editor, or closing it, releases every note it holds. The pointer condition is there because the window backend doesn't report focus loss on Windows or Linux.
- Keys light up while their note is held, from the strip or from the host's MIDI.
- Notes from the strip play like MIDI notes on channel 0 (note mode, sustain pedal and voice stealing apply), but they don't reach the host's MIDI output and aren't recorded.
- Some hosts keep keystrokes for their own shortcuts. In REAPER, keys reach the plugin while its window has focus; if they don't, enable "Send all keyboard input to plugin" in the FX window's menu. The mouse always works.

### Kit mode

In **Kit** mode each key plays its own **pad**: a whole sound (exciter, object and every voice control) stored for that key with a tune (±48 semitones) and a level. Keys without a pad are silent. The post chain (filter, drive, BODY, spread, Space, clipper, limiter, Output) is shared by the whole kit, so a kit's kick and hats go through the same drive and room. Moving a knob doesn't change pads that are already sounding.

When Play is set to Kit, a kit bar appears above the strip:

- **Click a key** to select it (and play it). Selecting a key with a pad loads its sound into the editor, so you can tweak it.
- **Assign** stores the editor's current sound on the selected key. Its tune keeps the pitch you last heard the sound at: dial a sound in Tonal mode, play it on C4, then assign it to 38 and the pad plays at C4's pitch (+22 st). Clicking an empty key in Kit mode plays nothing, so it doesn't count as hearing the sound there. **Tune** changes the pitch afterwards.
- **Clear** removes the selected key's pad.
- The kit zone is C2 to B3 (36–59); keys outside it are greyed and ignore the mouse and computer keys until **Full keyboard** is on. Host MIDI plays every key that has a pad, whatever the toggle.
- Keys with a pad get a coloured band; the selected key an outline.

A kit is saved with presets and with your DAW project. Loading a pad moves every voice parameter, which a host recording automation will record, as when loading a preset.

**BREAKING:** the old Kit mode, which mapped 10-note slots of the keyboard to objects, is gone. Presets and projects saved in the old Kit mode load in Tonal mode, with their other values kept; host automation of Note Mode (formerly Play Mode) that selects Kit now selects the new mode, which is silent until pads are assigned.

## Repository Layout

```text
.
├── Cargo.toml
├── .github/workflows/     ci.yml: checks, bundles and the nightly (Linux, Windows, macOS)
├── docs/                  ARCHITECTURE.md, BACKLOG.md, PRD.md, detailed-specs/, archive/
├── openspec/              specs/ (31 capabilities) and changes/ (active and archived)
├── presets/factory/       214 .corrosion-preset files in folders (written by seed_presets)
├── scripts/               bundle.sh, check_bundle.sh, validate-plugins.sh, verify-local.sh,
│                          check_wav.py, bench_ratios.py …; archive/ (old bundle wrappers)
├── src/
│   ├── lib.rs             Plugin trait impls, MIDI routing (incl. CCs), note setup,
│   │                      macros, note modes, state migration, drive/output smoothers
│   ├── params.rs          130+ host parameters (ExciterType×16, Object×9,
│   │                      QualityMode×4, PlayMode×3, character×14, macros×4 …)
│   ├── kit/               Kit mode: pads, the lock-free pad table, persistence
│   ├── gui_notes.rs       Editor notes to the audio thread (lock-free ring)
│   ├── voice/             Voice + VoiceManager (16-voice pool, stealing, expression state)
│   ├── dsp/               Resonators, exciters, interaction bus, transforms,
│   │                      profiles, envelopes, post-processing
│   ├── presets/           Versioned schema, JSON migration, user folders;
│   │                      factory/ builds the factory bank and its levels
│   ├── offline/           OfflineRenderer + analyze_post_chain_aliasing
│   ├── randomizer/        Parameter randomization
│   ├── gui/               egui editor (feature-gated)
│   └── bin/
│       ├── render.rs          --suite family|rust|damage|aliasing|all
│       ├── render_presets.rs  Batch-render presets to WAV + manifest (plugin output path)
│       └── seed_presets.rs    Level and write the factory bank (presets::factory)
├── tests/                 no_alloc, automation_stress, preset_roundtrip, preset_loading,
│                          body, drive_dynamics, stereo_width, velocity_*, chain_distinct,
│                          limiter, damage_rattle, bow_exciter, mseg_loop_safety,
│                          plugin_metrics, fixtures/ (golden hashes, legacy presets), daw/
└── benches/               Criterion benchmarks (performance)
```

## Build And Run

Requires stable Rust 1.99 or newer.

```bash
# Test using the repo's default local target from .cargo/config.toml
cargo test --workspace --no-default-features

# Bundle the plugins (VST3 and CLAP) into target/bundled/<platform>/, and check them
scripts/bundle.sh linux            # or: windows (MSVC), macos (universal, ad-hoc signed)
scripts/check_bundle.sh linux

# Windows with the mingw-w64 GNU toolchain (the maintainer's host, no MSVC)
CORROSION_WINDOWS_TARGET=x86_64-pc-windows-gnu scripts/bundle.sh windows

# Offline debug renderer (suites: family, rust, damage, aliasing, all)
cargo run --target x86_64-unknown-linux-gnu --bin render -- --suite all

# Render all presets to stereo WAVs + manifest in output/, through the plugin's
# own output path (voices, drive, post chain, output gain, limiter). Defaults:
# note 60, velocity 127, released at 1 s, 2 s long, 48 kHz. Options: --note,
# --velocity, --note-off, --duration, --sample-rate, --contains, --limit, --out-dir
cargo run --release --target x86_64-unknown-linux-gnu --bin render_presets

# Level and write the 214 factory presets and the drum pads (and src/presets/factory/levels.rs)
cargo run --release --bin seed_presets

# Run the benchmark harness on the supported native no-GUI lane
cargo bench --no-default-features --target x86_64-unknown-linux-gnu --bench performance
```

The web app (`web/`, its own Cargo workspace) builds with `web/scripts/build.sh` into `web/dist`, which needs the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` at the version in `web/Cargo.lock`; `web/scripts/cloudflare-build.sh` installs both when missing (it is Cloudflare's build command). Serve `web/dist` with any static server, for example `python3 -m http.server --directory web/dist`.

The repository's `.cargo/config.toml` defaults unqualified `cargo` commands to `x86_64-pc-windows-gnu` and builds into `../corrosion-target`, so the maintainer's Windows host doesn't need `--target` on every command. On Linux or macOS, pass `--target` per command or set `CARGO_BUILD_TARGET=<your-host-triple>`. `scripts/bundle.sh` passes each platform's target itself and honours `CARGO_TARGET_DIR`. The old `bundle.sh` and `bundle-win.sh` are kept in `scripts/archive/` as wrappers around it. The Linux GUI build needs the X11 and GL development packages listed in `.github/workflows/ci.yml`.

## Validation

If you have the tools installed, the local validation flow is:

```bash
# Full local verification lane that mirrors the Linux CI
./scripts/verify-local.sh

# Or run the major steps manually
cargo fmt --check
cargo clippy --workspace --all-targets --no-default-features -- -D warnings
cargo test --workspace --no-default-features
cargo test --lib --target x86_64-unknown-linux-gnu
cargo test --no-default-features --test no_alloc   # debug build only
cargo run --target x86_64-unknown-linux-gnu --bin render -- --suite family --out-dir /tmp/corrosion-verify-render
python3 scripts/check_wav.py /tmp/corrosion-verify-render/pipe_comparison.wav
scripts/bundle.sh linux release

# Host/plugin validators (pluginval, clap-validator) when installed locally
./scripts/validate-plugins.sh linux
```

**CI** (`.github/workflows/ci.yml`) runs on every push to `dev` and `main`, on pull requests from forks, and by hand:

- **checks** on Linux (formatting, clippy with warnings denied, the workspace tests on musl, the GUI library tests, the real-time safety tests, docs with warnings denied, the benchmarks compiled, the renderer smoke test and its WAV check), Windows with MSVC and macOS (clippy, the workspace and GUI tests, the renderer smoke test and the real-time safety tests);
- **bundle**: VST3 and CLAP built on each platform's own runner (macOS universal and ad-hoc signed), their layout checked, then validated with clap-validator and pluginval;
- **nightly**: after a fully green run on `main`, the `nightly` pre-release is replaced with that commit's six zips. Runs on `dev` and pull requests never publish, and a failing run leaves the previous nightly in place.

DAW smoke tests (`tests/daw/`) remain manual. The aliasing tests assert the whole chain at Render stays under −18 dB (`offline::tests::render_mode_alias_ratio_stays_within_budget`; the Lorenz drive isn't oversampled), that Render beats Eco (`higher_quality_reduces_alias_ratio`), and that the clipper alone, with and without the hard clamp after it, folds a 5 kHz tone at twice the ceiling back at least 40 dB down in Normal and 60 dB in Render (`clipper_alias_bounds_hold`, `clipper_alias_bounds_hold_through_the_hard_clamp`). The factory bank tests check counts, determinism, a quality floor for every preset and drum pad, and each folder's character.

Benchmarks are local and manual: `cargo bench --no-default-features --target x86_64-unknown-linux-gnu --bench performance` measures voice rendering, stochastic excitation, post-processing and offline rendering. Run-to-run noise is ±10–15 %; compare within one run (`scripts/bench_ratios.py`).

## Real-Time Safety

- **No heap allocation** in `process()`, including note-on for every object, quality mode and exciter, kit pads, voice stealing, live MIDI expression, editor notes and `reset()`. Each voice preallocates its worst-case modal bank (`MAX_MODES`) and rebuilds it in place at note-on. Verified by `tests/no_alloc.rs`, which must run in a **debug** build (`cargo test --no-default-features --test no_alloc`): the checking allocator only exists under `debug_assertions`, and a release run passes vacuously. Builds off Windows enable nih-plug's `assert_process_allocs`. `Cargo.toml` leaves it off on Windows (its allocator hook once hung nih-plug's test setup there), so CI turns it on for this test alone (`--features nih-plug/assert_process_allocs`), and the check runs on all three platforms. `the_checker_is_armed` proves the checker is installed, so a run can't pass vacuously.
- **No locks / blocking / I/O / logging** in the audio callback. The kit's pad table is a seqlock read without locking; editor notes arrive through a lock-free ring.
- **Fixed voice array**: 16 `Voice` instances (`MAX_VOICES`), each owning all 16 exciter structs (allocation-free dispatch).
- **Denormal + NaN/Inf guards** at every voice boundary; non-finite peak holds sort as quietest within their stealing tier, so a corrupted voice is reclaimed.
- **Per-buffer parameter setters**: post-chain parameters update once per buffer (not per sample), with audio-rate one-pole smoothers (~20 ms, first-call snap) inside the ladder filter, the Lorenz drive, and the master drive and output gain paths.

## Docs

Active design and implementation references:

- `docs/ARCHITECTURE.md`: as-built architecture, signal chain, and status of claimed work.
- `docs/BACKLOG.md`: open work only (sound and behaviour items, deferred features, testing and release infrastructure), each with an acceptance check.
- `openspec/specs/`: the behaviour contract, one spec per capability; `openspec/changes/archive/` holds every change's proposal, design, measurements and tasks.
- `FABLE-CODE-REVIEW-2026-07-07.md`: the current code review; each fixed finding names its OpenSpec change. The earlier review (2026-05-27) is in `docs/archive/CODE-REVIEW-2026-05-27.md`, superseded.
- `docs/PRD.md`: product requirements (see §0 for current-state reconciliation).
- `docs/detailed-specs/`: DSP design notes (each file carries an implementation-status banner; treat them as design targets, not descriptions of shipped behaviour; see ARCHITECTURE.md §9).

## Roadmap

From `docs/BACKLOG.md`, which has the details and acceptance checks for each item, plus the two larger features planned next.

**Planned features:**

- **Sequencer with per-step locks** (PRD §18): step storage, per-step parameter snapshots, host-transport sync, a sequencer panel and preset persistence.
- **Full MIDI CC implementation**: today only CC1, CC64, CC120, CC121 and CC123 act. Planned: map any CC to any parameter (MIDI learn), with the mapping saved in presets and projects.

**Sound and behaviour:**

- **Release Snap** control for the friction exciters, to bound the snap when a bowed, scraped or dragged note is released.
- **Env Amount at 0** silences the friction exciters (its help now says so): decide whether it should blend toward a flat envelope instead.
- **Gesture Modifiers** act only on the friction exciters' envelope, but the panel shows for every exciter.
- **The default patch's decay**: a struck Pipe, I-Beam or Taut Cable rings at the voice's safety clamp for 2–3 s; decide by ear whether it should decay sooner.
- **MSEG PingPong** loops between the chosen loop stages instead of always Decay → Sustain.

**Deferred features:**

- **MSEG as a routable modulation source**, with a mod matrix (PRD §12).
- **Effect mode** (PRD §9.4): audio input excites the resonator; needs a stereo input and feedback safety.
- **Output level meter** in the header (today only the diode stage's clip indicator).

**Testing and release infrastructure:**

- DAW smoke automation across REAPER / Bitwig / Ardour / Live / FL.
- Visual regression tests for the GUI (screenshot comparison).
- Persisted Criterion benchmark baselines.

## Community & Discussions

GitHub Discussions are open for anything that isn't a concrete reproducible bug. Use them before opening an Issue if your topic is in any of these buckets:

- **Questions**: usage, build setup, signal-flow understanding.
- **Ideas**: feature proposals, preset concepts, parameter-tuning suggestions.
- **DSP review**: algorithmic critique, modal-synthesis theory, oversampling/aliasing trade-offs.
- **Architecture discussion**: module boundaries, voice/manager design, post-chain wiring.

Issues are reserved for **confirmed, reproducible bugs** (steps to reproduce + expected vs. actual). Anything labeled `question`, `idea`, `discussion`, `DSP review`, or `parameter tuning` is more likely to find traction in Discussions than in the issue tracker.

When you post, please distinguish between:

- **Confirmed bugs**: steps to reproduce, host/version, expected vs. actual output.
- **Subjective sound-design feedback**: "this preset sounds harsh" / "I'd like more warmth here".
- **DSP theory concerns**: references to specs, papers, or competitive plugins where they apply.
- **Implementation cleanup**: refactor or readability proposals, ideally with a sketch of the change.
- **Experimental ideas**: speculative directions worth chewing over before anyone commits to building them.

## License

Copyright (C) 2026 The Corrosion Synthesizer Authors.

Corrosion is free software, released under the [GNU General Public License v3](LICENSE) or (at your option) any later version. It comes with no warranty; see the license for details.

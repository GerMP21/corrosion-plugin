# Corrosion — Plugin Code Review (2026-11-14)

**Date:** 2026-11-14
**Scope:** the plugin only. That is the Rust crate under `src/`: entry point and host glue (`lib.rs`), parameters, the voice engine and manager, the resonators, interaction bus and envelopes, the exciters, the post chain, presets and import/export, kits, the GUI note queue and file dialog, and the randomizer. The web app, the site and the Wiki (`web/`) are **out of scope**, at the maintainer's request.
**Method:**
- A read of the audio path, line by line where it runs per sample. The GUI, offline and tool paths were read by target.
- Every correctness finding was checked against the callers, guards and tests that reach it.
- Each suspected bug was then run as a throwaway integration test (not committed). Their numbers are quoted below.
- Two suspicions were **ruled out** that way; they are listed under FYI so nobody re-opens them.

**Baseline:** `dev` at `c20393e`. Every finding is a defect that already exists in the base. None is a regression from the last changes (`add-preset-import-export`, `refine-wiki-naming-and-clips`, the `nih_log` patch).

**Previous review:** `FABLE-CODE-REVIEW-2026-07-07.md`. All of its findings are closed. This review does not repeat them.

**Severity labels** (as in `agent-skills:code-review-and-quality`):

| Label | Meaning |
|---|---|
| **Critical** | Broken functionality; fix first |
| **Required** | Should be fixed |
| **Optional** | Worth doing |
| **Nit** | Minor |
| **FYI** | Context only, no action |

Every finding that changes how the plugin sounds or behaves must update the Wiki in English and Spanish in the same change (CLAUDE.md).

---

## Summary

| ID | Severity | Area | Finding |
|---|---|---|---|
| C1 | Critical | Voice / ADSR | Specialty exciters (Pneumatic Jet, Electromagnetic Hum, Particle Rain) never stop if released before the decay ends with Sustain 0 |
| R1 | Required | Post chain / Space | Switching Space mode away and back replays a stale Factory tail, and the switch itself cuts the wet signal instantly |
| R2 | Required | Post chain / Space | Space Amount is not smoothed: it steps once per host block |
| R3 | Required | HRTF spread | Width is discontinuous at 0: a 100 Hz mono tone drops 3.7 dB between Width 0 and 0.001 |
| R4 | Required | Sample-rate rule | The HRTF proximity filters and the spring's dispersion filter use raw per-sample constants, so they change with the host rate |
| R5 | Required | Corrugated Drag | Ridge position accumulates in `f32` without wrapping, so long held notes lose ridge-rate precision |
| R6 | Required (decide) | Envelopes / Drone | Drone mode's forced Decay→Sustain loop restarts Decay at peak, a periodic step in the force envelope |
| R7 | Required | Kits / input | A pad's `level_db` from a file is unclamped (+800 dB gives an infinite gain) |
| R8 | Required (decide) | Lorenz drive | The tube saturator is non-monotonic (louder in gives quieter out above 1.5) |
| O1–O10 | Optional | Performance and architecture | Per-block recomputation, per-sample divisions and `tan`/`sin`, delay-time zipper, giant files, mono/stereo duplication, enum mapping |
| N1–N10 | Nit | Tidiness | Dead match, misleading state, test-only public API, stale comments, empty URL/EMAIL, `expect` on data |
| F1–F7 | FYI | Context | Seqlock caveat, input-independent chaos, filter not transparent, limiter window, ruled-out suspicions |

**Verdict:** the engine is in good health. It has real-time safety tests, bit-identity discipline, migrations and wide test coverage. C1 is the one real bug: a stuck note that a user can reach with plain parameter values. R1–R3 and R5 are audible quality issues. R6 and R8 need a decision by ear before code changes. The Optional items are cleanups with measurable but small gains.

---

## Critical

### C1. Specialty ADSR stalls when Sustain is 0 and the note is released before the decay ends

`src/voice/mod.rs:931-951` (the `ExciterFamily::Specialty` arm of `process_force_envelope`)

```rust
AdsrPhase::Release => {
    let release_rate = self.adsr_sustain / self.adsr_release.max(dt);
    self.adsr_level = (self.adsr_level - release_rate * dt).max(0.0);
```

The release slope comes from the **sustain level**, not from the level at note-off.
- With **Env Sustain = 0** (its range is 0–1), `release_rate` is 0. A note released during Attack or Decay then holds its level forever.
- The voice keeps driving the exciter and never deactivates. It occupies a voice slot until it is stolen or a panic arrives.
- With Sustain > 0, releasing from above the sustain level takes longer than the set Release, by a factor of level ÷ sustain. Releasing from 1.0 with Sustain 0.1 takes 10 times Env Release.

**Measured** (throwaway test): Pipe, note 60, attack 0.05 s, decay 0.3 s, release 0.5 s. The figures are the voice-manager output RMS between 19 and 20 s after note-off.

| Exciter | Sustain | Hold | RMS at 19–20 s | Voices still sounding |
|---|---|---|---|---|
| 13 Pneumatic Jet | 0 | 0.1 s (in Decay) | **0.0730** | **1** |
| 14 Electromagnetic Hum | 0 | 0.1 s | **0.0883** | **1** |
| 16 Particle Rain | 0 | 0.1 s | **0.0712** | **1** |
| 13 / 14 / 16 | 0 | 1 s (decay done) | 0 | 0 |
| 13 / 14 / 16 | 0.7 | 0.1 s | 0 | 0 |
| 13 / 14 / 16 | 0.05 | 0.02 s | 0 | 0 |

**Fix:**
- Do what the MSEG already does (`release_level`, `src/dsp/envelopes/mod.rs:237`): store the level at note-off and ramp from it, `release_rate = release_from / release`.
- That also makes Env Release mean "time to silence" from wherever the note was.

**Impact on presets:**
- Every Specialty preset whose notes are released above their sustain level will release faster than now.
- So run the bank tests and look at `curated_measurements_table`.
- Add a regression test from the table above: Sustain 0, released in Decay, the voice ends within Release plus a margin.
- Update the Wiki's ADSR section and Troubleshooting ("a note keeps sounding after I let go"), in English and Spanish.

---

## Required

### R1. Switching Space mode replays a stale tail and cuts the wet signal

`src/dsp/post_processing/post_chain.rs:224-237`, `243-257`

Only the selected Space processor runs (`process_space`). The others keep their buffers frozen.
- **Switching back** to a mode resumes whatever tail it held when you left it, even minutes later.
- **Switching away** cuts the wet signal in one sample, a click on a ringing tail.

**Measured:**
- Factory, Space Amount 1, a 50 ms burst, then 50 ms of silence.
- Switch to Off for 1 s, then back to Factory with silent input.
- The first 100 ms read an RMS of **0.053** (about −25 dBFS) that should be 0.
- Echo and Spring keep their buffers the same way. Echo replays its stale repeats once the read head reaches them.

**Fix:**
- On a mode change, clear the processors that just became inactive (`fill(0.0)` is allocation-free).
- Better: crossfade the old and new wet signals over 10–20 ms, then clear the old one.
- Add a test: "a mode re-entered after silence starts silent".

### R2. Space Amount steps once per block

`src/lib.rs:1094-1095` → `src/dsp/post_processing/post_chain.rs:160`, applied at `post_chain.rs:228-233`

Parameters reach the post chain once per host block.
- BODY, master Drive, Output, the filter, Drive Amount, Diode Softness/Asymmetry and Machinery Movement each have a per-sample smoother.
- **Space Amount has none.** Automating it zippers in steps of one block (about 10 ms at 512 samples).

**Fix:** a 20 ms one-pole smoother in `PostProcessingChain`, primed on the first set, like `body_amount`. The output is identical when the amount is static.

### R3. Width is discontinuous at 0

`src/dsp/post_processing/hrtf_spread.rs:59-61`

At `spread_width < 0.001` the stage is bypassed. Above it, the mid goes through the proximity low-pass/high-pass mix, whose low-frequency gain is `0.5 + 0.3·proximity` (0.65 at the default proximity).

**Measured:** a 100 Hz mono sine at 0.5 peak, proximity 0.5.

| Width | Output RMS |
|---|---|
| 0 | 0.3536 |
| 0.001 | 0.2297 (−3.7 dB) |
| 0.01 | 0.2297 |

So sweeping Width up from 0 drops the level and changes the tone at once. Re-entering from bypass also starts from stale filter and delay state.

**Fix:** crossfade the bypassed and processed paths over a short Width range (for example 0–0.05), or always run the filters so their state stays current. It changes the sound near 0, so update the Wiki's Width text.

### R4. Per-sample constants that ignore the sample rate

The rule in CLAUDE.md is that per-sample constants are tuned at 48 kHz, converted with `dsp::utils::rate`, and exactly their 48 kHz value at 48 kHz. Two stages break it:
- `src/dsp/post_processing/hrtf_spread.rs:88-93`: the proximity one-poles use `0.8 / 0.2` and `0.5 / 0.5`. At 96 kHz their corners move up an octave, so Listener Proximity sounds different at 44.1, 48 and 96 kHz.
- `src/dsp/post_processing/space.rs:253-255`: the spring's dispersion one-pole, `dispersion_coeff` (0.5–0.9 per sample), is not rate-converted either.

**Fix:** derive the coefficients with `rate::smoother` (or from a cutoff in hertz, as the echo's damping does). Prove they are bit-identical at 48 kHz with a test. Add each stage to the rate-invariance tests that H10 added.

### R5. Corrugated Drag's ridge position grows without bound in `f32`

`src/dsp/exciters/corrugated_drag.rs:68-71`

`self.position += v_rel · step` is never wrapped, and the bump is `sin(2π·position / ridge_spacing)`. The arithmetic, at the default drag speed (0.5, scaled by velocity) and 48 kHz:
- `position` grows about 240 per second.
- After 60 s it is about 14 400, where the `f32` spacing is 0.00098 against a per-sample step of about 0.005. That is a rounding error of up to 10–20 % of each step.
- After 2 minutes, around 28 800, the spacing (0.002) is 40 % of the step.

So the ridge rate, which sets the pitch of the corrugation, drifts and quantizes on long Drone notes. The bow exciter wraps its position (`bow.rs:122`); this one doesn't.

**Fix:** wrap `position` by `ridge_spacing` (the bump is periodic in it), or keep the phase in `[0, 1)`. It changes rounding from the first sample, so check the render hashes and the bank tests. Expect presets to move by noise only.

### R6. Drone mode's forced loop steps the force envelope (decide by ear)

`src/lib.rs:716-720` forces `Forward` looping from Decay (3) to Sustain (4) for friction exciters in Drone mode. `src/dsp/envelopes/mod.rs:309` starts Decay at `l_peak`.
- When Decay finishes, the loop jumps back to Decay, so the level **steps from the sustain level to the peak**.
- That repeats every MSEG Decay. At the defaults that is 0.5 → 0.75 every 0.3 s.
- Without the loop, Sustain already holds for as long as the gate does. So the loop doesn't make friction voices "ring indefinitely", as its comment says. It turns the envelope into a hard-edged sawtooth tremolo.

**Decision:** if the pulsing is wanted, start the looped Decay from the current level (a smooth sawtooth) or ramp the step over a few milliseconds. If it isn't, drop the forced loop.

Either way it changes Drone-mode sound and presets, so it needs a listen first, the bank tests, and the Wiki's Drone mode and Troubleshooting pages.

### R7. Kit pad level is unclamped from files

`src/kit/mod.rs:197`: `gain: 10f32.powf(self.level_db / 20.0)`

`tune` is clamped to ±48, but `level_db` comes straight from a preset file or a saved project. The editor has no control for it, so only a file sets it.
- A pad with `"level_db": 800` builds with `gain = inf` (measured).
- In the throwaway test the downstream clamps kept the output finite, but nothing guarantees it.
- A hand-edited or corrupted file should not be able to ask for infinite gain.

**Fix:** clamp `level_db` on load, for example to −60…+72 dB (the factory pads use −40…+56.7 dB). Cover it in `kit/tests.rs`.

### R8. The Lorenz drive's saturator is non-monotonic (decide by ear)

`src/dsp/post_processing/lorenz_drive.rs:193-201`

```rust
sign * (0.5 + (abs_x - 0.5) / (1.0 + (abs_x - 0.5).powi(2)))
```

`u/(1+u²)` peaks at `u = 1`.
- The output reaches 1.0 at `|x| = 1.5`, then **falls**: 0.85 at 3 and 0.71 at the clamp of 5.
- The input reaches that region easily. `driven = input·(1 + 0.5·amount)` is ×3.5 at Drive Amount 5, and the chaos can nearly double it again.
- So hot peaks fold down: louder in, quieter out. That foldback adds upper harmonics, and the drive runs without oversampling, so they alias.

**Decision:** keep it as an intended foldback and document it (ARCHITECTURE and the Wiki's Drive page), or make the curve monotonic above the knee.

A monotonic curve changes the sound of every preset with a high Drive Amount, so it needs the bank tests and a reseed.

---

## Optional

### O1. The post chain is reconfigured from scratch every block

`src/lib.rs:1052` calls `post_chain.set_sample_rate` on every block. Each call:
- recomputes the `exp()` smoothing coefficients of every stage;
- recomputes the FEM body's mode tables;
- rescales the reverb's comb and allpass delays;
- calls the filter's `update_coefficients` (`tan`).

Then every `set_*_params` call runs, even with nothing changed. The cost is small, but it is pure waste.

**Fix:** call `set_sample_rate` only when the rate changes (in `prepare`, or guarded by a cached value).

### O2. FEM body: per-sample divisions and work at BODY 0

`src/dsp/post_processing/fem_body.rs:94-115` computes these every sample, for each of its 8 modes:
- `omega_norm = 2π·f0 / sample_rate` and its square (a division);
- `material_gain`;
- `1 / (i + 1)`.

The body also runs while BODY is 0 (`post_chain.rs:218`).

**Fix:**
- Precompute the per-mode coefficients in `update_mode_frequencies` / `update_decay`.
- Skip the body while the smoothed amount and its target are both 0, clearing its state when it goes idle so it restarts clean.
- Prove the output is bit-identical when BODY > 0.

### O3. The filter calls `tan()` every sample while Component Tolerance is above 0

`src/dsp/post_processing/wdf_filter.rs` (`process` → `update_coefficients`): with Component Tolerance on, `drifting` is true, so `tan()` runs every sample on both channels. The drift has a 0.3 s time constant.

**Fix:** update the drifted `g` every 16–32 samples, or use a polynomial approximation of `tan` for small changes.

### O4. Strike-position wander recomputes every mode's `sin` per sample

`src/dsp/interaction.rs:95-102` and the coefficient refresh that follows. When wander is above 0, every mode's position coefficient (`sin(nπp)`) is recomputed every sample.

**Fix:** update it at a control rate (every 32 samples, interpolated), or use a recurrence for `sin(nθ)`.

### O5. Delay-length controls jump their read heads

The echo already interpolates (Hermite) for modulation, so smoothing the delay is cheap.
- **Echo Delay Time** (`space.rs:402`): unsmoothed, so automating it or changing the tempo-synced division clicks.
- **Factory Size** (`space.rs:108-120`) and **Spring Tank Size** (`space.rs:231-236`): they change their delay lengths with `index %= delay`.

**Fix:** for Delay Time, smooth `base_delay` with the movement smoother's time constant (tape-like glide). For the two Size controls, at least ramp or crossfade.

### O6. Very large source files

| File | Lines |
|---|---|
| `src/gui/editor.rs` | 6 931 |
| `src/voice/mod.rs` | 4 555 |
| `src/lib.rs` | 4 492 (about 2 950 of them tests) |
| `src/params.rs` | 3 159 |
| `src/dsp/resonators/core.rs` | 2 678 |

`kit/` already uses a sibling `tests.rs`.

**Fix:**
- Move the test modules of `lib.rs`, `voice/mod.rs` and `core.rs` to sibling files.
- Split `editor.rs` by panel: browser, kit bar, keyboard, randomizer, import/export.
- Split `voice/mod.rs` into envelope, dispatch and expression.

Mechanical moves; do them in a change of their own, separate from behaviour changes.

### O7. Mono and stereo render paths are duplicated

- `src/voice/mod.rs:1832` (`process_sample`) and `:1908` (`process_sample_stereo`) repeat the same pipeline.
- `src/dsp/resonators/core.rs` duplicates its mono and stereo mode loops.

A fix applied to one path and not the other is the classic drift risk. Some of these duplicates exist only for tests or the offline tools.

**Fix:** one inner function generic over the output (a const generic or a small trait). Keep it `#[inline(always)]` in the hot loop, and prove the output is bit-identical with render hashes.

### O8. Integer-to-enum mapping inline in `apply_post_chain_params`

`src/lib.rs:1077-1092` maps `space_mode` and `quality_mode` with literal `match` arms (0..3). The same mapping exists for other enums elsewhere.

**Fix:** `From<i32>` (or `from_int`) on `SpaceMode` and `PostQualityMode`, beside the enums, as `Object::from_int` already does.

### O9. Changing Quality clears the clipper's filters

`src/dsp/post_processing/oversampled_clipper.rs` (`set_oversample_factor`) zeroes the half-band states when the factor changes. Switching quality while audio plays gives a short discontinuity.

**Fix:** crossfade 2–5 ms, or accept it and say so in the Quality tooltip.

### O10. The Factory reverb is mono

`src/dsp/post_processing/post_chain.rs:247-249` sums the input to mono and returns `(wet, wet)`. The Spring runs one tank per channel and the Echo is stereo.

**Fix:** if a wider room is wanted, offset the right channel's comb delays (a few samples, mutually prime). It changes the sound, so do it only by choice.

---

## Nit

- **N1.** `src/dsp/resonators/core.rs:1753`: every arm of `match self.profile` returns `mode_sum`, so the `match` is dead. Replace it with `mode_sum`.
- **N2.** `src/dsp/resonators/core.rs:1170` (`rebuild_in_place`) writes defaults into `size_scale`, `rust_amount` and their siblings rather than the values the bank was built with. The state then reads wrong in a debugger or a test. Store the real values, or drop the fields if nothing reads them.
- **N3.** `src/dsp/interaction.rs:109`: `calculate_mode_coefficients` is public, allocates a `Vec` and is used only by tests. Make it `#[cfg(test)]` or move it to the tests. Also drop the "Removed 2026-05-28 (P3.1)" comments (`:20`, `:125`, `:216`); git has the history.
- **N4.** `src/voice/mod.rs:1891`: `process_exciter_stereo(_width)` is a pass-through wrapper; inline it. At `:1944`, the "never fires today" comment is stale.
- **N5.** `src/voice/mod.rs:1203`, `:1213`, `:1224`: `set_sample_rate(self.sample_rate)` on the wire brush, metal pipe and metal chain at every note-on duplicates what `set_rate` already did.
- **N6.** `src/voice/mod.rs:940-944`: the ADSR's `Sustain` arm checks `!self.active` to enter Release, but `release_force_envelope` already moves the phase to Release on note-off. The branch is unreachable. Remove it with the C1 fix.
- **N7.** `src/dsp/envelopes/mod.rs:10-11`: `#[derive]` sits between the doc comments, so the struct's doc is split. Also, `process_sample` divides `1.0 / sample_rate` every sample, and the voice calls `mseg.set_sample_rate` every sample (`voice/mod.rs:957`). Cache `dt` when the rate changes.
- **N8.** `src/lib.rs:1385-1386`: `URL` and `EMAIL` are empty, so hosts show blank vendor links. Set `URL` to `https://corrosion-vst.org/`.
- **N9.** `src/kit/mod.rs` (`KitPad::params`): `.expect("pad values parse as a preset")` turns a bad value in data into a panic. It runs in `initialize` and on the GUI thread. Return a `Result` and skip the pad with a log line.
- **N10.** Several stages have silent safety clamps that shape loud signals:
  - the HRTF spread at ±1.5 (`hrtf_spread.rs:109`);
  - the body at ±10 per mode and ±1 overall;
  - the drive at ±1, the filter's saturators.

  These are fine as guards, but document them in ARCHITECTURE so nobody mistakes them for the diode stage.
- **N11.** `src/presets/mod.rs:1122`: Import reads any picked file whole (`read_to_string`) before it checks it. Choosing a multi-gigabyte file by mistake would stall the editor. Check `metadata().len()` against a limit (for example 1 MB) first.

---

## FYI

- **F1.** `src/kit/mod.rs` (`KitTable::get`): the seqlock reads the slot with `read_volatile` while a writer may be writing. Under the Rust/C++ memory model that is formally a data race, even though the result is discarded. It is the standard seqlock caveat, and `MaybeUninit` prevents any invalid-value assumption. In practice it is fine on every target CI builds. A strictly sound alternative is a double buffer per slot with an atomic index.
- **F2.** `src/dsp/post_processing/lorenz_drive.rs`: the attractor doesn't depend on the input, and both channels start at `x = 0.1`. So Chaos Depth is a deterministic chaotic LFO, identical in left and right. It is not signal-dependent and not decorrelated. That is fine if intended; the Wiki should not say otherwise.
- **F3.** `src/dsp/post_processing/wdf_filter.rs`: at its defaults (20 kHz, resonance 0) the filter is not transparent above about 0.7 peak, where the stage saturators start. Resonance also lowers the passband by `1/(1 + 3.5·res)`, −13 dB at 1, even with the cutoff fully open. That is ladder behaviour, worth one line in the Wiki's Filter page if it isn't there.
- **F4.** `src/dsp/post_processing/lookahead_limiter.rs`: the window is 48 samples at every rate: 1 ms at 48 kHz, 0.5 ms at 96 kHz, as documented. The linear scan is fine at that size.
- **F5.** **Ruled out: a click when stealing a voice.** 16 sustained voices, then a 17th note. The largest sample step in the 4 samples after the steal was **0.015**, against **0.045** in the 100 ms before. Stealing the quietest voice keeps it inaudible, so no fade is needed.
- **F6.** **Ruled out: path traversal on Import and Save.** `sanitize_file_name` and `user_subfolder` refuse to leave the user folder, with tests (`../../etc/passwd` → `_.._etc_passwd`). Typed parsing plus `into_params` clamps every parameter of an imported or restored preset. Kit pad values go through the same loader; only `level_db` (R7) skips it.
- **F7.** `src/gui/file_dialog.rs`: the dialog thread is detached and answers through a one-slot channel. A dialog left open when the editor closes just finishes into a dropped receiver. macOS runs modal on the main thread, as AppKit requires. Nothing reaches the audio thread.

---

## Security (plugin)

- **Network:** none. Nothing listens or connects.
- **File I/O:** only on the GUI thread or in `initialize`: presets, import/export, Open folder. Paths come from the system dialog or from sanitized names.
- **Untrusted input** (preset files, project state): parsed by `serde_json`, migrated, then rebuilt through the parameter constructors, which clamp. The exceptions are R7 (`level_db`), N9 (`expect`) and N11 (unbounded read).
- **`unsafe`:** the kit seqlock (F1), and parameter pointers taken from the plugin's own `param_map` (editor and kit), which outlive their users. No other `unsafe` was found in the audio path.
- **Dependencies:** `rfd` 0.18 (GUI only) and the vendored `nih_log` patch, which replaces `atty` (Dependabot alerts #1 and #2). No new findings.

---

## Suggested order

1. **C1.** A small, contained fix with a regression test; then reseed if Specialty presets move.
2. **R1, R2, R3.** All in the post chain; one change, `smooth-space-and-width`, with tests per scenario.
3. **R4 and R5.** Bit-identical at 48 kHz for R4; noise-level preset movement for R5.
4. **R7, N9, N11.** Input hardening, together.
5. **R6 and R8.** Only after the maintainer listens and decides.
6. **O1–O5.** Performance and zipper fixes, measured with `scripts/bench_ratios.py`, medians of 5.
7. **O6–O8 and the Nits.** Mechanical tidy-up, in their own change, render hashes unchanged.

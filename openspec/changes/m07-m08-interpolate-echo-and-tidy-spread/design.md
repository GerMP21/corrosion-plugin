# Design

## Context

- `FactoryEcho::process_stereo` computes `delay_samples = (base_delay · (1 + modulation)).round()` and reads one sample. The modulation is `sin(2π·phase) · movement · 0.1` with a 0.05–2.05 Hz rate. Wet = read · `(1 − 0.6·hf_damping)`; the buffer is written with `input + wet · 0.45`.
- `space-effects` already requires the echo's first repeat to land within one sample of 0.8 s at every rate with no modulation, the Delay Time sweep to be monotonic, and synced divisions to stay on the beat.
- `HrtfSpread::process` writes the mid signal into two buffers and reads both at one index.

## Goals / Non-Goals

**Goals:** zipper-free modulation; a knob that does what its name says; no redundant spread state.

**Non-Goals:**
- Smoothing Delay Time automation (it changes per block, a separate concern).
- Changing the spread's sound or adding a real ITD (the maintainer chose bit-identity).

## Decisions

1. **4-point cubic Hermite interpolation for the echo read.** At a fractional position of exactly 0 it returns the stored sample, so unmodulated delays stay bit-exact and the existing timing scenarios hold. *Alternative:* linear interpolation. Cheaper, but it low-passes the repeats by an amount that moves with the fraction, which modulation turns into a faint flutter.
2. **HF Damping is a one-pole low-pass inside the feedback loop,** cutoff mapped exponentially from 20 kHz at 0 (filter bypassed exactly at 0) to 1.5 kHz at 1. The coefficient comes from the cutoff in hertz, so it is sample-rate independent (the sample-rate rule). Inside the loop, each repeat passes the filter once more, which is how tape and bucket-brigade echoes darken.
3. **Fixed wet level 0.7,** the old gain at the default 0.5. At the default the repeats' low end is unchanged, and the knob no longer moves the level at all. *Alternative:* keep the old gain law and add the filter. Then one knob would set two things, the problem M1 describes elsewhere.
4. **Spread: delete `delay_buffer_right`** and read the mid delay once; keep the arithmetic in the same order so the output is bit-identical. Proven with a one-off render hash against a clean worktree (hashes aren't portable across libm, so it can't be a CI assertion).

## Risks / Trade-offs

- **[Sessions using Echo sound different]** → Expected and listed in the proposal; there is no parameter change to migrate.
- **[A low-pass in the loop lowers the loop's high-frequency gain only]** → The loop gain at DC is 0.45 · 0.7, as today at the default, so the echo can't run away.
- **[Hermite interpolation overshoots slightly on transients]** → The wet path is bounded by the same feedback gain; the clipper stage follows.

## Migration Plan

None: parameter IDs, ranges and stored values are unchanged.

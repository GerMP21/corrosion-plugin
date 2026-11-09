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

## As built

- **Unmodulated reads stay whole samples:** with Machinery Movement 0 the delay is rounded as before, so repeats are single samples at the configured time (`unmodulated_echo_is_a_single_sample`). With movement above 0 the read is fractional (Hermite, `t = 0` returns the stored sample). The delay is clamped to at least 2 samples so the interpolator's newest point is always written.
- **Damping filter:** coefficient `1 − exp(−2π·fc/fs)`, re-derived in `set_parameters` and `set_sample_rate`; exactly bypassed (coefficient 1) at 0, where the old gain law and the new one differ only by the fixed wet level.
- **Spread:** `delay_buffer_left`/`delay_buffer_right` became one `delay_buffer`; both channels' proximity filters read the same delayed mid, as before.

## Migration Plan

None: parameter IDs, ranges and stored values are unchanged.

## Measurements

### Tests on the code before the change (task 1.1)

- **Glide:** fails. The largest step of the wet signal (1 kHz sine at amplitude 0.5, Machinery Movement 1, Delay Time 0.25) is 0.242, against 0.0655 for the sine itself (3.7 times).
- **HF Damping darkens, each repeat darker:** both fail. The old law only scales level, so the 8 kHz / 200 Hz ratio is 1.0 at every setting and repeat.
- **Unmodulated single sample, and same tone at every rate:** pass. The second passes trivially because the old damping wasn't a filter; it guards the new one.

### After the change (tasks 2.1–2.2)

| Measure | Value | Bound |
|---|---|---|
| Glide: wet step / sine step | 0.0765 / 0.0655 = 1.17 | ≤ 1.2 |
| 200 Hz first repeat at HF Damping 0 / 0.5 / 1 | 0.4950 / 0.4948 / 0.4910 (−0.07 dB at 1) | within 0.5 dB |
| 8 kHz first repeat at 0 / 0.5 / 1 | 0.4950 / 0.2925 / 0.0955 | at 1, ≥ 12 dB below 200 Hz: −14.2 dB |
| 8 kHz / 200 Hz, repeats 1–3 at damping 1 | 0.195, 0.038, 0.0081 | decreasing |
| 8 kHz first repeat vs 48 kHz | +0.07 dB (44.1 kHz), −0.30 dB (96 kHz) | within 1 dB |

The glide margin is small (1.17 against 1.2). The rest of the wet signal's steps come from Doppler shift (the sweep moves the delay by up to ±10 %) and from the feedback copies adding up, not from interpolation.

### Spread (task 3.1)

Hashes of 48 000 samples of stereo noise (Width 1, proximity 0.7) through `HrtfSpread`, on the base commit (`32ff176`, clean worktree) and after the change: 44.1 kHz `9f0d356e2027f7b3`, 48 kHz `294286414e810dfc`, 192 kHz `04ae045bb796a881`, identical on both.

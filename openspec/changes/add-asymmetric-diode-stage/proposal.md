# Proposal

## Why

After `m06-m09-smooth-starvation-and-oversample-clipper`, the clipper starts 2.3 dB under the Analog Ceiling when oversampling, so a hot patch gets a little peak compression. The maintainer liked that and wants it to be a feature that sounds good (2026-11-09). Today it can't be: the diode curve is a hard knee (its slope drops from 1 to 0.026 at the threshold, so it adds harsh odd harmonics), Diode Softness moves the output by less than 0.01 across its whole range, the curve is symmetric, the character differs between Eco and the other quality modes, and the brickwall panel's red dot is decorative.

## What Changes

- **A diode-like soft knee:** Diode Softness sets the knee's width, from a hard knee at 0 to a knee that starts 14 dB under the curve's limit at 1 (4.4 dB under at the default 0.5). The curve is smooth (no slope jump), so it adds fewer high harmonics than today's.
- **New parameter Diode Asymmetry** (`diode_asymmetry`, 0–1): the negative half conducts earlier and more firmly than the positive one, as with one diode against two in series. At 1 the negative limit is 6 dB under the positive one. This adds even harmonics. Default 0.25 (negative limit 1.5 dB lower).
- **No DC:** a 2 Hz DC blocker follows the curve while Diode Asymmetry is above 0, and peaks stay under the Analog Ceiling after it.
- **The same character in every quality mode:** Eco runs the same curve and headroom as Normal, High and Render; quality only changes how much the clipping aliases. The headroom (today ÷ 1.3) is re-measured for the new curve.
- **The red dot shows the diode working:** off under 0.1 dB of reduction, brightening to full at 6 dB, with an instant attack and about 300 ms of fall. It's the plugin's first audio-to-editor meter.
- **Migration:** user presets and saved projects from before this change get Diode Softness 0 and Diode Asymmetry 0, the setting closest to the current sound, on both the preset loader and `filter_state`. New instances and the regenerated factory presets use the new defaults.
- **BREAKING:** existing host automation of Diode Softness follows the new meaning (the same position now sets a different knee). Automation lanes can't be migrated.
- Documented as a feature in the README.

## Capabilities

### New Capabilities
- `diode-stage`: the clipper's curve as a sound-shaping control: knee width (Diode Softness), polarity asymmetry (Diode Asymmetry), DC removal, the same character in every quality mode, the red dot's activity indicator, and how older state maps onto the new controls.

### Modified Capabilities
- `output-clipper`: the threshold is the same in every quality mode, and Eco is no longer the bare curve on each sample (it applies the curve's headroom, and the DC blocker when asymmetric). This capability was created by `m06-m09-smooth-starvation-and-oversample-clipper` (archived 2026-11-09).

## Impact

- **Order:** after `m01-m04-m05-clarify-parameters` (which settles parameter display names) and before `add-parameter-tooltips` (so its texts describe the final controls). `m06-m09-smooth-starvation-and-oversample-clipper` is archived (2026-11-09), so `openspec/specs/output-clipper/` exists for the MODIFIED delta.
- **Code:** `src/dsp/post_processing/oversampled_clipper.rs` (curve, asymmetry, smoothing, DC blocker, reduction meter), `src/dsp/post_processing/post_chain.rs` (parameters, meter read-out), `src/params.rs` (new parameter), `src/lib.rs` (wiring, the shared meter, `filter_state` migration), `src/presets/mod.rs` (field, `migrate_preset_json`), `src/gui/editor.rs` (Asymmetry fader, the dot), the randomizer (range for the new control), `src/bin/seed_presets.rs` (regenerate the factory bank).
- **Sound:** every patch that reaches the clipper changes: softer knee, even harmonics by default, the same in Eco. Migrated presets and projects stay close to the current sound (hard knee, symmetric).
- **CPU:** the curve runs at up to 16× per channel; measured with `plugin_output_path_sixteen_voices` and its Render twin.
- **Docs:** README (feature description, controls), `docs/ARCHITECTURE.md` (output protection, quality modes).

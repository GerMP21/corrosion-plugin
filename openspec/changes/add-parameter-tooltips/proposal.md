# Proposal

## Why

The editor shows about 140 controls with short labels ("Rosin Grip", "Slip Curve", "Rust", "Movement"), and many exciter and object slots mean something different for each model. Without the source code or the docs, a user can't tell what a control does or which way to turn it. A small ⓘ badge with a hover explanation answers that inside the plugin.

## What Changes

- A small ⓘ badge after the label of every parameter control: knobs, faders and sliders. The badge is a 1 px circle with an "i", drawn in the muted grey of the label text. Hovering the badge, and only the badge, shows a tooltip of one or two sentences: what the control does and what turning it up sounds like.
- Exciter and object slot controls take their help text from the model that is selected, so the tooltip changes with the exciter or object (for example, slot 1 is "Hand Mass" for Hand Strike and "Bow Pressure" for Bow).
- **Trial feature:** selectors (Exciter, Object, Space mode, Play Mode, Limiter Mode, Randomizer mode and the other combos) get a ⓘ badge too, explaining the selected option. It's marked as a trial in the code and docs so it can be removed later without touching the parameter tooltips.
- A coverage test makes sure every control the editor draws has non-empty help text, for every exciter, object and Space mode.
- No DSP, parameter, preset or host-state change. The sound is untouched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `editor-controls`: adds the requirement that each control has an inline help badge with a hover tooltip, that slot help follows the selected model, that selectors show trial help, and that the help covers every drawn control.

## Impact

- `src/gui/editor.rs`: a badge helper drawn with the painter, because egui's default fonts have no ⓘ (U+24D8) glyph. Also a `help` field on `ExciterControlSpec` and `ResonatorControlSpec` (and on the panel specs for the selectors), a help table for the global parameters, the widget helpers taking the help text, and tests.
- Layout: the badge adds width to labels that are already tight at the 75 % default ("Restitution Bounciness", "Amplitude Randomization"). `editor_content_fits_the_window_width` must keep passing.
- Docs: `docs/ARCHITECTURE.md` (editor section) and the README (user-visible).
- Ordering: comes after `m01-m04-m05-clarify-parameters`, so the help texts are written once, against the final parameter names (Onset Level, unique names, Output in dB).

# Design

## Context

- The editor (`src/gui/editor.rs`) draws parameter controls through a few helpers: `industrial_knob` / `industrial_knob_text`, `industrial_fader` / `industrial_fader_text`, `industrial_vfader`, `vfader_global` and `industrial_combo`. Each helper paints its own label.
- Exciter and object panels are built from the spec tables `ExciterControlSpec` (61 entries) and `ResonatorControlSpec` (30 entries). Each entry has a `label`, and the same parameter slot means different things for different models. The panel specs already carry a `description`.
- Global controls (Filter, Drive, material transforms, Space, Output, macros, envelope, …) are drawn by direct calls with literal labels.
- egui's default fonts (Ubuntu-Light, Hack, NotoEmoji, emoji-icon-font) were checked: none has ⓘ (U+24D8). Only NotoEmoji has ℹ (U+2139), which has no circle. A text label "ⓘ" would render as a missing-glyph box.
- Tests already render the real editor headlessly for every exciter, object and Space mode, and collect drawn parameters (`DRAWN_PARAMS`, `all_drawn_parameter_ids`).

## Goals / Non-Goals

**Goals:**
- One badge style, painted rather than taken from a font, that looks the same on every host and font setup.
- Help text kept next to the label it explains, so a renamed or added control can't silently lose its help.
- A test that fails when a drawn control has no help.
- Selector help isolated, so removing the trial is a small deletion.

**Non-Goals:**
- Localisation. Help is English, like the rest of the UI.
- Rich tooltips (images, value ranges, links).
- Help for non-parameter UI (preset browser buttons, randomizer actions, the scale selector).
- Any change to parameters, DSP, presets or host state.

## Decisions

### 1. Paint the badge with the painter
An `info_badge(ui, scale, help) -> Response` helper allocates a small square (side ≈ the label's text height) with `Sense::hover()`. It paints a circle with a 1 px stroke in `MUTED_GREY` and centres an "i" in the same colour, sized from `scale` like the other label metrics. The stroke stays 1 physical pixel at every scale; only the radius and the glyph scale. The badge doesn't brighten on hover, as the maintainer specified (a 1 px circle and an "i" in `MUTED_GREY`). The tooltip is `response.on_hover_text(help)`, so only the badge's rect triggers it.
- *Alternative:* a U+2139 text label. Rejected: no circle, and it depends on the font fallback.
- *Alternative:* embedding a font that has U+24D8. Rejected: adds binary size and a font dependency for one glyph.

### 2. The badge goes inside each widget helper, right of the label
Each helper paints its label and then the badge immediately to its right, with a small gap, in one horizontal layout. Labels that are centred over knobs are centred together with the badge, so the label doesn't drift sideways. Because the helpers own the badge, there's no call site where a control can appear without one.

### 3. Where the help text lives
- **Slots:** add `help: &'static str` to `ExciterControlSpec` and `ResonatorControlSpec`. The panel loop passes `spec.help` to the helper, so help follows the selected model with no extra state.
- **Global parameters:** pass the help as an argument at each call site, next to the literal label, in a `help` module or constants block with one `const` per control. The helpers take `help: &str` as a required argument, so the compiler forces every call site to supply one.
- *Alternative:* one table keyed by parameter ID. Rejected: it can't express slot meanings that change per model, and it separates the text from the label.

### 4. Selector help as an isolated trial
`industrial_combo` takes `help: Option<&str>`. Each selector option's text comes from a function or table in a clearly marked `// Trial: selector help (add-parameter-tooltips)` block. Where a description already exists (the exciter and resonator panel specs' `description`), it's reused rather than duplicated. Removing the trial means deleting that block and passing `None`; the parameter tooltips don't depend on it.

### 5. Coverage test
While a frame renders, each helper records the help text it was given, keyed by the parameter pointer (test-only, like `note_drawn`). Selectors record theirs too. The test renders every exciter, object and Space mode and asserts, for each drawn control:
- the help is non-empty;
- it has at most two sentences (counting `. `, `! ` and `? ` terminators);
- it's at most about 200 characters, so the tooltip stays readable.

A unit test also checks that every `ExciterControlSpec` and `ResonatorControlSpec` entry has non-empty help, including models the render loop might skip.

### 6. Writing the text
Each text is one or two sentences: what the control does, then what turning it up sounds like. Use the parameter names that exist after `m01-m04-m05-clarify-parameters`. Ground the text in the DSP code and `docs/detailed-specs/` rather than guesswork. For example, Rust: "Corrodes the object: shortens the decay and darkens the upper modes. Higher values sound duller and drier."

## Risks / Trade-offs

- [The badge widens labels that are already tight at 75 %] → The badge is sized from the label text height, with a small gap. `editor_content_fits_the_window_width` stays the gate. If a column overflows, shorten the label (they're display labels, not parameter names) rather than shrink the badge below legibility.
- [Help text drifts from the DSP when behaviour changes] → The text sits next to the label in the spec tables, so a change that touches a control's label or table entry shows its help in the same diff. A note in `docs/ARCHITECTURE.md` says to update help with behaviour changes.
- [Tooltips are hard to test headlessly] → The test checks the data path (every drawn control got help). That hovering the badge, and not the knob, opens the tooltip is checked once by feeding a pointer position over the badge and over the knob into the headless context and inspecting egui's tooltip layer. A REAPER screenshot is the visual check.
- [About 140 texts is a lot to write accurately] → Write them per panel in reviewable commits, and check each against the code that implements the parameter.

## As built (2026-11-10)

- **Where the help lives (decision 3 changed):** one table, `help_for(label)` in `src/gui/editor.rs`, keyed by the control's display label (case-insensitive). The widget helpers look it up themselves, so no call site changed and none can draw a control without asking for help. Slot labels are model-specific, so slot help still follows the selected exciter or object. The spec structs didn't get a `help` field. Done this way to keep the change small (maintainer asked for speed). The cost: a label without an entry gets empty help at run time instead of a compile error; `every_drawn_control_has_help` and `every_slot_spec_has_help` catch that in CI. Labels shared by two models (Wire Stiffness: brush wires and spring coils) get one text that covers both.
- **Badge:** `paint_label_with_badge` lays the label out and paints the badge to its right as one unit, anchored where the label used to be (so centred labels stay centred). `paint_info_badge` paints a circle with a stroke of one physical pixel and an "i", both `MUTED_GREY`, sized from the label's font, and registers its own hover area (`ui.interact`, `Sense::hover`) with `on_hover_text`.
- **Selectors (trial):** `selector_help(label, selected)`, marked as a trial. Exciter and object options reuse the panel `description`. To drop the trial, delete `selector_help` and the marked block in `industrial_combo`.

## Measurements

- `every_drawn_control_has_help`: every control drawn for the 16 exciters, 9 objects and 4 Space modes has help of one or two sentences and at most 200 characters.
- `hovering_the_badge_shows_help` / `hovering_the_knob_shows_no_help`: a pointer on the Cutoff knob's badge renders the help text in a tooltip after 1.2 s of frames; a pointer on the knob body doesn't.
- `editor_content_fits_the_window_width` still passes at every UI scale and every exciter: no label had to be shortened.
- **Not done:** the tests were written together with the implementation, not run red first (task 1.x, to save time). The REAPER screenshot (task 4.2) wasn't taken: the GUI hasn't been checked visually in a host.

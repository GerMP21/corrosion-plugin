# Tasks

## 1. Tests first

- [ ] 1.1 Add test-only recording of the help text each widget helper receives, and a coverage test that renders every exciter, object and Space mode and asserts each drawn control has non-empty help of at most two sentences and about 200 characters. Verify it fails (no help exists yet).
- [ ] 1.2 Add a unit test that every `ExciterControlSpec` and `ResonatorControlSpec` entry has non-empty help. Verify it fails to compile or fails.
- [ ] 1.3 Add a headless hover test: a pointer over a control's badge opens a tooltip, and a pointer over the knob body does not. Verify it fails.

## 2. Badge and helpers

- [ ] 2.1 Implement `info_badge(ui, scale, help)`: a painted circle with a 1 px stroke and a centred "i", both in `MUTED_GREY`, sized from `scale`, with `Sense::hover()` and `on_hover_text`. Verify by test 1.3 and a screenshot.
- [ ] 2.2 Give the knob, fader, vfader and `vfader_global` helpers a required `help: &str` argument, and draw the badge right of the label (centred together with it where labels are centred). Verify that `cargo build` flags every call site.
- [ ] 2.3 Give `industrial_combo` an optional `help`, with the selector texts in a block marked as a trial (`add-parameter-tooltips`). Verify that passing `None` removes the badge.

## 3. Help texts

- [ ] 3.1 Add `help` to `ExciterControlSpec` and write the 61 exciter slot texts from the exciter code and `docs/detailed-specs/exciter-algorithms.md`. Verify by test 1.2.
- [ ] 3.2 Add `help` to `ResonatorControlSpec` and write the 30 object slot texts from `docs/detailed-specs/resonator-algorithms.md`. Verify by test 1.2.
- [ ] 3.3 Write help for the global controls: material transforms, filter, drive, body, spread, Space, clipper, limiter, output, envelope and macros. Use the names after `m01-m04-m05-clarify-parameters`. Verify by test 1.1.
- [ ] 3.4 Write the trial selector texts (exciter and object options reuse the panel `description`; Space mode, Play Mode, Limiter Mode, Randomizer mode and others get short texts). Verify by test 1.1.

## 4. Layout and visual check

- [ ] 4.1 Run `editor_content_fits_the_window_width`, `layout_scale_never_exceeds_the_window` and the other editor tests at 75 % and every UI scale. If a column overflows, shorten display labels rather than shrinking the badge, and record what changed in `design.md`.
- [ ] 4.2 Take REAPER screenshots (the patched nih-plug screenshot build) of a column at 75 % with a tooltip open, to confirm the badge reads as a 1 px circled "i". Record the result in `design.md`.

## 5. Docs and close-out

- [ ] 5.1 Update `docs/ARCHITECTURE.md` (editor section: badge, where help lives, the selector trial and how to remove it) and the README (user-visible hover help). Verify by reading the diff.
- [ ] 5.2 Run `cargo fmt --check`, clippy with `-D warnings` (CI lane, plus the GUI lane with no new warnings), `cargo test --workspace --no-default-features`, `cargo test --lib` and `cargo doc`. Verify all pass, record the counts in `design.md`, then commit and push to `dev`.
- [ ] 5.3 Update "Current state" in `CLAUDE.md`. Verify the row reads done.

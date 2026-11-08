# Design

## Context

- `Preset::into_params` starts from `CorrosionParams::default()` and replaces many fields with parameters built by local helpers (`float_param`, `int_param`, `skewed_float_param`) that set name, value and range, and either no formatter (`int_param`, so UI Scale loses its "50%" strings) or a generic six-decimal one (`float_param`, which the live parameters don't use). Since H8 removed `load_state`, its users are the editor's preset load (H4, which normalizes each value against the live parameter, so display metadata doesn't matter there), the offline renderer (H13) and tests.
- `params.rs` builds the live parameters inline in `CorrosionParams::default()`; a few already have shared constructors (`exciter_param`, `sync_rate_param`, `position_wander_rate_param`, …).

## Goals / Non-Goals

**Goals:** no dead state, names that say what the code does, docs that match it, and one way to build each parameter.

**Non-Goals:**
- Any change to the sound. Decorrelating or high-passing the rattle would change it, so M17 stops at naming and documentation.
- Making the interaction bus bidirectional (M10 removes the pretence instead).
- Rebuilding held notes when quality mode changes (M14 is documented; the maintainer can revisit it).

## Decisions

1. **One constructor per parameter.** Move each inline constructor in `CorrosionParams::default()` that `into_params` also needs into a `params.rs` function taking the value (`ui_scale_param(v)`, `filter_cutoff_param(v)`, …), and have both call it. `into_params` stops using the bare helpers. A test checks, for every parameter in the map, that the preset-built parameter has the same name, range (normalized value of the same plain value) and `value_to_string` output as the live one. *Alternative:* copy formatters after construction. nih-plug params don't expose that.
2. **Exciter fallback stays HandStrike** (the default exciter) for values outside 1–16, documented on `ExciterType::from_int` and pinned by a test, rather than rejecting a whole user preset over one bad field.
3. **Sound-neutral proof:** a one-off render hash of a damaged, stereo, friction-plus-hit scenario against a clean worktree, recorded in Measurements.
4. **CODE-REVIEW.md** moves to `docs/archive/CODE-REVIEW-2026-05-27.md` with a header saying it's superseded by `FABLE-CODE-REVIEW-2026-07-07.md`; links to it are updated.

## Risks / Trade-offs

- **[Refactoring parameter construction could change a default or a range]** → The equality test covers every parameter, and the preset round-trip and level tests stay green.

## Open Questions

- M14: should a quality change also rebuild held notes' banks? Deferred; documenting it doesn't preclude doing it later.

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

## Measurements

### Tests first (task 1.1)

On the code before this change (`tests/preset_roundtrip.rs`):

| Test | Scenario | Before |
|---|---|---|
| `preset_built_parameters_display_like_the_live_ones` | Every parameter | fails: 76 parameters show different text across the 50 factory presets (e.g. Beam Shear "0.450000" vs live "0.45"; Body "0.410000" vs "0.41000003") |
| `ui_scale_and_filter_cutoff_display_like_the_live_ones` | UI Scale and Filter Cutoff | fails: UI Scale shows "2", not "100%" |
| `out_of_range_exciter_loads_as_hand_strike` | Exciter 0 and 17 | passes (pins today's fallback; also −3 and 99) |
| `every_valid_exciter_loads_and_saves_as_itself` | Every valid exciter | passes |

Names and normalization already matched (m01 fixed the names); only the display text differed.

**Reference render hashes** (`tests/tidy_render_hash.rs`, ignored; Damage 6, Rust 1.5, Width 1.5, Internal Rattle 0.8; C3 and G3 overlapping, 2 s at 48 kHz through the whole plugin path), clean worktree of the previous commit, gnu:

| Exciter | Hash |
|---|---|
| Bow (friction) | `0xb5a3c286a3733683` |
| Hand Strike (hit) | `0xd3bc23131499e3bd` |
| Hard Mallet (hit) | `0xeac413ddf848d891` |

### After

- **Task 2.1:** a script turned the inline constructors in `CorrosionParams::default()` into 136 `pub(crate) fn <id>_param(value)` functions, each clamping to its own range (77 `FloatParam`/`IntParam::new`, 59 `display_float_param`); the four parameters that already had shared builders (Sync Rate, Pressure, Speed, Roughness) are called with their value clamped to 0–1. `into_params` calls the same functions, and its `float_param`, `int_param` and `skewed_float_param` helpers are deleted. All four 1.1 tests pass, and `tests/preset_roundtrip.rs` passes (21).
- **Render hashes** after tasks 2.1 and 3.1: identical to the reference for all three exciters. `print_48k_exciter_render_hashes` (default patch, 16 exciters) is identical to a clean worktree too.
- `preset_levels_are_sane` reports the same three placeholder presets under its floor as before (Cable Bow −13.6, Suspension Cable Pluck −15.1, Workshop Plate Scrape −15.3 dB): no level changed.

### As built

- **M10 (decision on wording):** the proposal said the interaction bus is feed-forward. It isn't: the resonator writes its contact displacement and velocity each sample and `Voice::process_exciter` feeds them back to the exciter (`src/voice/mod.rs`, `get_displacement` / `get_velocity`). Only `exciter_force` was dead. It and `set_exciter_force` are removed, and `InteractionState`'s doc now says what is exchanged; the bus is still described as bidirectional rather than feed-forward.
- **M18:** the README and `docs/ARCHITECTURE.md` also said "8 `Voice` instances"; corrected to 16 (`MAX_VOICES`).
- **M2, M19:** confirmed resolved in the code (AD panel for hit exciters; `params::sync_division` with six equal bands shared by the echo and the display) and marked in the review.

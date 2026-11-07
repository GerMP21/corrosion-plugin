# Design

## Context

- **Mapping:** `sync_rate_to_delay_time` (`src/lib.rs`) treats values below 0.05 as Off. Above that it splits `(v − 0.05)/0.95` into six equal bands over the `DIVISIONS` beat table.
- **Parameter:** `sync_rate` is a plain `FloatParam` with default 0.25 and no value formatter.
- **Presets:** the seeder writes `p.extra.sync_rate` from the parameter default, except Hanging Chain Echo, which sets 0.5.

## Goals / Non-Goals

**Goals:**
- Sync is off by default.
- The displayed value always matches the behaviour.

**Non-Goals:**
- Changing the band layout or adding triplet and dotted divisions (finding M19).
- A GUI control for sync (H5).

## Decisions

1. **Default 0.0.** It is the bottom of the Off region, so a small nudge doesn't switch sync on by accident.
2. **One source of truth for the bands.** Move the Off threshold and the band index computation into a small function used by both `sync_rate_to_delay_time` and the parameter's `value_to_string`, so the display can't drift from the behaviour.
   - *Alternative:* an `IntParam` enum. That changes the parameter type, which breaks host automation and saved sessions.
3. **Regenerate the presets from the seeder.** It already takes `sync_rate` from the parameter default. Hanging Chain Echo's explicit 0.5 stays.

## Risks / Trade-offs

- **[A user who relied on the default 1/8 sync loses it]** → Only for new instances. Saved sessions and presets restore their stored value.
- **[Hosts cache display strings]** → No impact on stored state; the value is unchanged.

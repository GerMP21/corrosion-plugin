# Proposal

## Why

`sync_rate` defaults to 0.25, but sync is Off only below 0.05. So in any host that reports a tempo, a new instance's Echo is locked to a 1/8 note and the Delay Time knob silently does nothing. The parameter's value is also shown as a bare number, so nothing tells the user that sync is on (fable review 2026-07-07, finding **H3**).

## What Changes

- **Sync is off by default.** `sync_rate` defaults to 0.0, so Delay Time works out of the box.
- **The parameter says what it does.** The host and GUI show "Off" or the selected division ("1/16", "1/8", "1/4", "1/2", "1/1", "2/1") instead of a raw number.
- **Factory presets:** regenerated presets store 0.0 unless they set sync on purpose. Only Hanging Chain Echo does (0.5, a 1/4 note), and it keeps it. The other 49 presets don't use Echo, so they sound the same.
- **Not here:** a GUI control for sync is part of finding H5's change (`h05-expose-missing-gui-controls`).

## Capabilities

### New Capabilities
- `tempo-sync`: when the Echo follows the host tempo, and how the sync setting is presented.

### Modified Capabilities
<!-- none -->

## Impact

- **Code:** `src/params.rs` (the default and a value formatter); `src/lib.rs` (shares the division table with the formatter).
- **Presets:** regenerated `presets/factory/*` (`sync_rate` 0.25 → 0.0 in 49 files).
- **Docs:** the review's H3 status.

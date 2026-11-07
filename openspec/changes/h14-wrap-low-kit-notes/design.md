# Design

## Context

`note_to_kit_object(note)` computes `((note − 24).max(0) / 10).rem_euclid(9)`. Integer division truncates toward zero, and `.max(0)` sends every negative offset to slot 0. The Kit-mode tests only cover notes 24 and up.

## Goals / Non-Goals

**Goals:** a symmetric wrap, as documented.

**Non-Goals:**
- Changing the slot width, the block start or the object order.
- Per-slot pitch handling.

## Decisions

1. **`(note − 24).div_euclid(10).rem_euclid(9)`.** `div_euclid` floors toward negative infinity, so the slots below 24 are full 10-note slots continuing the pattern downward. Above 24 the result is identical to the current code.
   - *Alternative:* mirror the slots below 24. That isn't what the doc promises, and it would put Pipe next to Pipe at the boundary.

## Risks / Trade-offs

- **[Users who played low notes expecting Pipe hear other objects]** → That's the documented behaviour, and the change is limited to MIDI 0–23 in Kit mode.

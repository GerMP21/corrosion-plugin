# Proposal

## Why

When every voice is busy, the voice manager steals the voice with the lowest decaying peak and treats every sounding voice alike. A note triggered a few milliseconds ago hasn't reached its peak yet, so it counts as the quietest and is stolen by the next note. Several quick notes over a full pool keep replacing each other, while the older notes, including released tails, keep sounding. This was found by the C3 voice-stealing test. The pool is also small for this instrument: long metallic tails, Drone mode and chords run out of 8 voices quickly, which makes stealing frequent.

## What Changes

- **Stealing order:** released (tailing) voices are stolen before held ones. Among held voices, a note younger than a short protection window can't be stolen while an older held note can.
- **Within each group:** the quietest voice goes first, with the oldest losing ties. When every held voice is still protected, the oldest held voice goes.
- **Polyphony** rises from 8 to 16 voices (`MAX_VOICES` = 16).
- **Audible change:** fast notes over a full pool no longer cancel each other. Released tails give way before held notes. Twice as many notes and tails ring at once before any stealing.
- **Audible and CPU change:** dense passages sum up to twice as many voices into the post chain and limiter. Worst-case voice CPU doubles (measured at about 20 % of one core for 8 Render-quality Plate voices on the development machine, so about 40 % for 16).

## Capabilities

### New Capabilities
- `voice-allocation`: How many voices the instrument can sound at once, how free voices are chosen for new notes, and which voice is stolen when the pool is full.

### Modified Capabilities
- `realtime-safety`: "Voice stealing does not allocate" names a fixed 8-voice pool in its only scenario. It is replaced by "Stealing a voice does not allocate", which states the same guarantee for a full pool of any size.

## Impact

- **Code:**
  - `src/voice/manager.rs` (`MAX_VOICES`, pool construction, the steal selection, and a sample-rate-aware protection window);
  - the stealing tests in `src/voice/manager.rs`;
  - `tests/no_alloc.rs` and `tests/automation_stress.rs`, which use `MAX_VOICES` and pick up 16 automatically;
  - `benches/performance.rs` (a 16-voice benchmark).
- **Memory:** a `Voice` is about 1 KB inline plus its preallocated buffers, so the pool goes from about 8 KB to 16 KB inline plus 8 more voices' heap buffers, all allocated at construction.
- **Docs:** README (polyphony line, project layout), `docs/ARCHITECTURE.md` (diagram, `manager.rs` line, memory note), the `docs/PRD.md` "implemented" row, and the expression-headroom comment in `src/voice/mod.rs`.
- **Presets:** no preset or parameter changes.

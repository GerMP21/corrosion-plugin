# Spec Delta

## ADDED Requirements

### Requirement: Limiter latency is known before audio starts
When the plugin is initialized with Limiter Mode set to Lookahead, it SHALL report the Lookahead limiter's latency to the host during initialization, before the first audio block. With Limiter Mode Hard it SHALL report no latency.

#### Scenario: Session restored with Lookahead
- **WHEN** the plugin is initialized with Limiter Mode = Lookahead
- **THEN** it reports a latency equal to the Lookahead limiter's delay during initialization, and the first audio block reports nothing further

#### Scenario: Session restored with Hard
- **WHEN** the plugin is initialized with Limiter Mode = Hard
- **THEN** no latency other than zero is reported during initialization

#### Scenario: Mode changed while playing
- **WHEN** Limiter Mode changes from Hard to Lookahead between two audio blocks
- **THEN** the new latency is reported once, at the first block with the new mode

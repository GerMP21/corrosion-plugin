# Spec Delta

## Purpose

Defines when the Echo's delay follows the host tempo, what the sync setting defaults to, and how it is shown, so the free-running Delay Time control is never silently overridden.

## ADDED Requirements

### Requirement: Sync is off by default
A new plugin instance SHALL have tempo sync off, so the Echo's delay follows the Delay Time control even when the host reports a tempo.

#### Scenario: Fresh instance in a host with tempo
- **WHEN** a new instance runs in Echo mode with the host reporting 120 BPM, and Delay Time is changed
- **THEN** the echo's delay follows Delay Time

### Requirement: The sync setting shows its division
The sync parameter's displayed value SHALL be "Off" in its off region and otherwise the selected division, one of 1/16, 1/8, 1/4, 1/2, 1/1 and 2/1. The displayed division SHALL be the one the Echo uses at that value.

#### Scenario: Off and divisions
- **WHEN** the sync parameter is set to 0.0, 0.3 and 1.0
- **THEN** it is displayed as "Off", "1/8" and "2/1" respectively, and at 120 BPM the Echo's delay is free-running, 0.25 s and 4 s (clamped to the Echo's maximum)

#### Scenario: Display matches behavior across the range
- **WHEN** the sync parameter is swept from 0 to 1 in steps of 0.01
- **THEN** at every step the displayed division equals the division the Echo selects, or "Off" exactly where sync is off

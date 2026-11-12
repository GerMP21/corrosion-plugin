# Spec Delta

## Purpose

Corrosion runs fully in a web browser, on a free public page, with the same sound engine and editor as the plugin, so anyone can hear and test it without installing anything.

## ADDED Requirements

### Requirement: The page plays the plugin's engine
The web app SHALL produce audio with the same engine as the plugin builds, at the browser's audio sample rate, in stereo. Audio SHALL start only after a user gesture, as browsers require, and the page SHALL say how to start it.

#### Scenario: First note after opening the page
- **WHEN** a visitor opens the page, clicks to start audio and plays a note on the piano strip
- **THEN** the default patch sounds, and the output is not silent

#### Scenario: Same sound as the plugin
- **WHEN** a preset and a note pattern are rendered by the web engine and by the plugin's offline renderer at the same sample rate and quality
- **THEN** the two outputs match within a small tolerance (the build differs, so the comparison is a level and spectrum tolerance, not a bit match)

### Requirement: Every parameter and editor feature works
The web app SHALL show the plugin's editor with every control, the inline help, the preset browser, the randomizer, Note Mode and the kit editor. Moving any control SHALL change the sound as it does in the plugin.

#### Scenario: A control changes the sound
- **WHEN** a visitor moves a control (for example Cutoff) while a note is held
- **THEN** the held note's sound changes, as it does in the plugin

#### Scenario: Kit mode
- **WHEN** a visitor switches Note Mode to Kit, assigns a pad and plays its key
- **THEN** that key plays the pad's sound, and keys without a pad stay silent

### Requirement: Notes from every input
The web app SHALL accept notes from the editor's piano strip, from the computer keyboard (the same key map as the plugin), and from MIDI devices through Web MIDI where the browser provides it. Where Web MIDI is unavailable, the page SHALL say so, and the other inputs SHALL keep working.

#### Scenario: MIDI device
- **WHEN** a MIDI keyboard is connected in a browser with Web MIDI and the visitor allows it
- **THEN** its notes, velocity, pitch bend, mod wheel and sustain pedal play the synth

#### Scenario: Browser without Web MIDI
- **WHEN** the browser has no Web MIDI
- **THEN** the page says MIDI is unavailable, and the piano strip and computer keyboard still play

### Requirement: Tempo for synced effects
The web app SHALL provide a tempo the visitor can set, used by the Echo's tempo sync as a host tempo would be.

#### Scenario: Synced echo follows the tempo
- **WHEN** the Echo is synced to 1/4 and the visitor sets 120 BPM
- **THEN** the echo repeats every 0.5 s

### Requirement: Sessions and presets persist in the browser
The web app SHALL save the session (every parameter and the kit) in the browser and restore it on the next visit. User presets SHALL be saved in the browser, and visitors SHALL be able to download them as `.corrosion-preset` files and load such files, including presets made with the plugin. Factory presets SHALL be available without network access to extra files.

#### Scenario: Reload keeps the sound
- **WHEN** a visitor changes the sound, then reloads the page
- **THEN** the same sound and kit are restored

#### Scenario: Preset from the plugin
- **WHEN** a visitor loads a `.corrosion-preset` file saved by the plugin
- **THEN** it loads with the same values, migrated as the plugin migrates it

### Requirement: The page stays responsive and glitch-free
The web app SHALL choose defaults (quality mode and voice count) under which the default patch plays without audio dropouts on a typical laptop, and SHALL say when a setting exceeds what the browser can render in real time.

#### Scenario: Default patch on a typical laptop
- **WHEN** a visitor holds a chord on the default patch at the default quality
- **THEN** audio plays without dropouts

### Requirement: Public, free and linked to the source
The web app SHALL be served free and without sign-in from a public URL, redeployed from the repository's main branch. It SHALL show the version (commit) it was built from, link the source repository, and state the GPLv3 license.

#### Scenario: Version and license shown
- **WHEN** a visitor opens the page
- **THEN** it shows the commit it was built from, a link to the source, and the GPLv3 license

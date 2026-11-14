# Spec Delta

## ADDED Requirements

### Requirement: Every clip row says what it plays
Every row of clip buttons in the Wiki SHALL start with a visible caption, in the page's language, that names what the row plays:
- **A parameter row:** the control's label as the editor shows it. A parameter with no editor control uses its host name.
- **One item of a set:** the item and what it is compared on. For example, "Bow, on Tank" or "Pipe, with Felt Mallet".
- **A factory preset row:** that it plays a factory preset.
- **A recipe row:** that it plays the recipe before and after.
- **A whole-set row:** what the set holds. For example, "All exciters, on Tank" or "Drum kits".

A row's caption SHALL be distinct from the captions of the other clip rows on its page, unless both rows play the same clips.

#### Scenario: Envelope rows
- **WHEN** a reader opens the envelope page
- **THEN** the three MSEG rows read "Onset", "Attack" and "Release" before their buttons, and no two rows on the page share a caption

#### Scenario: Spanish page
- **WHEN** a reader opens the same page in Spanish
- **THEN** the captions use the same control labels (control names stay in English), and the set captions are in Spanish

#### Scenario: Missing caption fails the check
- **WHEN** a clip row would render without a caption
- **THEN** `npm run check` fails and names the page and the row

### Requirement: Parameter buttons state their value
Each Min, Mid and Max button of a parameter row SHALL state the value it plays, in its tooltip and its accessible name. The value SHALL be formatted as the plugin displays it, and a choice parameter SHALL show the option's name, not its index.

#### Scenario: A choice parameter
- **WHEN** a reader hovers the Max button of the Loop row
- **THEN** the tooltip names the loop mode it plays, for example "Ping-Pong", and not "2"

#### Scenario: A continuous parameter
- **WHEN** a reader hovers the Min button of the OUTPUT row
- **THEN** the tooltip shows the value with its unit, as the plugin shows it, for example "-inf dB"

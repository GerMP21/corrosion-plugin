# Spec Delta

## Purpose

The guide reads like an experienced sound designer sitting next to the reader in the studio: warm, conversational, a little witty, and always practical. It explains why the instrument behaves as it does and how to use that, without assuming DSP training and without talking down to the reader. The model is the best synthesizer manuals, the ones people read for pleasure, not the dry spec sheet.

## ADDED Requirements

### Requirement: Warm, conversational voice
Every guide page SHALL speak to the reader directly, in the second person, in a warm and conversational tone, free of corporate and marketing language. Light humour SHALL be welcome where it helps the explanation and SHALL NOT replace it.

#### Scenario: Corporate phrasing
- **WHEN** a page describes a control as "a powerful tool that lets you unleash rich textures"
- **THEN** the review rejects it and the sentence says what the control does to the sound instead

#### Scenario: A joke instead of a fact
- **WHEN** a quip stands where the page should say what a control does
- **THEN** the review asks for the fact; the quip may stay beside it

### Requirement: Natural Spanish
Spanish pages SHALL keep the same voice, addressing the reader as *tú*, and SHALL read as natural Spanish rather than a word-for-word translation.

#### Scenario: Calqued sentence
- **WHEN** a Spanish sentence keeps the English word order and idiom
- **THEN** the review asks for it to be rewritten as a Spanish writer would say it

### Requirement: Explain why, not only what
A control's description SHALL say what it does to the sound and, where it helps the reader use it, why: the physical or circuit behaviour behind it, in plain words (a struck point near a node leaves that mode silent; a hotter metal is softer, so it rings lower and darker). It SHALL NOT stop at restating the label ("Cutoff controls the cutoff").

#### Scenario: Label restated
- **WHEN** a control's paragraph only paraphrases its name
- **THEN** the review asks for what the listener hears and the reason behind it

### Requirement: Practical sound-design tips
Every page about a part of the instrument SHALL include practical advice from a sound designer's point of view: sweet spots, useful combinations, what to try when a sound is too dull, harsh, quiet or wild, and the traps to avoid. Tips SHALL be checked against the plugin (by ear, by a render or by the code) before they are published.

#### Scenario: Page without tips
- **WHEN** an output-chain page lists its controls but gives no advice on using them
- **THEN** the review asks for tips on that page

### Requirement: Hands-on steps
Every page SHALL open with what the subject does to the sound and a short hands-on step the reader can do at once, in the plugin or the web app. Longer topics SHALL include step-by-step walkthroughs that build a sound from a named starting point (a factory preset or the default patch), one numbered step per action, with the values as the plugin displays them.

#### Scenario: Walkthrough step
- **WHEN** a walkthrough says "make it darker"
- **THEN** the review asks for the control and the value, e.g. "turn **Cutoff** down to about 2 kHz"

### Requirement: Respect the reader
The guide SHALL assume no DSP or physics training, and SHALL NOT talk down to the reader. A technical term SHALL be explained in plain words, or linked to the glossary, the first time it appears on a page. Maths SHALL appear only when it is the simplest way to say something useful (pitch is roughly the key divided by Size), never to show off.

#### Scenario: Unexplained jargon
- **WHEN** a page first mentions "modes" without an explanation or a glossary link
- **THEN** the review asks for one

### Requirement: Accuracy over charm
The style SHALL NOT bend the facts. Every claim, number, tip and joke SHALL agree with the plugin, and the requirements of the `user-guide` capability (numbers from data, documented surprises, reproducible recipes) apply unchanged. Where a behaviour is subtle or limited, the page SHALL say so plainly rather than oversell it.

#### Scenario: Subtle control
- **WHEN** a control's effect is hard to hear under a dry note (such as HF Damping)
- **THEN** the page says it is subtle and where to hear it, rather than promising a dramatic change

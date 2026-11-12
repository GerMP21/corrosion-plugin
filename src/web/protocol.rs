//! The messages between the web editor and the web engine (`add-web-app`).
//!
//! The editor and the engine are two wasm instances (the page's main thread
//! and its AudioWorklet), so they share nothing: the editor encodes
//! [`Command`]s into a byte batch, the page posts it to the worklet, and the
//! engine decodes it before its next block. The engine answers with a
//! [`Report`]. Both directions are encoded here so the two sides can't drift.
//!
//! A batch is a sequence of `[tag: u8][len: u32 LE][payload: len bytes]`.

use crate::gui_notes::NoteMask;

const PARAM: u8 = 1;
const MIDI: u8 = 2;
const EDITOR_NOTE_ON: u8 = 3;
const EDITOR_NOTE_OFF: u8 = 4;
const RELEASE_EDITOR_NOTES: u8 = 5;
const TEMPO: u8 = 6;
const KIT: u8 = 7;

/// One message from the editor to the engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command<'a> {
    /// Set parameter `index` (`param_map()` order) to a normalized value.
    Param {
        index: u32,
        normalized: f32,
    },
    /// A MIDI message (Web MIDI or the page's own keys).
    Midi([u8; 3]),
    /// A note from the editor's keyboard strip or computer keys.
    EditorNoteOn {
        note: u8,
        velocity: u8,
    },
    EditorNoteOff {
        note: u8,
    },
    /// Stop every note the editor turned on (the page was hidden).
    ReleaseEditorNotes,
    /// Host tempo in BPM; 0 or less means none.
    Tempo(f32),
    /// The whole kit, as the `kit` persist field's JSON.
    Kit(&'a str),
}

impl Command<'_> {
    /// Append this command to `batch`.
    pub fn encode(&self, batch: &mut Vec<u8>) {
        let mut frame = |tag: u8, payload: &[u8]| {
            batch.push(tag);
            batch.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            batch.extend_from_slice(payload);
        };
        match *self {
            Command::Param { index, normalized } => {
                let mut payload = [0u8; 8];
                payload[..4].copy_from_slice(&index.to_le_bytes());
                payload[4..].copy_from_slice(&normalized.to_le_bytes());
                frame(PARAM, &payload);
            }
            Command::Midi(bytes) => frame(MIDI, &bytes),
            Command::EditorNoteOn { note, velocity } => frame(EDITOR_NOTE_ON, &[note, velocity]),
            Command::EditorNoteOff { note } => frame(EDITOR_NOTE_OFF, &[note]),
            Command::ReleaseEditorNotes => frame(RELEASE_EDITOR_NOTES, &[]),
            Command::Tempo(bpm) => frame(TEMPO, &bpm.to_le_bytes()),
            Command::Kit(json) => frame(KIT, json.as_bytes()),
        }
    }
}

/// The commands in `batch`, in order. Stops at the first malformed frame;
/// unknown tags are skipped. Allocation-free.
pub fn decode(batch: &[u8]) -> impl Iterator<Item = Command<'_>> {
    let mut rest = batch;
    std::iter::from_fn(move || loop {
        if rest.len() < 5 {
            return None;
        }
        let tag = rest[0];
        let len = u32::from_le_bytes([rest[1], rest[2], rest[3], rest[4]]) as usize;
        let payload = rest.get(5..5 + len)?;
        rest = &rest[5 + len..];
        let f32_at = |at: usize| {
            payload
                .get(at..at + 4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let command = match tag {
            PARAM if len == 8 => Some(Command::Param {
                index: u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]),
                normalized: f32_at(4)?,
            }),
            MIDI if len == 3 => Some(Command::Midi([payload[0], payload[1], payload[2]])),
            EDITOR_NOTE_ON if len == 2 => Some(Command::EditorNoteOn {
                note: payload[0],
                velocity: payload[1],
            }),
            EDITOR_NOTE_OFF if len == 1 => Some(Command::EditorNoteOff { note: payload[0] }),
            RELEASE_EDITOR_NOTES => Some(Command::ReleaseEditorNotes),
            TEMPO if len == 4 => Some(Command::Tempo(f32_at(0)?)),
            KIT => std::str::from_utf8(payload).ok().map(Command::Kit),
            _ => None,
        };
        if command.is_some() {
            return command;
        }
    })
}

/// What the engine tells the editor, a few times a second.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Report {
    /// The diode stage's largest reduction since the last report (dB).
    pub diode_reduction_db: f32,
    /// Voices sounding.
    pub voices: u32,
    /// Notes held, for the keyboard strip's highlights.
    pub held: NoteMask,
}

/// [`Report`]'s encoded size.
pub const REPORT_LEN: usize = 24;

impl Report {
    pub fn encode(&self) -> [u8; REPORT_LEN] {
        let mut out = [0u8; REPORT_LEN];
        out[..4].copy_from_slice(&self.diode_reduction_db.to_le_bytes());
        out[4..8].copy_from_slice(&self.voices.to_le_bytes());
        out[8..16].copy_from_slice(&self.held.0[0].to_le_bytes());
        out[16..].copy_from_slice(&self.held.0[1].to_le_bytes());
        out
    }

    /// `None` if `bytes` isn't a report.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let bytes: &[u8; REPORT_LEN] = bytes.try_into().ok()?;
        let word = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
        Some(Self {
            diode_reduction_db: f32::from_le_bytes(bytes[..4].try_into().unwrap()),
            voices: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            held: NoteMask([word(8), word(16)]),
        })
    }
}

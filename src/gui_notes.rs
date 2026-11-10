//! Notes played from the editor's keyboard (`add-editor-keyboard`).
//!
//! The editor (GUI thread) plays notes with the mouse or the computer
//! keyboard; the audio thread drains them at the start of each block and
//! handles them as host notes on channel 0. Between the two sits
//! [`EditorNotes`](crate::gui_notes::EditorNotes): a preallocated, lock-free single-producer,
//! single-consumer ring, a release-all flag for an editor that closes, and
//! the held-note mask the audio thread publishes for the key highlights.
//!
//! The GUI side is [`NoteSender`](crate::gui_notes::NoteSender). It tracks which notes it holds and why
//! (mouse, computer key), keeps a backlog when the ring is full so no event
//! is lost, and releases everything it holds when the editor loses focus or
//! closes. Nothing here depends on the GUI toolkit, so the CI lane tests it.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Ring capacity in events. A frame rarely produces more than a handful;
/// the GUI-side backlog covers the rest.
pub const QUEUE_CAPACITY: usize = 256;

/// Velocity of notes played on the computer keyboard (100/127).
pub const KEY_VELOCITY: u8 = 100;

/// Lowest and highest notes of the on-screen strip, C2 and C6 (C2 so the
/// kit zone, MIDI 36–59, is on screen: `per-key-kits`).
pub const STRIP_LOW_NOTE: u8 = 36;
pub const STRIP_HIGH_NOTE: u8 = 84;

/// Physical keys, as letters, that play C4 (60) to C5 (72) in order.
pub const KEY_LETTERS: [char; 13] = [
    'A', 'W', 'S', 'E', 'D', 'F', 'T', 'G', 'Y', 'H', 'U', 'J', 'K',
];

/// First note of the computer-key octave, C4.
pub const KEY_LOW_NOTE: u8 = 60;

/// First note of the computer-key octave in Kit mode, C2: the keys play
/// MIDI 36–48, the bottom of the kit zone (`per-key-kits`).
pub const KIT_KEY_LOW_NOTE: u8 = 36;

/// One note event from the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorNoteEvent {
    /// Start `note` at `velocity` (1–127).
    On { note: u8, velocity: u8 },
    /// Stop `note`.
    Off { note: u8 },
}

impl EditorNoteEvent {
    fn pack(self) -> u32 {
        match self {
            Self::On { note, velocity } => {
                1 << 16 | u32::from(velocity.clamp(1, 127)) << 8 | u32::from(note & 0x7f)
            }
            Self::Off { note } => u32::from(note & 0x7f),
        }
    }

    fn unpack(word: u32) -> Self {
        let note = (word & 0x7f) as u8;
        if word & (1 << 16) != 0 {
            Self::On {
                note,
                velocity: ((word >> 8) & 0x7f) as u8,
            }
        } else {
            Self::Off { note }
        }
    }
}

/// The note for a computer key, given as its letter, if it is mapped.
pub fn key_note(letter: char) -> Option<u8> {
    key_note_from(letter, KEY_LOW_NOTE)
}

/// The note for a computer key when the keys start at `base`.
pub fn key_note_from(letter: char, base: u8) -> Option<u8> {
    key_index(letter).map(|index| base + index as u8)
}

fn key_index(letter: char) -> Option<usize> {
    let upper = letter.to_ascii_uppercase();
    KEY_LETTERS.iter().position(|&l| l == upper)
}

/// The letter a note is played with on the computer keyboard, if any.
pub fn note_letter(note: u8) -> Option<char> {
    note_letter_from(note, KEY_LOW_NOTE)
}

/// The letter a note is played with when the keys start at `base`.
pub fn note_letter_from(note: u8, base: u8) -> Option<char> {
    note.checked_sub(base)
        .and_then(|index| KEY_LETTERS.get(usize::from(index)).copied())
}

/// Whether `note` is a black key (C♯, D♯, F♯, G♯ or A♯).
pub fn is_black_key(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}

/// State shared between the editor and the audio thread.
pub struct EditorNotes {
    slots: Box<[AtomicU32]>,
    /// Events written so far (producer side).
    head: AtomicUsize,
    /// Events read so far (consumer side).
    tail: AtomicUsize,
    /// Serialises producers. Only the GUI takes it, never the audio
    /// thread, so the consumer stays lock-free even if a host briefly
    /// holds two editor instances.
    producer: Mutex<()>,
    /// Set by a closing editor: release every note the editor started.
    release_all: AtomicBool,
    /// Notes held by any voice, published by the audio thread per block.
    held: [AtomicU64; 2],
}

impl Default for EditorNotes {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorNotes {
    /// An empty queue. Allocates its ring once, here.
    pub fn new() -> Self {
        Self {
            slots: (0..QUEUE_CAPACITY).map(|_| AtomicU32::new(0)).collect(),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            producer: Mutex::new(()),
            release_all: AtomicBool::new(false),
            held: [AtomicU64::new(0), AtomicU64::new(0)],
        }
    }

    /// Push one event; `false` if the ring is full. GUI side.
    pub fn try_push(&self, event: EditorNoteEvent) -> bool {
        let _guard = self
            .producer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) >= QUEUE_CAPACITY {
            return false;
        }
        self.slots[head % QUEUE_CAPACITY].store(event.pack(), Ordering::Relaxed);
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Hand every queued event to `handle`, oldest first. Audio side:
    /// lock-free and allocation-free.
    pub fn drain(&self, mut handle: impl FnMut(EditorNoteEvent)) {
        let head = self.head.load(Ordering::Acquire);
        let mut tail = self.tail.load(Ordering::Relaxed);
        while tail != head {
            let word = self.slots[tail % QUEUE_CAPACITY].load(Ordering::Relaxed);
            tail = tail.wrapping_add(1);
            self.tail.store(tail, Ordering::Release);
            handle(EditorNoteEvent::unpack(word));
        }
    }

    /// Ask the audio thread to release every note the editor started.
    pub fn request_release_all(&self) {
        self.release_all.store(true, Ordering::Release);
    }

    /// Take a pending release-all request. Audio side.
    pub fn take_release_all(&self) -> bool {
        self.release_all.swap(false, Ordering::AcqRel)
    }

    /// Publish the notes held by the voices. Audio side.
    pub fn publish_held(&self, notes: impl Iterator<Item = u8>) {
        let mut mask = NoteMask::default();
        for note in notes {
            mask.set(note, true);
        }
        self.held[0].store(mask.0[0], Ordering::Relaxed);
        self.held[1].store(mask.0[1], Ordering::Relaxed);
    }

    /// The last published held notes.
    pub fn held(&self) -> NoteMask {
        NoteMask([
            self.held[0].load(Ordering::Relaxed),
            self.held[1].load(Ordering::Relaxed),
        ])
    }
}

/// A set of MIDI notes 0–127.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoteMask(pub [u64; 2]);

impl NoteMask {
    pub fn contains(&self, note: u8) -> bool {
        let note = note & 0x7f;
        self.0[usize::from(note >> 6)] & (1 << (note & 63)) != 0
    }

    pub fn set(&mut self, note: u8, on: bool) {
        let note = note & 0x7f;
        let bit = 1 << (note & 63);
        let word = &mut self.0[usize::from(note >> 6)];
        if on {
            *word |= bit;
        } else {
            *word &= !bit;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0 == [0, 0]
    }

    /// The notes in the set, lowest first. Allocation-free.
    pub fn iter(&self) -> impl Iterator<Item = u8> + '_ {
        (0..128u8).filter(move |&note| self.contains(note))
    }
}

/// Why the editor holds a note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteSource {
    Mouse,
    Key,
}

impl NoteSource {
    fn bit(self) -> u8 {
        match self {
            Self::Mouse => 1,
            Self::Key => 2,
        }
    }
}

/// The editor's side of [`EditorNotes`].
pub struct NoteSender {
    shared: Arc<EditorNotes>,
    backlog: VecDeque<EditorNoteEvent>,
    /// Per note, the [`NoteSource`] bits holding it.
    holders: [u8; 128],
    /// The computer keys' first note ([`KEY_LOW_NOTE`], or
    /// [`KIT_KEY_LOW_NOTE`] in Kit mode).
    key_base: u8,
    /// Per computer key, the note its press started, so its release stops
    /// that note even if the base moved while it was held.
    key_notes: [Option<u8>; KEY_LETTERS.len()],
}

impl NoteSender {
    pub fn new(shared: Arc<EditorNotes>) -> Self {
        Self {
            shared,
            backlog: VecDeque::new(),
            holders: [0; 128],
            key_base: KEY_LOW_NOTE,
            key_notes: [None; KEY_LETTERS.len()],
        }
    }

    /// Set the computer keys' first note (`per-key-kits`).
    pub fn set_key_base(&mut self, base: u8) {
        self.key_base = base;
    }

    /// The computer keys' first note.
    pub fn key_base(&self) -> u8 {
        self.key_base
    }

    /// The shared state, for the held-note highlights.
    pub fn shared(&self) -> &EditorNotes {
        &self.shared
    }

    /// Hold `note` for `source`. The note starts only if nothing held it.
    pub fn press(&mut self, note: u8, velocity: u8, source: NoteSource) {
        let note = note & 0x7f;
        let holders = &mut self.holders[usize::from(note)];
        if *holders == 0 {
            self.backlog
                .push_back(EditorNoteEvent::On { note, velocity });
        }
        *holders |= source.bit();
        self.flush();
    }

    /// Stop holding `note` for `source`. The note stops once nothing holds
    /// it; releasing a note `source` doesn't hold does nothing.
    pub fn release(&mut self, note: u8, source: NoteSource) {
        let note = note & 0x7f;
        let holders = &mut self.holders[usize::from(note)];
        if *holders & source.bit() == 0 {
            return;
        }
        *holders &= !source.bit();
        if *holders == 0 {
            self.backlog.push_back(EditorNoteEvent::Off { note });
        }
        self.flush();
    }

    /// A computer-key event: `letter` is the physical key as a letter.
    /// Presses play only when `gate_open` (editor focused, no text field
    /// taking input) and not on auto-repeat; releases always apply.
    pub fn key_event(&mut self, letter: char, pressed: bool, repeat: bool, gate_open: bool) {
        let Some(index) = key_index(letter) else {
            return;
        };
        if !pressed {
            let note = self.key_notes[index]
                .take()
                .unwrap_or(self.key_base + index as u8);
            self.release(note, NoteSource::Key);
        } else if gate_open && !repeat {
            let note = self.key_base + index as u8;
            if let Some(previous) = self.key_notes[index].replace(note) {
                if previous != note {
                    self.release(previous, NoteSource::Key);
                }
            }
            self.press(note, KEY_VELOCITY, NoteSource::Key);
        }
    }

    /// Release every note the editor holds (focus lost).
    pub fn release_all_held(&mut self) {
        self.key_notes = [None; KEY_LETTERS.len()];
        for note in 0..128u8 {
            if self.holders[usize::from(note)] != 0 {
                self.holders[usize::from(note)] = 0;
                self.backlog.push_back(EditorNoteEvent::Off { note });
            }
        }
        self.flush();
    }

    /// Forget what the editor holds without sending anything: the editor
    /// reopened, and the audio thread already released its notes when it
    /// closed.
    pub fn forget_held(&mut self) {
        self.holders = [0; 128];
        self.key_notes = [None; KEY_LETTERS.len()];
        self.backlog.clear();
    }

    /// Whether `source` holds `note`.
    pub fn holds(&self, note: u8, source: NoteSource) -> bool {
        self.holders[usize::from(note & 0x7f)] & source.bit() != 0
    }

    /// Whether the editor holds `note` for any reason.
    pub fn holds_any(&self, note: u8) -> bool {
        self.holders[usize::from(note & 0x7f)] != 0
    }

    /// Events still waiting for room in the ring.
    pub fn pending(&self) -> usize {
        self.backlog.len()
    }

    /// Move backlog events into the ring while it has room. Called on each
    /// press and release, and once per frame.
    pub fn flush(&mut self) {
        while let Some(&event) = self.backlog.front() {
            if !self.shared.try_push(event) {
                break;
            }
            self.backlog.pop_front();
        }
    }
}

impl Drop for NoteSender {
    /// The editor closed: release what it holds, and tell the audio thread
    /// to release every editor note even if no frame runs again.
    fn drop(&mut self) {
        self.release_all_held();
        self.shared.request_release_all();
    }
}

/// The audio thread's side: which notes the editor turned on, so a
/// release-all touches only those.
#[derive(Clone, Copy, Debug, Default)]
pub struct EditorNoteState {
    on: NoteMask,
    editor_was_open: bool,
}

impl EditorNoteState {
    /// Note whether the editor window is open, once per block. When it has
    /// just closed, ask for a release-all: the editor's state outlives its
    /// window, so this is where a close is seen.
    pub fn observe_editor_open(&mut self, open: bool, shared: &EditorNotes) {
        if self.editor_was_open && !open {
            shared.request_release_all();
        }
        self.editor_was_open = open;
    }

    /// Drain the editor's events into `handle`, then honour a release-all
    /// by sending a note-off for each note the editor still has on.
    /// Allocation-free.
    pub fn drain(&mut self, shared: &EditorNotes, mut handle: impl FnMut(EditorNoteEvent)) {
        shared.drain(|event| {
            match event {
                EditorNoteEvent::On { note, .. } => self.on.set(note, true),
                EditorNoteEvent::Off { note } => self.on.set(note, false),
            }
            handle(event);
        });
        if shared.take_release_all() {
            for word in 0..2 {
                let mut bits = self.on.0[word];
                while bits != 0 {
                    let note = (word * 64) as u8 + bits.trailing_zeros() as u8;
                    bits &= bits - 1;
                    handle(EditorNoteEvent::Off { note });
                }
            }
            self.on = NoteMask::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drained(shared: &EditorNotes) -> Vec<EditorNoteEvent> {
        let mut events = Vec::new();
        shared.drain(|event| events.push(event));
        events
    }

    fn on(note: u8, velocity: u8) -> EditorNoteEvent {
        EditorNoteEvent::On { note, velocity }
    }

    fn off(note: u8) -> EditorNoteEvent {
        EditorNoteEvent::Off { note }
    }

    #[test]
    fn events_pack_and_unpack() {
        for event in [on(0, 1), on(127, 127), on(60, 100), off(0), off(127)] {
            assert_eq!(EditorNoteEvent::unpack(event.pack()), event);
        }
    }

    /// Spec "Order is kept".
    #[test]
    fn drain_keeps_the_push_order() {
        let shared = EditorNotes::new();
        for event in [on(60, 90), off(60), on(62, 90)] {
            assert!(shared.try_push(event));
        }
        assert_eq!(drained(&shared), vec![on(60, 90), off(60), on(62, 90)]);
        assert!(drained(&shared).is_empty());
    }

    #[test]
    fn a_full_ring_refuses_events() {
        let shared = EditorNotes::new();
        for index in 0..QUEUE_CAPACITY {
            assert!(shared.try_push(off((index % 128) as u8)));
        }
        assert!(!shared.try_push(off(1)));
        assert_eq!(drained(&shared).len(), QUEUE_CAPACITY);
        assert!(shared.try_push(off(1)));
    }

    /// Spec "A full queue loses nothing".
    #[test]
    fn a_full_ring_with_a_backlog_loses_nothing() {
        let shared = Arc::new(EditorNotes::new());
        let mut sender = NoteSender::new(shared.clone());
        let mut sent = Vec::new();
        for round in 0..3 {
            for note in 0..128u8 {
                sender.press(note, 64, NoteSource::Mouse);
                sender.release(note, NoteSource::Mouse);
                sent.push(on(note, 64));
                sent.push(off(note));
            }
            if round > 0 {
                assert!(sender.pending() > 0, "round {round} never filled the ring");
            }
        }
        let mut received = Vec::new();
        while received.len() < sent.len() {
            received.extend(drained(&shared));
            sender.flush();
        }
        assert_eq!(received, sent);
        assert_eq!(sender.pending(), 0);
    }

    #[test]
    fn wrapping_indices_keep_working() {
        let shared = EditorNotes::new();
        shared.head.store(usize::MAX - 2, Ordering::Relaxed);
        shared.tail.store(usize::MAX - 2, Ordering::Relaxed);
        for note in 0..8 {
            assert!(shared.try_push(off(note)));
        }
        let notes: Vec<_> = drained(&shared);
        assert_eq!(notes, (0..8).map(off).collect::<Vec<_>>());
    }

    /// Spec "The home row plays C4 to C5".
    #[test]
    fn the_home_row_plays_c4_to_c5() {
        let shared = Arc::new(EditorNotes::new());
        let mut sender = NoteSender::new(shared.clone());
        for letter in "AWSEDFTGYHUJK".chars() {
            sender.key_event(letter, true, false, true);
        }
        let notes: Vec<_> = drained(&shared)
            .into_iter()
            .map(|event| match event {
                EditorNoteEvent::On { note, velocity } => {
                    assert_eq!(velocity, KEY_VELOCITY);
                    note
                }
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(notes, (60..=72).collect::<Vec<_>>());
        for note in 60..=72 {
            assert_eq!(key_note(note_letter(note).unwrap()), Some(note));
        }
        assert_eq!(note_letter(59), None);
        assert_eq!(note_letter(73), None);
    }

    /// Spec "Auto-repeat does not retrigger".
    #[test]
    fn auto_repeat_does_not_retrigger() {
        let shared = Arc::new(EditorNotes::new());
        let mut sender = NoteSender::new(shared.clone());
        sender.key_event('A', true, false, true);
        for _ in 0..5 {
            sender.key_event('A', true, true, true);
        }
        sender.key_event('A', false, false, true);
        assert_eq!(drained(&shared), vec![on(60, KEY_VELOCITY), off(60)]);
    }

    /// A key still down when focus returns keeps sending repeats; they must
    /// not restart the note the focus loss released.
    #[test]
    fn repeats_after_a_focus_loss_do_not_restart_the_note() {
        let shared = Arc::new(EditorNotes::new());
        let mut sender = NoteSender::new(shared.clone());
        sender.key_event('A', true, false, true);
        sender.release_all_held();
        let _ = drained(&shared);
        for _ in 0..3 {
            sender.key_event('A', true, true, true);
        }
        assert!(drained(&shared).is_empty());
    }

    /// Spec "Unmapped keys do nothing".
    #[test]
    fn unmapped_keys_do_nothing() {
        let shared = Arc::new(EditorNotes::new());
        let mut sender = NoteSender::new(shared.clone());
        for letter in "QZXLP1;".chars() {
            sender.key_event(letter, true, false, true);
            sender.key_event(letter, false, false, true);
        }
        assert!(drained(&shared).is_empty());
    }

    /// Spec "Unfocused editor ignores keys" (and a text field taking input,
    /// which closes the same gate).
    #[test]
    fn a_closed_gate_ignores_key_presses() {
        let shared = Arc::new(EditorNotes::new());
        let mut sender = NoteSender::new(shared.clone());
        for letter in "AWS".chars() {
            sender.key_event(letter, true, false, false);
            sender.key_event(letter, false, false, false);
        }
        assert!(drained(&shared).is_empty());
    }

    /// Spec "Losing focus releases held notes".
    #[test]
    fn losing_focus_releases_key_and_mouse_notes_once() {
        let shared = Arc::new(EditorNotes::new());
        let mut sender = NoteSender::new(shared.clone());
        sender.key_event('A', true, false, true);
        sender.press(67, 80, NoteSource::Mouse);
        let _ = drained(&shared);
        sender.release_all_held();
        assert_eq!(drained(&shared), vec![off(60), off(67)]);
        sender.key_event('A', false, false, true);
        sender.release(67, NoteSource::Mouse);
        assert!(drained(&shared).is_empty());
    }

    #[test]
    fn a_note_held_by_mouse_and_key_stops_when_both_let_go() {
        let shared = Arc::new(EditorNotes::new());
        let mut sender = NoteSender::new(shared.clone());
        sender.press(60, 80, NoteSource::Mouse);
        sender.key_event('A', true, false, true);
        sender.release(60, NoteSource::Mouse);
        assert_eq!(drained(&shared), vec![on(60, 80)]);
        sender.key_event('A', false, false, true);
        assert_eq!(drained(&shared), vec![off(60)]);
    }

    /// Spec "Closing the editor releases held notes": the editor's state
    /// outlives its window, so the audio thread sees the close as the
    /// window's open flag falling, and releases every editor note then.
    #[test]
    fn a_closing_window_releases_every_editor_note() {
        let shared = Arc::new(EditorNotes::new());
        let mut audio = EditorNoteState::default();
        let mut sender = NoteSender::new(shared.clone());
        let mut handled = Vec::new();
        audio.observe_editor_open(true, &shared);
        sender.press(60, 80, NoteSource::Mouse);
        sender.key_event('D', true, false, true);
        audio.drain(&shared, |event| handled.push(event));
        handled.clear();
        // Still open: nothing is released.
        audio.observe_editor_open(true, &shared);
        audio.drain(&shared, |event| handled.push(event));
        assert!(handled.is_empty());
        audio.observe_editor_open(false, &shared);
        audio.drain(&shared, |event| handled.push(event));
        assert_eq!(handled, vec![off(60), off(64)]);
        // Reopened: the strip starts clean and a key plays again.
        handled.clear();
        audio.observe_editor_open(true, &shared);
        sender.forget_held();
        sender.key_event('D', true, false, true);
        audio.drain(&shared, |event| handled.push(event));
        assert_eq!(handled, vec![on(64, KEY_VELOCITY)]);
    }

    /// A never-opened editor (tests, offline rendering) releases nothing.
    #[test]
    fn a_never_opened_editor_releases_nothing() {
        let shared = EditorNotes::new();
        let mut audio = EditorNoteState::default();
        shared.try_push(on(60, 80));
        audio.observe_editor_open(false, &shared);
        let mut handled = Vec::new();
        audio.drain(&shared, |event| handled.push(event));
        assert_eq!(handled, vec![on(60, 80)]);
    }

    /// Dropping the sender (the plugin itself going away)
    /// releases its notes, and the release-all reaches the audio side even
    /// when the note-offs could not be queued.
    #[test]
    fn closing_the_editor_releases_every_editor_note() {
        let shared = Arc::new(EditorNotes::new());
        let mut audio = EditorNoteState::default();
        let mut sender = NoteSender::new(shared.clone());
        sender.press(60, 80, NoteSource::Mouse);
        sender.key_event('D', true, false, true);
        let mut handled = Vec::new();
        audio.drain(&shared, |event| handled.push(event));
        // Fill the ring so the drop's note-offs cannot be queued.
        while shared.try_push(on(100, 1)) {}
        drop(sender);
        handled.clear();
        audio.drain(&shared, |event| handled.push(event));
        let tail: Vec<_> = handled
            .iter()
            .copied()
            .filter(|event| !matches!(event, EditorNoteEvent::On { note: 100, .. }))
            .collect();
        assert_eq!(tail, vec![off(60), off(64), off(100)]);
    }

    /// The release-all only stops notes the editor turned on.
    #[test]
    fn release_all_touches_only_editor_notes() {
        let shared = Arc::new(EditorNotes::new());
        let mut audio = EditorNoteState::default();
        shared.try_push(on(60, 80));
        shared.try_push(on(62, 80));
        shared.try_push(off(60));
        shared.request_release_all();
        let mut handled = Vec::new();
        audio.drain(&shared, |event| handled.push(event));
        assert_eq!(handled, vec![on(60, 80), on(62, 80), off(60), off(62)]);
        handled.clear();
        shared.request_release_all();
        audio.drain(&shared, |event| handled.push(event));
        assert!(handled.is_empty());
    }

    #[test]
    fn the_held_mask_round_trips() {
        let shared = EditorNotes::new();
        shared.publish_held([0u8, 48, 63, 64, 84, 127].into_iter());
        let held = shared.held();
        assert_eq!(
            held.iter().collect::<Vec<_>>(),
            vec![0, 48, 63, 64, 84, 127]
        );
        shared.publish_held(std::iter::empty());
        assert!(shared.held().is_empty());
    }

    #[test]
    fn black_keys_are_the_sharps() {
        let blacks: Vec<_> = (60..72).filter(|&n| is_black_key(n)).collect();
        assert_eq!(blacks, vec![61, 63, 66, 68, 70]);
    }
}

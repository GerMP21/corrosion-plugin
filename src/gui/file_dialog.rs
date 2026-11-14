//! Import and Export's system file dialogs (`add-preset-import-export`).
//!
//! The dialog runs on its own short-lived thread on Windows and Linux, so the
//! editor keeps drawing while it is open, and its answer comes back through a
//! one-slot channel the editor polls each frame. macOS needs AppKit's dialogs
//! on the main thread, where the editor already runs, so there it opens
//! modally in place. Audio never waits on any of this.
//!
//! On Linux `rfd` asks the XDG desktop portal (then zenity). When neither
//! answers it only logs the error and returns nothing, exactly as if the user
//! had cancelled. A person can't cancel a dialog within [`UNSEEN_DIALOG`], so
//! an empty answer faster than that means no dialog was shown.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::presets::PRESET_EXTENSION;

/// An empty answer faster than this means no dialog was shown.
pub const UNSEEN_DIALOG: Duration = Duration::from_millis(250);

/// Which dialog is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Pick one or more preset files to import.
    Import,
    /// Pick where to write the current sound, proposing `file_name`.
    Export { file_name: String },
}

/// What a dialog answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The paths picked (one for Export).
    Picked(Vec<PathBuf>),
    /// The user closed the dialog without picking.
    Cancelled,
    /// No dialog could be shown.
    Unavailable,
}

/// Turn a dialog's raw result and how long it took into an [`Answer`].
pub fn classify(picked: Option<Vec<PathBuf>>, elapsed: Duration) -> Answer {
    match picked {
        Some(paths) if !paths.is_empty() => Answer::Picked(paths),
        _ if elapsed < UNSEEN_DIALOG => Answer::Unavailable,
        _ => Answer::Cancelled,
    }
}

/// An open dialog whose answer hasn't been read yet.
pub struct Pending {
    pub request: Request,
    // In a `Mutex` only because the editor's state must be `Sync`; the GUI
    // thread is the only reader.
    answer: Mutex<Receiver<Answer>>,
}

impl Pending {
    /// The answer, once the dialog has closed. A dialog thread that died
    /// without answering counts as no dialog.
    pub fn poll(&self) -> Option<Answer> {
        let Ok(answer) = self.answer.lock() else {
            return Some(Answer::Unavailable);
        };
        match answer.try_recv() {
            Ok(answer) => Some(answer),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Answer::Unavailable),
        }
    }
}

/// Open the dialog for `request`.
pub fn open(request: Request) -> Pending {
    let (send, answer) = mpsc::sync_channel(1);
    let shown = request.clone();
    let ask = move || {
        let start = Instant::now();
        let picked = show(&shown);
        let _ = send.send(classify(picked, start.elapsed()));
    };
    if cfg!(target_os = "macos") {
        ask();
    } else if std::thread::Builder::new()
        .name("corrosion-file-dialog".into())
        .spawn(ask)
        .is_err()
    {
        // The closure (and its sender) is dropped: `poll` reads that as
        // no dialog.
    }
    Pending {
        request,
        answer: Mutex::new(answer),
    }
}

/// Show the dialog and wait for it (blocking the calling thread).
fn show(request: &Request) -> Option<Vec<PathBuf>> {
    let dialog = rfd::FileDialog::new().add_filter("Corrosion preset", &[PRESET_EXTENSION]);
    match request {
        Request::Import => dialog.set_title("Import presets").pick_files(),
        Request::Export { file_name } => dialog
            .set_title("Export preset")
            .set_file_name(format!("{file_name}.{PRESET_EXTENSION}"))
            .save_file()
            .map(|path| vec![path]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pick_is_a_pick_however_fast() {
        let paths = vec![PathBuf::from("a.corrosion-preset")];
        assert_eq!(
            classify(Some(paths.clone()), Duration::ZERO),
            Answer::Picked(paths)
        );
    }

    #[test]
    fn an_instant_empty_answer_means_no_dialog() {
        assert_eq!(
            classify(None, Duration::from_millis(5)),
            Answer::Unavailable
        );
        assert_eq!(
            classify(Some(Vec::new()), Duration::ZERO),
            Answer::Unavailable
        );
    }

    #[test]
    fn a_slow_empty_answer_is_a_cancel() {
        assert_eq!(classify(None, Duration::from_secs(3)), Answer::Cancelled);
        assert_eq!(classify(None, UNSEEN_DIALOG), Answer::Cancelled);
    }

    #[test]
    fn a_dead_dialog_thread_counts_as_no_dialog() {
        let (send, answer) = mpsc::sync_channel::<Answer>(1);
        drop(send);
        let pending = Pending {
            request: Request::Import,
            answer: Mutex::new(answer),
        };
        assert_eq!(pending.poll(), Some(Answer::Unavailable));
    }
}

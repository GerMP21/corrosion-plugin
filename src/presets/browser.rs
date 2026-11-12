//! User presets kept by a web page instead of a folder (`add-web-app`).
//!
//! In the browser build the editor has no file system: its user presets live
//! here, in memory. The page fills the store from its own storage
//! (IndexedDB) at start with [`replace_all`], and persists what the editor
//! saves by draining [`take_saved`]. Folders are `/`-separated paths, as in
//! a user preset folder; a preset's key is its folder and sanitized file
//! name, so saving the same name in the same folder replaces it (after the
//! editor's confirmation, as on disk).

use std::io;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::{sanitize_file_name, user_subfolder_name, Preset, SaveOutcome, UserPreset};
use crate::CorrosionParams;

/// One stored preset, as the page keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserPreset {
    /// Folder, `/`-separated, `""` for the root.
    pub folder: String,
    /// File name without extension (`sanitize_file_name` of the name).
    pub file: String,
    /// The `.corrosion-preset` JSON.
    pub json: String,
}

impl BrowserPreset {
    fn same_place(&self, other: &Self) -> bool {
        self.folder == other.folder && self.file.eq_ignore_ascii_case(&other.file)
    }
}

static STORE: Mutex<Vec<BrowserPreset>> = Mutex::new(Vec::new());
static SAVED: Mutex<Vec<BrowserPreset>> = Mutex::new(Vec::new());

fn store() -> std::sync::MutexGuard<'static, Vec<BrowserPreset>> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Replace every stored preset (the page's storage, at start).
pub fn replace_all(presets: Vec<BrowserPreset>) {
    *store() = presets;
}

/// The stored presets that parse, sorted as `list_user_presets` sorts a
/// folder's, and how many don't parse.
pub fn list() -> (Vec<UserPreset>, usize) {
    let mut skipped = 0;
    let mut presets: Vec<UserPreset> = store()
        .iter()
        .filter_map(|stored| match Preset::from_json_str(&stored.json) {
            Ok(preset) => Some(UserPreset {
                path: format!("{}/{}.corrosion-preset", stored.folder, stored.file).into(),
                folder: stored.folder.clone(),
                preset,
            }),
            Err(_) => {
                skipped += 1;
                None
            }
        })
        .collect();
    presets.sort_by_key(|p| {
        (
            !p.folder.is_empty(),
            p.folder.to_lowercase(),
            p.preset.name.to_lowercase(),
        )
    });
    (presets, skipped)
}

/// The folders holding stored presets, root first (always present).
pub fn folders() -> Vec<String> {
    let mut folders = vec![String::new()];
    for stored in store().iter() {
        if !folders.contains(&stored.folder) {
            folders.push(stored.folder.clone());
        }
    }
    folders[1..].sort_by_key(|f| f.to_lowercase());
    folders
}

/// Save `params` as `name` in `folder`, as `save_user_preset` does on
/// disk: an existing preset there is replaced only with `overwrite`.
pub fn save(
    folder: &str,
    name: &str,
    params: &CorrosionParams,
    overwrite: bool,
) -> io::Result<SaveOutcome> {
    let name = name.trim();
    if name.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "enter a preset name",
        ));
    }
    let folder = user_subfolder_name(folder)?;
    let json = serde_json::to_string_pretty(&Preset::from_params(name, params))
        .map_err(io::Error::other)?;
    Ok(put(
        BrowserPreset {
            folder,
            file: sanitize_file_name(name),
            json,
        },
        overwrite,
    ))
}

/// Add a preset file the visitor opened, in the root under its own name,
/// replacing one there. Returns the parsed preset, or the parse error.
pub fn import(json: &str) -> io::Result<Preset> {
    let preset = Preset::from_json_str(json)?;
    put(
        BrowserPreset {
            folder: String::new(),
            file: sanitize_file_name(&preset.name),
            json: json.to_string(),
        },
        true,
    );
    Ok(preset)
}

fn put(preset: BrowserPreset, overwrite: bool) -> SaveOutcome {
    let path = format!("{}/{}.corrosion-preset", preset.folder, preset.file).into();
    let mut store = store();
    match store.iter().position(|stored| stored.same_place(&preset)) {
        Some(_) if !overwrite => return SaveOutcome::NeedsConfirmation(path),
        Some(at) => store[at] = preset.clone(),
        None => store.push(preset.clone()),
    }
    SAVED.lock().unwrap_or_else(|e| e.into_inner()).push(preset);
    SaveOutcome::Saved(path)
}

/// The presets saved or imported since the last call, for the page to
/// persist.
pub fn take_saved() -> Vec<BrowserPreset> {
    std::mem::take(&mut *SAVED.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The store is process-global: one test drives it start to finish.
    #[test]
    fn saves_lists_confirms_imports_and_reports() {
        replace_all(Vec::new());
        take_saved();
        let params = CorrosionParams::default();
        assert!(matches!(
            save("", "Deep", &params, false),
            Ok(SaveOutcome::Saved(_))
        ));
        assert!(matches!(
            save("Basses", "Low", &params, false),
            Ok(SaveOutcome::Saved(_))
        ));
        // Same name and folder: confirm first, then replace.
        assert!(matches!(
            save("", "deep", &params, false),
            Ok(SaveOutcome::NeedsConfirmation(_))
        ));
        assert!(matches!(
            save("", "Deep", &params, true),
            Ok(SaveOutcome::Saved(_))
        ));
        // A folder outside the store is refused, as on disk.
        assert!(save("../x", "Bad", &params, false).is_err());
        assert!(save("", "  ", &params, false).is_err());

        let (presets, skipped) = list();
        assert_eq!(skipped, 0);
        let names: Vec<_> = presets
            .iter()
            .map(|p| (p.folder.as_str(), p.preset.name.as_str()))
            .collect();
        assert_eq!(names, [("", "Deep"), ("Basses", "Low")]);
        assert_eq!(folders(), ["", "Basses"]);

        let saved = take_saved();
        assert_eq!(saved.len(), 3);
        assert!(take_saved().is_empty());

        // A file from the plugin, then an unreadable entry.
        let json = serde_json::to_string(&Preset::from_params("From Disk", &params)).unwrap();
        assert_eq!(import(&json).unwrap().name, "From Disk");
        assert!(import("not json").is_err());
        let mut all = store().clone();
        all.push(BrowserPreset {
            folder: String::new(),
            file: "broken".into(),
            json: "{".into(),
        });
        replace_all(all);
        let (presets, skipped) = list();
        assert_eq!((presets.len(), skipped), (3, 1));
        replace_all(Vec::new());
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Where OpenRef keeps its files: its settings, eframe's `app.ron`, in `%APPDATA%\Open3D\OpenRef`, and
//! its recovery files in `%LOCALAPPDATA%\Open3D\OpenRef\recovery`, or in `OpenRef` inside the folders
//! that `OPEN3D_SETTINGS_DIR` and `OPEN3D_DATA_DIR` name (D-042, D-044).
//!
//! At start, before eframe reads its settings, [`prepare`] copies the folders of an OpenRef from
//! before into the new ones, once, and leaves the old ones as they were (D-048, D-049): eframe's own
//! folder, `%APPDATA%\OpenRef\data`, and `%LOCALAPPDATA%\OpenRef`. When a copy fails, that run uses
//! the old folder, as before, and the next start tries again.

use crate::open3d_settings::{self, CarryOver};
use std::path::{Path, PathBuf};

/// The app's name: its folders, and eframe's app id.
pub const APP: &str = "OpenRef";
/// eframe's file in the settings folder.
pub const SETTINGS_FILE: &str = "app.ron";
/// Only debug builds read it: a folder in place of the user's `AppData`, whose `Roaming` and `Local`
/// stand for the old folders, so that a test never reads the person's own.
pub const TEST_OLD_FOLDERS: &str = "OPENREF_TEST_OLD_FOLDERS";

/// The folders this run uses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Folders {
    /// eframe's `app.ron`, with OpenRef's settings and the window's place.
    pub settings_file: Option<PathBuf>,
    /// The autosaves of boards with changes (`recovery.rs`).
    pub recovery: Option<PathBuf>,
    /// What could not be carried over, to tell the person.
    pub problems: Vec<String>,
}

/// The folders of this run, once the old ones have been carried over. Call it before eframe starts.
pub fn prepare() -> Folders {
    let (old_settings, old_data) = old_folders();
    let settings = carry(old_settings, open3d_settings::app_settings_dir(APP));
    let data = carry(old_data, open3d_settings::app_data_dir(APP));
    Folders {
        settings_file: settings.0.map(|dir| dir.join(SETTINGS_FILE)),
        recovery: data.0.map(|dir| dir.join("recovery")),
        problems: settings.1.into_iter().chain(data.1).collect(),
    }
}

/// The folders OpenRef used before: eframe's own folder for `app.ron` and `%LOCALAPPDATA%\OpenRef`.
fn old_folders() -> (Option<PathBuf>, Option<PathBuf>) {
    #[cfg(debug_assertions)]
    if let Some(dir) = std::env::var_os(TEST_OLD_FOLDERS).filter(|dir| !dir.is_empty()) {
        let dir = PathBuf::from(dir);
        return (Some(dir.join("Roaming").join(APP).join("data")), Some(dir.join("Local").join(APP)));
    }
    let local = std::env::var_os("LOCALAPPDATA").filter(|dir| !dir.is_empty()).map(|dir| PathBuf::from(dir).join(APP));
    (eframe::storage_dir(APP), local)
}

/// Carries `old` over into `new` and gives the folder to use, with what went wrong, if anything:
/// `new`, or `old` when the copy failed, so that this run does not make `new` and lose the copy.
fn carry(old: Option<PathBuf>, new: Option<PathBuf>) -> (Option<PathBuf>, Option<String>) {
    let (Some(old), Some(new)) = (old.as_deref(), new.as_deref()) else { return (new.or(old), None) };
    match open3d_settings::carry_over(old, new) {
        CarryOver::Nothing => (Some(new.to_path_buf()), None),
        CarryOver::Copied { left_out, .. } if left_out.is_empty() => (Some(new.to_path_buf()), None),
        CarryOver::Copied { left_out, .. } => (Some(new.to_path_buf()), Some(left_out_text(old, &left_out))),
        CarryOver::Failed(reason) => (Some(old.to_path_buf()), Some(reason)),
    }
}

/// The files of `old` that were left out, and why.
fn left_out_text(old: &Path, left_out: &[(PathBuf, String)]) -> String {
    left_out.iter().map(|(path, why)| format!("{} ({why})", old.join(path).display())).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own, removed when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("openref-folders-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_old_folder_is_copied_once_and_the_new_one_used() {
        let scratch = Scratch::new("copy");
        let (old, new) = (scratch.0.join("Roaming").join("OpenRef").join("data"), scratch.0.join("Open3D").join("OpenRef"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join(SETTINGS_FILE), "{\"settings\": \"(lang:es)\"}").unwrap();
        assert_eq!(carry(Some(old.clone()), Some(new.clone())), (Some(new.clone()), None));
        assert_eq!(std::fs::read(new.join(SETTINGS_FILE)).unwrap(), std::fs::read(old.join(SETTINGS_FILE)).unwrap());
        // From then on the new folder is used as it is, and the old one is not read.
        std::fs::write(new.join(SETTINGS_FILE), "{}").unwrap();
        assert_eq!(carry(Some(old.clone()), Some(new.clone())), (Some(new.clone()), None));
        assert_eq!(std::fs::read(new.join(SETTINGS_FILE)).unwrap(), b"{}");
        assert_eq!(std::fs::read(old.join(SETTINGS_FILE)).unwrap(), b"{\"settings\": \"(lang:es)\"}");
    }

    #[test]
    fn a_copy_that_fails_leaves_the_old_folder_in_use_and_says_why() {
        let scratch = Scratch::new("fail");
        let old = scratch.0.join("old");
        std::fs::create_dir_all(&old).unwrap();
        // The folder the new one goes in is a file.
        let blocked = scratch.0.join("Open3D");
        std::fs::write(&blocked, b"a file").unwrap();
        let (folder, problem) = carry(Some(old.clone()), Some(blocked.join("OpenRef")));
        assert_eq!(folder, Some(old));
        assert!(problem.is_some_and(|problem| !problem.is_empty()));
    }

    #[test]
    fn without_one_of_the_folders_nothing_is_copied() {
        let scratch = Scratch::new("none");
        let old = scratch.0.join("old");
        std::fs::create_dir_all(&old).unwrap();
        let new = scratch.0.join("new");
        assert_eq!(carry(Some(old.clone()), None), (Some(old), None));
        assert_eq!(carry(None, Some(new.clone())), (Some(new.clone()), None));
        assert!(!new.exists());
        assert_eq!(carry(None, None), (None, None));
    }

    #[test]
    fn a_file_left_out_is_named_with_why() {
        let text = left_out_text(Path::new("old"), &[(Path::new("recovery").join("a.oref"), "in use".into())]);
        assert_eq!(text, format!("{} (in use)", Path::new("old").join("recovery").join("a.oref").display()));
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later
//! Autosave: while a board has unsaved changes, a copy is written now and then to
//! `%LOCALAPPDATA%\OpenRef\recovery` (a normal `.oref`, named after this process). A clean close
//! removes it; if OpenRef stopped without closing, the next start finds it and offers it back.

use crate::board_file::{self, View};
use crate::images::Source;
use crate::scene::{ImageKey, Scene};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

/// Seconds between autosaves while there are changes.
pub const EVERY: f64 = 60.0;

#[derive(Serialize, Deserialize)]
struct Meta {
    /// The board's own file, when it had one.
    original: Option<PathBuf>,
}

/// A copy left by an OpenRef that did not close.
#[derive(Clone, Debug)]
pub struct Orphan {
    pub board: PathBuf,
    pub original: Option<PathBuf>,
}

impl Orphan {
    /// Removes the copy and its notes.
    pub fn discard(&self) {
        let _ = std::fs::remove_file(&self.board);
        let _ = std::fs::remove_file(self.board.with_extension("json"));
    }
}

pub fn folder() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("OpenRef").join("recovery"))
}

/// The newest copy whose OpenRef is no longer running.
pub fn find_orphan(dir: &Path, alive: impl Fn(u32) -> bool) -> Option<Orphan> {
    let mut found: Vec<(std::time::SystemTime, Orphan)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            let pid: u32 = name.strip_prefix("autosave-")?.strip_suffix(".oref")?.parse().ok()?;
            if alive(pid) {
                return None;
            }
            let modified = entry.metadata().and_then(|m| m.modified()).ok()?;
            let original = std::fs::read(path.with_extension("json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Meta>(&bytes).ok())
                .and_then(|meta| meta.original);
            Some((modified, Orphan { board: path, original }))
        })
        .collect();
    found.sort_by_key(|(time, _)| *time);
    found.pop().map(|(_, orphan)| orphan)
}

pub struct Autosave {
    dir: Option<PathBuf>,
    pid: u32,
    /// The scene last written, so an unchanged board is not written again.
    written: Option<Scene>,
    /// When the last copy was started (seconds, egui time).
    last: f64,
    running: Option<Receiver<Result<(), String>>>,
}

impl Autosave {
    pub fn new(dir: Option<PathBuf>) -> Self {
        Self { dir, pid: std::process::id(), written: None, last: 0.0, running: None }
    }

    fn board_path(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(format!("autosave-{}.oref", self.pid)))
    }

    /// Writes a copy in the background when it is time and the board changed since the last one.
    pub fn tick(
        &mut self,
        now: f64,
        dirty: bool,
        scene: &Scene,
        view: View,
        sources: impl FnOnce() -> HashMap<ImageKey, Source>,
        original: Option<PathBuf>,
    ) {
        if let Some(rx) = &self.running {
            match rx.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => self.running = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
            }
        }
        if !dirty || scene.is_empty() || now - self.last < EVERY || self.written.as_ref() == Some(scene) {
            return;
        }
        let (Some(dir), Some(path)) = (self.dir.clone(), self.board_path()) else { return };
        self.last = now;
        self.written = Some(scene.clone());
        let scene = scene.clone();
        let sources = sources();
        let (tx, rx) = channel();
        self.running = Some(rx);
        std::thread::spawn(move || {
            let result = (|| {
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                board_file::save(&path, &scene, Some(view), |key| sources.get(&key).cloned())?;
                let meta = serde_json::to_vec(&Meta { original }).map_err(|e| e.to_string())?;
                std::fs::write(path.with_extension("json"), meta).map_err(|e| e.to_string())
            })();
            let _ = tx.send(result);
        });
    }

    /// Nothing to recover any more (saved, or closed on purpose).
    pub fn forget(&mut self) {
        if let Some(rx) = self.running.take() {
            let _ = rx.recv();
        }
        if let Some(path) = self.board_path() {
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(path.with_extension("json"));
        }
        self.written = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::encode_png;
    use crate::scene::Item;
    use egui::{pos2, Pos2};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("openref-recovery-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_copy_is_written_found_after_a_crash_and_forgotten_after_a_save() {
        let dir = scratch("cycle");
        let source = Source::from_bytes(encode_png(4, 4, &[9u8; 64]).unwrap().into(), "a.png".into(), None).unwrap();
        let mut scene = Scene::default();
        scene.items.push(Item::image(1, 1, 4, 4, Pos2::ZERO));
        let mut auto = Autosave::new(Some(dir.clone()));
        let view = View { center: pos2(0.0, 0.0), zoom: 1.0 };
        let sources: HashMap<u64, Source> = HashMap::from([(1, source)]);
        let original = Some(PathBuf::from("board.oref"));
        auto.tick(100.0, true, &scene, view, || sources.clone(), original.clone());
        let _ = auto.running.take().unwrap().recv();
        // Our own process is alive: nothing to offer.
        assert!(find_orphan(&dir, |_| true).is_none());
        // After a "crash" it is offered, with the board's own file.
        let orphan = find_orphan(&dir, |_| false).expect("the copy");
        assert_eq!(orphan.original, original);
        let board = board_file::load(&orphan.board).unwrap();
        assert_eq!(board.scene.items, scene.items);
        // Unchanged boards are not written twice; a save forgets the copy.
        auto.tick(500.0, true, &scene, view, || sources, None);
        assert!(auto.running.is_none());
        auto.forget();
        assert!(find_orphan(&dir, |_| false).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn nothing_is_written_without_changes_or_too_soon() {
        let dir = scratch("quiet");
        let mut scene = Scene::default();
        scene.items.push(Item::note(1, "x".into(), Pos2::ZERO, egui::vec2(10.0, 10.0)));
        let mut auto = Autosave::new(Some(dir.clone()));
        let view = View { center: Pos2::ZERO, zoom: 1.0 };
        auto.tick(100.0, false, &scene, view, HashMap::new, None);
        assert!(auto.running.is_none(), "not dirty");
        auto.tick(30.0, true, &scene, view, HashMap::new, None);
        assert!(auto.running.is_none(), "too soon after start");
        let _ = std::fs::remove_dir_all(dir);
    }
}

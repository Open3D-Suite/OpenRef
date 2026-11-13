// SPDX-License-Identifier: GPL-3.0-or-later
//! Undo and redo. A step is the whole list of items before a change: items are small (pixels
//! live in the image store, which only grows), so snapshots are simple and cannot drift.

use crate::scene::Scene;

pub struct History {
    undo: Vec<Scene>,
    redo: Vec<Scene>,
    limit: usize,
}

impl Default for History {
    fn default() -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), limit: 500 }
    }
}

impl History {
    /// Remembers the scene as it was before a change.
    pub fn record(&mut self, before: Scene) {
        self.undo.push(before);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// The scene to go back to, given the current one.
    pub fn undo(&mut self, current: &Scene) -> Option<Scene> {
        let previous = self.undo.pop()?;
        self.redo.push(current.clone());
        Some(previous)
    }

    pub fn redo(&mut self, current: &Scene) -> Option<Scene> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        Some(next)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Item;
    use egui::Pos2;

    fn scene_with(n: u64) -> Scene {
        let mut scene = Scene::default();
        for id in 1..=n {
            scene.items.push(Item::image(id, 1, 10, 10, Pos2::ZERO));
        }
        scene
    }

    #[test]
    fn undo_and_redo_walk_the_steps() {
        let mut history = History::default();
        let (a, b, c) = (scene_with(1), scene_with(2), scene_with(3));
        history.record(a.clone());
        history.record(b.clone());
        assert_eq!(history.undo(&c), Some(b.clone()));
        assert_eq!(history.undo(&b), Some(a.clone()));
        assert_eq!(history.undo(&a), None);
        assert_eq!(history.redo(&a), Some(b.clone()));
        assert_eq!(history.redo(&b), Some(c));
        assert!(!history.can_redo());
    }

    #[test]
    fn a_new_change_forgets_what_could_be_redone() {
        let mut history = History::default();
        history.record(scene_with(1));
        history.undo(&scene_with(2));
        assert!(history.can_redo());
        history.record(scene_with(1));
        assert!(!history.can_redo());
    }
}

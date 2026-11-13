// SPDX-License-Identifier: GPL-3.0-or-later
//! The board on screen: the camera, drawing the items, and the mouse gestures that select, move,
//! scale and rotate them. Every change to the items goes through [`Editor::change`] (or a gesture
//! that records the scene it started from), so it can be undone.

use crate::board_file::View;
use crate::history::History;
use crate::images::{ImageStore, Source, Textures};
use crate::scene::{wrap_angle, ImageKey, Item, ItemId, ItemKind, Scene};
use egui::{
    pos2, vec2, Align2, Color32, CursorIcon, FontId, Mesh, PointerButton, Pos2, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2,
};
use std::collections::BTreeSet;
use std::f32::consts::FRAC_PI_4;

pub const BACKGROUND: Color32 = Color32::from_gray(36);
const ACCENT: Color32 = Color32::from_rgb(0, 160, 255);
const HANDLE: f32 = 5.0;
const ROTATE_RING: f32 = 22.0;
const MIN_ZOOM: f32 = 0.005;
const MAX_ZOOM: f32 = 100.0;

/// World point at the middle of the board area, and screen points per world unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub center: Pos2,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self { center: Pos2::ZERO, zoom: 1.0 }
    }
}

impl Camera {
    pub fn to_screen(&self, viewport: Rect, world: Pos2) -> Pos2 {
        viewport.center() + (world - self.center) * self.zoom
    }

    pub fn to_world(&self, viewport: Rect, screen: Pos2) -> Pos2 {
        self.center + (screen - viewport.center()) / self.zoom
    }

    /// Zooms keeping the world point under `screen` where it is.
    pub fn zoom_at(&mut self, viewport: Rect, screen: Pos2, factor: f32) {
        let before = self.to_world(viewport, screen);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.to_world(viewport, screen);
        self.center += before - after;
    }

    /// Shows a world rectangle as large as fits, with a margin in screen points.
    pub fn fit(&mut self, viewport: Rect, rect: Rect, margin: f32) {
        let room = (viewport.size() - Vec2::splat(2.0 * margin)).max(Vec2::splat(1.0));
        let size = rect.size().max(Vec2::splat(1.0));
        self.zoom = (room.x / size.x).min(room.y / size.y).clamp(MIN_ZOOM, MAX_ZOOM);
        self.center = rect.center();
    }

    pub fn view(&self) -> View {
        View { center: self.center, zoom: self.zoom }
    }
}

enum Gesture {
    None,
    Move { start: Pos2, before: Scene },
    Scale { anchor: Pos2, start: Pos2, before: Scene },
    Rotate { pivot: Pos2, start_angle: f32, before: Scene },
    Marquee { start: Pos2, base: BTreeSet<ItemId> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Handle {
    /// A corner of the selection frame (0 = top-left, clockwise): drag to scale.
    Corner(usize),
    /// Just outside a corner: drag to rotate.
    Rotate,
}

pub struct Editor {
    pub scene: Scene,
    pub selection: BTreeSet<ItemId>,
    pub history: History,
    pub images: ImageStore,
    pub camera: Camera,
    /// Changed since the board was last saved or opened.
    pub dirty: bool,
    /// The board area on screen in the last frame.
    pub viewport: Rect,
    gesture: Gesture,
    /// Fit everything in view once the board area is known.
    fit_pending: bool,
}

impl Editor {
    pub fn new(images: ImageStore) -> Self {
        Self {
            scene: Scene::default(),
            selection: BTreeSet::new(),
            history: History::default(),
            images,
            camera: Camera::default(),
            dirty: false,
            viewport: Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0)),
            gesture: Gesture::None,
            fit_pending: false,
        }
    }

    /// Replaces the board (new or opened). `view` = where to look from; None fits everything.
    pub fn replace(&mut self, scene: Scene, images: Vec<(ImageKey, Source)>, view: Option<View>) {
        self.images.clear();
        for (key, source) in images {
            self.images.insert(key, source);
        }
        self.scene = scene;
        self.selection.clear();
        self.history.clear();
        self.gesture = Gesture::None;
        self.dirty = false;
        match view {
            Some(view) => self.camera = Camera { center: view.center, zoom: view.zoom },
            None => {
                self.camera = Camera::default();
                self.fit_pending = true;
            }
        }
    }

    /// Applies an undoable change to the items.
    pub fn change(&mut self, f: impl FnOnce(&mut Scene, &BTreeSet<ItemId>)) {
        let before = self.scene.clone();
        f(&mut self.scene, &self.selection);
        if self.scene != before {
            self.history.record(before);
            self.dirty = true;
        }
        self.drop_missing_from_selection();
    }

    fn drop_missing_from_selection(&mut self) {
        let ids = self.scene.ids();
        self.selection.retain(|id| ids.contains(id));
    }

    pub fn undo(&mut self) {
        self.gesture = Gesture::None;
        if let Some(scene) = self.history.undo(&self.scene) {
            self.scene = scene;
            self.dirty = true;
            self.drop_missing_from_selection();
        }
    }

    pub fn redo(&mut self) {
        self.gesture = Gesture::None;
        if let Some(scene) = self.history.redo(&self.scene) {
            self.scene = scene;
            self.dirty = true;
            self.drop_missing_from_selection();
        }
    }

    /// The selection, or every item when nothing is selected.
    pub fn targets(&self) -> BTreeSet<ItemId> {
        if self.selection.is_empty() {
            self.scene.ids()
        } else {
            self.selection.clone()
        }
    }

    /// Puts new images on the board in a row centred on `at` (world) and selects them.
    pub fn add_images(&mut self, sources: Vec<Source>, at: Pos2) {
        if sources.is_empty() {
            return;
        }
        let was_empty = self.scene.is_empty();
        let gap = 10.0;
        let total: f32 = sources.iter().map(|s| s.width as f32 + gap).sum::<f32>() - gap;
        let mut x = at.x - total / 2.0;
        let mut added = Vec::new();
        for source in sources {
            let (w, h) = (source.width, source.height);
            let key = self.images.add(source);
            added.push((key, w, h, pos2(x + w as f32 / 2.0, at.y)));
            x += w as f32 + gap;
        }
        let mut new_ids = BTreeSet::new();
        self.change(|scene, _| {
            for (key, w, h, center) in added {
                let id = scene.new_id();
                scene.items.push(Item::image(id, key, w, h, center));
                new_ids.insert(id);
            }
        });
        self.selection = new_ids;
        if was_empty {
            self.focus(true);
        }
    }

    /// Fits the selection (or everything) in view.
    pub fn focus(&mut self, everything: bool) {
        let ids = if everything { self.scene.ids() } else { self.targets() };
        if let Some(rect) = self.scene.bounds_of(&ids) {
            self.camera.fit(self.viewport, rect, 30.0);
        }
    }

    /// Where pasted things go: under the mouse when it is over the board, else the middle.
    pub fn drop_point(&self, pointer: Option<Pos2>) -> Pos2 {
        match pointer {
            Some(p) if self.viewport.contains(p) => self.camera.to_world(self.viewport, p),
            _ => self.camera.center,
        }
    }

    fn frame_corners(&self) -> Option<[Pos2; 4]> {
        match self.selection.len() {
            0 => None,
            1 => self.scene.get(*self.selection.first()?).map(Item::corners),
            _ => {
                let r = self.scene.bounds_of(&self.selection)?;
                Some([r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()])
            }
        }
    }

    fn handle_at(&self, screen: Pos2) -> Option<Handle> {
        let corners = self.frame_corners()?.map(|c| self.camera.to_screen(self.viewport, c));
        let center = corners.iter().fold(Vec2::ZERO, |acc, c| acc + c.to_vec2()) / 4.0;
        let mut best: Option<(f32, Handle)> = None;
        for (i, corner) in corners.iter().enumerate() {
            let d = corner.distance(screen);
            // Outside means farther from the frame's middle than the corner is.
            let outside = (screen.to_vec2() - center).length() > (corner.to_vec2() - center).length();
            let handle = if d <= HANDLE + 3.0 {
                Some(Handle::Corner(i))
            } else if outside && d <= ROTATE_RING {
                Some(Handle::Rotate)
            } else {
                None
            };
            if let Some(h) = handle {
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, h));
                }
            }
        }
        best.map(|(_, h)| h)
    }

    /// Draws the board and handles the mouse. Returns the board's response (for the menu).
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let rect = ui.max_rect();
        self.viewport = rect;
        if self.fit_pending {
            self.fit_pending = false;
            self.focus(true);
        }
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let pointer = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        let modifiers = ui.input(|i| i.modifiers);

        // Zoom with the wheel (and pinch), around the mouse.
        if let Some(p) = pointer {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            let factor = (scroll.y * 0.0015).exp() * pinch;
            if (factor - 1.0).abs() > f32::EPSILON {
                self.camera.zoom_at(rect, p, factor);
            }
        }
        // Pan with the middle button.
        if response.dragged_by(PointerButton::Middle) {
            self.camera.center -= response.drag_delta() / self.camera.zoom;
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        }

        self.primary_gestures(ui, &response, pointer, modifiers);

        if response.secondary_clicked() {
            if let Some(p) = pointer {
                match self.scene.hit(self.camera.to_world(rect, p)) {
                    Some(id) if !self.selection.contains(&id) => self.selection = BTreeSet::from([id]),
                    None if !modifiers.ctrl => self.selection.clear(),
                    _ => {}
                }
            }
        }

        // Cursor over handles.
        if matches!(self.gesture, Gesture::None) {
            if let Some(p) = pointer {
                match self.handle_at(p) {
                    Some(Handle::Corner(i)) => ui.ctx().set_cursor_icon(if i % 2 == 0 {
                        CursorIcon::ResizeNwSe
                    } else {
                        CursorIcon::ResizeNeSw
                    }),
                    Some(Handle::Rotate) => ui.ctx().set_cursor_icon(CursorIcon::Alias),
                    None => {}
                }
            }
        }

        self.paint(ui, rect, pointer);
        response
    }

    fn primary_gestures(&mut self, ui: &Ui, response: &Response, pointer: Option<Pos2>, modifiers: egui::Modifiers) {
        let rect = self.viewport;
        let world = |editor: &Self, p: Pos2| editor.camera.to_world(rect, p);
        let adding = modifiers.ctrl || modifiers.shift;

        if response.clicked_by(PointerButton::Primary) {
            if let Some(p) = pointer {
                match (self.scene.hit(world(self, p)), adding) {
                    (Some(id), true) => {
                        if !self.selection.remove(&id) {
                            self.selection.insert(id);
                        }
                    }
                    (Some(id), false) => self.selection = BTreeSet::from([id]),
                    (None, false) => self.selection.clear(),
                    (None, true) => {}
                }
            }
        }

        if response.drag_started_by(PointerButton::Primary) {
            let origin = ui.input(|i| i.pointer.press_origin()).or(pointer);
            if let Some(origin) = origin {
                let start = world(self, origin);
                if let Some(handle) = self.handle_at(origin) {
                    let corners = self.frame_corners().unwrap_or([start; 4]);
                    let center = pos2(
                        corners.iter().map(|c| c.x).sum::<f32>() / 4.0,
                        corners.iter().map(|c| c.y).sum::<f32>() / 4.0,
                    );
                    let before = self.scene.clone();
                    self.gesture = match handle {
                        Handle::Corner(i) => {
                            let anchor = if modifiers.alt { center } else { corners[(i + 2) % 4] };
                            Gesture::Scale { anchor, start, before }
                        }
                        Handle::Rotate => {
                            let v = start - center;
                            Gesture::Rotate { pivot: center, start_angle: v.y.atan2(v.x), before }
                        }
                    };
                } else if let Some(id) = self.scene.hit(start) {
                    if !self.selection.contains(&id) {
                        if !adding {
                            self.selection.clear();
                        }
                        self.selection.insert(id);
                    }
                    self.gesture = Gesture::Move { start, before: self.scene.clone() };
                } else {
                    let base = if adding { self.selection.clone() } else { BTreeSet::new() };
                    self.gesture = Gesture::Marquee { start, base };
                }
            }
        }

        if response.dragged_by(PointerButton::Primary) {
            if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                let now = world(self, p);
                let selection = self.selection.clone();
                match &self.gesture {
                    Gesture::Move { start, before } => {
                        let mut delta = now - *start;
                        if modifiers.shift {
                            if delta.x.abs() > delta.y.abs() {
                                delta.y = 0.0;
                            } else {
                                delta.x = 0.0;
                            }
                        }
                        let mut scene = before.clone();
                        scene.translate(&selection, delta);
                        self.scene = scene;
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    }
                    Gesture::Scale { anchor, start, before } => {
                        let d = *start - *anchor;
                        let factor = ((now - *anchor).dot(d) / d.length_sq().max(f32::EPSILON)).max(0.01);
                        let mut scene = before.clone();
                        scene.scale_about(&selection, *anchor, factor);
                        self.scene = scene;
                    }
                    Gesture::Rotate { pivot, start_angle, before } => {
                        let v = now - *pivot;
                        let mut angle = wrap_angle(v.y.atan2(v.x) - start_angle);
                        if modifiers.shift {
                            // Snap the result to 45° steps (the item's own angle when there is one).
                            let base = match selection.first() {
                                Some(id) if selection.len() == 1 => before.get(*id).map_or(0.0, |i| i.rotation),
                                _ => 0.0,
                            };
                            angle = ((base + angle) / FRAC_PI_4).round() * FRAC_PI_4 - base;
                        }
                        let mut scene = before.clone();
                        scene.rotate_about(&selection, *pivot, angle);
                        self.scene = scene;
                    }
                    Gesture::Marquee { start, base } => {
                        let area = Rect::from_two_pos(*start, now);
                        let mut selection = base.clone();
                        selection.extend(self.scene.touching(area));
                        self.selection = selection;
                    }
                    Gesture::None => {}
                }
            }
        }

        if response.drag_stopped() {
            match std::mem::replace(&mut self.gesture, Gesture::None) {
                Gesture::Move { before, .. } | Gesture::Scale { before, .. } | Gesture::Rotate { before, .. } => {
                    if before != self.scene {
                        self.history.record(before);
                        self.dirty = true;
                    }
                }
                Gesture::Marquee { .. } | Gesture::None => {}
            }
        }
    }

    fn paint(&mut self, ui: &Ui, rect: Rect, pointer: Option<Pos2>) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, BACKGROUND);
        let ppp = ui.ctx().pixels_per_point();
        let camera = self.camera;
        let screen = |p: Pos2| camera.to_screen(rect, p);

        for item in &self.scene.items {
            let corners = item.corners().map(screen);
            let bounds = Rect::from_points(&corners);
            if !bounds.intersects(rect) {
                continue;
            }
            let ItemKind::Image { image, crop, grayscale } = &item.kind;
            let (source_w, source_h) = match self.images.source(*image) {
                Some(s) => (s.width as f32, s.height as f32),
                None => (1.0, 1.0),
            };
            // How many screen pixels the whole image would be wide at this zoom.
            let on_screen_px = item.world_size().x * camera.zoom * ppp * (source_w / crop.width().max(1.0));
            let texture = {
                let wanted = self.images.textures(*image, *grayscale).and_then(|t| t.for_width(on_screen_px)).cloned();
                // While the grayscale version is being made, show the colour one.
                wanted.or_else(|| self.images.textures(*image, false).and_then(|t| t.for_width(on_screen_px)).cloned())
            };
            match texture {
                Some(texture) => {
                    let uv = Rect::from_min_max(
                        pos2(crop.min.x / source_w, crop.min.y / source_h),
                        pos2(crop.max.x / source_w, crop.max.y / source_h),
                    );
                    let tint = Color32::from_white_alpha((item.opacity.clamp(0.0, 1.0) * 255.0).round() as u8);
                    let mut mesh = Mesh::with_texture(texture.id());
                    let uvs = [uv.left_top(), uv.right_top(), uv.right_bottom(), uv.left_bottom()];
                    for (pos, uv) in corners.iter().zip(uvs) {
                        mesh.vertices.push(egui::epaint::Vertex { pos: *pos, uv, color: tint });
                    }
                    mesh.add_triangle(0, 1, 2);
                    mesh.add_triangle(0, 2, 3);
                    painter.add(Shape::mesh(mesh));
                }
                None => {
                    let failed = matches!(self.images.textures(*image, false), Some(Textures::Failed(_)) | None);
                    let fill = if failed { Color32::from_rgb(90, 40, 40) } else { Color32::from_gray(55) };
                    painter.add(Shape::convex_polygon(corners.to_vec(), fill, Stroke::NONE));
                    let label = if failed { "Cannot show this image" } else { "Loading…" };
                    if bounds.width() > 80.0 && bounds.height() > 24.0 {
                        painter.text(
                            bounds.center(),
                            Align2::CENTER_CENTER,
                            label,
                            FontId::proportional(13.0),
                            Color32::from_gray(170),
                        );
                    }
                }
            }
        }

        // Outlines: the hovered item, the selected ones, and the frame with its handles.
        let busy = !matches!(self.gesture, Gesture::None);
        if let (Some(p), false) = (pointer, busy) {
            if let Some(item) = self.scene.hit(camera.to_world(rect, p)).and_then(|id| self.scene.get(id)) {
                if !self.selection.contains(&item.id) {
                    let corners = item.corners().map(screen);
                    painter.add(Shape::closed_line(corners.to_vec(), Stroke::new(1.0, ACCENT.gamma_multiply(0.6))));
                }
            }
        }
        for item in self.scene.items.iter().filter(|item| self.selection.contains(&item.id)) {
            let corners = item.corners().map(screen);
            painter.add(Shape::closed_line(corners.to_vec(), Stroke::new(1.5, ACCENT)));
        }
        if let Some(corners) = self.frame_corners() {
            let corners = corners.map(screen);
            if self.selection.len() > 1 {
                painter.add(Shape::closed_line(corners.to_vec(), Stroke::new(1.0, ACCENT.gamma_multiply(0.7))));
            }
            if !matches!(self.gesture, Gesture::Marquee { .. }) {
                for c in corners {
                    let r = Rect::from_center_size(c, Vec2::splat(HANDLE * 2.0));
                    painter.rect(r, 1.0, Color32::WHITE, Stroke::new(1.0, ACCENT), StrokeKind::Middle);
                }
            }
        }
        if let (Gesture::Marquee { start, .. }, Some(p)) = (&self.gesture, ui.input(|i| i.pointer.interact_pos())) {
            let area = Rect::from_two_pos(screen(*start), p);
            painter.rect(area, 0.0, ACCENT.gamma_multiply(0.12), Stroke::new(1.0, ACCENT), StrokeKind::Inside);
        }

        if self.scene.is_empty() {
            let c = rect.center();
            let hint = Color32::from_gray(150);
            painter.text(c - vec2(0.0, 12.0), Align2::CENTER_CENTER, "Paste (Ctrl+V) or drop images here", FontId::proportional(18.0), hint);
            painter.text(
                c + vec2(0.0, 16.0),
                Align2::CENTER_CENTER,
                "Right-click for everything else",
                FontId::proportional(13.0),
                Color32::from_gray(110),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))
    }

    #[test]
    fn screen_and_world_points_round_trip() {
        let camera = Camera { center: pos2(120.0, -40.0), zoom: 0.37 };
        let p = pos2(512.0, 77.0);
        let back = camera.to_screen(viewport(), camera.to_world(viewport(), p));
        assert!((back - p).length() < 1e-3);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_mouse() {
        let mut camera = Camera::default();
        let mouse = pos2(700.0, 100.0);
        let before = camera.to_world(viewport(), mouse);
        camera.zoom_at(viewport(), mouse, 2.5);
        let after = camera.to_world(viewport(), mouse);
        assert!((before - after).length() < 1e-3);
        assert!((camera.zoom - 2.5).abs() < 1e-6);
    }

    #[test]
    fn fitting_shows_the_whole_rectangle() {
        let mut camera = Camera::default();
        let target = Rect::from_min_size(pos2(1000.0, 1000.0), vec2(4000.0, 1000.0));
        camera.fit(viewport(), target, 20.0);
        let a = camera.to_screen(viewport(), target.min);
        let b = camera.to_screen(viewport(), target.max);
        assert!(viewport().shrink(19.0).contains(a) && viewport().shrink(19.0).contains(b));
        assert!((b.x - a.x - 760.0).abs() < 1e-2, "the wide side fills the room");
    }
}

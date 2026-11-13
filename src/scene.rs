// SPDX-License-Identifier: GPL-3.0-or-later
//! What is on a board: items placed in world space, and the geometry to move them around.
//!
//! World units are image pixels at scale 1. An item's local frame has its origin at the item's
//! centre; a local point goes to the world by flipping, scaling, rotating and moving (in that order).

use egui::{pos2, vec2, Color32, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub type ItemId = u64;
/// Which stored image an item shows. Several items can show the same image.
pub type ImageKey = u64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKind,
    /// Centre of the item in world space.
    pub center: Pos2,
    pub scale: f32,
    /// Radians, clockwise on screen (y grows downwards).
    pub rotation: f32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    #[serde(default = "one")]
    pub opacity: f32,
}

fn one() -> f32 {
    1.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemKind {
    Image {
        image: ImageKey,
        /// The part of the image shown, in the image's pixels (the whole image when not cropped).
        crop: Rect,
        #[serde(default)]
        grayscale: bool,
    },
    Note {
        text: String,
        /// Letter size in world units (at scale 1).
        font_size: f32,
        color: Color32,
        fill: Color32,
        /// The note's box, measured from its text (fonts live in the window, so the window
        /// measures and stores it here; the geometry only reads it).
        size: Vec2,
    },
}

pub const NOTE_FONT_SIZE: f32 = 24.0;
pub const NOTE_COLOR: Color32 = Color32::from_gray(235);
pub const NOTE_FILL: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 150);

/// Space around a note's text, as a share of its letter size.
pub const NOTE_PADDING: f32 = 0.5;

impl Item {
    pub fn note(id: ItemId, text: String, center: Pos2, size: Vec2) -> Self {
        Self {
            id,
            kind: ItemKind::Note { text, font_size: NOTE_FONT_SIZE, color: NOTE_COLOR, fill: NOTE_FILL, size },
            center,
            scale: 1.0,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            opacity: 1.0,
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self.kind, ItemKind::Image { .. })
    }

    pub fn image(id: ItemId, image: ImageKey, width: u32, height: u32, center: Pos2) -> Self {
        Self {
            id,
            kind: ItemKind::Image {
                image,
                crop: Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32)),
                grayscale: false,
            },
            center,
            scale: 1.0,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            opacity: 1.0,
        }
    }

    /// Size of the item in its own frame (before scaling).
    pub fn local_size(&self) -> Vec2 {
        match &self.kind {
            ItemKind::Image { crop, .. } => crop.size(),
            ItemKind::Note { size, .. } => size.max(Vec2::splat(1.0)),
        }
    }

    /// Size on the board, ignoring rotation.
    pub fn world_size(&self) -> Vec2 {
        self.local_size() * self.scale
    }

    pub fn image_key(&self) -> Option<ImageKey> {
        match &self.kind {
            ItemKind::Image { image, .. } => Some(*image),
            ItemKind::Note { .. } => None,
        }
    }

    /// Image pixel under a world point (in the whole image, not only the cropped part).
    pub fn image_pixel(&self, world: Pos2) -> Option<Pos2> {
        match &self.kind {
            ItemKind::Image { crop, .. } => Some(crop.center() + self.to_local(world)),
            ItemKind::Note { .. } => None,
        }
    }

    pub fn to_world(&self, local: Vec2) -> Pos2 {
        let flipped = vec2(
            if self.flip_x { -local.x } else { local.x },
            if self.flip_y { -local.y } else { local.y },
        );
        self.center + rotate(flipped * self.scale, self.rotation)
    }

    pub fn to_local(&self, world: Pos2) -> Vec2 {
        let v = rotate(world - self.center, -self.rotation) / self.scale.max(f32::EPSILON);
        vec2(
            if self.flip_x { -v.x } else { v.x },
            if self.flip_y { -v.y } else { v.y },
        )
    }

    /// The four corners in world space: top-left, top-right, bottom-right, bottom-left of the
    /// item's own frame (so a flipped item's "top-left" is wherever its image's top-left went).
    pub fn corners(&self) -> [Pos2; 4] {
        let h = self.local_size() / 2.0;
        [
            self.to_world(vec2(-h.x, -h.y)),
            self.to_world(vec2(h.x, -h.y)),
            self.to_world(vec2(h.x, h.y)),
            self.to_world(vec2(-h.x, h.y)),
        ]
    }

    pub fn contains(&self, world: Pos2) -> bool {
        let local = self.to_local(world);
        let h = self.local_size() / 2.0;
        local.x.abs() <= h.x && local.y.abs() <= h.y
    }

    /// Axis-aligned bounds in world space.
    pub fn bounds(&self) -> Rect {
        Rect::from_points(&self.corners())
    }
}

pub fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Keeps an angle in (-PI, PI].
pub fn wrap_angle(angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let a = angle.rem_euclid(tau);
    if a > std::f32::consts::PI {
        a - tau
    } else {
        a
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    /// Back to front: the last item is drawn on top.
    pub items: Vec<Item>,
    pub next_id: ItemId,
}

impl Scene {
    pub fn new_id(&mut self) -> ItemId {
        self.next_id += 1;
        self.next_id
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }

    /// The topmost item under a world point.
    pub fn hit(&self, world: Pos2) -> Option<ItemId> {
        self.items.iter().rev().find(|item| item.contains(world)).map(|item| item.id)
    }

    /// Items whose bounds touch a world rectangle.
    pub fn touching(&self, rect: Rect) -> Vec<ItemId> {
        self.items.iter().filter(|item| item.bounds().intersects(rect)).map(|item| item.id).collect()
    }

    pub fn bounds_of(&self, ids: &BTreeSet<ItemId>) -> Option<Rect> {
        self.items
            .iter()
            .filter(|item| ids.contains(&item.id))
            .map(Item::bounds)
            .reduce(|a, b| a.union(b))
    }

    pub fn bounds(&self) -> Option<Rect> {
        self.items.iter().map(Item::bounds).reduce(|a, b| a.union(b))
    }

    pub fn ids(&self) -> BTreeSet<ItemId> {
        self.items.iter().map(|item| item.id).collect()
    }

    fn each(&mut self, ids: &BTreeSet<ItemId>, mut f: impl FnMut(&mut Item)) {
        for item in self.items.iter_mut().filter(|item| ids.contains(&item.id)) {
            f(item);
        }
    }

    pub fn translate(&mut self, ids: &BTreeSet<ItemId>, delta: Vec2) {
        self.each(ids, |item| item.center += delta);
    }

    /// Scales items and the distances between them about a fixed world point.
    pub fn scale_about(&mut self, ids: &BTreeSet<ItemId>, anchor: Pos2, factor: f32) {
        self.each(ids, |item| {
            item.center = anchor + (item.center - anchor) * factor;
            item.scale *= factor;
        });
    }

    pub fn rotate_about(&mut self, ids: &BTreeSet<ItemId>, pivot: Pos2, angle: f32) {
        self.each(ids, |item| {
            item.center = pivot + rotate(item.center - pivot, angle);
            item.rotation = wrap_angle(item.rotation + angle);
        });
    }

    /// Mirrors items left-right (or top-bottom) about a vertical (horizontal) line through `pivot`.
    pub fn flip_about(&mut self, ids: &BTreeSet<ItemId>, pivot: Pos2, horizontal: bool) {
        self.each(ids, |item| {
            // Notes move to the mirrored place but their text stays readable.
            let mirror = item.is_image();
            if horizontal {
                item.center.x = 2.0 * pivot.x - item.center.x;
                item.flip_x ^= mirror;
            } else {
                item.center.y = 2.0 * pivot.y - item.center.y;
                item.flip_y ^= mirror;
            }
            item.rotation = wrap_angle(-item.rotation);
        });
    }

    /// Back to scale 1, no rotation and no flips, keeping each centre.
    pub fn reset_transform(&mut self, ids: &BTreeSet<ItemId>) {
        self.each(ids, |item| {
            item.scale = 1.0;
            item.rotation = 0.0;
            item.flip_x = false;
            item.flip_y = false;
        });
    }

    pub fn set_grayscale(&mut self, ids: &BTreeSet<ItemId>, on: bool) {
        self.each(ids, |item| {
            if let ItemKind::Image { grayscale, .. } = &mut item.kind {
                *grayscale = on;
            }
        });
    }

    /// Shows another part of an image, keeping the image itself where it is on the board.
    /// `crop` is in image pixels and is kept inside `image_size`, at least one pixel big.
    pub fn set_crop(&mut self, id: ItemId, crop: Rect, image_size: Vec2) {
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else { return };
        let ItemKind::Image { crop: current, .. } = &item.kind else { return };
        let current = *current;
        let full = Rect::from_min_size(Pos2::ZERO, image_size);
        let mut crop = crop.intersect(full);
        if crop.width() < 1.0 {
            crop.max.x = (crop.min.x + 1.0).min(full.max.x);
            crop.min.x = crop.max.x - 1.0;
        }
        if crop.height() < 1.0 {
            crop.max.y = (crop.min.y + 1.0).min(full.max.y);
            crop.min.y = crop.max.y - 1.0;
        }
        let shift = crop.center() - current.center();
        item.center = item.to_world(shift);
        if let ItemKind::Image { crop: slot, .. } = &mut item.kind {
            *slot = crop;
        }
    }

    /// Shows whole images again.
    pub fn reset_crop(&mut self, ids: &BTreeSet<ItemId>, image_size: impl Fn(ImageKey) -> Option<Vec2>) {
        let crops: Vec<(ItemId, Vec2)> = self
            .items
            .iter()
            .filter(|item| ids.contains(&item.id))
            .filter_map(|item| Some((item.id, image_size(item.image_key()?)?)))
            .collect();
        for (id, size) in crops {
            self.set_crop(id, Rect::from_min_size(Pos2::ZERO, size), size);
        }
    }

    /// Changes a note's letter size and, when given, its text and background colours. The box
    /// has to be measured again for the new size (see [`Self::set_note_text`]).
    pub fn set_note_style(&mut self, id: ItemId, new_font_size: f32, colors: Option<(Color32, Color32)>) {
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else { return };
        if let ItemKind::Note { font_size, color, fill, .. } = &mut item.kind {
            *font_size = new_font_size.max(1.0);
            if let Some((text, background)) = colors {
                *color = text;
                *fill = background;
            }
        }
    }

    /// Changes a note's text and the box measured for it.
    pub fn set_note_text(&mut self, id: ItemId, new_text: String, new_size: Vec2) {
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else { return };
        if !matches!(item.kind, ItemKind::Note { .. }) {
            return;
        }
        // Grow and shrink from the top-left corner, like typing does.
        let corner = item.to_world(-item.local_size() / 2.0);
        if let ItemKind::Note { text, size, .. } = &mut item.kind {
            *text = new_text;
            *size = new_size;
        }
        let new_half = item.local_size() / 2.0;
        item.center = corner + rotate(new_half * item.scale, item.rotation);
    }

    pub fn set_opacity(&mut self, ids: &BTreeSet<ItemId>, opacity: f32) {
        self.each(ids, |item| item.opacity = opacity.clamp(0.05, 1.0));
    }

    pub fn remove(&mut self, ids: &BTreeSet<ItemId>) {
        self.items.retain(|item| !ids.contains(&item.id));
    }

    /// Copies of the items, on top of everything, moved by `offset`. Returns the new ids.
    pub fn duplicate(&mut self, ids: &BTreeSet<ItemId>, offset: Vec2) -> BTreeSet<ItemId> {
        let copies: Vec<Item> = self.items.iter().filter(|item| ids.contains(&item.id)).cloned().collect();
        let mut new_ids = BTreeSet::new();
        for mut item in copies {
            item.id = self.new_id();
            item.center += offset;
            new_ids.insert(item.id);
            self.items.push(item);
        }
        new_ids
    }

    /// Moves the items to the top (or the bottom) of the drawing order, keeping their own order.
    pub fn restack(&mut self, ids: &BTreeSet<ItemId>, to_front: bool) {
        let (mut chosen, rest): (Vec<Item>, Vec<Item>) =
            self.items.drain(..).partition(|item| ids.contains(&item.id));
        if to_front {
            self.items = rest;
            self.items.append(&mut chosen);
        } else {
            self.items = chosen;
            self.items.extend(rest);
        }
    }

    /// Lays the items out in rows that fill a rectangle of the given aspect (width / height),
    /// tallest first, with `gap` world units between them. The block keeps its top-left corner.
    pub fn pack(&mut self, ids: &BTreeSet<ItemId>, aspect: f32, gap: f32) {
        let Some(start) = self.bounds_of(ids) else { return };
        let mut order: Vec<(usize, Vec2)> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| ids.contains(&item.id))
            .map(|(index, item)| (index, item.bounds().size()))
            .collect();
        let sizes: Vec<Vec2> = order.iter().map(|(_, size)| *size).collect();
        order.sort_by(|a, b| b.1.y.total_cmp(&a.1.y).then(a.0.cmp(&b.0)));
        let places = shelf_pack(&order.iter().map(|(_, size)| *size).collect::<Vec<_>>(), aspect, gap);
        debug_assert_eq!(sizes.len(), places.len());
        for ((index, size), place) in order.iter().zip(places) {
            let item = &mut self.items[*index];
            let target = start.min + place.to_vec2() + *size / 2.0;
            let off = item.bounds().center() - item.center;
            item.center = target - off;
        }
    }
}

/// Top-left positions for boxes (already sorted) laid in rows, aiming at a block whose
/// width / height is close to `aspect`. Tries every row width that is a box edge and keeps the
/// layout whose aspect is closest; boxes keep their order.
pub fn shelf_pack(sizes: &[Vec2], aspect: f32, gap: f32) -> Vec<Pos2> {
    if sizes.is_empty() {
        return Vec::new();
    }
    let layout = |max_width: f32| -> (Vec<Pos2>, Vec2) {
        let mut places = Vec::with_capacity(sizes.len());
        let (mut x, mut y, mut row_h, mut width) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for size in sizes {
            if x > 0.0 && x + size.x > max_width + 0.5 {
                y += row_h + gap;
                x = 0.0;
                row_h = 0.0;
            }
            places.push(pos2(x, y));
            x += size.x + gap;
            row_h = row_h.max(size.y);
            width = width.max(x - gap);
        }
        (places, vec2(width, y + row_h))
    };
    let widest = sizes.iter().map(|s| s.x).fold(0.0, f32::max);
    let total: f32 = sizes.iter().map(|s| s.x + gap).sum::<f32>() - gap;
    // Candidate row widths: from the widest box to everything in one row, in small steps.
    let steps = 64;
    let mut best: Option<(f32, Vec<Pos2>)> = None;
    for step in 0..=steps {
        let max_width = widest + (total - widest) * step as f32 / steps as f32;
        let (places, size) = layout(max_width);
        let got = size.x / size.y.max(f32::EPSILON);
        let score = (got.ln() - aspect.max(f32::EPSILON).ln()).abs();
        if best.as_ref().is_none_or(|(s, _)| score < *s) {
            best = Some((score, places));
        }
    }
    best.map(|(_, places)| places).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn close(a: Pos2, b: Pos2) -> bool {
        (a - b).length() < 1e-3
    }

    fn ids(list: &[ItemId]) -> BTreeSet<ItemId> {
        list.iter().copied().collect()
    }

    #[test]
    fn local_and_world_points_round_trip_with_every_transform() {
        let mut item = Item::image(1, 1, 200, 100, pos2(10.0, 20.0));
        item.scale = 1.5;
        item.rotation = 0.7;
        item.flip_x = true;
        let local = vec2(30.0, -12.0);
        let back = item.to_local(item.to_world(local));
        assert!((back - local).length() < 1e-3);
    }

    #[test]
    fn a_quarter_turn_swaps_the_bounds() {
        let mut item = Item::image(1, 1, 200, 100, Pos2::ZERO);
        item.rotation = FRAC_PI_2;
        let size = item.bounds().size();
        assert!((size.x - 100.0).abs() < 1e-3 && (size.y - 200.0).abs() < 1e-3);
    }

    #[test]
    fn a_flipped_item_has_its_first_corner_on_the_other_side() {
        let mut item = Item::image(1, 1, 200, 100, Pos2::ZERO);
        item.flip_x = true;
        assert!(close(item.corners()[0], pos2(100.0, -50.0)));
    }

    #[test]
    fn hits_find_the_topmost_item() {
        let mut scene = Scene::default();
        scene.items.push(Item::image(1, 1, 100, 100, Pos2::ZERO));
        scene.items.push(Item::image(2, 1, 100, 100, pos2(50.0, 0.0)));
        assert_eq!(scene.hit(pos2(30.0, 0.0)), Some(2));
        assert_eq!(scene.hit(pos2(-40.0, 0.0)), Some(1));
        assert_eq!(scene.hit(pos2(500.0, 0.0)), None);
    }

    #[test]
    fn scaling_about_a_point_keeps_that_point() {
        let mut scene = Scene::default();
        scene.items.push(Item::image(1, 1, 100, 100, pos2(50.0, 50.0)));
        scene.scale_about(&ids(&[1]), Pos2::ZERO, 2.0);
        let item = &scene.items[0];
        assert!(close(item.corners()[0], Pos2::ZERO));
        assert!(close(item.corners()[2], pos2(200.0, 200.0)));
    }

    #[test]
    fn rotating_a_group_turns_positions_and_items() {
        let mut scene = Scene::default();
        scene.items.push(Item::image(1, 1, 10, 10, pos2(100.0, 0.0)));
        scene.rotate_about(&ids(&[1]), Pos2::ZERO, FRAC_PI_2);
        let item = &scene.items[0];
        assert!(close(item.center, pos2(0.0, 100.0)));
        assert!((item.rotation - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn flipping_twice_gives_back_the_item() {
        let mut scene = Scene::default();
        let mut item = Item::image(1, 1, 100, 50, pos2(30.0, 10.0));
        item.rotation = 0.4;
        scene.items.push(item.clone());
        scene.flip_about(&ids(&[1]), pos2(0.0, 0.0), true);
        assert!(close(scene.items[0].center, pos2(-30.0, 10.0)));
        scene.flip_about(&ids(&[1]), pos2(0.0, 0.0), true);
        let back = &scene.items[0];
        assert!(close(back.center, item.center));
        assert!((back.rotation - item.rotation).abs() < 1e-5 && !back.flip_x);
    }

    #[test]
    fn a_flip_mirrors_what_is_seen() {
        // A point on the right of an unflipped item ends up on the left after a horizontal flip.
        let mut scene = Scene::default();
        let mut item = Item::image(1, 1, 100, 50, Pos2::ZERO);
        item.rotation = 0.3;
        scene.items.push(item.clone());
        let before = item.to_world(vec2(40.0, 10.0));
        scene.flip_about(&ids(&[1]), Pos2::ZERO, true);
        let after = scene.items[0].to_world(vec2(40.0, 10.0));
        assert!(close(after, pos2(-before.x, before.y)));
    }

    #[test]
    fn angles_wrap_into_one_turn() {
        assert!((wrap_angle(3.0 * PI) - PI).abs() < 1e-5);
        assert!((wrap_angle(-FRAC_PI_2) + FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn duplicates_go_on_top_with_new_ids() {
        let mut scene = Scene::default();
        let id = scene.new_id();
        scene.items.push(Item::image(id, 1, 10, 10, Pos2::ZERO));
        let copies = scene.duplicate(&ids(&[id]), vec2(5.0, 5.0));
        assert_eq!(scene.items.len(), 2);
        assert!(!copies.contains(&id));
        assert!(close(scene.items[1].center, pos2(5.0, 5.0)));
    }

    #[test]
    fn restacking_keeps_the_relative_order() {
        let mut scene = Scene::default();
        for id in 1..=4 {
            scene.items.push(Item::image(id, 1, 10, 10, Pos2::ZERO));
        }
        scene.restack(&ids(&[1, 3]), true);
        let order: Vec<ItemId> = scene.items.iter().map(|i| i.id).collect();
        assert_eq!(order, vec![2, 4, 1, 3]);
        scene.restack(&ids(&[4]), false);
        let order: Vec<ItemId> = scene.items.iter().map(|i| i.id).collect();
        assert_eq!(order, vec![4, 2, 1, 3]);
    }

    #[test]
    fn packing_does_not_overlap_and_follows_the_aspect() {
        let mut scene = Scene::default();
        let sizes = [(300, 200), (100, 100), (250, 120), (80, 300), (200, 200), (120, 90)];
        for (n, (w, h)) in sizes.iter().enumerate() {
            scene.items.push(Item::image(n as u64 + 1, 1, *w, *h, pos2(n as f32 * 37.0, 0.0)));
        }
        let all = scene.ids();
        scene.pack(&all, 16.0 / 9.0, 10.0);
        for a in &scene.items {
            for b in &scene.items {
                if a.id < b.id {
                    let overlap = a.bounds().shrink(0.01).intersects(b.bounds().shrink(0.01));
                    assert!(!overlap, "{} and {} overlap", a.id, b.id);
                }
            }
        }
        let size = scene.bounds().unwrap().size();
        let aspect = size.x / size.y;
        assert!(aspect > 1.0 && aspect < 3.5, "aspect {aspect}");
    }

    #[test]
    fn packing_one_row_when_the_window_is_very_wide() {
        let places = shelf_pack(&[vec2(10.0, 10.0), vec2(10.0, 10.0), vec2(10.0, 10.0)], 100.0, 0.0);
        assert!(places.iter().all(|p| p.y == 0.0));
    }

    #[test]
    fn cropping_keeps_the_image_where_it_was() {
        let mut scene = Scene::default();
        let mut item = Item::image(1, 1, 400, 300, pos2(50.0, -20.0));
        item.scale = 0.75;
        item.rotation = 0.6;
        item.flip_x = true;
        scene.items.push(item.clone());
        // Where the image's pixel (300, 120) is on the board, before and after cropping around it.
        let pixel = pos2(300.0, 120.0);
        let before = item.to_world(pixel - item.image_pixel(item.center).unwrap());
        scene.set_crop(1, Rect::from_min_max(pos2(250.0, 100.0), pos2(390.0, 200.0)), vec2(400.0, 300.0));
        let after_item = &scene.items[0];
        assert_eq!(after_item.local_size(), vec2(140.0, 100.0));
        let after = after_item.to_world(pixel - after_item.image_pixel(after_item.center).unwrap());
        assert!(close(before, after), "{before:?} became {after:?}");
        assert!(close(after_item.image_pixel(after).unwrap(), pixel));
    }

    #[test]
    fn crops_stay_inside_the_image() {
        let mut scene = Scene::default();
        scene.items.push(Item::image(1, 1, 100, 100, Pos2::ZERO));
        scene.set_crop(1, Rect::from_min_max(pos2(-50.0, 90.0), pos2(40.0, 300.0)), vec2(100.0, 100.0));
        assert_eq!(scene.items[0].local_size(), vec2(40.0, 10.0));
        scene.set_crop(1, Rect::from_min_max(pos2(99.9, 0.0), pos2(99.9, 50.0)), vec2(100.0, 100.0));
        assert_eq!(scene.items[0].local_size().x, 1.0);
        scene.reset_crop(&ids(&[1]), |_| Some(vec2(100.0, 100.0)));
        assert_eq!(scene.items[0].local_size(), vec2(100.0, 100.0));
    }

    #[test]
    fn a_growing_note_keeps_its_top_left_corner() {
        let mut scene = Scene::default();
        let mut note = Item::note(1, "hi".into(), pos2(100.0, 100.0), vec2(40.0, 30.0));
        note.rotation = 0.3;
        note.scale = 2.0;
        scene.items.push(note);
        let corner = scene.items[0].corners()[0];
        scene.set_note_text(1, "hello there".into(), vec2(150.0, 60.0));
        assert!(close(scene.items[0].corners()[0], corner));
        assert_eq!(scene.items[0].local_size(), vec2(150.0, 60.0));
    }

    #[test]
    fn flipping_a_note_moves_it_but_keeps_it_readable() {
        let mut scene = Scene::default();
        scene.items.push(Item::note(1, "x".into(), pos2(30.0, 0.0), vec2(10.0, 10.0)));
        scene.flip_about(&ids(&[1]), Pos2::ZERO, true);
        assert!(close(scene.items[0].center, pos2(-30.0, 0.0)));
        assert!(!scene.items[0].flip_x);
    }

    #[test]
    fn notes_survive_json() {
        let note = Item::note(4, "line one\nline two".into(), pos2(1.0, 2.0), vec2(80.0, 50.0));
        let back: Item = serde_json::from_str(&serde_json::to_string(&note).unwrap()).unwrap();
        assert_eq!(back, note);
    }

    #[test]
    fn items_survive_json() {
        let mut item = Item::image(3, 7, 64, 32, pos2(1.0, 2.0));
        item.rotation = 0.5;
        item.flip_y = true;
        let text = serde_json::to_string(&item).unwrap();
        let back: Item = serde_json::from_str(&text).unwrap();
        assert_eq!(back, item);
    }
}

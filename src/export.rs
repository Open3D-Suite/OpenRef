// SPDX-License-Identifier: GPL-3.0-or-later
//! Taking things off the board: single images as files, and the board (or part of it) as one PNG.
//!
//! The board picture is made like the screen is: the items become egui shapes, egui tessellates
//! them into triangles (notes included, with their letters from egui's glyph atlas), and the
//! triangles are filled here on the CPU at full resolution, so it does not depend on the window size.

use crate::images::{self, Source};
use crate::scene::{ImageKey, Item, ItemKind, NOTE_PADDING};
use egui::epaint::{ClippedShape, Primitive, TextShape, Vertex};
use egui::{Color32, FontId, Mesh, Pos2, Rect, Shape, Stroke, TextureId, Vec2, pos2};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The longest side of an exported board picture, in pixels.
pub const MAX_BOARD_SIDE: u32 = 8192;
const MAX_FONT_PX: f32 = 400.0;

/// An image as a file of its own: the original bytes when the board shows it untouched,
/// otherwise a PNG with its crop, flips and grayscale applied (rotation and scale are left out).
pub fn image_file(item: &Item, source: &Source) -> Result<(Vec<u8>, String), String> {
    let ItemKind::Image { crop, grayscale, .. } = &item.kind else {
        return Err("not an image".into());
    };
    let whole = Rect::from_min_size(Pos2::ZERO, Vec2::new(source.width as f32, source.height as f32));
    if !*grayscale && !item.flip_x && !item.flip_y && crop.min.round() == whole.min && crop.max.round() == whole.max {
        return Ok((source.bytes.to_vec(), source.ext.clone()));
    }
    let full = images::decode_full(&source.bytes, *grayscale)?;
    let r = crop.intersect(Rect::from_min_size(Pos2::ZERO, Vec2::new(full.width() as f32, full.height() as f32)));
    let mut cut = image::imageops::crop_imm(
        &full,
        r.min.x.round() as u32,
        r.min.y.round() as u32,
        r.width().round().max(1.0) as u32,
        r.height().round().max(1.0) as u32,
    )
    .to_image();
    if item.flip_x {
        image::imageops::flip_horizontal_in_place(&mut cut);
    }
    if item.flip_y {
        image::imageops::flip_vertical_in_place(&mut cut);
    }
    Ok((images::encode_png(cut.width(), cut.height(), cut.as_raw())?, "png".into()))
}

/// A file name in `dir` for `name` with `ext`, adding " (2)", " (3)"… when it is taken.
pub fn unique_path(dir: &Path, name: &str, ext: &str) -> PathBuf {
    // Not Path::file_stem: on Windows "a:b.png" would read as a drive letter.
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let clean: String = stem.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).collect();
    let clean = if clean.trim().is_empty() { "image".to_string() } else { clean.trim().to_string() };
    let mut path = dir.join(format!("{clean}.{ext}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{clean} ({n}).{ext}"));
        n += 1;
    }
    path
}

/// Everything the background thread needs to paint a board picture.
pub struct BoardJob {
    pub width: u32,
    pub height: u32,
    pub background: Color32,
    pub meshes: Vec<Mesh>,
    /// The glyph atlas the note meshes point into.
    pub atlas: egui::ColorImage,
    /// Images the meshes point into, by `TextureId::User` number: the bytes and grayscale.
    pub images: HashMap<u64, (Source, bool)>,
}

/// Turns the chosen items into meshes in picture pixels (`scale` pixels per world unit).
/// Must run on the window's thread: it lays out text with the window's fonts.
pub fn prepare_board(ctx: &egui::Context, items: &[Item], sources: &HashMap<ImageKey, Source>, scale: f32, background: Color32) -> Option<BoardJob> {
    let bounds = items.iter().map(Item::bounds).reduce(|a, b| a.union(b))?;
    let width = (bounds.width() * scale).ceil().max(1.0) as u32;
    let height = (bounds.height() * scale).ceil().max(1.0) as u32;
    let map = |p: Pos2| pos2((p.x - bounds.min.x) * scale, (p.y - bounds.min.y) * scale);
    let mut shapes = Vec::new();
    let mut textures: HashMap<u64, (Source, bool)> = HashMap::new();

    for item in items {
        let corners = item.corners().map(map);
        let alpha = item.opacity.clamp(0.0, 1.0);
        match &item.kind {
            ItemKind::Image { image, crop, grayscale } => {
                let Some(source) = sources.get(image) else { continue };
                let id = image * 2 + u64::from(*grayscale);
                textures.entry(id).or_insert_with(|| (source.clone(), *grayscale));
                let (w, h) = (source.width as f32, source.height as f32);
                let tint = Color32::from_white_alpha((alpha * 255.0).round() as u8);
                let mut mesh = Mesh::with_texture(TextureId::User(id));
                let pixels = [crop.left_top(), crop.right_top(), crop.right_bottom(), crop.left_bottom()];
                for (pos, pixel) in corners.iter().zip(pixels) {
                    mesh.vertices.push(Vertex { pos: *pos, uv: pos2(pixel.x / w, pixel.y / h), color: tint });
                }
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                shapes.push(Shape::mesh(mesh));
            }
            ItemKind::Note { text, font_size, color, fill, .. } => {
                shapes.push(Shape::convex_polygon(corners.to_vec(), fill.gamma_multiply(alpha), Stroke::NONE));
                let px = (font_size * item.scale * scale).min(MAX_FONT_PX);
                if px >= 2.0 {
                    let galley = ctx.fonts_mut(|f| f.layout_no_wrap(text.clone(), FontId::proportional(px), color.gamma_multiply(alpha)));
                    let at = map(item.to_world(-item.local_size() / 2.0 + Vec2::splat(NOTE_PADDING * font_size)));
                    shapes.push(Shape::Text(TextShape::new(at, galley, *color).with_angle(item.rotation)));
                }
            }
        }
    }
    let clip = Rect::from_min_size(Pos2::ZERO, Vec2::new(width as f32, height as f32));
    let clipped: Vec<ClippedShape> = shapes
        .into_iter()
        .map(|shape| ClippedShape { clip_rect: clip, shape, transform_after_tessellation: egui::emath::TSTransform::IDENTITY })
        .collect();
    let meshes = ctx
        .tessellate(clipped, 1.0)
        .into_iter()
        .filter_map(|p| match p.primitive {
            Primitive::Mesh(mesh) => Some(mesh),
            Primitive::Callback(_) => None,
        })
        .collect();
    let atlas = ctx.fonts(|f| f.image());
    Some(BoardJob { width, height, background, meshes, atlas, images: textures })
}

/// A texture as premultiplied RGBA bytes.
struct Texture {
    width: usize,
    height: usize,
    pixels: Vec<[u8; 4]>,
}

impl Texture {
    fn from_rgba(image: &RgbaImage) -> Self {
        let pixels = image
            .pixels()
            .map(|p| {
                let [r, g, b, a] = p.0;
                let m = |c: u8| ((c as u16 * a as u16 + 127) / 255) as u8;
                [m(r), m(g), m(b), a]
            })
            .collect();
        Self { width: image.width() as usize, height: image.height() as usize, pixels }
    }

    fn from_color_image(image: &egui::ColorImage) -> Self {
        Self { width: image.size[0], height: image.size[1], pixels: image.pixels.iter().map(|c| c.to_array()).collect() }
    }

    /// Bilinear sample at UV (0..1), premultiplied, as 0..1 floats.
    fn sample(&self, uv: Pos2) -> [f32; 4] {
        let x = (uv.x * self.width as f32 - 0.5).clamp(0.0, (self.width - 1) as f32);
        let y = (uv.y * self.height as f32 - 0.5).clamp(0.0, (self.height - 1) as f32);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let at = |x: usize, y: usize| self.pixels[y * self.width + x];
        let mut out = [0.0; 4];
        for (c, value) in out.iter_mut().enumerate() {
            let top = at(x0, y0)[c] as f32 * (1.0 - fx) + at(x1, y0)[c] as f32 * fx;
            let bottom = at(x0, y1)[c] as f32 * (1.0 - fx) + at(x1, y1)[c] as f32 * fx;
            *value = (top * (1.0 - fy) + bottom * fy) / 255.0;
        }
        out
    }
}

/// Paints the job into a picture (straight, not premultiplied, alpha) and encodes it as PNG.
pub fn render_board(job: BoardJob) -> Result<Vec<u8>, String> {
    let mut textures: HashMap<TextureId, Texture> = HashMap::new();
    textures.insert(TextureId::default(), Texture::from_color_image(&job.atlas));
    for (id, (source, gray)) in &job.images {
        let full = images::decode_full(&source.bytes, *gray)?;
        textures.insert(TextureId::User(*id), Texture::from_rgba(&full));
    }
    let (w, h) = (job.width as usize, job.height as usize);
    let back = job.background.to_array().map(|c| c as f32 / 255.0);
    let mut canvas: Vec<[f32; 4]> = vec![back; w * h];
    for mesh in &job.meshes {
        let Some(texture) = textures.get(&mesh.texture_id) else { continue };
        for tri in mesh.indices.chunks_exact(3) {
            let v = [mesh.vertices[tri[0] as usize], mesh.vertices[tri[1] as usize], mesh.vertices[tri[2] as usize]];
            fill_triangle(&mut canvas, w, h, v, texture);
        }
    }
    let mut out = RgbaImage::new(job.width, job.height);
    for (pixel, c) in out.pixels_mut().zip(canvas) {
        let a = c[3].clamp(0.0, 1.0);
        let un = |v: f32| if a > 0.0 { (v / a * 255.0).round().clamp(0.0, 255.0) as u8 } else { 0 };
        pixel.0 = [un(c[0]), un(c[1]), un(c[2]), (a * 255.0).round() as u8];
    }
    images::encode_png(job.width, job.height, out.as_raw())
}

/// Fills one triangle with "over" blending of premultiplied colours. Pixel centres on a shared
/// edge belong to one triangle only (top-left rule), so half-transparent quads have no seam.
fn fill_triangle(canvas: &mut [[f32; 4]], w: usize, h: usize, mut v: [Vertex; 3], texture: &Texture) {
    let edge = |a: Pos2, b: Pos2, p: Pos2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let mut area = edge(v[0].pos, v[1].pos, v[2].pos);
    if area.abs() < 1e-9 {
        return;
    }
    if area < 0.0 {
        v.swap(1, 2);
        area = -area;
    }
    let (p0, p1, p2) = (v[0].pos, v[1].pos, v[2].pos);
    // With this winding (y down), an edge is "top" when horizontal going right, "left" going up.
    let top_left = |a: Pos2, b: Pos2| (a.y == b.y && b.x > a.x) || b.y < a.y;
    let bias = [top_left(p1, p2), top_left(p2, p0), top_left(p0, p1)];
    let min_x = p0.x.min(p1.x).min(p2.x).floor().max(0.0) as usize;
    let max_x = (p0.x.max(p1.x).max(p2.x).ceil().max(0.0) as usize).min(w);
    let min_y = p0.y.min(p1.y).min(p2.y).floor().max(0.0) as usize;
    let max_y = (p0.y.max(p1.y).max(p2.y).ceil().max(0.0) as usize).min(h);
    let colors = v.map(|vertex| vertex.color.to_array().map(|c| c as f32 / 255.0));
    for y in min_y..max_y {
        for x in min_x..max_x {
            let p = pos2(x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge(p1, p2, p);
            let w1 = edge(p2, p0, p);
            let w2 = edge(p0, p1, p);
            let inside = |w: f32, b: bool| w > 0.0 || (w == 0.0 && b);
            if !(inside(w0, bias[0]) && inside(w1, bias[1]) && inside(w2, bias[2])) {
                continue;
            }
            let (b0, b1, b2) = (w0 / area, w1 / area, w2 / area);
            let uv = pos2(b0 * v[0].uv.x + b1 * v[1].uv.x + b2 * v[2].uv.x, b0 * v[0].uv.y + b1 * v[1].uv.y + b2 * v[2].uv.y);
            let texel = texture.sample(uv);
            let mut src = [0.0f32; 4];
            for (c, s) in src.iter_mut().enumerate() {
                let tint = b0 * colors[0][c] + b1 * colors[1][c] + b2 * colors[2][c];
                *s = texel[c] * tint;
            }
            let dst = &mut canvas[y * w + x];
            let keep = 1.0 - src[3];
            for c in 0..4 {
                dst[c] = src[c] + dst[c] * keep;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn png_source(width: u32, height: u32, rgba: [u8; 4], name: &str) -> Source {
        let pixels: Vec<u8> = (0..width * height).flat_map(|_| rgba).collect();
        Source::from_bytes(Arc::from(images::encode_png(width, height, &pixels).unwrap()), name.into(), None).unwrap()
    }

    #[test]
    fn untouched_images_keep_their_original_bytes() {
        let source = png_source(10, 6, [10, 20, 30, 255], "a.png");
        let item = Item::image(1, 1, 10, 6, Pos2::ZERO);
        let (bytes, ext) = image_file(&item, &source).unwrap();
        assert_eq!((bytes.as_slice(), ext.as_str()), (&source.bytes[..], "png"));
    }

    #[test]
    fn cropped_images_are_cut() {
        let source = png_source(10, 6, [10, 20, 30, 255], "a.png");
        let mut scene = crate::scene::Scene::default();
        scene.items.push(Item::image(1, 1, 10, 6, Pos2::ZERO));
        scene.set_crop(1, Rect::from_min_max(pos2(2.0, 1.0), pos2(7.0, 5.0)), Vec2::new(10.0, 6.0));
        let (bytes, _) = image_file(&scene.items[0], &source).unwrap();
        let cut = image::load_from_memory(&bytes).unwrap();
        assert_eq!((cut.width(), cut.height()), (5, 4));
    }

    #[test]
    fn taken_names_get_a_number() {
        let dir = std::env::temp_dir().join(format!("openref-names-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("ref.png"), b"x").unwrap();
        assert_eq!(unique_path(&dir, "ref.png", "png").file_name().unwrap(), "ref (2).png");
        assert_eq!(unique_path(&dir, "a:b?.jpg", "jpg").file_name().unwrap(), "a_b_.jpg");
        let _ = std::fs::remove_dir_all(dir);
    }

    fn texture(color: [u8; 4]) -> Texture {
        Texture { width: 1, height: 1, pixels: vec![color] }
    }

    fn quad(rect: Rect, color: Color32) -> [[Vertex; 3]; 2] {
        let v = |p: Pos2| Vertex { pos: p, uv: Pos2::ZERO, color };
        let [a, b, c, d] = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()].map(v);
        [[a, b, c], [a, c, d]]
    }

    #[test]
    fn a_half_transparent_quad_has_no_seam() {
        let (w, h) = (8, 8);
        let mut canvas = vec![[0.0f32; 4]; w * h];
        let tex = texture([255, 255, 255, 255]);
        for tri in quad(Rect::from_min_max(pos2(0.0, 0.0), pos2(8.0, 8.0)), Color32::from_white_alpha(128)) {
            fill_triangle(&mut canvas, w, h, tri, &tex);
        }
        let first = canvas[0][3];
        assert!(canvas.iter().all(|p| (p[3] - first).abs() < 1e-6), "every pixel painted exactly once");
        assert!((first - 128.0 / 255.0).abs() < 1e-3);
    }

    #[test]
    fn a_board_picture_has_the_board_size_and_the_image_colour() {
        let source = png_source(20, 10, [200, 40, 40, 255], "red.png");
        let mut sources = HashMap::new();
        sources.insert(1u64, source);
        let item = Item::image(1, 1, 20, 10, pos2(100.0, 100.0));
        let ctx = egui::Context::default();
        let mut job = None;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            job = prepare_board(ui.ctx(), std::slice::from_ref(&item), &sources, 2.0, Color32::BLACK);
        });
        output.textures_delta.clear();
        let png = render_board(job.unwrap()).unwrap();
        let picture = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(picture.dimensions(), (40, 20));
        assert_eq!(picture.get_pixel(20, 10).0, [200, 40, 40, 255]);
    }
}

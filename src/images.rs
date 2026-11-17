// SPDX-License-Identifier: GPL-3.0-or-later
//! The images on a board: the original file bytes (what gets saved) and the textures drawn.
//!
//! Each image is decoded on its own thread into a few sizes (the largest one capped by what the
//! graphics card takes), so a far-away image is drawn from a small texture and looks smooth.

use crate::scene::ImageKey;
use egui::{ColorImage, TextureHandle, TextureOptions};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// Smallest texture made for an image (its longest side, in pixels).
const SMALLEST_LEVEL: u32 = 256;

/// An image as it came in: its file bytes and what is known without decoding it.
#[derive(Clone, Debug)]
pub struct Source {
    pub bytes: Arc<[u8]>,
    /// File extension for its format, lower case ("png", "jpg"…).
    pub ext: String,
    /// A name for people: the file name, or "Pasted image".
    pub name: String,
    /// Size once upright (after the photo's orientation tag is applied).
    pub width: u32,
    pub height: u32,
    /// Where it came from on disk, when it did.
    pub origin: Option<String>,
}

impl Source {
    /// Reads what an image file holds without decoding its pixels.
    pub fn from_bytes(bytes: Arc<[u8]>, name: String, origin: Option<String>) -> Result<Self, String> {
        let (ext, width, height) = probe(&bytes)?;
        if width == 0 || height == 0 {
            return Err("the image is empty".into());
        }
        Ok(Self { bytes, ext, name, width, height, origin })
    }

    pub fn from_path(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Self::from_bytes(bytes.into(), name, Some(path.to_string_lossy().into_owned()))
    }

    /// An image from raw pixels (a screenshot on the clipboard): kept as a PNG file.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8], name: &str) -> Result<Self, String> {
        let png = encode_png(width, height, rgba)?;
        Self::from_bytes(png.into(), name.to_string(), None)
    }
}

/// Format extension and upright size of an image file, read from its header.
fn probe(bytes: &[u8]) -> Result<(String, u32, u32), String> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| e.to_string())?;
    let format = reader.format().ok_or_else(|| "not an image this program can read".to_string())?;
    let ext = format.extensions_str().first().copied().unwrap_or("img").to_string();
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (mut width, mut height) = decoder.dimensions();
    if let Ok(orientation) = decoder.orientation() {
        use image::metadata::Orientation::*;
        if matches!(orientation, Rotate90 | Rotate270 | Rotate90FlipH | Rotate270FlipH) {
            std::mem::swap(&mut width, &mut height);
        }
    }
    Ok((ext, width, height))
}

pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out).write_image(rgba, width, height, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?;
    Ok(out)
}

use image::ImageEncoder;

/// Decodes an image upright, at full size.
pub fn decode_full(bytes: &[u8], gray: bool) -> Result<RgbaImage, String> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| e.to_string())?;
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let orientation = decoder.orientation().ok();
    let mut image = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    if let Some(orientation) = orientation {
        image.apply_orientation(orientation);
    }
    let mut rgba = image.into_rgba8();
    if gray {
        for pixel in rgba.pixels_mut() {
            let [r, g, b, _] = pixel.0;
            let y = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32).round() as u8;
            pixel.0[0] = y;
            pixel.0[1] = y;
            pixel.0[2] = y;
        }
    }
    Ok(rgba)
}

/// Decodes an image upright and returns it at several sizes, largest first.
pub fn decode_levels(bytes: &[u8], max_side: u32, gray: bool) -> Result<Vec<ColorImage>, String> {
    let rgba = decode_full(bytes, gray)?;
    let mut levels = Vec::new();
    let (w, h) = rgba.dimensions();
    let mut side = w.max(h).min(max_side.max(1));
    loop {
        let factor = side as f32 / w.max(h) as f32;
        let (lw, lh) = (((w as f32 * factor).round() as u32).max(1), ((h as f32 * factor).round() as u32).max(1));
        let level = if (lw, lh) == (w, h) { rgba.clone() } else { image::imageops::thumbnail(&rgba, lw, lh) };
        levels.push(ColorImage::from_rgba_unmultiplied([lw as usize, lh as usize], level.as_raw()));
        if side <= SMALLEST_LEVEL {
            break;
        }
        side = (side / 2).max(SMALLEST_LEVEL);
    }
    Ok(levels)
}

/// One set of textures (normal or grayscale) of an image.
pub enum Textures {
    NotAsked,
    Waiting,
    /// Largest first.
    Ready(Vec<TextureHandle>),
    Failed(String),
}

impl Textures {
    /// The smallest texture still at least `screen_px` wide (the largest when none is).
    pub fn for_width(&self, screen_px: f32) -> Option<&TextureHandle> {
        let Textures::Ready(levels) = self else { return None };
        levels.iter().rev().find(|t| t.size()[0] as f32 >= screen_px).or_else(|| levels.first())
    }
}

pub struct Entry {
    pub source: Source,
    pub color: Textures,
    pub gray: Textures,
}

type Decoded = (ImageKey, bool, Result<Vec<ColorImage>, String>);

pub struct ImageStore {
    entries: HashMap<ImageKey, Entry>,
    next_key: ImageKey,
    sender: Sender<Decoded>,
    receiver: Receiver<Decoded>,
    max_side: u32,
    ctx: egui::Context,
}

impl ImageStore {
    pub fn new(ctx: egui::Context, max_side: u32) -> Self {
        let (sender, receiver) = channel();
        Self { entries: HashMap::new(), next_key: 0, sender, receiver, max_side, ctx }
    }

    /// Adds an image and starts decoding it. Returns its key.
    pub fn add(&mut self, source: Source) -> ImageKey {
        self.next_key += 1;
        let key = self.next_key;
        self.insert(key, source);
        key
    }

    /// Adds an image under a known key (when a board is opened).
    pub fn insert(&mut self, key: ImageKey, source: Source) {
        self.next_key = self.next_key.max(key);
        self.entries.insert(key, Entry { source, color: Textures::NotAsked, gray: Textures::NotAsked });
        self.request(key, false);
    }

    pub fn source(&self, key: ImageKey) -> Option<&Source> {
        self.entries.get(&key).map(|entry| &entry.source)
    }

    /// Every image's source (cheap: the bytes are shared), for work on another thread.
    pub fn sources(&self) -> HashMap<ImageKey, Source> {
        self.entries.iter().map(|(key, entry)| (*key, entry.source.clone())).collect()
    }

    /// The textures to draw; asks for the grayscale ones the first time they are needed.
    pub fn textures(&mut self, key: ImageKey, gray: bool) -> Option<&Textures> {
        if gray && matches!(self.entries.get(&key)?.gray, Textures::NotAsked) {
            self.request(key, true);
        }
        let entry = self.entries.get(&key)?;
        Some(if gray { &entry.gray } else { &entry.color })
    }

    fn request(&mut self, key: ImageKey, gray: bool) {
        let Some(entry) = self.entries.get_mut(&key) else { return };
        let slot = if gray { &mut entry.gray } else { &mut entry.color };
        *slot = Textures::Waiting;
        let bytes = entry.source.bytes.clone();
        let (sender, ctx, max_side) = (self.sender.clone(), self.ctx.clone(), self.max_side);
        std::thread::spawn(move || {
            let result = decode_levels(&bytes, max_side, gray);
            let _ = sender.send((key, gray, result));
            ctx.request_repaint();
        });
    }

    /// Turns decoded images into textures. Returns true when something new arrived.
    pub fn poll(&mut self) -> bool {
        let mut any = false;
        while let Ok((key, gray, result)) = self.receiver.try_recv() {
            any = true;
            let Some(entry) = self.entries.get_mut(&key) else { continue };
            let textures = match result {
                Ok(levels) => Textures::Ready(
                    levels
                        .into_iter()
                        .enumerate()
                        .map(|(n, level)| {
                            let name = format!("image-{key}-{}-{n}", if gray { "gray" } else { "color" });
                            self.ctx.load_texture(name, level, TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)))
                        })
                        .collect(),
                ),
                Err(error) => Textures::Failed(error),
            };
            if gray {
                entry.gray = textures;
            } else {
                entry.color = textures;
            }
        }
        any
    }

    /// Forgets every image (a new or another board).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn is_decoding(&self) -> bool {
        self.entries.values().any(|e| matches!(e.color, Textures::Waiting) || matches!(e.gray, Textures::Waiting))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let rgba: Vec<u8> = (0..width * height).flat_map(|i| [(i % 256) as u8, 128, 64, 255]).collect();
        encode_png(width, height, &rgba).unwrap()
    }

    #[test]
    fn sources_know_their_format_and_size() {
        let source = Source::from_bytes(png(30, 20).into(), "a.png".into(), None).unwrap();
        assert_eq!((source.ext.as_str(), source.width, source.height), ("png", 30, 20));
    }

    #[test]
    fn what_is_not_an_image_is_refused() {
        assert!(Source::from_bytes(b"hello there".to_vec().into(), "x".into(), None).is_err());
    }

    #[test]
    fn levels_halve_down_to_the_smallest_and_respect_the_cap() {
        let levels = decode_levels(&png(1500, 600), 1024, false).unwrap();
        let widths: Vec<usize> = levels.iter().map(|l| l.size[0]).collect();
        assert_eq!(widths, vec![1024, 512, 256]);
        assert_eq!(levels[0].size[1], 410);
    }

    #[test]
    fn a_small_image_has_one_level_at_its_own_size() {
        let levels = decode_levels(&png(40, 30), 8192, false).unwrap();
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].size, [40, 30]);
    }

    #[test]
    fn grayscale_levels_have_equal_channels() {
        let levels = decode_levels(&png(8, 8), 8192, true).unwrap();
        assert!(levels[0].pixels.iter().all(|p| p.r() == p.g() && p.g() == p.b()));
    }
}

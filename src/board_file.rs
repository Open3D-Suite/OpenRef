// SPDX-License-Identifier: GPL-3.0-or-later
//! The `.oref` board file: a zip with `board.json` (the items, the view and a list of images) and
//! the images' original files under `images/`, so one file carries the whole board and any zip
//! tool can take the pictures out of it.

use crate::images::Source;
use crate::scene::{ImageKey, Item, Scene};
use egui::Pos2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::Path;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

pub const EXTENSION: &str = "oref";
const FORMAT: &str = "openref-board";
const VERSION: u32 = 1;
const BOARD_JSON: &str = "board.json";

/// Where the board was looked at from when it was saved.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub center: Pos2,
    pub zoom: f32,
}

#[derive(Serialize, Deserialize)]
struct BoardJson {
    format: String,
    version: u32,
    app: String,
    view: Option<View>,
    next_id: u64,
    images: Vec<ImageRecord>,
    items: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
struct ImageRecord {
    key: ImageKey,
    file: String,
    name: String,
    #[serde(default)]
    origin: Option<String>,
}

/// A board read from a file.
pub struct Board {
    pub scene: Scene,
    pub view: Option<View>,
    pub images: Vec<(ImageKey, Source)>,
}

/// Writes the board next to `path` first and then puts it in place, so a failed save never
/// leaves half a file where the good one was. Only images some item shows are saved.
pub fn save(path: &Path, scene: &Scene, view: Option<View>, source: impl Fn(ImageKey) -> Option<Source>) -> Result<(), String> {
    let keys: BTreeSet<ImageKey> = scene.items.iter().filter_map(Item::image_key).collect();
    let mut records = Vec::new();
    let mut files = Vec::new();
    for key in keys {
        let image = source(key).ok_or_else(|| format!("image {key} is missing"))?;
        let file = format!("images/{key}.{}", image.ext);
        records.push(ImageRecord { key, file: file.clone(), name: image.name.clone(), origin: image.origin.clone() });
        files.push((file, image));
    }
    let json = BoardJson {
        format: FORMAT.into(),
        version: VERSION,
        app: format!("OpenRef {}", crate::VERSION),
        view,
        next_id: scene.next_id,
        images: records,
        items: scene.items.clone(),
    };
    let text = serde_json::to_vec_pretty(&json).map_err(|e| e.to_string())?;

    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = std::path::PathBuf::from(part);
    let write = || -> Result<(), String> {
        let file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
        let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file(BOARD_JSON, deflated).map_err(|e| e.to_string())?;
        zip.write_all(&text).map_err(|e| e.to_string())?;
        for (name, image) in &files {
            // Already-compressed formats are stored as they are; the rest is deflated.
            let method = match image.ext.as_str() {
                "jpg" | "jpeg" | "png" | "webp" | "gif" => CompressionMethod::Stored,
                _ => CompressionMethod::Deflated,
            };
            let options = SimpleFileOptions::default().compression_method(method).large_file(image.bytes.len() > u32::MAX as usize / 2);
            zip.start_file(name.as_str(), options).map_err(|e| e.to_string())?;
            zip.write_all(&image.bytes).map_err(|e| e.to_string())?;
        }
        let mut inner = zip.finish().map_err(|e| e.to_string())?;
        inner.flush().map_err(|e| e.to_string())?;
        inner.get_ref().sync_all().map_err(|e| e.to_string())?;
        Ok(())
    };
    if let Err(error) = write() {
        let _ = std::fs::remove_file(&part);
        return Err(error);
    }
    std::fs::rename(&part, path).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        e.to_string()
    })
}

pub fn load(path: &Path) -> Result<Board, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|_| "this is not an OpenRef board".to_string())?;
    let json: BoardJson = {
        let mut entry = zip.by_name(BOARD_JSON).map_err(|_| "this is not an OpenRef board".to_string())?;
        let mut text = Vec::new();
        entry.read_to_end(&mut text).map_err(|e| e.to_string())?;
        serde_json::from_slice(&text).map_err(|e| format!("the board is damaged: {e}"))?
    };
    if json.format != FORMAT {
        return Err("this is not an OpenRef board".into());
    }
    if json.version > VERSION {
        return Err(format!("the board was saved by a newer OpenRef ({}); update OpenRef to open it", json.app));
    }
    let mut images = Vec::new();
    for record in json.images {
        let mut entry = zip.by_name(&record.file).map_err(|_| format!("the board lacks {}", record.file))?;
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        let source = Source::from_bytes(bytes.into(), record.name, record.origin).map_err(|e| format!("{}: {e}", record.file))?;
        images.push((record.key, source));
    }
    let keys: BTreeSet<ImageKey> = images.iter().map(|(key, _)| *key).collect();
    let items: Vec<Item> = json.items.into_iter().filter(|item| item.image_key().is_none_or(|k| keys.contains(&k))).collect();
    let next_id = items.iter().map(|item| item.id).max().unwrap_or(0).max(json.next_id);
    Ok(Board { scene: Scene { items, next_id }, view: json.view, images })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::encode_png;
    use egui::pos2;

    fn source(width: u32, height: u32, name: &str) -> Source {
        let rgba = vec![200u8; (width * height * 4) as usize];
        Source::from_bytes(encode_png(width, height, &rgba).unwrap().into(), name.into(), None).unwrap()
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("openref-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("board.oref")
    }

    #[test]
    fn a_board_comes_back_as_it_was_saved() {
        let path = scratch("roundtrip");
        let (a, b) = (source(20, 10, "a.png"), source(5, 5, "b.png"));
        let mut scene = Scene::default();
        let mut first = Item::image(scene.new_id(), 1, 20, 10, pos2(3.0, 4.0));
        first.rotation = 0.25;
        first.flip_x = true;
        scene.items.push(first);
        let (second, third) = (scene.new_id(), scene.new_id());
        scene.items.push(Item::image(second, 2, 5, 5, pos2(-8.0, 1.0)));
        scene.items.push(Item::image(third, 1, 20, 10, pos2(40.0, 0.0)));
        let view = View { center: pos2(1.0, 2.0), zoom: 0.5 };
        let images = [(1u64, a.clone()), (2u64, b.clone()), (3u64, source(2, 2, "unused.png"))];
        save(&path, &scene, Some(view), |key| images.iter().find(|(k, _)| *k == key).map(|(_, s)| s.clone())).unwrap();

        let board = load(&path).unwrap();
        assert_eq!(board.scene, scene);
        assert_eq!(board.view, Some(view));
        assert_eq!(board.images.len(), 2, "an image no item shows is not saved");
        let (_, back) = board.images.iter().find(|(k, _)| *k == 1).unwrap();
        assert_eq!(back.bytes, a.bytes);
        assert_eq!(back.name, "a.png");
        assert!(!path.with_extension("oref.part").exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn other_files_are_refused() {
        let path = scratch("refuse");
        std::fs::write(&path, b"not a zip").unwrap();
        assert!(load(&path).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

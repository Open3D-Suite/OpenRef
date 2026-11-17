// SPDX-License-Identifier: GPL-3.0-or-later
//! The window: actions and their shortcuts, the right-click menu, the clipboard, dropped files,
//! opening and saving boards, and the question before unsaved changes are lost.

use crate::board_file::{self, EXTENSION};
use crate::canvas::{BACKGROUND, Editor};
use crate::images::{self, ImageStore, Source};
use crate::platform;
use crate::scene::{Align, ItemKind, MatchSize, Scene};
use crate::text::{Lang, tf, tr};
use crate::{export, recovery};
use eframe::egui::{self, Color32, Key, KeyboardShortcut, Modifiers, Vec2};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Image files OpenRef reads.
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "jfif", "bmp", "gif", "tga", "webp"];

/// Opens the main window and runs until it is closed. `files` are boards or images to open.
pub fn run(files: Vec<PathBuf>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("OpenRef")
            .with_title("OpenRef")
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")).unwrap_or_default())
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([240.0, 160.0])
            .with_drag_and_drop(true)
            // Made see-through-capable from the start: Windows only allows it when the window is created.
            .with_transparent(true),
        // Ctrl+C / Ctrl+V reach the board as keys: it reads images and files, not only text.
        clipboard_shortcuts: false,
        // OpenGL: on Windows it is the renderer whose window can really be see-through (with wgpu
        // the transparent parts were blended with white, measured on 2026-11-13).
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native("OpenRef", options, Box::new(move |cc| Ok(Box::new(OpenRefApp::new(cc, files)))))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    New,
    Open,
    Save,
    SaveAs,
    ImportImages,
    Paste,
    Copy,
    Undo,
    Redo,
    SelectAll,
    SelectNone,
    Delete,
    Duplicate,
    FlipHorizontal,
    FlipVertical,
    ResetTransform,
    Grayscale,
    BringToFront,
    SendToBack,
    Pack,
    Focus,
    FocusAll,
    Opacity(u8),
    AlwaysOnTop,
    AddNote,
    EditNote,
    Crop,
    ResetCrop,
    Frameless,
    ClickThrough,
    TransparentBackground,
    /// How much of the window shows, in percent.
    SeeThrough(u8),
    /// More (true) or less see-through, by steps.
    Fade(bool),
    PickColor,
    /// A note's letter size, in world units.
    NoteSize(u8),
    /// One of [`NOTE_STYLES`].
    NoteStyle(u8),
    ArrangeByName,
    Align(Align),
    /// Horizontally (true) or vertically.
    Distribute(bool),
    MatchSize(MatchSize),
    ExportImages,
    ExportBoard,
    SetLanguage(Lang),
}

/// Note looks: name, text colour, background.
const NOTE_STYLES: &[(&str, Color32, Color32)] = &[
    ("Dark", crate::scene::NOTE_COLOR, crate::scene::NOTE_FILL),
    ("Light", Color32::from_gray(25), Color32::from_rgb(240, 240, 240)),
    ("Yellow", Color32::from_gray(25), Color32::from_rgb(255, 222, 89)),
    ("No background", crate::scene::NOTE_COLOR, Color32::TRANSPARENT),
];

const SEE_THROUGH_STEPS: &[u8] = &[100, 85, 70, 55, 40, 25];

/// Every keyboard shortcut. Longer ones first, so Ctrl+Shift+Z is not read as Ctrl+Z.
const SHORTCUTS: &[(Modifiers, Key, Action)] = &[
    (Modifiers::CTRL.plus(Modifiers::ALT), Key::T, Action::ClickThrough),
    (Modifiers::CTRL.plus(Modifiers::ALT), Key::B, Action::TransparentBackground),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::F, Action::Frameless),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::E, Action::ExportBoard),
    (Modifiers::CTRL.plus(Modifiers::ALT), Key::N, Action::ArrangeByName),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::ArrowDown, Action::Fade(true)),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::ArrowUp, Action::Fade(false)),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::S, Action::SaveAs),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Z, Action::Redo),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::T, Action::ResetTransform),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::A, Action::AlwaysOnTop),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::N, Action::New),
    (Modifiers::CTRL.plus(Modifiers::SHIFT), Key::C, Action::ResetCrop),
    (Modifiers::ALT.plus(Modifiers::SHIFT), Key::H, Action::FlipHorizontal),
    (Modifiers::ALT.plus(Modifiers::SHIFT), Key::V, Action::FlipVertical),
    (Modifiers::CTRL, Key::N, Action::AddNote),
    (Modifiers::CTRL, Key::E, Action::ExportImages),
    (Modifiers::CTRL, Key::O, Action::Open),
    (Modifiers::CTRL, Key::S, Action::Save),
    (Modifiers::CTRL, Key::I, Action::ImportImages),
    (Modifiers::CTRL, Key::V, Action::Paste),
    (Modifiers::CTRL, Key::C, Action::Copy),
    (Modifiers::CTRL, Key::Z, Action::Undo),
    (Modifiers::CTRL, Key::Y, Action::Redo),
    (Modifiers::CTRL, Key::A, Action::SelectAll),
    (Modifiers::CTRL, Key::D, Action::Duplicate),
    (Modifiers::CTRL, Key::P, Action::Pack),
    (Modifiers::CTRL, Key::PageUp, Action::BringToFront),
    (Modifiers::CTRL, Key::PageDown, Action::SendToBack),
    (Modifiers::CTRL, Key::Space, Action::FocusAll),
    (Modifiers::ALT, Key::G, Action::Grayscale),
    (Modifiers::NONE, Key::Delete, Action::Delete),
    (Modifiers::NONE, Key::Backspace, Action::Delete),
    (Modifiers::NONE, Key::Escape, Action::SelectNone),
    (Modifiers::NONE, Key::Space, Action::Focus),
    (Modifiers::NONE, Key::Enter, Action::EditNote),
    (Modifiers::NONE, Key::C, Action::Crop),
];

fn shortcut_text(lang: Lang, action: Action) -> String {
    let t = |en: &'static str| tr(lang, en);
    SHORTCUTS
        .iter()
        .find(|(_, _, a)| *a == action)
        .map(|(m, k, _)| {
            let mut parts = Vec::new();
            if m.ctrl || m.command {
                parts.push("Ctrl");
            }
            if m.alt {
                parts.push("Alt");
            }
            if m.shift {
                parts.push(tr(lang, "Shift"));
            }
            let key = match k {
                Key::Delete => t("Del"),
                Key::Space => t("Space"),
                Key::Enter => t("Enter"),
                Key::Escape => t("Escape"),
                Key::Backspace => t("Backspace"),
                Key::ArrowUp => t("Up"),
                Key::ArrowDown => t("Down"),
                Key::PageUp => t("PgUp"),
                Key::PageDown => t("PgDn"),
                _ => k.name(),
            };
            parts.push(key);
            parts.join("+")
        })
        .unwrap_or_default()
}

/// What to do once the user has answered about unsaved changes.
#[derive(Clone, Debug)]
enum Then {
    New,
    Open(Option<PathBuf>),
    Close,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    always_on_top: bool,
    /// The folder of the last board or image opened or saved.
    folder: Option<PathBuf>,
    frameless: bool,
    /// How much of the window shows (1 = opaque).
    see_through: f32,
    transparent_background: bool,
    /// English unless the user chose Spanish (not taken from Windows).
    lang: Lang,
}

impl Default for Settings {
    fn default() -> Self {
        Self { always_on_top: false, folder: None, frameless: false, see_through: 1.0, transparent_background: false, lang: Lang::En }
    }
}

pub struct OpenRefApp {
    editor: Editor,
    path: Option<PathBuf>,
    settings: Settings,
    /// A short message at the bottom of the board, and when it was shown.
    notice: Option<(String, f64)>,
    asking: Option<Then>,
    /// Decided (answered, or nothing to lose): done at the start of the next frame.
    pending: Option<Then>,
    /// Closing after the user said so (the next close request goes through).
    closing: bool,
    title: String,
    clipboard: Option<arboard::Clipboard>,
    /// Files given on the command line, opened on the first frame.
    startup: Vec<PathBuf>,
    /// The colour picker was started from the menu (it also works while S is held).
    picking: bool,
    /// While the window lets the mouse through: the system-wide shortcut that ends it.
    click_through: Option<platform::GlobalShortcut>,
    autosave: recovery::Autosave,
    /// A copy left by an OpenRef that did not close, to offer back.
    offer: Option<recovery::Orphan>,
    /// A board picture being made on another thread: where it goes, and the answer.
    exporting: Option<(PathBuf, std::sync::mpsc::Receiver<Result<(), String>>)>,
}

impl OpenRefApp {
    fn new(cc: &eframe::CreationContext<'_>, startup: Vec<PathBuf>) -> Self {
        let max_side = cc
            .gl
            .as_ref()
            .map(|gl| {
                use eframe::glow::HasContext;
                // The one `unsafe` outside the Windows calls: OpenGL has no safe way to ask for this number.
                // SAFETY: a plain query on the context eframe just made current.
                #[allow(unsafe_code)]
                let side = unsafe { gl.get_parameter_i32(eframe::glow::MAX_TEXTURE_SIZE) };
                side.max(1024) as u32
            })
            .unwrap_or(8192)
            .min(8192);
        let settings: Settings = cc.storage.and_then(|s| eframe::get_value(s, "settings")).unwrap_or_default();
        if settings.always_on_top {
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        }
        if settings.frameless {
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        }
        let mut editor = Editor::new(ImageStore::new(cc.egui_ctx.clone(), max_side));
        editor.see_through = settings.see_through.clamp(0.1, 1.0);
        editor.transparent_background = settings.transparent_background;
        editor.lang = settings.lang;
        Self {
            editor,
            path: None,
            settings,
            notice: None,
            asking: None,
            pending: None,
            closing: false,
            title: String::new(),
            clipboard: arboard::Clipboard::new().ok(),
            startup,
            picking: false,
            click_through: None,
            autosave: recovery::Autosave::new(recovery::folder()),
            offer: recovery::folder().and_then(|dir| recovery::find_orphan(&dir, platform::process_alive)),
            exporting: None,
        }
    }

    fn set_see_through(&mut self, ctx: &egui::Context, value: f32) {
        let value = value.clamp(0.1, 1.0);
        self.settings.see_through = value;
        self.editor.see_through = value;
        self.say_with(ctx, "Window {percent} % visible", &[("percent", &format!("{:.0}", value * 100.0))]);
    }

    fn set_click_through(&mut self, ctx: &egui::Context, on: bool) {
        if on == self.click_through.is_some() {
            return;
        }
        if on {
            match platform::GlobalShortcut::ctrl_alt('T', ctx.clone()) {
                Ok(shortcut) => {
                    self.click_through = Some(shortcut);
                    ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
                    self.say(ctx, "The mouse now goes through OpenRef. Ctrl+Alt+T brings it back.");
                }
                Err(error) => self.say_with(ctx, "Click-through is not available: {error}", &[("error", &error)]),
            }
        } else {
            self.click_through = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.say(ctx, "OpenRef takes the mouse again");
        }
    }

    /// Writes the selected images (or all) into a folder, each as its own file.
    fn export_images(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let lang = self.settings.lang;
        let targets = self.editor.targets();
        let items: Vec<_> = self.editor.scene.items.iter().filter(|i| targets.contains(&i.id) && i.is_image()).cloned().collect();
        if items.is_empty() {
            self.say(ctx, "There are no images to export");
            return;
        }
        let mut dialog = rfd::FileDialog::new().set_title(tr(lang, "Export images to…")).set_parent(frame);
        if let Some(folder) = self.dialog_folder() {
            dialog = dialog.set_directory(folder);
        }
        let Some(dir) = dialog.pick_folder() else { return };
        self.remember_folder(&dir);
        let (mut written, mut failed) = (0, Vec::new());
        for item in &items {
            let Some(source) = item.image_key().and_then(|k| self.editor.images.source(k)) else { continue };
            let result = export::image_file(item, source).and_then(|(bytes, ext)| {
                let path = export::unique_path(&dir, &source.name, &ext);
                std::fs::write(&path, bytes).map_err(|e| e.to_string())
            });
            match result {
                Ok(()) => written += 1,
                Err(error) => failed.push(format!("{}: {error}", source.name)),
            }
        }
        match failed.first() {
            None if written == 1 => self.say(ctx, "1 image exported"),
            None => self.say_with(ctx, "{count} images exported", &[("count", &written)]),
            Some(first) => self.say_with(ctx, "{count} exported; could not export {what}", &[("count", &written), ("what", first)]),
        }
    }

    /// Saves the selection (or the whole board) as one PNG, at the full size of its images.
    fn export_board(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let lang = self.settings.lang;
        if self.exporting.is_some() {
            self.say(ctx, "A picture is still being made");
            return;
        }
        let targets = self.editor.targets();
        let items: Vec<_> = self.editor.scene.items.iter().filter(|i| targets.contains(&i.id)).cloned().collect();
        let Some(bounds) = items.iter().map(|i| i.bounds()).reduce(|a, b| a.union(b)) else {
            self.say(ctx, "The board is empty");
            return;
        };
        // As sharp as the most shrunk image needs, within the size limit.
        let wanted = items.iter().filter(|i| i.is_image()).map(|i| 1.0 / i.scale.max(1e-6)).fold(1.0f32, f32::max);
        let limit = export::MAX_BOARD_SIDE as f32 / bounds.width().max(bounds.height()).max(1.0);
        let scale = wanted.min(limit);
        let mut dialog = rfd::FileDialog::new()
            .set_title(tr(lang, "Export board as PNG"))
            .add_filter(tr(lang, "PNG image"), &["png"])
            .set_file_name(self.board_name() + ".png")
            .set_parent(frame);
        if let Some(folder) = self.dialog_folder() {
            dialog = dialog.set_directory(folder);
        }
        let Some(mut path) = dialog.save_file() else { return };
        if path.extension().is_none() {
            path.set_extension("png");
        }
        self.remember_folder(&path);
        let background = if self.settings.transparent_background { Color32::TRANSPARENT } else { BACKGROUND };
        let sources = self.editor.images.sources();
        let Some(job) = export::prepare_board(ctx, &items, &sources, scale, background) else { return };
        let (w, h) = (job.width, job.height);
        let (tx, rx) = std::sync::mpsc::channel();
        let target = path.clone();
        let repaint = ctx.clone();
        std::thread::spawn(move || {
            let result = export::render_board(job).and_then(|png| std::fs::write(&target, png).map_err(|e| e.to_string()));
            let _ = tx.send(result);
            repaint.request_repaint();
        });
        self.exporting = Some((path, rx));
        self.say_with(ctx, "Making a {width} × {height} picture…", &[("width", &w), ("height", &h)]);
    }

    /// Changes the selected notes' letter size and/or colours, keeping their top-left corners.
    fn style_notes(&mut self, ctx: &egui::Context, font: Option<f32>, colors: Option<(Color32, Color32)>) {
        let targets = self.editor.targets();
        let changes: Vec<(u64, String, f32, Vec2)> = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| targets.contains(&i.id))
            .filter_map(|i| match &i.kind {
                ItemKind::Note { text, font_size, .. } => {
                    let size = font.unwrap_or(*font_size);
                    Some((i.id, text.clone(), size, crate::canvas::measure_note(ctx, text, size)))
                }
                ItemKind::Image { .. } => None,
            })
            .collect();
        if changes.is_empty() {
            self.say(ctx, "Select notes to change them");
            return;
        }
        self.editor.change(|scene, _| {
            for (id, text, size, box_size) in changes {
                scene.set_note_style(id, size, colors);
                scene.set_note_text(id, text, box_size);
            }
        });
    }

    /// A notice in the chosen language.
    fn say(&mut self, ctx: &egui::Context, en: &'static str) {
        let text = tr(self.settings.lang, en);
        self.notify(ctx, text);
    }

    /// A notice from a template with fields, in the chosen language.
    fn say_with(&mut self, ctx: &egui::Context, en: &'static str, fields: &[(&str, &dyn std::fmt::Display)]) {
        let text = tf(self.settings.lang, en, fields);
        self.notify(ctx, text);
    }

    fn notify(&mut self, ctx: &egui::Context, text: impl Into<String>) {
        self.notice = Some((text.into(), ctx.input(|i| i.time)));
    }

    fn remember_folder(&mut self, path: &Path) {
        let folder = if path.is_dir() { Some(path.to_path_buf()) } else { path.parent().map(Path::to_path_buf) };
        if folder.is_some() {
            self.settings.folder = folder;
        }
    }

    fn dialog_folder(&self) -> Option<PathBuf> {
        self.settings.folder.clone().filter(|f| f.is_dir())
    }

    fn run_action(&mut self, ctx: &egui::Context, frame: &eframe::Frame, action: Action) {
        let lang = self.settings.lang;
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let editor = &mut self.editor;
        match action {
            Action::New => self.ask_then(Then::New),
            Action::Open => self.ask_then(Then::Open(None)),
            Action::Save => {
                self.save(ctx, frame, false);
            }
            Action::SaveAs => {
                self.save(ctx, frame, true);
            }
            Action::ImportImages => {
                let mut dialog =
                    rfd::FileDialog::new().set_title(tr(lang, "Add images")).add_filter(tr(lang, "Images"), IMAGE_EXTENSIONS).set_parent(frame);
                if let Some(folder) = self.dialog_folder() {
                    dialog = dialog.set_directory(folder);
                }
                if let Some(files) = dialog.pick_files() {
                    if let Some(first) = files.first() {
                        self.remember_folder(first);
                    }
                    let at = self.editor.drop_point(None);
                    self.import_paths(ctx, &files, at);
                }
            }
            Action::Paste => self.paste(ctx, pointer),
            Action::Copy => self.copy(ctx),
            Action::Undo => editor.undo(),
            Action::Redo => editor.redo(),
            Action::SelectAll => editor.selection = editor.scene.ids(),
            Action::SelectNone => editor.selection.clear(),
            Action::Delete => editor.change(|scene, selection| scene.remove(selection)),
            Action::Duplicate => {
                let mut copies = Default::default();
                let offset = egui::vec2(20.0, 20.0) / editor.camera.zoom;
                editor.change(|scene, selection| copies = scene.duplicate(selection, offset));
                editor.selection = copies;
            }
            Action::FlipHorizontal | Action::FlipVertical => {
                let targets = editor.targets();
                if let Some(bounds) = editor.scene.bounds_of(&targets) {
                    let horizontal = action == Action::FlipHorizontal;
                    editor.change(|scene, _| scene.flip_about(&targets, bounds.center(), horizontal));
                }
            }
            Action::ResetTransform => {
                let targets = editor.targets();
                editor.change(|scene, _| scene.reset_transform(&targets));
            }
            Action::Grayscale => {
                let targets = editor.targets();
                // Turn it on unless every target already has it.
                let all_gray = editor
                    .scene
                    .items
                    .iter()
                    .filter(|i| targets.contains(&i.id))
                    .all(|i| !matches!(i.kind, ItemKind::Image { grayscale: false, .. }));
                editor.change(|scene, _| scene.set_grayscale(&targets, !all_gray));
            }
            Action::AddNote => {
                let at = editor.drop_point(pointer);
                editor.add_note(ctx, String::new(), at);
            }
            Action::EditNote => editor.edit_selected_note(),
            Action::Crop => {
                if editor.single().is_some_and(|i| i.is_image()) {
                    editor.start_crop();
                } else {
                    self.say(ctx, "Select one image to crop");
                }
            }
            Action::ResetCrop => {
                let targets = editor.targets();
                let sizes: std::collections::HashMap<_, _> =
                    editor.scene.items.iter().filter_map(|i| i.image_key()).filter_map(|k| Some((k, editor.image_size(k)?))).collect();
                editor.change(|scene, _| scene.reset_crop(&targets, |k| sizes.get(&k).copied()));
            }
            Action::Frameless => {
                self.settings.frameless = !self.settings.frameless;
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(!self.settings.frameless));
                if self.settings.frameless {
                    self.say(ctx, "No frame: move the window from the bar at the top");
                }
            }
            Action::ClickThrough => {
                let on = self.click_through.is_none();
                self.set_click_through(ctx, on);
            }
            Action::TransparentBackground => {
                self.settings.transparent_background = !self.settings.transparent_background;
                self.editor.transparent_background = self.settings.transparent_background;
            }
            Action::SeeThrough(percent) => self.set_see_through(ctx, percent as f32 / 100.0),
            Action::Fade(more) => {
                let now = (self.settings.see_through * 100.0).round() as u8;
                let next = if more {
                    SEE_THROUGH_STEPS.iter().copied().find(|s| *s < now).unwrap_or(*SEE_THROUGH_STEPS.last().unwrap())
                } else {
                    SEE_THROUGH_STEPS.iter().rev().copied().find(|s| *s > now).unwrap_or(100)
                };
                self.set_see_through(ctx, next as f32 / 100.0);
            }
            Action::PickColor => {
                self.picking = true;
                self.say(ctx, "Click anywhere on the screen to copy its colour (drag out of the window)");
            }
            Action::NoteSize(size) => self.style_notes(ctx, Some(size as f32), None),
            Action::NoteStyle(n) => {
                let (_, text, fill) = NOTE_STYLES[n as usize % NOTE_STYLES.len()];
                self.style_notes(ctx, None, Some((text, fill)));
            }
            Action::ArrangeByName => {
                let targets = self.editor.targets();
                let mut order: Vec<(String, u64)> = self
                    .editor
                    .scene
                    .items
                    .iter()
                    .filter(|i| targets.contains(&i.id))
                    .map(|i| {
                        let name = match &i.kind {
                            ItemKind::Image { image, .. } => self.editor.images.source(*image).map(|s| s.name.clone()).unwrap_or_default(),
                            ItemKind::Note { text, .. } => text.clone(),
                        };
                        (name.to_lowercase(), i.id)
                    })
                    .collect();
                order.sort();
                let ids: Vec<u64> = order.into_iter().map(|(_, id)| id).collect();
                let editor = &mut self.editor;
                let aspect = editor.viewport.width() / editor.viewport.height().max(1.0);
                let gap = 8.0 / editor.camera.zoom.max(0.01);
                editor.change(|scene, _| scene.lay_out_in_order(&ids, aspect, gap));
                editor.focus(editor.selection.is_empty());
            }
            Action::Align(how) => {
                let targets = editor.targets();
                editor.change(|scene, _| scene.align(&targets, how));
            }
            Action::Distribute(horizontal) => {
                let targets = editor.targets();
                if targets.len() < 3 {
                    self.say(ctx, "Distributing needs three items or more");
                } else {
                    editor.change(|scene, _| scene.distribute(&targets, horizontal));
                }
            }
            Action::MatchSize(how) => {
                let targets = editor.targets();
                editor.change(|scene, _| scene.match_size(&targets, how));
            }
            Action::ExportImages => self.export_images(ctx, frame),
            Action::ExportBoard => self.export_board(ctx, frame),
            Action::SetLanguage(new) => {
                self.settings.lang = new;
                self.editor.lang = new;
                self.title.clear();
            }
            Action::BringToFront => editor.change(|scene, selection| scene.restack(selection, true)),
            Action::SendToBack => editor.change(|scene, selection| scene.restack(selection, false)),
            Action::Pack => {
                let targets = editor.targets();
                let aspect = editor.viewport.width() / editor.viewport.height().max(1.0);
                // The gap is 8 screen points at the zoom the items will be seen at, roughly.
                let gap = 8.0 / editor.camera.zoom.max(0.01);
                editor.change(|scene, _| scene.pack(&targets, aspect, gap));
                editor.focus(editor.selection.is_empty());
            }
            Action::Focus => editor.focus(false),
            Action::FocusAll => editor.focus(true),
            Action::Opacity(percent) => {
                let targets = editor.targets();
                editor.change(|scene, _| scene.set_opacity(&targets, percent as f32 / 100.0));
            }
            Action::AlwaysOnTop => {
                self.settings.always_on_top = !self.settings.always_on_top;
                let level = if self.settings.always_on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                let text = if self.settings.always_on_top { "Always on top" } else { "Not on top" };
                self.say(ctx, text);
            }
        }
    }

    /// Asks about unsaved changes first when there are any.
    fn ask_then(&mut self, then: Then) {
        if self.editor.dirty && !self.editor.scene.is_empty() {
            self.asking = Some(then);
        } else {
            self.asking = None;
            self.pending = Some(then);
        }
    }

    fn import_paths(&mut self, ctx: &egui::Context, paths: &[PathBuf], at: egui::Pos2) {
        let mut sources = Vec::new();
        let mut problems = Vec::new();
        for path in paths {
            if path.is_dir() {
                continue;
            }
            match Source::from_path(path) {
                Ok(source) => sources.push(source),
                Err(error) => problems.push(format!("{}: {error}", path.file_name().unwrap_or_default().to_string_lossy())),
            }
        }
        let count = sources.len();
        self.editor.add_images(sources, at);
        match (count, problems.first()) {
            (_, Some(problem)) if problems.len() == 1 => self.say_with(ctx, "Could not add {what}", &[("what", problem)]),
            (_, Some(_)) => self.say_with(ctx, "Could not add {count} files", &[("count", &problems.len())]),
            (0, None) => {}
            (1, None) => self.say(ctx, "1 image added"),
            (n, None) => self.say_with(ctx, "{count} images added", &[("count", &n)]),
        }
    }

    /// Pastes files copied in Explorer, an image, or the path of an image file.
    fn paste(&mut self, ctx: &egui::Context, pointer: Option<egui::Pos2>) {
        let lang = self.settings.lang;
        let at = self.editor.drop_point(pointer);
        let Some(clipboard) = self.clipboard.as_mut() else {
            self.say(ctx, "The clipboard is not available");
            return;
        };
        if let Ok(files) = clipboard.get().file_list()
            && !files.is_empty()
        {
            let boards: Vec<&PathBuf> = files.iter().filter(|f| is_board(f)).collect();
            if let (Some(board), true) = (boards.first(), files.len() == 1) {
                let board = (*board).clone();
                self.ask_then(Then::Open(Some(board)));
                return;
            }
            self.import_paths(ctx, &files, at);
            return;
        }
        if let Ok(image) = clipboard.get_image() {
            match Source::from_rgba(image.width as u32, image.height as u32, &image.bytes, tr(lang, "Pasted image")) {
                Ok(source) => {
                    self.editor.add_images(vec![source], at);
                    self.say(ctx, "Image pasted");
                }
                Err(error) => self.say_with(ctx, "Could not paste the image: {error}", &[("error", &error)]),
            }
            return;
        }
        if let Ok(text) = clipboard.get_text() {
            let trimmed = text.trim().trim_matches('"');
            let path = PathBuf::from(trimmed);
            if path.is_file() {
                self.import_paths(ctx, &[path], at);
                return;
            }
            if !trimmed.is_empty() {
                // Any other text becomes a note.
                let text = text.trim_end().replace("\r\n", "\n");
                self.editor.add_note(ctx, text, at);
                return;
            }
        }
        self.say(ctx, "There is nothing to paste");
    }

    /// Copies the selected image (as seen: cropped, flipped, grayscale) to the clipboard.
    fn copy(&mut self, ctx: &egui::Context) {
        let editor = &self.editor;
        let mut chosen = editor.scene.items.iter().filter(|i| editor.selection.contains(&i.id));
        let (Some(item), None) = (chosen.next(), chosen.next()) else {
            self.say(ctx, "Select one image or note to copy");
            return;
        };
        let ItemKind::Image { image, crop, grayscale } = &item.kind else {
            if let ItemKind::Note { text, .. } = &item.kind {
                let text = text.clone();
                ctx.copy_text(text);
                self.say(ctx, "Note copied");
            }
            return;
        };
        let Some(source) = editor.images.source(*image) else { return };
        let result = images::decode_full(&source.bytes, *grayscale).map(|full| {
            let r = crop.intersect(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(full.width() as f32, full.height() as f32)));
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
            cut
        });
        let copied = match (result, self.clipboard.as_mut()) {
            (Ok(cut), Some(clipboard)) => clipboard
                .set_image(arboard::ImageData { width: cut.width() as usize, height: cut.height() as usize, bytes: cut.into_raw().into() })
                .map_err(|e| e.to_string()),
            (Ok(_), None) => Err("the clipboard is not available".into()),
            (Err(error), _) => Err(error),
        };
        match copied {
            Ok(()) => self.say(ctx, "Image copied"),
            Err(error) => self.say_with(ctx, "Could not copy: {error}", &[("error", &error)]),
        }
    }

    fn save(&mut self, ctx: &egui::Context, frame: &eframe::Frame, choose: bool) -> bool {
        let lang = self.settings.lang;
        let path = match (&self.path, choose) {
            (Some(path), false) => path.clone(),
            _ => {
                let mut dialog = rfd::FileDialog::new()
                    .set_title(tr(lang, "Save board"))
                    .add_filter(tr(lang, "OpenRef board"), &[EXTENSION])
                    .set_file_name(self.board_name() + "." + EXTENSION)
                    .set_parent(frame);
                if let Some(folder) = self.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).or(self.dialog_folder()) {
                    dialog = dialog.set_directory(folder);
                }
                match dialog.save_file() {
                    Some(path) if path.extension().is_none() => path.with_extension(EXTENSION),
                    Some(path) => path,
                    None => return false,
                }
            }
        };
        let images = &self.editor.images;
        match board_file::save(&path, &self.editor.scene, Some(self.editor.camera.view()), |key| images.source(key).cloned()) {
            Ok(()) => {
                self.remember_folder(&path);
                self.path = Some(path);
                self.editor.dirty = false;
                self.autosave.forget();
                self.say(ctx, "Saved");
                true
            }
            Err(error) => {
                self.say_with(ctx, "Could not save: {error}", &[("error", &error)]);
                false
            }
        }
    }

    fn open(&mut self, ctx: &egui::Context, frame: &eframe::Frame, path: Option<PathBuf>) {
        let lang = self.settings.lang;
        let path = match path {
            Some(path) => path,
            None => {
                let mut dialog =
                    rfd::FileDialog::new().set_title(tr(lang, "Open board")).add_filter(tr(lang, "OpenRef board"), &[EXTENSION]).set_parent(frame);
                if let Some(folder) = self.dialog_folder() {
                    dialog = dialog.set_directory(folder);
                }
                match dialog.pick_file() {
                    Some(path) => path,
                    None => return,
                }
            }
        };
        match board_file::load(&path) {
            Ok(board) => {
                self.editor.replace(board.scene, board.images, board.view);
                self.autosave.forget();
                self.remember_folder(&path);
                self.path = Some(path);
            }
            Err(error) => self.say_with(ctx, "Could not open the board: {error}", &[("error", &error)]),
        }
    }

    fn board_name(&self) -> String {
        let lang = self.settings.lang;
        self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| tr(lang, "Untitled").into())
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let mark = if self.editor.dirty && !self.editor.scene.is_empty() { "*" } else { "" };
        let title = format!("{}{mark} – OpenRef", self.board_name());
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn handle_keys(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if self.asking.is_some() {
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            // Typing in a note: Ctrl+C / X / V come as keys (see `run`), so hand them to the text box.
            let clipboard_text = self.clipboard.as_mut().and_then(|c| c.get_text().ok());
            ctx.input_mut(|input| {
                if input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::C)) {
                    input.events.push(egui::Event::Copy);
                }
                if input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::X)) {
                    input.events.push(egui::Event::Cut);
                }
                if input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::V))
                    && let Some(text) = clipboard_text
                {
                    input.events.push(egui::Event::Paste(text.replace("\r\n", "\n")));
                }
            });
            return;
        }
        if self.editor.is_cropping() {
            let done = ctx.input_mut(|input| {
                [Key::Enter, Key::Escape, Key::C].into_iter().any(|key| input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, key)))
            });
            if done {
                self.editor.finish_crop();
                return;
            }
        }
        let mut actions = Vec::new();
        ctx.input_mut(|input| {
            for (modifiers, key, action) in SHORTCUTS {
                if input.consume_shortcut(&KeyboardShortcut::new(*modifiers, *key)) {
                    actions.push(*action);
                }
            }
        });
        for action in actions {
            self.run_action(ctx, frame, action);
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let at = self.editor.drop_point(pointer);
        let paths: Vec<PathBuf> = dropped.iter().map(|f| f.path().to_path_buf()).collect();
        if let [board] = paths.as_slice()
            && is_board(board)
        {
            self.ask_then(Then::Open(Some(board.clone())));
            return;
        }
        if let Some(first) = paths.first() {
            self.remember_folder(first);
        }
        self.import_paths(ctx, &paths, at);
    }

    fn menu(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame) {
        let lang = self.settings.lang;
        let t = |en: &'static str| tr(lang, en);
        let has_items = !self.editor.scene.is_empty();
        let has_selection = !self.editor.selection.is_empty();
        let ctx = ui.ctx().clone();
        let mut chosen = None;
        let mut item = |ui: &mut egui::Ui, label: &str, action: Action, enabled: bool| {
            let button = egui::Button::new(label).shortcut_text(shortcut_text(lang, action));
            if ui.add_enabled(enabled, button).clicked() {
                chosen = Some(action);
                ui.close();
            }
        };
        let single = self.editor.single().map(|i| i.is_image());
        let has_notes = self.editor.scene.items.iter().any(|i| self.editor.selection.contains(&i.id) && !i.is_image());
        // Short at the top level, the rest in submenus, so the menu fits small windows.
        item(ui, t("Paste"), Action::Paste, true);
        item(ui, t("Copy"), Action::Copy, single.is_some());
        item(ui, t("Add images…"), Action::ImportImages, true);
        item(ui, t("Add note"), Action::AddNote, true);
        ui.separator();
        item(ui, t("Undo"), Action::Undo, self.editor.history.can_undo());
        item(ui, t("Redo"), Action::Redo, self.editor.history.can_redo());
        item(ui, t("Duplicate"), Action::Duplicate, has_selection);
        item(ui, t("Delete"), Action::Delete, has_selection);
        ui.separator();
        ui.menu_button(t("Image"), |ui| {
            item(ui, t("Crop"), Action::Crop, single == Some(true));
            item(ui, t("Reset crop"), Action::ResetCrop, has_items);
            item(ui, t("Flip horizontally"), Action::FlipHorizontal, has_items);
            item(ui, t("Flip vertically"), Action::FlipVertical, has_items);
            item(ui, t("Grayscale"), Action::Grayscale, has_items);
            item(ui, t("Reset transform"), Action::ResetTransform, has_items);
            ui.menu_button(t("Opacity"), |ui| {
                for percent in [100u8, 75, 50, 25] {
                    item(ui, &format!("{percent} %"), Action::Opacity(percent), has_items);
                }
            });
        });
        ui.add_enabled_ui(has_notes, |ui| {
            ui.menu_button(t("Note"), |ui| {
                item(ui, t("Edit note"), Action::EditNote, single == Some(false));
                ui.separator();
                for (n, (name, _, _)) in NOTE_STYLES.iter().enumerate() {
                    item(ui, t(name), Action::NoteStyle(n as u8), true);
                }
                ui.separator();
                for (name, size) in [(t("Small text"), 16u8), (t("Medium text"), 24), (t("Large text"), 40), (t("Huge text"), 72)] {
                    item(ui, name, Action::NoteSize(size), true);
                }
            });
        });
        ui.menu_button(t("Arrange"), |ui| {
            item(ui, t("Select all"), Action::SelectAll, has_items);
            item(ui, t("Pack to window"), Action::Pack, has_items);
            item(ui, t("Bring to front"), Action::BringToFront, has_selection);
            item(ui, t("Send to back"), Action::SendToBack, has_selection);
            ui.separator();
            item(ui, t("Arrange by name"), Action::ArrangeByName, has_items);
            ui.menu_button(t("Align"), |ui| {
                for (label, how) in [
                    (t("Left edges"), Align::Left),
                    (t("Right edges"), Align::Right),
                    (t("Top edges"), Align::Top),
                    (t("Bottom edges"), Align::Bottom),
                    (t("Centres in a column"), Align::CenterX),
                    (t("Centres in a row"), Align::CenterY),
                ] {
                    item(ui, label, Action::Align(how), has_selection);
                }
            });
            ui.menu_button(t("Distribute"), |ui| {
                item(ui, t("Horizontally"), Action::Distribute(true), has_selection);
                item(ui, t("Vertically"), Action::Distribute(false), has_selection);
            });
            ui.menu_button(t("Same size"), |ui| {
                item(ui, t("Same height"), Action::MatchSize(MatchSize::Height), has_items);
                item(ui, t("Same width"), Action::MatchSize(MatchSize::Width), has_items);
                item(ui, t("Same area"), Action::MatchSize(MatchSize::Area), has_items);
            });
            ui.separator();
            item(ui, t("Frame selection"), Action::Focus, has_items);
            item(ui, t("Frame everything"), Action::FocusAll, has_items);
        });
        ui.separator();
        item(ui, t("Pick a colour"), Action::PickColor, true);
        ui.menu_button(t("Board"), |ui| {
            item(ui, t("New board"), Action::New, true);
            item(ui, t("Open board…"), Action::Open, true);
            item(ui, t("Save board"), Action::Save, has_items);
            item(ui, t("Save board as…"), Action::SaveAs, has_items);
            ui.separator();
            item(ui, t("Export images…"), Action::ExportImages, has_items);
            item(ui, t("Export as PNG…"), Action::ExportBoard, has_items);
        });
        ui.menu_button(t("Window"), |ui| {
            // A bullet marks the current choice (the default fonts have no check mark).
            let check = |on: bool, label: &str| if on { format!("• {label}") } else { label.to_string() };
            item(ui, &check(self.settings.always_on_top, t("Always on top")), Action::AlwaysOnTop, true);
            item(ui, &check(self.settings.frameless, t("No frame")), Action::Frameless, true);
            item(ui, &check(self.settings.transparent_background, t("Transparent background")), Action::TransparentBackground, true);
            item(ui, t("Click-through"), Action::ClickThrough, true);
            ui.menu_button(t("Language"), |ui| {
                item(ui, &check(lang == Lang::En, "English"), Action::SetLanguage(Lang::En), true);
                item(ui, &check(lang == Lang::Es, "Español"), Action::SetLanguage(Lang::Es), true);
            });
            ui.menu_button(t("Visible"), |ui| {
                let now = (self.settings.see_through * 100.0).round() as u8;
                for percent in SEE_THROUGH_STEPS {
                    item(ui, &check(now == *percent, &format!("{percent} %")), Action::SeeThrough(*percent), true);
                }
            });
        });
        if let Some(action) = chosen {
            self.run_action(&ctx, frame, action);
        }
    }

    /// Offers back a board that was not saved when OpenRef last stopped.
    fn recovery_dialog(&mut self, ctx: &egui::Context) {
        let lang = self.settings.lang;
        let Some(orphan) = self.offer.clone() else { return };
        if self.asking.is_some() {
            return;
        }
        let name = orphan
            .original
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| format!("“{}”", s.to_string_lossy()))
            .unwrap_or_else(|| tr(lang, "A board").into());
        let mut answer = None;
        egui::Modal::new(egui::Id::unique("recover")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.heading(tr(lang, "Recover the board?"));
            ui.label(tf(lang, "{name} had changes that were not saved when OpenRef last stopped.", &[("name", &name)]));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "Recover")).clicked() {
                    answer = Some(true);
                }
                if ui.button(tr(lang, "Discard")).clicked() {
                    answer = Some(false);
                }
            });
        });
        match answer {
            Some(true) => {
                match board_file::load(&orphan.board) {
                    Ok(board) => {
                        self.editor.replace(board.scene, board.images, board.view);
                        self.editor.dirty = true;
                        self.path = orphan.original.clone();
                        self.say(ctx, "Board recovered: save it to keep it");
                    }
                    Err(error) => self.say_with(ctx, "Could not recover the board: {error}", &[("error", &error)]),
                }
                orphan.discard();
                self.offer = None;
            }
            Some(false) => {
                orphan.discard();
                self.offer = None;
            }
            None => {}
        }
    }

    fn ask_dialog(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let lang = self.settings.lang;
        let Some(then) = self.asking.clone() else { return };
        let mut answer: Option<Option<bool>> = None; // Some(Some(save?)) or Some(None) = cancel
        let response = egui::Modal::new(egui::Id::unique("unsaved")).show(ctx, |ui| {
            ui.set_width(320.0);
            ui.heading(tr(lang, "Save the changes?"));
            ui.label(tf(lang, "“{name}” has changes that are not saved.", &[("name", &self.board_name())]));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "Save")).clicked() {
                    answer = Some(Some(true));
                }
                if ui.button(tr(lang, "Don't save")).clicked() {
                    answer = Some(Some(false));
                }
                if ui.button(tr(lang, "Cancel")).clicked() {
                    answer = Some(None);
                }
            });
        });
        if response.should_close() && answer.is_none() {
            answer = Some(None);
        }
        match answer {
            Some(Some(save)) => {
                self.asking = None;
                if save && !self.save(ctx, frame, false) {
                    return;
                }
                self.pending = Some(then);
            }
            Some(None) => self.asking = None,
            None => {}
        }
    }
}

impl eframe::App for OpenRefApp {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.editor.images.poll();
        self.handle_drops(ctx);

        if self.click_through.as_ref().is_some_and(|s| s.take_pressed()) {
            self.set_click_through(ctx, false);
        }
        // The colour picker: from the menu, or while S is held.
        if self.picking && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.picking = false;
        }
        let holding_s = ctx.input(|i| i.key_down(Key::S) && i.modifiers.is_none()) && !ctx.egui_wants_keyboard_input();
        self.editor.locked = (self.picking || holding_s) && self.asking.is_none();

        self.handle_keys(ctx, frame);

        if let Some(then) = self.pending.take() {
            match then {
                Then::New => {
                    self.editor.replace(Scene::default(), Vec::new(), Some(crate::canvas::Camera::default().view()));
                    self.path = None;
                }
                Then::Open(path) => self.open(ctx, frame, path),
                Then::Close => {
                    self.closing = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.closing && self.editor.dirty && !self.editor.scene.is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.asking = Some(Then::Close);
        }
        self.update_title(ctx);

        // A finished board picture.
        if let Some((path, rx)) = &self.exporting
            && let Ok(result) = rx.try_recv()
        {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match result {
                Ok(()) => self.say_with(ctx, "Exported {name}", &[("name", &name)]),
                Err(error) => self.say_with(ctx, "Could not export {name}: {error}", &[("name", &name), ("error", &error)]),
            }
            self.exporting = None;
        }
        // Autosave now and then while there are unsaved changes.
        let now = ctx.input(|i| i.time);
        let view = self.editor.camera.view();
        let images = &self.editor.images;
        self.autosave.tick(now, self.editor.dirty, &self.editor.scene, view, || images.sources(), self.path.clone());
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let response = self.editor.show(ui);
        if !self.editor.locked {
            response.context_menu(|ui| self.menu(ui, frame));
        }
        if self.settings.frameless {
            let title = self.title.clone();
            crate::window::frameless_chrome(ui, &title);
        }
        if self.click_through.is_some() {
            crate::window::click_through_badge(ui, self.settings.lang);
        }
        if self.editor.locked
            && let Some(picked) = crate::window::color_picker(ui)
            && ui.input(|i| i.pointer.primary_released())
        {
            let hex = platform::describe(picked.rgb).hex;
            ui.ctx().copy_text(hex.clone());
            let ctx = ui.ctx().clone();
            self.say_with(&ctx, "{hex} copied", &[("hex", &hex)]);
            self.picking = false;
        }

        // Files from the command line, once the board area is known.
        if !self.startup.is_empty() {
            let files = std::mem::take(&mut self.startup);
            let ctx = ui.ctx().clone();
            match files.as_slice() {
                [board] if is_board(board) => self.open(&ctx, frame, Some(board.clone())),
                _ => {
                    let at = self.editor.drop_point(None);
                    self.import_paths(&ctx, &files, at);
                }
            }
        }

        if let Some((text, shown)) = &self.notice {
            let age = ui.input(|i| i.time) - shown;
            if age < 3.0 {
                let alpha = (1.0 - ((age - 2.5) / 0.5).clamp(0.0, 1.0)) as f32;
                let painter = ui.painter();
                let galley = painter.layout_no_wrap(text.clone(), egui::FontId::proportional(14.0), egui::Color32::WHITE.gamma_multiply(alpha));
                let at = ui.max_rect().left_bottom() + egui::vec2(14.0, -14.0 - galley.size().y);
                let back = egui::Rect::from_min_size(at, galley.size()).expand2(egui::vec2(10.0, 6.0));
                painter.rect_filled(back, 6.0, egui::Color32::from_black_alpha((200.0 * alpha) as u8));
                painter.galley(at, galley, egui::Color32::WHITE);
                ui.ctx().request_repaint();
            } else {
                self.notice = None;
            }
        }

        let ctx = ui.ctx().clone();
        self.ask_dialog(&ctx, frame);
        self.recovery_dialog(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Closed on purpose (saved, or the user said not to save): nothing to recover.
        self.autosave.forget();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "settings", &self.settings);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // The board paints its own background, so the window itself can be see-through.
        let _ = BACKGROUND;
        [0.0; 4]
    }
}

fn is_board(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}
